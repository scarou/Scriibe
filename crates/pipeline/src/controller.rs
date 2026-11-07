use crate::executor::Executor;
use chrono::{DateTime, Utc};
use scriibe_core::cleaner;
use scriibe_core::dictionary;
use scriibe_core::history::{Cleanup, Dictation, DictationTiming};
use scriibe_core::hotkey::{HotkeyEvent, HotkeyMonitor, Key};
use scriibe_core::microphone::MicLevel;
use scriibe_core::settings::{AppSettings, TranscriptionEngine};
use scriibe_core::text::{join_spelled_code, known_hallucination, word_count};
use scriibe_core::translation::TranslationLanguage;
use scriibe_core::typography::french_typography;
use scriibe_engines::http::{HttpRequest, Method};
use scriibe_engines::runtime::{Clock, Log};
use scriibe_engines::services::{Cleaner, Transcriber};
use scriibe_engines::whisper;
use scriibe_platform::apps::{AppInfo, ForegroundApp};
use scriibe_platform::audio::Recorder;
use scriibe_platform::indicator::{Indicator, IndicatorState};
use scriibe_platform::keys::KeyEvent;
use scriibe_platform::media::MediaControl;
use scriibe_platform::paste::{PasteOutcome, Paster, SelectedRead, SelectedText, Unreadable};
use scriibe_platform::sounds::Sounds;
use scriibe_platform::storage::{ConfigSource, HistorySink};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

/// Le raccourci de collage de l'ordinateur, tel qu'on le lit dans un message de la pastille.
const PASTE_SHORTCUT: &str = if cfg!(target_os = "macos") { "⌘V" } else { "Ctrl+V" };
/// Une dictée plus courte est ignorée (effleurement de la touche).
const MIN_DURATION: f64 = 0.3;
/// Après l'ajout de la touche de traduction, on attend avant de charger le modèle de traduction : Ctrl+Maj+lettre est un raccourci,
/// qui annule la dictée avant qu'un modèle soit chargé pour rien.
const TRANSLATION_WARM_UP_DELAY: Duration = Duration::from_millis(400);
/// Au bout de ce temps sans son assez fort, l'indicateur dit « Je n'entends rien… ».
const MIC_WATCH_AFTER: Duration = Duration::from_secs(2);

/// Là où une commande vocale agit hors du collage (l'interface de Scriibe). Rien n'y est écrit sur disque ni journalisé.
pub trait CommandSink: Send + Sync {
    /// « Ajouter au dictionnaire » : ouvre Scriibe sur l'onglet Dictionnaire, « Entendu » prérempli avec le texte, le curseur dans « Écrit ».
    /// Le texte est tenu en mémoire jusqu'à ce que la page le prenne (une seule fois). L'erreur est un texte fixe, jamais le texte lu.
    fn add_to_dictionary(&self, heard: SelectedText) -> Result<(), &'static str>;
}

/// Une sélection plus longue ne se met pas dans le dictionnaire (un mot ou une expression, sur une ligne).
const DICTIONARY_MAX_CHARS: usize = 80;

/// Ce que devient une transcription qui ressemble à une commande.
enum CommandOutcome {
    /// La commande est faite (ou refusée sans rien coller) : la dictée s'arrête là.
    Handled,
    /// Ce n'est pas une commande : la phrase suit le chemin d'une dictée ordinaire, avec peut-être une remarque pour la pastille.
    Text(Option<String>),
}

/// Tout ce que le chef d'orchestre utilise.
pub struct Deps {
    pub recorder: Arc<dyn Recorder>,
    pub paster: Arc<dyn Paster>,
    pub media: Arc<dyn MediaControl>,
    pub apps: Arc<dyn ForegroundApp>,
    pub sounds: Arc<dyn Sounds>,
    pub indicator: Arc<dyn Indicator>,
    pub config: Arc<dyn ConfigSource>,
    pub history: Arc<dyn HistorySink>,
    pub transcriber: Arc<Transcriber>,
    pub cleaner: Arc<Cleaner>,
    /// Pour les durées (le temps ne passe que quand on dort, dans les tests).
    pub clock: Arc<dyn Clock>,
    /// L'heure civile, pour dater une dictée dans l'historique.
    pub wall_clock: Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>,
    pub log: Arc<dyn Log>,
    pub executor: Arc<dyn Executor>,
    pub commands: Arc<dyn CommandSink>,
}

struct State {
    monitor: HotkeyMonitor,
    recording: bool,
    busy: bool,
    /// Maj tenue pendant cette dictée : le texte est collé traduit dans la langue des réglages.
    translating: bool,
    target: AppInfo,
    started_at: Duration,
    /// Quand charger le modèle de traduction (400 ms après l'ajout de la touche).
    translation_warm_up_due: Option<Duration>,
    input_description: String,
    mic_warned: bool,
    /// Combinaison de touches de modification seules : l'écoute attend `hold_ms` ; `hold_token` change à chaque appui et relâchement.
    holding: bool,
    hold_token: u64,
    /// ⌘ ou Maj ajoutée pendant l'attente : la traduction sera demandée quand l'écoute commencera.
    pending_translate: bool,
    /// Capture d'une combinaison dans les réglages : les touches ne dictent plus, elles sont notées.
    capture: Option<KeyCapture>,
    /// Ce qui rend le préchargement caduc (moteur, modèle, mode…) tel qu'il a été préchargé, et un changement vu depuis quand.
    preload_signature: Option<String>,
    preload_pending: Option<(String, Duration)>,
    preload_checked: Duration,
}

/// Un changement de moteur, de modèle ou de mode doit rester stable ce temps avant de relancer le préchargement (on parcourt une liste).
const PRELOAD_SETTLE: Duration = Duration::from_millis(1500);
const PRELOAD_CHECK_EVERY: Duration = Duration::from_millis(500);

/// Les réglages dont dépendent les modèles chargés et les prompts lus : s'ils changent, ce qui était préchargé ne sert plus.
fn preload_signature(settings: &AppSettings, words: &[String]) -> String {
    format!(
        "{:?}|{}|{}|{}|{:?}|{}|{}|{:?}|{}|{}|{:?}|{:?}|{:?}",
        settings.cleanup_engine,
        settings.active_cleanup_model(),
        settings.lm_studio_url,
        settings.ollama_url,
        settings.transcription_engine,
        settings.whisper_model_path,
        settings.llama_device,
        settings.translation_language,
        settings.cleanup_enabled,
        settings.skip_when_clean,
        settings.active_profile(),
        settings.language,
        words,
    )
}

/// La combinaison que la personne tient pendant qu'elle règle sa touche de dictée.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KeyCapture {
    /// Les touches dans l'ordre où elles ont été appuyées.
    pub keys: Vec<Key>,
    /// Tout a été relâché : la combinaison est complète.
    pub finished: bool,
}

struct Inner {
    deps: Deps,
    state: Mutex<State>,
    /// Événements du clavier reçus depuis le lancement (diagnostic : le journal les compte à la fin d'une capture).
    key_events: std::sync::atomic::AtomicU64,
    /// Quand le crochet du système a envoyé un événement pour la dernière fois (les touches relayées par la page ne servent que s'il se tait).
    last_hook_event: Mutex<Option<Duration>>,
    /// Les touches tenues d'après la page, quand elle relaie le clavier de la fenêtre de Scriibe.
    page_held: Mutex<Vec<Key>>,
    /// Le journal dit une fois que la page relaie les touches.
    page_announced: std::sync::atomic::AtomicBool,
    /// Le dernier collage réussi : sert à mettre une espace devant la dictée suivante (`scriibe_core::spacing`).
    last_paste: Mutex<Option<scriibe_core::spacing::LastPaste>>,
}

/// Un événement du crochet vu depuis moins de ce temps : une touche relayée par la page serait un doublon.
const HOOK_ECHO_WINDOW: Duration = Duration::from_millis(1000);

/// Le chef d'orchestre. Les appels (`handle_key`, `tick`) viennent d'un seul fil ; le traitement d'une dictée s'exécute à part.
#[derive(Clone)]
pub struct Controller {
    inner: Arc<Inner>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

fn seconds(duration: Duration) -> String {
    format!("{:.2} s", duration.as_secs_f64())
}

impl Controller {
    pub fn new(deps: Deps) -> Self {
        let binding = scriibe_platform::keys::binding(&deps.config.settings());
        let indicator = Arc::clone(&deps.indicator);
        deps.recorder.set_level_listener(Some(Arc::new(move |level| indicator.set_level(level))));
        let state = State {
            monitor: HotkeyMonitor::with_binding(binding),
            recording: false,
            busy: false,
            translating: false,
            target: AppInfo::unknown(),
            started_at: Duration::ZERO,
            translation_warm_up_due: None,
            input_description: String::new(),
            mic_warned: false,
            holding: false,
            hold_token: 0,
            pending_translate: false,
            capture: None,
            preload_signature: None,
            preload_pending: None,
            preload_checked: Duration::ZERO,
        };
        scriibe_platform::keys::set_hook_rules(scriibe_platform::keys::hook_rules(&state.monitor.binding));
        Self {
            inner: Arc::new(Inner {
                deps,
                state: Mutex::new(state),
                key_events: std::sync::atomic::AtomicU64::new(0),
                last_hook_event: Mutex::new(None),
                page_held: Mutex::new(Vec::new()),
                page_announced: std::sync::atomic::AtomicBool::new(false),
                last_paste: Mutex::new(None),
            }),
        }
    }

    pub fn is_recording(&self) -> bool {
        lock(&self.inner.state).recording
    }

    pub fn is_busy(&self) -> bool {
        lock(&self.inner.state).busy
    }

    /// Un événement de clavier du système. Le crochet du système ne fait que les poster : cette méthode peut prendre son temps.
    pub fn handle_key(&self, event: KeyEvent) {
        *lock(&self.inner.last_hook_event) = Some(self.inner.deps.clock.now());
        self.inner.key_events.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.dispatch_key(event);
    }

    /// Une touche vue par la page : quand la fenêtre de Scriibe a le focus, le crochet du système ne reçoit rien (S06, cause inconnue),
    /// et la page relaie alors le clavier (`key` : la touche, d'après `KeyboardEvent.code`). Ignorée pendant la capture d'une combinaison (la
    /// capture a son propre relais) et tant que le crochet parle encore (doublon). Rend vrai si la touche a servi.
    pub fn handle_page_key(&self, key: Key, down: bool) -> bool {
        if self.capture().is_some() || self.hook_spoke_recently() {
            return false;
        }
        let event = scriibe_platform::keys::translate_key(&mut lock(&self.inner.page_held), key, down);
        let Some(event) = event else { return false };
        self.announce_page_keys();
        self.dispatch_key(event);
        true
    }

    /// La fenêtre perd le focus avec des touches tenues : on ne verra pas leur relâchement, donc on le fait ici.
    pub fn release_page_keys(&self) {
        let held = std::mem::take(&mut *lock(&self.inner.page_held));
        if held.is_empty() || self.capture().is_some() {
            return;
        }
        self.inner.deps.log.write("touches: la fenêtre de Scriibe a perdu le focus, touches tenues relâchées");
        let mut remaining = held.clone();
        for key in held.into_iter().rev() {
            remaining.retain(|k| *k != key);
            self.dispatch_key(KeyEvent::Changed { changed: key, down: false, held: remaining.clone() });
        }
    }

    fn hook_spoke_recently(&self) -> bool {
        let last = *lock(&self.inner.last_hook_event);
        last.is_some_and(|at| self.inner.deps.clock.now().saturating_sub(at) < HOOK_ECHO_WINDOW)
    }

    fn announce_page_keys(&self) {
        if !self.inner.page_announced.swap(true, std::sync::atomic::Ordering::Relaxed) {
            let events = self.key_event_count();
            self.inner.deps.log.write(&format!(
                "touches: relayées par la page (la fenêtre de Scriibe a le focus ; {events} événements du clavier reçus du système depuis le lancement)"
            ));
        }
    }

    fn dispatch_key(&self, event: KeyEvent) {
        let KeyEvent::Changed { changed, down, held } = event;
        if self.capture_key(changed, down, &held) {
            return;
        }
        let (events, hold_ms) = {
            let mut state = lock(&self.inner.state);
            // Les touches choisies se relisent à chaque événement (les réglages sont en cache : un contrôle de date de fichier).
            let binding = scriibe_platform::keys::binding(&self.inner.deps.config.settings());
            if state.monitor.binding != binding {
                scriibe_platform::keys::set_hook_rules(scriibe_platform::keys::hook_rules(&binding));
                state.monitor = HotkeyMonitor::with_binding(binding);
            }
            (state.monitor.handle_change(changed, down, &held), state.monitor.binding.hold_ms)
        };
        for event in events {
            match event {
                HotkeyEvent::Press if hold_ms == 0 => self.press(),
                HotkeyEvent::Press => self.start_hold(hold_ms),
                HotkeyEvent::Release => {
                    self.end_hold();
                    self.release();
                }
                HotkeyEvent::Command => {
                    let waiting = {
                        let mut state = lock(&self.inner.state);
                        if state.holding {
                            state.pending_translate = true;
                        }
                        state.holding
                    };
                    if !waiting {
                        self.request_translation();
                    }
                }
                HotkeyEvent::OtherKey => {
                    self.end_hold();
                    self.cancel();
                }
            }
        }
    }

    /// Une combinaison de touches de modification seules est tenue : l'écoute ne commence qu'après `hold_ms` ms, sauf si la touche est
    /// relâchée ou si une autre touche est tapée entre-temps (un raccourci clavier n'ouvre pas le micro).
    fn start_hold(&self, hold_ms: u32) {
        let token = {
            let mut state = lock(&self.inner.state);
            state.hold_token += 1;
            state.holding = true;
            state.pending_translate = false;
            state.hold_token
        };
        let this = self.clone();
        let spawned = std::thread::Builder::new().name("scriibe-appui-long".into()).spawn(move || {
            std::thread::sleep(Duration::from_millis(u64::from(hold_ms)));
            let translate = {
                let mut state = lock(&this.inner.state);
                if state.hold_token != token || !state.holding {
                    return;
                }
                state.holding = false;
                std::mem::take(&mut state.pending_translate)
            };
            this.press();
            if translate {
                this.request_translation();
            }
        });
        if spawned.is_err() {
            self.end_hold();
            self.press();
        }
    }

    fn end_hold(&self) {
        let mut state = lock(&self.inner.state);
        state.hold_token += 1;
        state.holding = false;
        state.pending_translate = false;
    }

    /// Commence à noter la combinaison que la personne tient (réglage de la touche de dictée) : tant que la capture dure, les touches ne
    /// dictent pas.
    pub fn start_capture(&self) {
        self.end_hold();
        let mut state = lock(&self.inner.state);
        state.monitor.reset();
        state.capture = Some(KeyCapture::default());
        drop(state);
        // Pendant la capture, Alt gauche et Windows ne doivent rien ouvrir non plus, et aucune touche n'est retenue.
        scriibe_platform::keys::set_hook_rules(scriibe_platform::keys::HookRules { mask_menu: true, ..Default::default() });
        self.inner
            .deps
            .log
            .write(&format!("capture des touches: commencée ({} événements du clavier reçus depuis le lancement)", self.key_event_count()));
    }

    pub fn stop_capture(&self) {
        let was_active = lock(&self.inner.state).capture.take().is_some();
        if was_active {
            let binding = scriibe_platform::keys::binding(&self.inner.deps.config.settings());
            scriibe_platform::keys::set_hook_rules(scriibe_platform::keys::hook_rules(&binding));
            self.inner.deps.log.write(&format!(
                "capture des touches: arrêtée ({} événements du clavier reçus depuis le lancement)",
                self.key_event_count()
            ));
        }
    }

    fn key_event_count(&self) -> u64 {
        self.inner.key_events.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// L'état de la capture : `None` si elle n'est pas en cours.
    pub fn capture(&self) -> Option<KeyCapture> {
        lock(&self.inner.state).capture.clone()
    }

    /// Un événement de clavier venu de la page (le crochet du système ne voit rien quand la fenêtre de Scriibe a le focus) : il ne sert qu'à
    /// la capture, jamais à dicter. Rend faux si aucune capture n'est en cours.
    pub fn feed_capture(&self, event: KeyEvent) -> bool {
        let KeyEvent::Changed { changed, down, held } = event;
        self.capture_key(changed, down, &held)
    }

    /// Note la touche si la capture est en cours (rend vrai : l'événement ne sert alors pas à dicter).
    fn capture_key(&self, changed: Key, down: bool, held: &[Key]) -> bool {
        let mut state = lock(&self.inner.state);
        let Some(capture) = state.capture.as_mut() else { return false };
        if capture.finished {
            return true;
        }
        let first = down && !capture.keys.contains(&changed);
        if first {
            capture.keys.push(changed);
        }
        let finished_now = !down && held.is_empty() && !capture.keys.is_empty();
        if finished_now {
            capture.finished = true;
        }
        let count = capture.keys.len();
        drop(state);
        // Seulement le nombre de touches : le journal ne dit jamais ce qui est tapé.
        if first || finished_now {
            let note = if finished_now { "combinaison complète" } else { "touche notée" };
            let app = self.inner.deps.apps.current().name;
            self.inner
                .deps
                .log
                .write(&format!("capture des touches: {note} ({count} touche(s) dans la combinaison), fenêtre active : {app}"));
        }
        true
    }

    /// Quand le moteur, le modèle, le mode ou le dictionnaire changent (réglages, menu de l'icône, fichier modifié à la main), relance le
    /// préchargement une fois le changement stable : la dictée suivante ne paie pas le chargement du modèle ni la lecture du prompt.
    fn refresh_preload(&self, now: Duration) {
        {
            let mut state = lock(&self.inner.state);
            if state.recording || state.busy || now.saturating_sub(state.preload_checked) < PRELOAD_CHECK_EVERY {
                return;
            }
            state.preload_checked = now;
        }
        let deps = &self.inner.deps;
        let signature = preload_signature(&deps.config.settings(), &deps.config.dictionary().words);
        let reload = {
            let mut state = lock(&self.inner.state);
            if state.preload_signature.is_none() {
                // Le premier relevé est celui du lancement, préchargé par `preload`.
                state.preload_signature = Some(signature);
                return;
            }
            if state.preload_signature.as_deref() == Some(signature.as_str()) {
                state.preload_pending = None;
                return;
            }
            match &state.preload_pending {
                Some((seen, since)) if *seen == signature => {
                    if now.saturating_sub(*since) >= PRELOAD_SETTLE {
                        state.preload_pending = None;
                        state.preload_signature = Some(signature);
                        true
                    } else {
                        false
                    }
                }
                _ => {
                    state.preload_pending = Some((signature, now));
                    false
                }
            }
        };
        if reload {
            deps.log.write("préchargement: réglages changés, modèles rechargés");
            self.preload();
        }
    }

    /// À appeler une à deux fois par seconde : alerte de micro muet pendant l'écoute, préchauffage de la traduction.
    pub fn tick(&self) {
        let deps = &self.inner.deps;
        let now = deps.clock.now();
        self.refresh_preload(now);
        let (warm_translation, watch_mic) = {
            let mut state = lock(&self.inner.state);
            if !state.recording {
                return;
            }
            let warm = state.translation_warm_up_due.filter(|due| now >= *due).is_some();
            if warm {
                state.translation_warm_up_due = None;
            }
            (warm && state.translating, now.saturating_sub(state.started_at) >= MIC_WATCH_AFTER)
        };
        if warm_translation {
            let settings = deps.config.settings();
            let words = deps.config.dictionary().words;
            let language = settings.translation_language;
            let cleaner = Arc::clone(&deps.cleaner);
            deps.executor.spawn("prechauffage-traduction", Box::new(move || cleaner.warm_up(&settings, true, &words, Some(language))));
        }
        if watch_mic {
            let level = MicLevel::assess(deps.recorder.loudest_so_far());
            let warned = level != MicLevel::Ok;
            let mut state = lock(&self.inner.state);
            // Tant que le micro est muet ou faible, puis une dernière fois quand le niveau remonte.
            if warned || state.mic_warned {
                state.mic_warned = warned;
                drop(state);
                deps.indicator.set_mic_level(level);
            }
        }
    }

    fn press(&self) {
        let deps = &self.inner.deps;
        {
            let state = lock(&self.inner.state);
            if state.recording || state.busy {
                return;
            }
        }
        let settings = deps.config.settings();
        let app = deps.apps.current();
        let info = match deps.recorder.start(&settings.microphone_uid) {
            Ok(info) => info,
            Err(message) => {
                deps.log.write(&format!("micro: ouverture impossible ({message})"));
                deps.indicator.flash(IndicatorState::Message(message), 2.5);
                return;
            }
        };
        if let Some(reason) = &info.fallback {
            let chosen = if settings.microphone_name.is_empty() { "sans nom" } else { settings.microphone_name.as_str() };
            deps.log.write(&format!("micro: « {chosen} » {reason}, micro du système utilisé ({})", info.description));
        }
        {
            let mut state = lock(&self.inner.state);
            state.recording = true;
            state.translating = false;
            state.translation_warm_up_due = None;
            state.mic_warned = false;
            state.target = app;
            state.started_at = deps.clock.now();
            state.input_description = info.description;
        }
        deps.indicator.show(IndicatorState::Listening);
        deps.log.write(&format!(
            "dictée: appui — micro ouvert ({}), appli : {}",
            info_description(&deps.recorder),
            lock(&self.inner.state).target.name
        ));
        // Avant le son de début : le média s'arrête, et le micro ne l'entend pas.
        deps.media.pause_if_playing(settings.pause_media);
        deps.sounds.play(&settings.start_sound_name, settings.sound_volume);
        self.warm_up(&settings);
    }

    /// Au lancement du programme : charge les modèles locaux et lit les prompts (mise au propre, puis traduction) sans attendre une
    /// dictée. Sur un processeur sans carte graphique, lire un prompt prend ≈ 30 s : la première dictée les trouve déjà lus.
    /// Les deux prompts se lisent l'un après l'autre (en parallèle, ils se ralentiraient mutuellement).
    pub fn preload(&self) {
        let deps = &self.inner.deps;
        let settings = deps.config.settings();
        self.warm_up_transcription(&settings);
        let words = deps.config.dictionary().words;
        let cleaner = Arc::clone(&deps.cleaner);
        let log = Arc::clone(&deps.log);
        let clock = Arc::clone(&deps.clock);
        deps.executor.spawn(
            "prechargement",
            Box::new(move || {
                let started = clock.now();
                cleaner.warm_up(&settings, false, &words, None);
                if settings.has_cleanup_engine() {
                    cleaner.warm_up(&settings, true, &words, Some(settings.translation_language));
                }
                log.write(&format!(
                    "préchargement: mise au propre et traduction prêtes en {:.1} s",
                    clock.now().saturating_sub(started).as_secs_f64()
                ));
            }),
        );
    }

    /// Pendant que la personne parle : démarre Whisper si besoin, charge le modèle local ou ouvre les connexions cloud.
    fn warm_up(&self, settings: &AppSettings) {
        let deps = &self.inner.deps;
        let words = deps.config.dictionary().words;
        self.warm_up_transcription(settings);
        let cleaner = Arc::clone(&deps.cleaner);
        let for_cleanup = settings.clone();
        deps.executor.spawn("prechauffage-nettoyage", Box::new(move || cleaner.warm_up(&for_cleanup, false, &words, None)));
    }

    fn warm_up_transcription(&self, settings: &AppSettings) {
        let deps = &self.inner.deps;
        let transcriber = Arc::clone(&deps.transcriber);
        let log = Arc::clone(&deps.log);
        let for_transcription = settings.clone();
        deps.executor.spawn(
            "prechauffage-transcription",
            Box::new(move || {
                if for_transcription.transcription_engine == TranscriptionEngine::Whisper {
                    let model = whisper::resolved_model(&for_transcription.whisper_model_path, &(transcriber.detected_whisper_models)());
                    let idle = for_transcription.whisper_idle_minutes;
                    transcriber.whisper.set_idle_limit((idle > 0).then(|| Duration::from_secs(idle as u64 * 60)));
                    if let Err(e) = transcriber.whisper.ensure_running(&model) {
                        log.write(&format!("préchauffage: Whisper pas prêt ({e})"));
                    }
                } else {
                    let mut request = HttpRequest::get("https://api.deepgram.com/v1/listen", Duration::from_secs(5));
                    request.method = Method::Head;
                    let _ = transcriber.http.send(request);
                }
            }),
        );
    }

    /// Maj ajoutée pendant l'écoute : l'indicateur annonce la langue, et le prompt de la traduction (autre consigne, autres exemples
    /// que la mise au propre) est préchauffé, même si le modèle sert déjà d'habitude.
    fn request_translation(&self) {
        let deps = &self.inner.deps;
        let settings = deps.config.settings();
        // Sans moteur de mise au propre, Maj ne traduit pas : la dictée reste en français.
        if !settings.has_cleanup_engine() {
            return;
        }
        {
            let mut state = lock(&self.inner.state);
            if !state.recording || state.translating {
                return;
            }
            state.translating = true;
            state.translation_warm_up_due = Some(deps.clock.now() + TRANSLATION_WARM_UP_DELAY);
        }
        deps.indicator.set_translation(Some(settings.translation_language.name().to_string()));
    }

    /// Une autre touche pendant l'appui : c'est un raccourci, pas une dictée.
    fn cancel(&self) {
        let deps = &self.inner.deps;
        {
            let mut state = lock(&self.inner.state);
            if !state.recording {
                return;
            }
            state.recording = false;
            state.translation_warm_up_due = None;
        }
        deps.log.write("dictée: annulée (une autre touche a été tapée)");
        deps.recorder.stop();
        deps.media.resume_if_paused();
        deps.indicator.show(IndicatorState::Hidden);
    }

    fn release(&self) {
        let deps = &self.inner.deps;
        let (translating, target, input) = {
            let mut state = lock(&self.inner.state);
            if !state.recording {
                return;
            }
            state.recording = false;
            state.translation_warm_up_due = None;
            (state.translating, state.target.clone(), state.input_description.clone())
        };
        let released_at = deps.clock.now();
        let recording = deps.recorder.stop();
        deps.log.write(&format!("dictée: relâchement — {:.1} s de son", recording.duration));
        deps.media.resume_if_paused();
        if recording.duration < MIN_DURATION {
            deps.indicator.show(IndicatorState::Hidden);
            return;
        }
        let settings = deps.config.settings();
        deps.sounds.play(&settings.end_sound_name, settings.sound_volume);
        // Un micro muet (pris par une autre appli, mauvais micro choisi…) ferait inventer une phrase à Whisper.
        if MicLevel::assess(recording.level) == MicLevel::Silent {
            deps.log.write(&format!("dictée: aucun son capté (niveau RMS max {:.4}) — micro : {input}", recording.level));
            let name = input.split(',').next().unwrap_or("").to_string();
            deps.indicator.flash(IndicatorState::Message(format!("Aucun son capté — vérifie le micro ({name})")), 4.0);
            return;
        }
        deps.indicator.show(IndicatorState::Working);
        lock(&self.inner.state).busy = true;
        let this = self.clone();
        deps.executor.spawn(
            "dictee",
            Box::new(move || {
                let job = Job {
                    wav: recording.wav,
                    duration: recording.duration,
                    level: recording.level,
                    gain: recording.gain,
                    released_at,
                    target,
                    input,
                };
                this.process(&job, &settings, translating.then_some(settings.translation_language));
                lock(&this.inner.state).busy = false;
            }),
        );
    }

    /// Traite une prise déjà faite (essais en ligne de commande : un fichier WAV à la place du micro), sans attendre ni passer par
    /// l'exécuteur. Colle, journalise et enregistre comme une vraie dictée : l'appelant branche le collage et l'historique qu'il veut.
    pub fn process_recording(&self, recording: scriibe_platform::audio::Recording, translate: bool, input: &str) {
        let deps = &self.inner.deps;
        let settings = deps.config.settings();
        let job = Job {
            wav: recording.wav,
            duration: recording.duration,
            level: recording.level,
            gain: recording.gain,
            released_at: deps.clock.now(),
            target: deps.apps.current(),
            input: input.to_string(),
        };
        self.process(&job, &settings, translate.then_some(settings.translation_language));
    }

    /// Reconnaît une phrase de commande et, si elle exige une sélection, la lit. Le journal ne reçoit que l'identifiant de la commande et
    /// la longueur de la sélection ; l'historique n'est pas touché ; les messages de la pastille et les erreurs sont à texte fixe.
    fn try_command(&self, raw: &str, settings: &AppSettings) -> CommandOutcome {
        let deps = &self.inner.deps;
        let Some(command) = scriibe_core::commands::recognize_for(raw, settings) else {
            return CommandOutcome::Text(None);
        };
        let selected = if command.needs == scriibe_core::commands::Needs::Selection {
            match deps.paster.selected_text() {
                SelectedRead::Text(text) => Some(text),
                SelectedRead::NoSelection => {
                    deps.log.write(&format!("commande: {} ignorée (aucune sélection) — la phrase est collée comme du texte", command.id));
                    return CommandOutcome::Text(None);
                }
                SelectedRead::Unreadable(why) => {
                    deps.log.write(&format!("commande: {} ignorée ({}) — la phrase est collée comme du texte", command.id, why.reason()));
                    let note = match why {
                        Unreadable::NotTrusted => {
                            "Commande non faite : autorisation Accessibilité manquante — phrase collée comme du texte"
                        }
                        Unreadable::ClipboardNotText => {
                            "Commande non faite : le presse-papiers contient autre chose que du texte — phrase collée comme du texte"
                        }
                        Unreadable::Failed => "Commande non faite : sélection illisible — phrase collée comme du texte",
                    };
                    return CommandOutcome::Text(Some(note.into()));
                }
            }
        } else {
            None
        };
        match command.id.as_str() {
            "dictionnaire.ajouter" => {
                let Some(text) = selected else { return CommandOutcome::Text(None) };
                let length = text.char_count();
                deps.log.write(&format!("commande: {} (sélection : {length} signes)", command.id));
                let shown = text.expose().trim();
                if shown.is_empty() || shown.chars().count() > DICTIONARY_MAX_CHARS || shown.contains('\n') {
                    deps.indicator.flash(
                        IndicatorState::Message(
                            "Sélection trop longue pour le dictionnaire (80 signes au plus, sur une seule ligne)".into(),
                        ),
                        3.5,
                    );
                    return CommandOutcome::Handled;
                }
                let message = match deps.commands.add_to_dictionary(SelectedText::new(shown)) {
                    Ok(()) => "Ajouter au dictionnaire — complète « Écrit »",
                    Err(why) => why,
                };
                deps.indicator.flash(IndicatorState::Message(message.into()), 3.0);
                CommandOutcome::Handled
            }
            other => {
                deps.log.write(&format!("commande: {other} inconnue du chef d'orchestre"));
                CommandOutcome::Text(None)
            }
        }
    }

    /// Les temps comptent depuis le relâchement de la touche.
    fn process(&self, job: &Job, settings: &AppSettings, translate_to: Option<TranslationLanguage>) {
        let deps = &self.inner.deps;
        let dictionary = deps.config.dictionary();
        let elapsed_since = |start: Duration| deps.clock.now().saturating_sub(start);
        let weak = MicLevel::assess(job.level) == MicLevel::Weak;

        let raw = match deps.transcriber.transcribe(&job.wav, settings, &dictionary.words) {
            Ok(text) => text.trim().to_string(),
            Err(e) => {
                deps.log.write(&format!("dictée: transcription en échec ({e})"));
                deps.indicator.flash(IndicatorState::Message(e.to_string()), 3.0);
                return;
            }
        };
        let transcription = elapsed_since(job.released_at);

        let hallucination = (settings.transcription_engine == TranscriptionEngine::Whisper).then(|| known_hallucination(&raw)).flatten();
        if raw.is_empty() || hallucination.is_some() {
            // Le journal ne contient jamais le texte dicté : seulement le motif reconnu (une phrase type de Whisper).
            let why = match &hallucination {
                Some(pattern) => format!("phrase inventée par Whisper : « {pattern} »"),
                None => "transcription vide".to_string(),
            };
            deps.log.write(&format!("dictée: rien d'exploitable ({why}) — niveau {:.3} — micro : {}", job.level, job.input));
            // Un micro faible est la cause la plus probable : on le dit.
            let message = if weak { "Rien entendu — micro faible : rapproche-toi" } else { "Rien entendu" };
            deps.indicator.flash(IndicatorState::Message(message.into()), 2.5);
            return;
        }

        // Commande vocale : la phrase est toute la transcription (avant `dictionary::apply`, qui la réécrirait). La sélection n'est lue qu'ici.
        let command_note = match self.try_command(&raw, settings) {
            CommandOutcome::Handled => return,
            CommandOutcome::Text(note) => note,
        };

        let applied = dictionary::apply(&dictionary, &join_spelled_code(&raw), settings.list_bullet, settings.lists_on());
        let mut text = applied.text;
        let mut corrections = applied.count as i64;
        let mut note: Option<String> = command_note;
        let mut cleanup_time = "sautée".to_string();
        let mut model_time = Duration::ZERO;
        let mut outcome = if settings.uses_cleanup_model() { Cleanup::Short } else { Cleanup::None };

        // Traduction : mise au propre et traduction en un appel, quel que soit le mode ; écartée, on colle le français.
        let mut translated: Option<TranslationLanguage> = None;
        let mut translation_failure: Option<&str> = None;
        if let Some(language) = translate_to {
            let start = deps.clock.now();
            match deps.cleaner.translate(&text, language, settings, &dictionary.words) {
                Ok(out) => {
                    let fixed = dictionary::apply(&dictionary, &out, settings.list_bullet, false);
                    text = fixed.text;
                    corrections += fixed.count as i64;
                    translated = Some(language);
                    outcome = Cleanup::Translated;
                }
                Err(e) => {
                    // Écartée par le garde-fou : la dictée est remise au propre en français. Moteur injoignable ou trop lent :
                    // le rappeler doublerait l'attente, on colle directement le texte tel quel.
                    let rejected = e.message == cleaner::TRANSLATION_REJECTED;
                    translation_failure = Some(if rejected { "écartée" } else { "échouée" });
                    outcome = if rejected { Cleanup::Fallback } else { Cleanup::Failed };
                    note = Some(if rejected {
                        "Traduction écartée — collé en français".to_string()
                    } else {
                        format!("Collé en français, sans traduction ({e})")
                    });
                    deps.indicator.set_translation(None);
                }
            }
            let spent = elapsed_since(start);
            cleanup_time = format!("traduction {}", seconds(spent));
            model_time += spent;
        }

        // En Simple, trois mots ou moins : rien à reformuler, on gagne le temps du modèle (un mode de style, lui, s'applique).
        // Option « sauter si déjà propre » : Whisper a déjà fait le travail.
        let already_clean = cleaner::skips_cleanup(&text, settings, translate_to.is_some());
        if already_clean {
            cleanup_time = "sautée (déjà propre)".into();
            outcome = Cleanup::Skipped;
        }
        if translated.is_some() || translation_failure == Some("échouée") {
            // Déjà mise au propre par la traduction, ou moteur injoignable.
        } else if settings.uses_cleanup_model() && !already_clean && (word_count(&text) > 3 || !settings.active_profile().is_plain_simple())
        {
            let start = deps.clock.now();
            match deps.cleaner.cleanup(&text, settings, &dictionary.words) {
                Ok(cleaned) => {
                    let fixed = dictionary::apply(&dictionary, &cleaned, settings.list_bullet, settings.lists_on());
                    text = fixed.text;
                    corrections += fixed.count as i64;
                    // Après une traduction écartée, la décision reste « Traduction écartée ».
                    if translate_to.is_none() {
                        outcome = Cleanup::Model;
                    }
                }
                Err(e) => {
                    // Le garde-fou a écarté la réponse (texte brut collé), ou le modèle n'a pas répondu.
                    outcome = if e.message == cleaner::REWRITE_REJECTED { Cleanup::Rejected } else { Cleanup::Failed };
                    if note.is_none() {
                        note = Some(format!("Collé sans mise au propre ({e})"));
                    }
                }
            }
            let spent = elapsed_since(start);
            cleanup_time = if translate_to.is_none() { seconds(spent) } else { format!("{cleanup_time}, puis {}", seconds(spent)) };
            model_time += spent;
        }

        if settings.language == scriibe_core::settings::DictationLanguage::Fr && translated.is_none() {
            text = french_typography(&text);
        }
        // Une sélection active sera remplacée par le texte : il s'écrit alors au milieu d'une phrase, sans majuscule ni point final.
        let selection = deps.paster.selection();
        let replaces_selection = selection.active;
        let selection_report = selection.report;
        if replaces_selection {
            text = scriibe_core::inline::inside_a_sentence(&text, &dictionary);
        }
        // Trace du collage : un Ctrl+V envoyé n'est pas un texte reçu, et sans cette ligne un collage perdu en silence ne laisse rien.
        let focus_at_paste = deps.apps.current();
        let paste_start = deps.clock.now();
        // L'espace entre deux dictées n'est ajoutée qu'à ce qui part au collage : l'historique et « copier la dernière » gardent `text`.
        let to_paste = scriibe_core::spacing::for_paste(
            lock(&self.inner.last_paste).as_ref(),
            focus_at_paste.id.as_deref(),
            paste_start,
            &text,
            replaces_selection,
        );
        let pasted = deps.paster.paste(&to_paste, settings.restore_clipboard);
        *lock(&self.inner.last_paste) = match pasted {
            PasteOutcome::Pasted => scriibe_core::spacing::remember(focus_at_paste.id.as_deref(), deps.clock.now(), &text),
            PasteOutcome::CopiedOnly(_) => None,
        };
        deps.log.write(&format!(
            "collage: {} signes envoyés à {} (à l'appui : {}) en {}{}{}",
            to_paste.chars().count(),
            focus_at_paste.name,
            job.target.name,
            seconds(elapsed_since(paste_start)),
            match (replaces_selection, selection_report.as_str()) {
                (true, _) => " — sur une sélection (sans majuscule ni point final)".to_string(),
                (false, "") => String::new(),
                (false, report) => format!(" — sélection : {report}"),
            },
            match &pasted {
                PasteOutcome::Pasted => String::new(),
                PasteOutcome::CopiedOnly(_) => " — copié seulement".to_string(),
            }
        ));
        let total = elapsed_since(job.released_at);
        let timing = DictationTiming {
            transcription: transcription.as_secs_f64(),
            cleanup: model_time.as_secs_f64(),
            total: total.as_secs_f64(),
            outcome,
        };
        deps.log.write(&log_line(
            job,
            settings,
            deps.cleaner.model_label(settings),
            &seconds(transcription),
            &cleanup_time,
            translate_to,
            translation_failure,
            &seconds(total),
            weak,
        ));

        let mut dictation =
            Dictation::new((deps.wall_clock)(), &job.target.name, &raw, &text, word_count(&text) as i64, job.duration, corrections);
        dictation.bundle_id = job.target.id.clone();
        dictation.translation = translated;
        dictation.timing = Some(timing);
        dictation.mode = settings.rewriting_on().then(|| settings.active_mode_label());
        deps.history.record(dictation);

        if let PasteOutcome::CopiedOnly(reason) = pasted {
            deps.log.write(&format!("collage: {reason}"));
            deps.indicator.flash(IndicatorState::Message(format!("Copié, à coller avec {PASTE_SHORTCUT} — {reason}")), 4.0);
        } else if let Some(note) = note {
            deps.indicator.flash(IndicatorState::Message(note), 3.0);
        } else if weak {
            let language = translated.map(|l| format!(" en {}", l.name())).unwrap_or_default();
            deps.indicator
                .flash(IndicatorState::Message(format!("Collé{language} — micro faible : rapproche-toi ou monte le volume d'entrée")), 3.5);
        } else {
            deps.indicator.flash(IndicatorState::Done, 0.9);
        }
    }
}

fn info_description(recorder: &Arc<dyn Recorder>) -> String {
    recorder.input_description()
}

/// Ce qu'une dictée terminée apporte au traitement.
struct Job {
    wav: Vec<u8>,
    duration: f64,
    level: f32,
    /// Amplification appliquée au son (micro faible).
    gain: f32,
    released_at: Duration,
    target: AppInfo,
    input: String,
}

#[allow(clippy::too_many_arguments)]
fn log_line(
    job: &Job,
    settings: &AppSettings,
    model: String,
    transcription: &str,
    cleanup: &str,
    translate_to: Option<TranslationLanguage>,
    translation_failure: Option<&str>,
    total: &str,
    weak: bool,
) -> String {
    let engine = match settings.transcription_engine {
        TranscriptionEngine::Deepgram => "deepgram",
        TranscriptionEngine::Whisper => "whisper",
    };
    let mode = if settings.rewriting_on() {
        let base = match settings.active_custom_mode() {
            Some(_) => "personnel".to_string(),
            None => settings.rewrite_mode.raw().to_string(),
        };
        if settings.active_custom_mode().is_none() && settings.is_style_modified(settings.rewrite_mode) {
            format!("{base} modifié")
        } else {
            base
        }
    } else {
        "off".into()
    };
    let translation = translate_to
        .map(|l| format!(", traduction {}{}", l.name(), translation_failure.map(|f| format!(" {f}")).unwrap_or_default()))
        .unwrap_or_default();
    format!(
        "dictée: parole {:.1} s (niveau {:.3}) | transcription {transcription} ({engine}) | mise au propre {cleanup} ({} {model}, mode {mode}{translation}) | total après relâche {total}{}{} | micro : {}",
        job.duration,
        job.level,
        settings.cleanup_engine.name(),
        if weak { " | micro faible" } else { "" },
        if job.gain > 1.05 { format!(" | gain ×{:.1}", job.gain) } else { String::new() },
        job.input
    )
}
