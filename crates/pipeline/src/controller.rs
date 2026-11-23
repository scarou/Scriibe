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
use scriibe_platform::indicator::{Failure, FailureKind, Indicator, IndicatorState};
use scriibe_platform::keys::KeyEvent;
use scriibe_platform::media::MediaControl;
use scriibe_platform::paste::{PasteOutcome, Paster, SelectedRead, SelectedText, Unreadable};
use scriibe_platform::sounds::{Sounds, ERROR_SOUND};
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
/// Une écoute qui dure autant laisse une ligne au journal (une seule) : état des touches vu par le crochet et par la page, pour comprendre une
/// touche dont le relâchement n'arrive pas (enquête C7, docs/ENQUETE_WINDOWS.md).
const LONG_DICTATION_NOTE_AFTER: Duration = Duration::from_secs(15);

/// Là où une commande vocale agit hors du collage (l'interface de Scriibe). Rien n'y est écrit sur disque ni journalisé.
pub trait CommandSink: Send + Sync {
    /// « Ajouter au dictionnaire » : ouvre Scriibe sur l'onglet Dictionnaire, « Entendu » prérempli avec le texte, le curseur dans « Écrit ».
    /// Le texte est tenu en mémoire jusqu'à ce que la page le prenne (une seule fois). L'erreur est un texte fixe, jamais le texte lu.
    fn add_to_dictionary(&self, heard: SelectedText) -> Result<(), &'static str>;
}

/// Une sélection plus longue ne se met pas dans le dictionnaire (un mot ou une expression, sur une ligne).
const DICTIONARY_MAX_CHARS: usize = 80;

/// Ce que devient la dictée au relâchement de la touche : du texte collé, du texte traduit, ou une commande (touche de commande tenue :
/// rien n'est alors jamais collé comme une dictée).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Dictation,
    Translation(TranslationLanguage),
    Command,
}

impl Intent {
    /// Pour les essais en ligne de commande et le banc : une dictée, ou une traduction dans la langue des réglages.
    pub fn dictation_or_translation(translate: bool, settings: &AppSettings) -> Self {
        if translate {
            Self::Translation(settings.translation_language)
        } else {
            Self::Dictation
        }
    }
}

/// Ce que devient une transcription qui ressemble à une commande.
enum CommandOutcome {
    /// La commande est faite (ou refusée sans rien coller) : la dictée s'arrête là.
    Handled,
    /// Ce n'est pas une commande : la phrase suit le chemin d'une dictée ordinaire, avec peut-être une remarque pour la pastille.
    Text(Option<String>),
    /// Comme `Text`, mais une commande reconnue n'a pas pu se faire : la remarque est une erreur qui reste affichée (chantier 5).
    TextAfterFailure(String),
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
    /// La touche de commande est tenue pendant cette dictée : la transcription est une commande, jamais du texte à coller.
    commanding: bool,
    target: AppInfo,
    started_at: Duration,
    /// La ligne « écoute longue » est déjà écrite pour cette dictée.
    long_noted: bool,
    /// Quand charger le modèle de traduction (400 ms après l'ajout de la touche).
    translation_warm_up_due: Option<Duration>,
    input_description: String,
    mic_warned: bool,
    /// Combinaison de touches de modification seules : l'écoute attend `hold_ms` ; `hold_token` change à chaque appui et relâchement.
    holding: bool,
    hold_token: u64,
    /// ⌘ ou Maj ajoutée pendant l'attente : la traduction sera demandée quand l'écoute commencera.
    pending_translate: bool,
    /// La touche de commande ajoutée pendant l'attente : la commande sera demandée quand l'écoute commencera.
    pending_command: bool,
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
    /// Le dernier texte collé par une dictée : « entre guillemets » le reprend quand aucune sélection n'est lisible.
    last_paste: Mutex<Option<LastPaste>>,
}

/// Le dernier collage d'une dictée, gardé en mémoire seulement (jamais au journal) et écrasé quand il est remplacé.
struct LastPaste {
    text: SelectedText,
    app_id: Option<String>,
    at: Duration,
}

/// Au-delà, le dernier collage est trop ancien pour que « entre guillemets » le reprenne sans sélection.
const LAST_PASTE_VALID: Duration = Duration::from_secs(120);

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

/// Même application au premier plan que celle de l'appui ? Une application inconnue (d'un côté ou de l'autre) ne l'est jamais : on ne peut
/// pas vérifier, donc la reformulation n'est ni lue ni collée.
fn same_app(target: &AppInfo, now: &AppInfo) -> bool {
    target.id.is_some() && now.id == target.id
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
            commanding: false,
            target: AppInfo::unknown(),
            started_at: Duration::ZERO,
            long_noted: false,
            translation_warm_up_due: None,
            input_description: String::new(),
            mic_warned: false,
            holding: false,
            hold_token: 0,
            pending_translate: false,
            pending_command: false,
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
        if self.capture().is_some() {
            return false;
        }
        let event = if self.hook_spoke_recently() {
            // Le crochet parle encore : la touche relayée est en principe un doublon. Une seule exception, le relâchement d'une touche de la
            // dictée en cours : un appui court (tenir, cliquer sur Scriibe, relâcher en moins d'une seconde) a le crochet muet depuis moins
            // d'une seconde, et ce relâchement est le seul qui reste (relecture C7, I1). Un vrai doublon est inoffensif : le second relâchement
            // trouve la dictée déjà finie.
            let held = lock(&self.inner.page_held).clone();
            let ends_dictation = !down && {
                let state = lock(&self.inner.state);
                state.monitor.is_down() && state.monitor.binding.dictation.contains(&key)
            };
            ends_dictation.then(|| KeyEvent::Changed { changed: key, down: false, held: held.into_iter().filter(|k| *k != key).collect() })
        } else {
            let mut held = lock(&self.inner.page_held);
            match scriibe_platform::keys::translate_key(&mut held, key, down) {
                Some(event) => Some(event),
                // Le relâchement d'une touche que la page n'a pas vue s'enfoncer : la dictée a commencé par le crochet avant que la fenêtre
                // de Scriibe prenne le focus, et ce relâchement est le seul qui reste (enquête C7, docs/ENQUETE_WINDOWS.md). Sans lui, la
                // dictée dure jusqu'à l'appui d'une autre touche. Sans dictée en cours, il ne sert à rien : ignoré comme avant.
                None if !down && lock(&self.inner.state).monitor.is_down() => {
                    Some(KeyEvent::Changed { changed: key, down: false, held: held.clone() })
                }
                None => None,
            }
        };
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
        // La touche de commande déjà tenue à l'appui : le préchauffage de la mise au propre, inutile sous cette touche, n'est pas fait.
        let command_first = events.contains(&HotkeyEvent::Press) && events.contains(&HotkeyEvent::CommandKey);
        for event in events {
            match event {
                HotkeyEvent::Press if hold_ms == 0 => self.press(command_first),
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
                HotkeyEvent::CommandKey => {
                    let waiting = {
                        let mut state = lock(&self.inner.state);
                        if state.holding {
                            state.pending_command = true;
                        }
                        state.holding
                    };
                    if !waiting {
                        self.request_command();
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
            state.pending_command = false;
            state.hold_token
        };
        let this = self.clone();
        let spawned = std::thread::Builder::new().name("scriibe-appui-long".into()).spawn(move || {
            std::thread::sleep(Duration::from_millis(u64::from(hold_ms)));
            let (translate, command) = {
                let mut state = lock(&this.inner.state);
                if state.hold_token != token || !state.holding {
                    return;
                }
                state.holding = false;
                (std::mem::take(&mut state.pending_translate), std::mem::take(&mut state.pending_command))
            };
            this.press(command);
            // La commande l'emporte sur la traduction.
            if command {
                this.request_command();
            } else if translate {
                this.request_translation();
            }
        });
        if spawned.is_err() {
            self.end_hold();
            self.press(false);
        }
    }

    fn end_hold(&self) {
        let mut state = lock(&self.inner.state);
        state.hold_token += 1;
        state.holding = false;
        state.pending_translate = false;
        state.pending_command = false;
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
        let (warm_translation, watch_mic, note_long) = {
            let mut state = lock(&self.inner.state);
            if !state.recording {
                return;
            }
            let note_long = !state.long_noted && now.saturating_sub(state.started_at) >= LONG_DICTATION_NOTE_AFTER;
            if note_long {
                state.long_noted = true;
            }
            let warm = state.translation_warm_up_due.filter(|due| now >= *due).is_some();
            if warm {
                state.translation_warm_up_due = None;
            }
            (warm && state.translating, now.saturating_sub(state.started_at) >= MIC_WATCH_AFTER, note_long)
        };
        if note_long {
            self.note_long_dictation(now);
        }
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

    /// Une seule ligne par écoute de plus de 15 s : ce que voient le crochet du système (dernier événement) et la page (touches tenues), et si
    /// la fenêtre de Scriibe est au premier plan. Ni les mots dits ni les touches elles-mêmes.
    fn note_long_dictation(&self, now: Duration) {
        let deps = &self.inner.deps;
        let hook = match *lock(&self.inner.last_hook_event) {
            Some(at) => format!("dernier événement du crochet il y a {:.0} s", now.saturating_sub(at).as_secs_f32()),
            None => "aucun événement du crochet depuis le lancement".to_string(),
        };
        let page_keys = lock(&self.inner.page_held).len();
        deps.log.write(&format!(
            "dictée: écoute depuis plus de 15 s — {hook} ; touches tenues vues par la page : {page_keys} ; {} événements du clavier reçus du système depuis le lancement ; appli au premier plan : {}",
            self.key_event_count(),
            deps.apps.current().name
        ));
    }

    /// `command_first` : la touche de commande était déjà tenue à l'appui, donc la mise au propre ne servira pas : pas de préchauffage.
    fn press(&self, command_first: bool) {
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
                self.fail_dictation(message);
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
            state.commanding = false;
            state.translation_warm_up_due = None;
            state.mic_warned = false;
            state.target = app;
            state.started_at = deps.clock.now();
            state.long_noted = false;
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
        self.warm_up(&settings, !command_first);
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
    fn warm_up(&self, settings: &AppSettings, with_cleanup: bool) {
        let deps = &self.inner.deps;
        self.warm_up_transcription(settings);
        if !with_cleanup {
            return;
        }
        let words = deps.config.dictionary().words;
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
            // Sous la touche de commande, rien ne se traduit : la commande l'emporte.
            if !state.recording || state.translating || state.commanding {
                return;
            }
            state.translating = true;
            state.translation_warm_up_due = Some(deps.clock.now() + TRANSLATION_WARM_UP_DELAY);
        }
        deps.indicator.set_translation(Some(settings.translation_language.name().to_string()));
    }

    /// La touche de commande ajoutée pendant l'écoute : la transcription sera une commande, jamais du texte collé. Elle l'emporte sur la
    /// traduction (indicateur effacé, préchauffage de la traduction annulé). Rien n'est préchauffé pour la mise au propre.
    fn request_command(&self) {
        let deps = &self.inner.deps;
        let was_translating = {
            let mut state = lock(&self.inner.state);
            if !state.recording || state.commanding {
                return;
            }
            state.commanding = true;
            state.translation_warm_up_due = None;
            std::mem::take(&mut state.translating)
        };
        if was_translating {
            deps.indicator.set_translation(None);
        }
    }

    /// Une autre touche pendant l'appui : c'est un raccourci, pas une dictée.
    fn cancel(&self) {
        let deps = &self.inner.deps;
        let started_at;
        {
            let mut state = lock(&self.inner.state);
            if !state.recording {
                return;
            }
            state.recording = false;
            state.translation_warm_up_due = None;
            started_at = state.started_at;
        }
        deps.log.write(&format!(
            "dictée: annulée (une autre touche a été tapée) après {:.1} s d'écoute",
            deps.clock.now().saturating_sub(started_at).as_secs_f32()
        ));
        deps.recorder.stop();
        deps.media.resume_if_paused();
        deps.indicator.show(IndicatorState::Hidden);
    }

    fn release(&self) {
        let deps = &self.inner.deps;
        let (translating, commanding, target, input) = {
            let mut state = lock(&self.inner.state);
            if !state.recording {
                return;
            }
            state.recording = false;
            state.translation_warm_up_due = None;
            (state.translating, state.commanding, state.target.clone(), state.input_description.clone())
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
            self.fail_dictation(format!("Aucun son capté — vérifie le micro ({name})"));
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
                let intent = if commanding {
                    Intent::Command
                } else if translating {
                    Intent::Translation(settings.translation_language)
                } else {
                    Intent::Dictation
                };
                this.process(&job, &settings, intent);
                lock(&this.inner.state).busy = false;
            }),
        );
    }

    /// Traite une prise déjà faite (essais en ligne de commande : un fichier WAV à la place du micro), sans attendre ni passer par
    /// l'exécuteur. Colle, journalise et enregistre comme une vraie dictée : l'appelant branche le collage et l'historique qu'il veut.
    pub fn process_recording(&self, recording: scriibe_platform::audio::Recording, intent: Intent, input: &str) {
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
        self.process(&job, &settings, intent);
    }

    /// Reconnaît une phrase de commande et, si elle exige une sélection, la lit. Le journal ne reçoit que l'identifiant de la commande et
    /// la longueur de la sélection ; l'historique n'est pas touché ; les messages de la pastille et les erreurs sont à texte fixe.
    fn try_command(&self, raw: &str, settings: &AppSettings) -> CommandOutcome {
        let Some(command) = scriibe_core::commands::recognize_for(raw, settings, scriibe_core::commands::Context::NoKey) else {
            return CommandOutcome::Text(None);
        };
        self.run_command(command, settings, scriibe_core::commands::Context::NoKey)
    }

    /// Sous la touche de commande : jamais rien de collé comme du texte. Une phrase qui n'est pas une commande donne un message fixe de la
    /// pastille (jamais les mots dits), et rien d'autre. La transcription n'est ni corrigée, ni mise au propre, ni gardée dans l'historique.
    fn process_command(&self, raw: &str, settings: &AppSettings, target: &AppInfo) {
        use scriibe_core::commands::{Context, Route};
        let deps = &self.inner.deps;
        match scriibe_core::commands::route_under_key(raw, settings) {
            Route::Command(command) => {
                // Le résultat est ignoré : sous la touche, `run_command` ne rend jamais « du texte à coller ».
                let _ = self.run_command(command, settings, Context::UnderKey);
            }
            Route::Close => {
                deps.log.write("commande: phrase proche d'une commande, non reconnue — rien n'est collé");
                self.refuse(FailureKind::NotRecognised, "Commande non reconnue");
            }
            // Ni une commande, ni une phrase proche : avec une sélection, la phrase est la consigne de reformulation (étape 4).
            Route::Instruction => self.rewrite_selection(raw, settings, target),
            Route::TooLong => {
                deps.log.write(&format!(
                    "commande: phrase de plus de {} mots, dictée prise par mégarde — rien n'est collé",
                    scriibe_core::commands::INSTRUCTION_MAX_WORDS
                ));
                self.refuse(FailureKind::NotRecognised, "Commande trop longue");
            }
            Route::Disabled => {
                deps.log.write("commande: phrase d'une commande désactivée — rien n'est fait");
                self.refuse(FailureKind::NotRecognised, "Commande désactivée");
            }
            Route::EmptySearch => {
                deps.log.write("commande: recherche sans rien à chercher — rien n'est ouvert");
                self.refuse(FailureKind::NotDone, "Rien à chercher");
            }
            Route::UnknownSearch => {
                deps.log.write("commande: « nouvelle recherche » avec un site inconnu — rien n'est ouvert");
                self.refuse(FailureKind::NotRecognised, "Site de recherche inconnu");
            }
        }
    }

    /// Reformulation de la sélection par la consigne dite (chantier 6, étape 4 ; `docs/CADRAGE_COMMANDES.md`, décision 2). La sélection
    /// part seulement vers un moteur sur cet ordinateur, ou vers le moteur qui a l'accord (contrôle dans `Cleaner::rewrite_selection`).
    /// L'application au premier plan est comparée à celle de l'appui deux fois : avant de lire la sélection (si elle a changé, rien n'est
    /// lu ni envoyé) et juste avant de coller (si elle a changé, le texte est seulement copié, marqué « ne pas garder »). Une application
    /// inconnue compte comme changée. Toute autre issue ne remplace rien. Journal : longueurs, moteur et durée ; pastille : messages fixes ; rien dans l'historique.
    fn rewrite_selection(&self, instruction: &str, settings: &AppSettings, target: &AppInfo) {
        let deps = &self.inner.deps;
        if !deps.paster.reads_selection() {
            deps.log.write("commande: reformulation impossible, ce système ne lit pas la sélection — rien n'est collé");
            self.refuse(FailureKind::NotDone, "Non disponible sous Linux");
            return;
        }
        // La sélection lue doit être celle de l'application de l'appui : sinon la reformulation d'un texte d'une autre application
        // pourrait être collée dans celle-ci au retour (relecture de la tâche 10).
        if !same_app(target, &deps.apps.current()) {
            deps.log.write("commande: reformulation impossible, l'application au premier plan a changé — rien n'est lu ni collé");
            self.refuse(FailureKind::NotDone, "Reformulation impossible : l'application a changé");
            return;
        }
        let selection = match deps.paster.selected_text() {
            SelectedRead::Text(text) if !text.expose().trim().is_empty() => text,
            SelectedRead::Text(_) | SelectedRead::NoSelection => {
                deps.log.write("commande: aucune commande reconnue et aucune sélection — rien n'est collé");
                self.refuse(FailureKind::NotRecognised, "Commande inconnue");
                return;
            }
            SelectedRead::Unreadable(why) => {
                deps.log.write(&format!("commande: reformulation impossible ({}) — rien n'est collé", why.reason()));
                self.refuse(FailureKind::NotDone, "Sélection illisible");
                return;
            }
        };
        deps.log.write(&format!(
            "commande: reformulation (sélection : {} signes, consigne : {} mots, moteur : {})",
            selection.char_count(),
            word_count(instruction),
            settings.cleanup_engine.name()
        ));
        let started = deps.clock.now();
        let result = deps.cleaner.rewrite_selection(selection.expose(), instruction, settings);
        drop(selection);
        let took = seconds(deps.clock.now().saturating_sub(started));
        use scriibe_engines::services::RewriteError;
        let text = match result {
            // Le texte reformulé contient la sélection : masqué au `Debug`, mémoire écrasée à la fin.
            Ok(text) => SelectedText::new(text),
            Err(error) => {
                let message = match &error {
                    RewriteError::NotAllowed => {
                        "Reformulation impossible : moteur hors de cet ordinateur, sans accord (Commandes vocales → Reformulation de la sélection)".to_string()
                    }
                    RewriteError::TooLong => "Sélection trop longue pour la reformulation (3 000 signes au plus)".to_string(),
                    RewriteError::Unchanged => "Rien à changer".to_string(),
                    RewriteError::Rejected => "Reformulation impossible : réponse du modèle écartée".to_string(),
                    RewriteError::Service(e) => format!("Reformulation impossible — {e}"),
                };
                let why = match &error {
                    RewriteError::NotAllowed => "sans accord".to_string(),
                    RewriteError::TooLong => "sélection trop longue".to_string(),
                    RewriteError::Unchanged => "rien à changer".to_string(),
                    RewriteError::Rejected => "réponse écartée".to_string(),
                    RewriteError::Service(e) => e.to_string(),
                };
                deps.log.write(&format!("commande: reformulation non faite ({why}, {took}) — rien n'est remplacé"));
                self.fail_command(FailureKind::NotDone, message);
                return;
            }
        };
        if !same_app(target, &deps.apps.current()) {
            deps.log.write(&format!(
                "commande: reformulation copiée, l'application au premier plan a changé ({} signes, {took}) — rien n'est collé",
                text.char_count()
            ));
            if deps.paster.copy_transient(text.expose()) {
                let message = format!("Reformulation copiée : l'application a changé, à coller avec {PASTE_SHORTCUT}");
                deps.indicator.flash(IndicatorState::Message(message), 4.0);
            } else {
                self.fail_command(FailureKind::NotDone, "Reformulation non collée : l'application a changé");
            }
            return;
        }
        match deps.paster.paste_transient(text.expose()) {
            PasteOutcome::Pasted => {
                deps.log.write(&format!("commande: reformulation collée ({} signes, {took})", text.char_count()));
                deps.indicator.flash(IndicatorState::Done, 0.9);
            }
            PasteOutcome::CopiedOnly(reason) => {
                deps.log.write(&format!("commande: reformulation copiée seulement ({reason}, {took})"));
                deps.indicator
                    .flash(IndicatorState::Message(format!("Reformulation copiée, à coller avec {PASTE_SHORTCUT} — {reason}")), 4.0);
            }
        }
    }

    /// Le message fixe de la pastille quand une commande ne se fait pas. Rien n'est collé.
    fn refuse(&self, kind: FailureKind, message: &'static str) -> CommandOutcome {
        self.fail_command(kind, message);
        CommandOutcome::Handled
    }

    /// Une commande ratée : la pastille garde l'erreur (jusqu'à la croix ou la dictée suivante) et le son d'erreur joue, sauf si la
    /// personne a coupé les sons (volume à 0, ou « Aucun » au début et à la fin).
    fn fail_command(&self, kind: FailureKind, message: impl Into<String>) {
        let deps = &self.inner.deps;
        deps.indicator.fail(Failure::new(kind, message));
        let settings = deps.config.settings();
        let no_sound = scriibe_core::settings::NO_SOUND;
        let silent = settings.sound_volume <= 0.0 || (settings.start_sound_name == no_sound && settings.end_sound_name == no_sound);
        if !silent {
            deps.sounds.play(ERROR_SOUND, settings.sound_volume);
        }
    }

    /// Une dictée en échec (micro, transcription) : la pastille garde l'erreur, sans son (le son d'erreur est celui des commandes).
    fn fail_dictation(&self, message: impl Into<String>) {
        self.inner.deps.indicator.fail(Failure::new(FailureKind::Dictation, message));
    }

    /// Une commande qui ne peut rien faire : sans la touche, la phrase suit le chemin d'une dictée ordinaire ; sous la touche, jamais
    /// de collage, un message fixe.
    fn nothing_to_do(&self, context: scriibe_core::commands::Context, message: &'static str) -> CommandOutcome {
        match context {
            scriibe_core::commands::Context::NoKey => CommandOutcome::Text(None),
            scriibe_core::commands::Context::UnderKey => self.refuse(FailureKind::NotDone, message),
        }
    }

    /// « Entre guillemets », « entre parenthèses »… sans sélection lisible (l'appli ne la rend pas, ou elle a disparu après un ⌘Z) : la commande
    /// porte sur la dernière dictée collée, si elle date de moins de 2 minutes et que la même appli est au premier plan. Un seul ⌘Z
    /// retire ce collage, puis le texte entouré est collé à sa place. `None` : pas une commande « autour », ou rien à reprendre.
    fn surround_last_paste(&self, command: &scriibe_core::commands::Recognized, settings: &AppSettings) -> Option<CommandOutcome> {
        let deps = &self.inner.deps;
        if !command.id.starts_with(scriibe_core::commands::SURROUND_PREFIX) {
            return None;
        }
        let now = deps.clock.now();
        let last = lock(&self.inner.last_paste).take()?;
        let current = deps.apps.current();
        if now.saturating_sub(last.at) > LAST_PASTE_VALID || last.app_id.is_none() || current.id != last.app_id {
            return None;
        }
        let wrapped = scriibe_core::commands::surround(&command.id, command.lang, last.text.expose())?;
        let wrapped = SelectedText::new(wrapped);
        deps.log.write(&format!(
            "commande: {} sur la dernière dictée collée ({} signes) — ⌘Z puis collage",
            command.id,
            last.text.char_count()
        ));
        let undo = scriibe_core::shortcut::Shortcut::parse("Mod+Z")?;
        if let Err(reason) = deps.paster.press_shortcut(&undo) {
            deps.log.write(&format!("commande: {} — ⌘Z non envoyé ({reason})", command.id));
            self.fail_command(FailureKind::NotDone, format!("Retour arrière non envoyé — {reason}"));
            return Some(CommandOutcome::Handled);
        }
        deps.clock.sleep(Duration::from_millis(120));
        match deps.paster.paste(wrapped.expose(), settings.restore_clipboard) {
            PasteOutcome::Pasted => {
                deps.indicator.flash(IndicatorState::Done, 0.9);
                // Le texte entouré devient le dernier collage : une seconde commande (« entre parenthèses ») peut l'entourer encore.
                *lock(&self.inner.last_paste) = Some(LastPaste { text: wrapped, app_id: last.app_id.clone(), at: deps.clock.now() });
            }
            PasteOutcome::CopiedOnly(reason) => {
                deps.log.write(&format!("commande: {} — copié seulement ({reason})", command.id));
                deps.indicator.flash(IndicatorState::Message(format!("Copié, à coller avec {PASTE_SHORTCUT} — {reason}")), 4.0);
            }
        }
        Some(CommandOutcome::Handled)
    }

    /// Exécute la commande reconnue. Le journal ne reçoit que l'identifiant de la commande et la longueur de la sélection.
    fn run_command(
        &self,
        command: scriibe_core::commands::Recognized,
        settings: &AppSettings,
        context: scriibe_core::commands::Context,
    ) -> CommandOutcome {
        let deps = &self.inner.deps;
        let under_key = context == scriibe_core::commands::Context::UnderKey;
        let selected = if command.needs == scriibe_core::commands::Needs::Selection {
            match deps.paster.selected_text() {
                SelectedRead::Text(text) => Some(text),
                SelectedRead::NoSelection => {
                    if let Some(done) = self.surround_last_paste(&command, settings) {
                        return done;
                    }
                    if under_key {
                        deps.log.write(&format!("commande: {} ignorée (aucune sélection) — rien n'est collé", command.id));
                        return self.refuse(FailureKind::NotDone, "Aucune sélection");
                    }
                    deps.log.write(&format!("commande: {} ignorée (aucune sélection) — la phrase est collée comme du texte", command.id));
                    return CommandOutcome::Text(None);
                }
                SelectedRead::Unreadable(why) => {
                    if !matches!(why, Unreadable::NotTrusted) {
                        if let Some(done) = self.surround_last_paste(&command, settings) {
                            return done;
                        }
                    }
                    if under_key {
                        deps.log.write(&format!("commande: {} ignorée ({}) — rien n'est collé", command.id, why.reason()));
                        return self.refuse(FailureKind::NotDone, "Sélection illisible");
                    }
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
                    return CommandOutcome::TextAfterFailure(note.into());
                }
            }
        } else {
            None
        };
        match command.id.as_str() {
            "dictionnaire.ajouter" => {
                let Some(text) = selected else { return self.nothing_to_do(context, "Aucune sélection") };
                let length = text.char_count();
                deps.log.write(&format!("commande: {} (sélection : {length} signes)", command.id));
                let shown = text.expose().trim();
                if shown.is_empty() || shown.chars().count() > DICTIONARY_MAX_CHARS || shown.contains('\n') {
                    self.fail_command(
                        FailureKind::NotDone,
                        "Sélection trop longue pour le dictionnaire (80 signes au plus, sur une seule ligne)",
                    );
                    return CommandOutcome::Handled;
                }
                match deps.commands.add_to_dictionary(SelectedText::new(shown)) {
                    Ok(()) => deps.indicator.flash(IndicatorState::Message("Ajouter au dictionnaire — complète « Écrit »".into()), 3.0),
                    Err(why) => self.fail_command(FailureKind::NotDone, why),
                }
                CommandOutcome::Handled
            }
            // Gras, italique, souligné (tâche 8) : pas de texte à écrire, on appuie le raccourci de l'appli, qui agit sur la sélection restée active
            // (un seul ⌘Z l'annule). Pas de Markdown : il s'écrirait tel quel dans un traitement de texte.
            id if id.starts_with(scriibe_core::commands::FORMAT_PREFIX) || id.starts_with(scriibe_core::commands::CUSTOM_PREFIX) => {
                // Les commandes de la personne (`perso.*`) n'exigent une sélection que si elle l'a demandé.
                let length = selected.as_ref().map(|t| t.char_count());
                if command.needs == scriibe_core::commands::Needs::Selection && selected.is_none() {
                    return self.nothing_to_do(context, "Aucune sélection");
                }
                match length {
                    Some(n) => deps.log.write(&format!("commande: {id} (sélection : {n} signes)")),
                    None => deps.log.write(&format!("commande: {id}")),
                }
                drop(selected);
                let text_of_shortcut = match id.strip_prefix(scriibe_core::commands::CUSTOM_PREFIX) {
                    Some(_) => settings.custom_commands.iter().find(|c| c.id == id).map(|c| c.shortcut.as_str()),
                    None => settings.format_shortcuts.for_command(id),
                };
                let Some(shortcut) = text_of_shortcut.and_then(scriibe_core::shortcut::Shortcut::parse) else {
                    self.fail_command(FailureKind::NotDone, "Raccourci invalide dans les réglages (exemple : Mod+B)");
                    return CommandOutcome::Handled;
                };
                match deps.paster.press_shortcut(&shortcut) {
                    Ok(()) => deps.indicator.flash(IndicatorState::Done, 0.9),
                    Err(reason) => {
                        deps.log.write(&format!("commande: {id} — raccourci non envoyé ({reason})"));
                        self.fail_command(FailureKind::NotDone, format!("Raccourci non envoyé — {reason}"));
                    }
                }
                CommandOutcome::Handled
            }
            // Caractères autour (tâche 7) : la sélection, restée active, est remplacée par un collage (un seul ⌘Z l'annule, comme une dictée).
            id if id.starts_with(scriibe_core::commands::SURROUND_PREFIX) => {
                let Some(text) = selected else { return self.nothing_to_do(context, "Aucune sélection") };
                deps.log.write(&format!("commande: {id} (sélection : {} signes)", text.char_count()));
                let Some(wrapped) = scriibe_core::commands::surround(id, command.lang, text.expose()) else {
                    self.fail_command(FailureKind::NotDone, "Rien à entourer (la sélection est vide)");
                    return CommandOutcome::Handled;
                };
                // Le texte entouré contient la sélection : mémoire effacée à la fin, jamais journalisé ni mis dans l'historique.
                let wrapped = SelectedText::new(wrapped);
                let pasted = deps.paster.paste(wrapped.expose(), settings.restore_clipboard);
                match pasted {
                    PasteOutcome::Pasted => deps.indicator.flash(IndicatorState::Done, 0.9),
                    PasteOutcome::CopiedOnly(reason) => {
                        deps.log.write(&format!("commande: {id} — copié seulement ({reason})"));
                        deps.indicator.flash(IndicatorState::Message(format!("Copié, à coller avec {PASTE_SHORTCUT} — {reason}")), 4.0);
                    }
                }
                CommandOutcome::Handled
            }
            // Recherches (étape 1b, sous la touche seulement) : l'adresse est construite avec la requête encodée, puis ouverte par la fonction
            // d'ouverture contrôlée. Le journal n'a que l'identifiant (ni le site de la personne, ni la requête) et la longueur de la requête.
            id if scriibe_core::web_search::is_search(id) => {
                let Some(query) = command.argument.as_deref() else { return self.nothing_to_do(context, "Rien à chercher") };
                let Some(url) = scriibe_core::web_search::target(id, command.lang, query, settings) else {
                    deps.log.write(&format!("commande: {id} — adresse non construite (site disparu, modèle refusé ou rien à chercher)"));
                    return self.nothing_to_do(context, "Recherche impossible");
                };
                deps.log.write(&format!("commande: {id} (requête : {} signes)", query.chars().count()));
                match deps.paster.open_address(&url) {
                    Ok(()) => deps.indicator.flash(IndicatorState::Done, 0.9),
                    Err(reason) => {
                        deps.log.write(&format!("commande: {id} — adresse non ouverte ({reason})"));
                        self.fail_command(FailureKind::NotDone, "Recherche non ouverte");
                    }
                }
                CommandOutcome::Handled
            }
            // Extraits (étape 2, sous la touche seulement) : le texte est collé **tel quel** par `paste_transient` (presse-papiers d'avant toujours
            // remis, texte marqué « ne pas garder »), sans passer par `process` : ni modèle, ni dictionnaire, ni typographie, rien dans
            // l'historique. Le journal n'a que l'identifiant et la longueur (ni le nom, ni le texte, ni les phrases).
            id if scriibe_core::snippet::is_snippet(id) => {
                let Some(text) = scriibe_core::snippet::text_of(id, settings) else {
                    deps.log.write(&format!("commande: {id} — extrait disparu ou vide"));
                    return self.nothing_to_do(context, "Extrait impossible");
                };
                deps.log.write(&format!("commande: {id} (extrait : {} signes)", text.chars().count()));
                match deps.paster.paste_transient(text) {
                    PasteOutcome::Pasted => deps.indicator.flash(IndicatorState::Done, 0.9),
                    PasteOutcome::CopiedOnly(reason) => {
                        deps.log.write(&format!("commande: {id} — copié seulement ({reason})"));
                        deps.indicator
                            .flash(IndicatorState::Message(format!("Extrait copié, à coller avec {PASTE_SHORTCUT} — {reason}")), 4.0);
                    }
                }
                CommandOutcome::Handled
            }
            // « Nouveau mail » (étape 3, sous la touche seulement) : `mailto:` seul, ou avec l'objet dicté, encodé, ouvert par la fonction
            // d'ouverture contrôlée. Rien n'est envoyé. Le journal n'a que l'identifiant et la longueur de l'objet, jamais l'objet.
            id if scriibe_core::mail::is_mail(id) => {
                let subject = scriibe_core::mail::clean_subject(command.argument.as_deref());
                match &subject {
                    Some(text) => deps.log.write(&format!("commande: {id} (objet : {} signes)", text.chars().count())),
                    None => deps.log.write(&format!("commande: {id}")),
                }
                match deps.paster.open_address(&scriibe_core::mail::url(subject.as_deref())) {
                    Ok(()) => deps.indicator.flash(IndicatorState::Done, 0.9),
                    Err(reason) => {
                        deps.log.write(&format!("commande: {id} — messagerie non ouverte ({reason})"));
                        self.fail_command(FailureKind::NotDone, "Mail non ouvert");
                    }
                }
                CommandOutcome::Handled
            }
            // Applications (étape 3, sous la touche seulement) : l'identifiant des réglages est retrouvé dans une énumération fraîche des
            // applications installées, et seule l'entrée de cette liste est lancée (aucun argument). Le journal n'a que `appli.<n>` : ni le
            // nom, ni l'identifiant système, ni les phrases ; la pastille est à texte fixe.
            id if scriibe_core::voice_app::is_app(id) => {
                let Some(app_id) = scriibe_core::voice_app::app_id_of(id, settings) else {
                    deps.log.write(&format!("commande: {id} — application disparue des réglages"));
                    return self.nothing_to_do(context, "Application introuvable");
                };
                deps.log.write(&format!("commande: {id}"));
                match deps.paster.open_app(app_id) {
                    Ok(()) => deps.indicator.flash(IndicatorState::Message("Ouverture de l'application".into()), 2.0),
                    Err(reason) => {
                        deps.log.write(&format!("commande: {id} — application non ouverte ({reason})"));
                        let message = if reason == scriibe_platform::installed_apps::APP_NOT_FOUND {
                            "Application introuvable"
                        } else {
                            "Application non ouverte"
                        };
                        self.fail_command(FailureKind::NotDone, message);
                    }
                }
                CommandOutcome::Handled
            }
            other => {
                deps.log.write(&format!("commande: {other} inconnue du chef d'orchestre"));
                self.nothing_to_do(context, "Commande inconnue")
            }
        }
    }

    /// Les temps comptent depuis le relâchement de la touche.
    fn process(&self, job: &Job, settings: &AppSettings, intent: Intent) {
        let deps = &self.inner.deps;
        let dictionary = deps.config.dictionary();
        let elapsed_since = |start: Duration| deps.clock.now().saturating_sub(start);
        let weak = MicLevel::assess(job.level) == MicLevel::Weak;

        let raw = match deps.transcriber.transcribe(&job.wav, settings, &dictionary.words) {
            Ok(text) => text.trim().to_string(),
            Err(e) => {
                deps.log.write(&format!("dictée: transcription en échec ({e})"));
                self.fail_dictation(e.to_string());
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

        // Sous la touche de commande, la transcription brute est une commande : ni dictionnaire, ni mise au propre, ni collage, ni historique.
        if intent == Intent::Command {
            self.process_command(&raw, settings, &job.target);
            return;
        }
        let translate_to = match intent {
            Intent::Translation(language) => Some(language),
            Intent::Dictation | Intent::Command => None,
        };

        // Commande vocale : la phrase est toute la transcription (avant `dictionary::apply`, qui la réécrirait). La sélection n'est lue qu'ici.
        let (command_note, command_failure) = match self.try_command(&raw, settings) {
            CommandOutcome::Handled => return,
            CommandOutcome::Text(note) => (note, None),
            CommandOutcome::TextAfterFailure(note) => (Some(note.clone()), Some(note)),
        };

        // S18 : signes dictés des langues de la dictée (les signes anglais ne s'appliquent pas à une dictée française).
        let signs_for = scriibe_core::spoken_signs::Active::of(settings.language);
        let applied = dictionary::apply_for(&dictionary, &join_spelled_code(&raw), settings.list_bullet, settings.lists_on(), signs_for);
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
                    let fixed =
                        dictionary::apply_for(&dictionary, &out, settings.list_bullet, false, scriibe_core::spoken_signs::Active::NEUTRAL);
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
                    let fixed = dictionary::apply_for(&dictionary, &cleaned, settings.list_bullet, settings.lists_on(), signs_for);
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
        // L'espace qui suit la dictée n'est ajoutée qu'à ce qui part au collage : l'historique et « copier la dernière » gardent `text`.
        let to_paste = scriibe_core::spacing::for_paste(&text, replaces_selection);
        let pasted = paste_text(deps.paster.as_ref(), &to_paste, settings.restore_clipboard, focus_at_paste.id.as_deref());
        *lock(&self.inner.last_paste) = match pasted {
            PasteOutcome::Pasted => {
                Some(LastPaste { text: SelectedText::new(to_paste.clone()), app_id: focus_at_paste.id.clone(), at: deps.clock.now() })
            }
            PasteOutcome::CopiedOnly(_) => None,
        };
        deps.log.write(&format!(
            "collage: {} signes envoyés à {} (à l'appui : {}) en {}{}{}{}",
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
            },
            match deps.paster.last_report().as_str() {
                "" => String::new(),
                detail => format!(" [{detail}]"),
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
            // Une commande reconnue qui n'a pas pu se faire (la phrase est collée comme du texte) : erreur qui reste affichée.
            if command_failure.as_deref() == Some(note.as_str()) {
                self.fail_command(FailureKind::NotDone, note);
            } else {
                deps.indicator.flash(IndicatorState::Message(note), 3.0);
            }
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

/// Identifiant de bundle de LibreOffice sur Mac.
const LIBREOFFICE_MAC_ID: &str = "org.libreoffice.script";

/// Colle `text`. Dans LibreOffice (Mac), Writer ignore un saut de ligne en fin de texte collé (« Bonjour, retour à la ligne » ne passait pas à
/// la ligne) : le texte est collé sans ses sauts de ligne finaux, puis une vraie touche Entrée est envoyée pour chacun. Au milieu d'un
/// texte le collage les rend bien, et ailleurs la touche Entrée enverrait un message : seul LibreOffice est concerné.
fn paste_text(paster: &dyn scriibe_platform::paste::Paster, text: &str, restore_clipboard: bool, app_id: Option<&str>) -> PasteOutcome {
    let body = text.trim_end_matches('\n');
    let returns = text.len() - body.len();
    if returns == 0 || app_id != Some(LIBREOFFICE_MAC_ID) {
        return paster.paste(text, restore_clipboard);
    }
    let outcome = if body.is_empty() { PasteOutcome::Pasted } else { paster.paste(body, restore_clipboard) };
    if matches!(outcome, PasteOutcome::Pasted) {
        for _ in 0..returns {
            paster.press_return();
        }
    }
    outcome
}
