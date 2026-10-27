use crate::executor::Executor;
use chrono::{DateTime, Utc};
use scriibe_core::cleaner;
use scriibe_core::dictionary;
use scriibe_core::history::{Cleanup, Dictation, DictationTiming};
use scriibe_core::hotkey::{HotkeyEvent, HotkeyMonitor};
use scriibe_core::microphone::MicLevel;
use scriibe_core::settings::{AppSettings, TranscriptionEngine};
use scriibe_core::text::{known_hallucination, word_count};
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
use scriibe_platform::paste::{PasteOutcome, Paster};
use scriibe_platform::sounds::Sounds;
use scriibe_platform::storage::{ConfigSource, HistorySink};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

/// Une dictée plus courte est ignorée (effleurement de la touche).
const MIN_DURATION: f64 = 0.3;
/// Après l'ajout de la touche de traduction, on attend avant de charger le modèle de traduction : Ctrl+Maj+lettre est un raccourci,
/// qui annule la dictée avant qu'un modèle soit chargé pour rien.
const TRANSLATION_WARM_UP_DELAY: Duration = Duration::from_millis(400);
/// Au bout de ce temps sans son assez fort, l'indicateur dit « Je n'entends rien… ».
const MIC_WATCH_AFTER: Duration = Duration::from_secs(2);

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
}

struct Inner {
    deps: Deps,
    state: Mutex<State>,
}

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
        let hotkey = deps.config.settings().hotkey;
        let indicator = Arc::clone(&deps.indicator);
        deps.recorder.set_level_listener(Some(Arc::new(move |level| indicator.set_level(level))));
        let state = State {
            monitor: HotkeyMonitor::new(hotkey),
            recording: false,
            busy: false,
            translating: false,
            target: AppInfo::unknown(),
            started_at: Duration::ZERO,
            translation_warm_up_due: None,
            input_description: String::new(),
            mic_warned: false,
        };
        Self { inner: Arc::new(Inner { deps, state: Mutex::new(state) }) }
    }

    pub fn is_recording(&self) -> bool {
        lock(&self.inner.state).recording
    }

    pub fn is_busy(&self) -> bool {
        lock(&self.inner.state).busy
    }

    /// Un événement de clavier du système. Le crochet du système ne fait que les poster : cette méthode peut prendre son temps.
    pub fn handle_key(&self, event: KeyEvent) {
        let events = match event {
            KeyEvent::Modifier { changed, held } => {
                let hotkey = self.inner.deps.config.settings().hotkey;
                let mut state = lock(&self.inner.state);
                if state.monitor.hotkey != hotkey {
                    state.monitor = HotkeyMonitor::new(hotkey);
                }
                state.monitor.handle(changed, &held)
            }
            KeyEvent::Typed => lock(&self.inner.state).monitor.handle_key_down(),
        };
        for event in events {
            match event {
                HotkeyEvent::Press => self.press(),
                HotkeyEvent::Release => self.release(),
                HotkeyEvent::Command => self.request_translation(),
                HotkeyEvent::OtherKey => self.cancel(),
            }
        }
    }

    /// À appeler une à deux fois par seconde : alerte de micro muet pendant l'écoute, préchauffage de la traduction.
    pub fn tick(&self) {
        let deps = &self.inner.deps;
        let now = deps.clock.now();
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
                    transcriber.whisper.server.set_idle_limit((idle > 0).then(|| Duration::from_secs(idle as u64 * 60)));
                    if let Err(e) = transcriber.whisper.server.ensure_running(&model) {
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

        let applied = dictionary::apply(&dictionary, &raw, settings.list_bullet, settings.lists_on());
        let mut text = applied.text;
        let mut corrections = applied.count as i64;
        let mut note: Option<String> = None;
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
        // Trace du collage : un Ctrl+V envoyé n'est pas un texte reçu, et sans cette ligne un collage perdu en silence ne laisse rien.
        let focus_at_paste = deps.apps.current();
        let paste_start = deps.clock.now();
        let pasted = deps.paster.paste(&text, settings.restore_clipboard);
        deps.log.write(&format!(
            "collage: {} signes envoyés à {} (à l'appui : {}) en {}{}",
            text.chars().count(),
            focus_at_paste.name,
            job.target.name,
            seconds(elapsed_since(paste_start)),
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
        deps.history.record(dictation);

        if let PasteOutcome::CopiedOnly(reason) = pasted {
            deps.log.write(&format!("collage: {reason}"));
            deps.indicator.flash(IndicatorState::Message(format!("Copié, à coller avec Ctrl+V — {reason}")), 4.0);
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
