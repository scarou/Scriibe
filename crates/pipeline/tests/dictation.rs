//! Le chef d'orchestre de bout en bout, avec des faux du système (clavier, micro, collage) et un monde simulé pour le réseau.

use chrono::{DateTime, Utc};
use scriibe_core::dictionary::{Correction, UserDictionary};
use scriibe_core::history::Cleanup;
use scriibe_core::hotkey::Key;
use scriibe_core::lang::{LanguageGuesser, NoGuesser, Odds};
use scriibe_core::settings::{AppSettings, CleanupEngine, Hotkey, TranscriptionEngine};
use scriibe_core::translation::TranslationLanguage;
use scriibe_engines::fs::FileSystem;
use scriibe_engines::http::HttpResponse;
use scriibe_engines::runtime::{Clock, KeyKind, Log};
use scriibe_engines::services::{Cleaner, Transcriber};
use scriibe_engines::testing::FakeWorld;
use scriibe_engines::whisper::{Whisper, WhisperLauncher};
use scriibe_pipeline::{Controller, Deps, Executor, InlineExecutor};
use scriibe_platform::apps::AppInfo;
use scriibe_platform::indicator::IndicatorState;
use scriibe_platform::keys::KeyEvent;
use scriibe_platform::paste::PasteOutcome;
use scriibe_platform::testing::{
    FakeApps, FakeConfig, FakeIndicator, FakeMedia, FakePaster, FakeRecorder, FakeSounds, MemoryHistory, StaticSecrets,
};
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Le temps du chef d'orchestre : il n'avance que si un test (ou un faux service) le fait avancer.
#[derive(Default)]
struct TestClock(AtomicU64);

impl TestClock {
    fn advance(&self, duration: Duration) {
        self.0.fetch_add(duration.as_micros() as u64, Ordering::SeqCst);
    }
}

impl Clock for TestClock {
    fn now(&self) -> Duration {
        Duration::from_micros(self.0.load(Ordering::SeqCst))
    }

    fn sleep(&self, duration: Duration) {
        self.advance(duration);
    }
}

type Job = Box<dyn FnOnce() + Send>;

/// Garde les tâches de fond jusqu'à ce qu'un test les lance.
#[derive(Default)]
struct ManualExecutor {
    jobs: Mutex<VecDeque<(String, Job)>>,
}

impl ManualExecutor {
    fn run_all(&self, only: &str) {
        loop {
            let next = {
                let mut jobs = self.jobs.lock().unwrap();
                let position = jobs.iter().position(|(name, _)| name.contains(only));
                position.and_then(|i| jobs.remove(i))
            };
            match next {
                Some((_, job)) => job(),
                None => return,
            }
        }
    }
}

impl Executor for ManualExecutor {
    fn spawn(&self, name: &str, job: Job) {
        self.jobs.lock().unwrap().push_back((name.to_string(), job));
    }
}

struct FakeFs(Vec<PathBuf>);

impl FileSystem for FakeFs {
    fn exists(&self, path: &Path) -> bool {
        self.0.iter().any(|f| f == path)
    }
    fn list_dir(&self, dir: &Path) -> Vec<String> {
        self.0.iter().filter(|f| f.parent() == Some(dir)).filter_map(|f| f.file_name()?.to_str().map(str::to_string)).collect()
    }
    fn walk(&self, root: &Path) -> Vec<PathBuf> {
        self.0.iter().filter(|f| f.starts_with(root)).cloned().collect()
    }
}

fn bin(name: &str) -> PathBuf {
    Path::new("/fake/bin").join(if cfg!(windows) { format!("{name}.exe") } else { name.to_string() })
}

struct Rig {
    world: Arc<FakeWorld>,
    clock: Arc<TestClock>,
    controller: Controller,
    recorder: Arc<FakeRecorder>,
    paster: Arc<FakePaster>,
    media: Arc<FakeMedia>,
    indicator: Arc<FakeIndicator>,
    sounds: Arc<FakeSounds>,
    history: Arc<MemoryHistory>,
    config: Arc<FakeConfig>,
    executor: Arc<ManualExecutor>,
    /// Le moteur de mise au propre répond « erreur serveur ».
    cleanup_fails: Arc<AtomicBool>,
}

fn transcript(text: &str) -> HttpResponse {
    let body = json!({"results": {"channels": [{"alternatives": [{"transcript": text}]}]}});
    HttpResponse { status: 200, body: body.to_string().into_bytes() }
}

fn completion(text: &str) -> HttpResponse {
    let body = json!({"choices": [{"message": {"content": text}, "finish_reason": "stop"}]});
    HttpResponse { status: 200, body: body.to_string().into_bytes() }
}

fn local_settings() -> AppSettings {
    AppSettings { hotkey: Hotkey::RightControl, cleanup_engine: CleanupEngine::LmStudio, ..AppSettings::default() }
}

/// Deepgram répond `heard` en 0,3 s ; LM Studio répond `cleaned` en 0,35 s. `inline` : les tâches de fond s'exécutent tout de suite.
fn rig_with(settings: AppSettings, heard: &'static str, cleaned: &'static str, inline: bool) -> Rig {
    rig_full(settings, heard, cleaned, inline, StaticSecrets::with(KeyKind::Deepgram, "cle-deepgram"), Arc::new(NoGuesser))
}

fn rig_full(
    settings: AppSettings,
    heard: &'static str,
    cleaned: &'static str,
    inline: bool,
    secrets: StaticSecrets,
    guesser: Arc<dyn LanguageGuesser + Send + Sync>,
) -> Rig {
    let world = FakeWorld::new();
    let cleanup_fails = Arc::new(AtomicBool::new(false));
    let clock = Arc::new(TestClock::default());
    let c = clock.clone();
    world.route("api.deepgram.com/v1/listen?", move |_| {
        c.advance(Duration::from_millis(300));
        Ok(transcript(heard))
    });
    let c = clock.clone();
    let fails = cleanup_fails.clone();
    world.route("/v1/chat/completions", move |request| {
        if fails.load(Ordering::SeqCst) {
            return Ok(HttpResponse { status: 500, body: Vec::new() });
        }
        let body: Value = serde_json::from_slice(&request.body).unwrap();
        // Le préchauffage (un jeton) ne coûte rien à la dictée : seul l'appel de la dictée prend 0,35 s.
        if body["max_tokens"] != 1 {
            c.advance(Duration::from_millis(350));
        }
        Ok(completion(cleaned))
    });
    // Le préchauffage de Deepgram (HEAD) n'a pas de route : il échoue sans bruit.
    let model = "/m/gemma-4-E4B-it-Q4_0.gguf";
    let fs: Arc<dyn FileSystem> = Arc::new(FakeFs(vec![
        PathBuf::from(model),
        bin("llama-server"),
        bin("whisper-server"),
        PathBuf::from("/m/ggml-large-v3-turbo.bin"),
    ]));
    let path = std::env::join_paths(["/fake/bin"]).unwrap();
    let secrets = Arc::new(secrets);
    let whisper = Arc::new(Whisper::new(WhisperLauncher::new(fs.clone(), 8178, Some(&path)), world.runtime()));
    let transcriber = Arc::new(Transcriber {
        http: world.clone(),
        secrets: secrets.clone(),
        whisper,
        detected_whisper_models: Arc::new(|| vec!["/m/ggml-large-v3-turbo.bin".to_string()]),
    });
    let cleaner = Arc::new(Cleaner {
        http: world.clone(),
        secrets,
        log: world.log.clone(),
        guesser,
        llama: None,
        llama_models: Arc::new(Vec::new),
        fs,
        warm_ups: Default::default(),
    });
    let recorder = Arc::new(FakeRecorder::default());
    let paster = Arc::new(FakePaster::default());
    let media = Arc::new(FakeMedia::default());
    let indicator = Arc::new(FakeIndicator::default());
    let sounds = Arc::new(FakeSounds::default());
    let history = Arc::new(MemoryHistory::default());
    let config = Arc::new(FakeConfig::new(settings));
    let executor = Arc::new(ManualExecutor::default());
    let executor_dyn: Arc<dyn Executor> = if inline { Arc::new(InlineExecutor) } else { executor.clone() };
    let wall: Arc<dyn Fn() -> DateTime<Utc> + Send + Sync> = Arc::new(|| DateTime::from_timestamp(1_790_000_000, 0).unwrap());
    let controller = Controller::new(Deps {
        recorder: recorder.clone(),
        paster: paster.clone(),
        media: media.clone(),
        apps: Arc::new(FakeApps(AppInfo { name: "notepad".into(), id: Some("notepad.exe".into()) })),
        sounds: sounds.clone(),
        indicator: indicator.clone(),
        config: config.clone(),
        history: history.clone(),
        transcriber,
        cleaner,
        clock: clock.clone(),
        wall_clock: wall,
        log: world.log.clone() as Arc<dyn Log>,
        executor: executor_dyn,
    });
    Rig { world, clock, controller, recorder, paster, media, indicator, sounds, history, config, executor, cleanup_fails }
}

/// Dictée complète : tout s'exécute tout de suite.
fn rig(heard: &'static str, cleaned: &'static str) -> Rig {
    rig_with(local_settings(), heard, cleaned, true)
}

impl Rig {
    fn key_down(&self) {
        self.controller.handle_key(KeyEvent::Changed { changed: Key::RightControl, down: true, held: vec![Key::RightControl] });
    }

    fn key_up(&self) {
        self.controller.handle_key(KeyEvent::Changed { changed: Key::RightControl, down: false, held: vec![] });
    }

    fn shift_down(&self) {
        self.controller.handle_key(KeyEvent::Changed {
            changed: Key::LeftShift,
            down: true,
            held: vec![Key::RightControl, Key::LeftShift],
        });
    }

    fn dictate(&self) {
        self.key_down();
        self.key_up();
    }

    fn cleanup_requests(&self) -> Vec<Value> {
        self.world
            .requests()
            .iter()
            .filter(|r| r.url.contains("/v1/chat/completions"))
            .map(|r| serde_json::from_slice::<Value>(&r.body).unwrap())
            .filter(|b| b["max_tokens"] != 1)
            .collect()
    }

    fn warm_up_requests(&self) -> Vec<Value> {
        self.world
            .requests()
            .iter()
            .filter(|r| r.url.contains("/v1/chat/completions"))
            .map(|r| serde_json::from_slice::<Value>(&r.body).unwrap())
            .filter(|b| b["max_tokens"] == 1)
            .collect()
    }

    fn transcription_requests(&self) -> usize {
        self.world.requests().iter().filter(|r| r.url.contains("api.deepgram.com/v1/listen?")).count()
    }
}

fn message(text: &str) -> IndicatorState {
    IndicatorState::Message(text.to_string())
}

#[test]
fn a_normal_dictation_is_transcribed_cleaned_pasted_and_recorded() {
    let r = rig("euh bonjour tout le monde on se voit demain", "Bonjour tout le monde, on se voit demain.");
    r.dictate();
    assert_eq!(r.paster.pasted(), vec![("Bonjour tout le monde, on se voit demain.".to_string(), true)]);
    assert_eq!(r.indicator.states(), vec![IndicatorState::Listening, IndicatorState::Working, IndicatorState::Done]);
    assert_eq!(r.sounds.played(), vec!["Tink", "Pop"]);
    assert_eq!(r.media.calls(), vec!["pause(false)", "resume"]);
    assert_eq!((r.recorder.starts().len(), r.recorder.stops()), (1, 1));
    assert!(!r.controller.is_recording() && !r.controller.is_busy());

    let saved = r.history.items();
    assert_eq!(saved.len(), 1);
    let d = &saved[0];
    assert_eq!(d.raw_text, "euh bonjour tout le monde on se voit demain");
    assert_eq!(d.final_text, "Bonjour tout le monde, on se voit demain.");
    assert_eq!((d.app_name.as_str(), d.bundle_id.as_deref(), d.words, d.duration), ("notepad", Some("notepad.exe"), 8, 2.0));
    let timing = d.timing.as_ref().unwrap();
    assert_eq!(timing.outcome, Cleanup::Model);
    assert!((timing.transcription - 0.3).abs() < 1e-6, "transcription {}", timing.transcription);
    assert!((timing.cleanup - 0.35).abs() < 1e-6, "mise au propre {}", timing.cleanup);
    assert!((timing.total - 0.65).abs() < 1e-6, "total {}", timing.total);
    let log = r.world.log.lines().join("\n");
    assert!(log.contains("transcription 0.30 s (deepgram)") && log.contains("total après relâche 0.65 s"), "{log}");
    assert!(log.contains("collage: 41 signes envoyés à notepad (à l'appui : notepad)"), "{log}");
}

#[test]
fn the_history_never_gets_a_key_or_a_wav_in_the_log() {
    let r = rig("bonjour à toute l'équipe ce matin", "Bonjour à toute l'équipe ce matin.");
    r.dictate();
    let log = r.world.log.lines().join("\n");
    assert!(!log.contains("cle-deepgram") && !log.contains("Bonjour"), "le journal ne contient ni clé ni texte dicté : {log}");
}

#[test]
fn french_typography_is_applied_after_the_model() {
    let r = rig("tu viens ce soir on mange a dix neuf heures", "Tu viens ce soir? On mange à 19h.");
    r.dictate();
    assert_eq!(r.paster.pasted()[0].0, "Tu viens ce soir\u{a0}? On mange à 19\u{a0}h.");
}

#[test]
fn another_key_while_holding_cancels_without_a_trace() {
    let r = rig("jamais utilisé", "jamais utilisé");
    r.key_down();
    r.controller.handle_key(KeyEvent::Changed { changed: Key::Vk(0x41), down: true, held: vec![Key::RightControl, Key::Vk(0x41)] });
    r.key_up();
    assert!(!r.controller.is_recording());
    assert_eq!(r.recorder.stops(), 1);
    assert_eq!(r.indicator.states(), vec![IndicatorState::Listening, IndicatorState::Hidden]);
    assert_eq!(r.media.calls(), vec!["pause(false)", "resume"]);
    assert_eq!(r.transcription_requests(), 0);
    assert!(r.paster.pasted().is_empty() && r.history.items().is_empty());
}

#[test]
fn typing_without_holding_the_key_does_nothing() {
    let r = rig("x", "x");
    r.controller.handle_key(KeyEvent::Changed { changed: Key::Vk(0x41), down: true, held: vec![Key::RightControl, Key::Vk(0x41)] });
    assert!(r.indicator.states().is_empty() && r.recorder.starts().is_empty());
}

#[test]
fn a_brush_of_the_key_is_ignored() {
    let r = rig("x", "x");
    r.recorder.set_recording(0.2, 0.05);
    r.dictate();
    assert_eq!(r.indicator.states(), vec![IndicatorState::Listening, IndicatorState::Hidden]);
    assert_eq!(r.transcription_requests(), 0);
    assert!(r.sounds.played() == vec!["Tink"], "pas de son de fin pour un effleurement");
}

#[test]
fn a_silent_microphone_sends_nothing() {
    let r = rig("x", "x");
    r.recorder.set_recording(2.0, 0.0005);
    r.dictate();
    let last = r.indicator.last().unwrap();
    assert!(matches!(&last, IndicatorState::Message(m) if m.starts_with("Aucun son capté — vérifie le micro (micro simulé)")), "{last:?}");
    assert_eq!(r.transcription_requests(), 0);
    assert!(r.history.items().is_empty());
}

#[test]
fn an_empty_transcript_says_nothing_was_heard() {
    let r = rig("   ", "x");
    r.dictate();
    assert_eq!(r.indicator.last(), Some(message("Rien entendu")));
    assert!(r.paster.pasted().is_empty() && r.history.items().is_empty());
    assert!(!r.controller.is_busy());
}

#[test]
fn a_weak_microphone_is_named_when_nothing_was_heard() {
    let r = rig("", "x");
    r.recorder.set_recording(2.0, 0.008);
    r.dictate();
    assert_eq!(r.indicator.last(), Some(message("Rien entendu — micro faible : rapproche-toi")));
}

#[test]
fn a_weak_microphone_is_flagged_after_a_good_paste() {
    let r = rig("bonjour tout le monde on commence", "Bonjour tout le monde, on commence.");
    r.recorder.set_recording(2.0, 0.008);
    r.dictate();
    assert_eq!(r.paster.pasted().len(), 1);
    let last = r.indicator.last().unwrap();
    assert!(matches!(&last, IndicatorState::Message(m) if m.starts_with("Collé — micro faible")), "{last:?}");
}

#[test]
fn a_model_that_fails_leaves_the_raw_text_and_says_so() {
    let r = rig("Bonjour tout le monde on commence.", "x");
    r.cleanup_fails.store(true, Ordering::SeqCst);
    r.dictate();
    assert_eq!(r.paster.pasted()[0].0, "Bonjour tout le monde on commence.", "le texte brut est collé");
    let last = r.indicator.last().unwrap();
    assert!(matches!(&last, IndicatorState::Message(m) if m.starts_with("Collé sans mise au propre")), "{last:?}");
    assert_eq!(r.history.items()[0].timing.as_ref().unwrap().outcome, Cleanup::Failed);
    assert!(!r.controller.is_busy());
}

#[test]
fn a_rewrite_the_guard_rejects_leaves_the_raw_text() {
    let r = rig("bonjour tout le monde on commence la réunion", "Je suis un assistant, voici une réponse sans rapport avec la dictée.");
    r.dictate();
    assert_eq!(r.paster.pasted()[0].0, "bonjour tout le monde on commence la réunion");
    assert_eq!(r.history.items()[0].timing.as_ref().unwrap().outcome, Cleanup::Rejected);
    let last = r.indicator.last().unwrap();
    assert!(matches!(&last, IndicatorState::Message(m) if m.contains("Reformulation écartée")), "{last:?}");
}

#[test]
fn three_words_or_fewer_skip_the_model_in_simple_mode() {
    let r = rig("bonjour à tous", "ne doit pas être utilisé");
    r.dictate();
    assert!(r.cleanup_requests().is_empty(), "aucun appel au modèle");
    assert_eq!(r.paster.pasted()[0].0, "bonjour à tous");
    assert_eq!(r.history.items()[0].timing.as_ref().unwrap().outcome, Cleanup::Short);
}

#[test]
fn literal_mode_pastes_what_was_heard_with_the_dictionary_applied() {
    let r = rig_with(
        AppSettings { rewrite_mode: scriibe_core::modes::RewriteMode::Literal, ..local_settings() },
        "voir le projet scribe demain matin",
        "x",
        true,
    );
    r.config.set_dictionary(UserDictionary {
        words: vec![],
        corrections: vec![Correction { heard: "scribe".into(), written: "Scriibe".into() }],
    });
    r.dictate();
    assert!(r.cleanup_requests().is_empty());
    assert_eq!(r.paster.pasted()[0].0, "voir le projet Scriibe demain matin");
    let d = &r.history.items()[0];
    assert_eq!((d.corrections, d.timing.as_ref().unwrap().outcome), (1, Cleanup::None));
}

#[test]
fn the_dictionary_is_applied_before_and_after_the_model() {
    let r = rig("on parle de scribe ce matin avec lui", "On parle de scribe ce matin avec lui.");
    r.config.set_dictionary(UserDictionary {
        words: vec!["Scriibe".into()],
        corrections: vec![Correction { heard: "scribe".into(), written: "Scriibe".into() }],
    });
    r.dictate();
    assert_eq!(r.paster.pasted()[0].0, "On parle de Scriibe ce matin avec lui.");
    assert_eq!(r.history.items()[0].corrections, 2);
}

#[test]
fn a_missing_deepgram_key_is_explained() {
    let r = rig_full(local_settings(), "x", "x", true, StaticSecrets::default(), Arc::new(NoGuesser));
    r.dictate();
    assert_eq!(r.indicator.last(), Some(message("Clé Deepgram manquante — voir Réglages")));
    assert!(!r.controller.is_busy() && r.paster.pasted().is_empty() && r.history.items().is_empty());
}

#[test]
fn a_microphone_that_will_not_open_is_reported_and_nothing_starts() {
    let r = rig("x", "x");
    r.recorder.fail_to_start("Aucun micro disponible");
    r.key_down();
    assert!(!r.controller.is_recording());
    assert_eq!(r.indicator.states(), vec![message("Aucun micro disponible")]);
    assert!(r.media.calls().is_empty() && r.sounds.played().is_empty());
    r.key_up();
    assert_eq!(r.recorder.stops(), 0);
}

#[test]
fn the_pressed_key_is_ignored_while_a_dictation_is_being_processed() {
    let r = rig_with(local_settings(), "bonjour tout le monde on commence", "Bonjour tout le monde, on commence.", false);
    r.dictate();
    assert!(r.controller.is_busy(), "le traitement attend son tour dans l'exécuteur");
    r.key_down();
    assert!(!r.controller.is_recording());
    assert_eq!(r.recorder.starts().len(), 1, "le micro ne s'ouvre pas deux fois");
    r.executor.run_all("");
    assert!(!r.controller.is_busy());
    assert_eq!(r.paster.pasted().len(), 1);
    r.key_up();
    // Une fois libre, une nouvelle dictée démarre.
    r.dictate();
    assert_eq!(r.recorder.starts().len(), 2);
}

#[test]
fn warm_up_starts_while_the_person_is_still_speaking() {
    let r = rig_with(local_settings(), "x", "x", false);
    r.key_down();
    r.executor.run_all("prechauffage");
    assert_eq!(r.warm_up_requests().len(), 1, "le modèle de mise au propre est préchauffé avec le vrai prompt");
    assert_eq!(r.transcription_requests(), 0);
    let prompt = r.warm_up_requests()[0]["messages"].to_string();
    assert!(!prompt.is_empty());
}

#[test]
fn preload_reads_both_prompts_at_launch_without_any_key() {
    let settings = AppSettings { transcription_engine: TranscriptionEngine::Whisper, ..local_settings() };
    let r = rig_with(settings, "x", "x", true);
    r.controller.preload();
    r.executor.run_all("");
    assert_eq!(r.warm_up_requests().len(), 2, "mise au propre puis traduction");
    assert!(!r.warm_up_requests()[0]["messages"].to_string().contains("anglais"));
    assert!(r.warm_up_requests()[1]["messages"].to_string().contains("anglais"));
    assert!(r.world.spawns().iter().any(|s| s.args.iter().any(|a| a.contains("ggml-large-v3-turbo"))), "Whisper est lancé");
    assert!(r.recorder.starts().is_empty(), "aucun micro ouvert");
}

#[test]
fn whisper_is_started_at_the_press_and_reused_for_the_transcription() {
    let settings = AppSettings { transcription_engine: TranscriptionEngine::Whisper, ..local_settings() };
    let r = rig_with(settings, "x", "Bonjour tout le monde, on commence.", false);
    r.world.route_fixed("/inference", 200, r#"{"text": " bonjour tout le monde on commence"}"#);
    r.key_down();
    r.executor.run_all("prechauffage");
    let launches = r.world.spawns().len();
    assert!(
        r.world.spawns().iter().any(|s| s.args.iter().any(|a| a.contains("ggml-large-v3-turbo"))),
        "Whisper est lancé avec le modèle repéré"
    );
    r.key_up();
    r.executor.run_all("");
    assert_eq!(r.world.spawns().len(), launches, "la transcription réutilise le serveur déjà lancé");
    assert_eq!(r.paster.pasted()[0].0, "Bonjour tout le monde, on commence.");
}

#[test]
fn a_phrase_invented_by_whisper_is_dropped() {
    let settings = AppSettings { transcription_engine: TranscriptionEngine::Whisper, ..local_settings() };
    let r = rig_with(settings, "x", "x", true);
    r.world.route_fixed("/inference", 200, r#"{"text": " Sous-titres réalisés para la communauté d'Amara.org"}"#);
    r.dictate();
    assert_eq!(r.indicator.last(), Some(message("Rien entendu")));
    assert!(r.paster.pasted().is_empty() && r.history.items().is_empty());
    let log = r.world.log.lines().join("\n");
    assert!(log.contains("phrase inventée par Whisper"), "{log}");
}

#[test]
fn shift_added_while_holding_translates_the_dictation() {
    let r = rig("bonjour tout le monde on commence", "Hello everyone, let's get started.");
    r.key_down();
    r.shift_down();
    assert_eq!(r.indicator.translations(), vec![Some("anglais".to_string())]);
    r.key_up();
    let sent = r.cleanup_requests();
    assert_eq!(sent.len(), 1);
    assert!(sent[0]["messages"].to_string().contains("anglais"), "{}", sent[0]["messages"]);
    // Pas de typographie française sur un texte traduit.
    assert_eq!(r.paster.pasted()[0].0, "Hello everyone, let's get started.");
    let d = &r.history.items()[0];
    assert_eq!(d.translation, Some(TranslationLanguage::En));
    assert_eq!(d.timing.as_ref().unwrap().outcome, Cleanup::Translated);
    let log = r.world.log.lines().join("\n");
    assert!(log.contains("traduction anglais"), "{log}");
}

#[test]
fn shift_held_before_the_key_also_translates() {
    let r = rig("bonjour tout le monde on commence", "Hello everyone, let's get started.");
    r.controller.handle_key(KeyEvent::Changed { changed: Key::LeftShift, down: true, held: vec![Key::LeftShift] });
    r.controller.handle_key(KeyEvent::Changed { changed: Key::RightControl, down: true, held: vec![Key::LeftShift, Key::RightControl] });
    assert_eq!(r.indicator.translations(), vec![Some("anglais".to_string())]);
    r.controller.handle_key(KeyEvent::Changed { changed: Key::RightControl, down: false, held: vec![Key::LeftShift] });
    assert_eq!(r.history.items()[0].translation, Some(TranslationLanguage::En));
}

/// Un détecteur de langue qui voit du français partout : la « traduction » est restée en français.
struct AlwaysFrench;

impl LanguageGuesser for AlwaysFrench {
    fn odds(&self, _: &str, _: TranslationLanguage) -> Odds {
        Odds { french: 0.95, target: 0.02 }
    }
}

#[test]
fn a_translation_the_guard_rejects_is_pasted_in_french() {
    let r = rig_full(
        local_settings(),
        "bonjour tout le monde on commence",
        "Bonjour tout le monde, on commence.",
        true,
        StaticSecrets::with(KeyKind::Deepgram, "k"),
        Arc::new(AlwaysFrench),
    );
    r.key_down();
    r.shift_down();
    r.key_up();
    assert_eq!(r.paster.pasted()[0].0, "Bonjour tout le monde, on commence.");
    assert_eq!(r.history.items()[0].translation, None);
    assert_eq!(r.history.items()[0].timing.as_ref().unwrap().outcome, Cleanup::Fallback);
    assert_eq!(r.indicator.last(), Some(message("Traduction écartée — collé en français")));
    assert_eq!(r.indicator.translations().last(), Some(&None));
}

#[test]
fn the_translation_model_is_loaded_400_ms_after_shift_and_only_if_still_translating() {
    let r = rig_with(local_settings(), "x", "x", true);
    r.key_down();
    let before = r.warm_up_requests().len();
    r.shift_down();
    r.controller.tick();
    assert_eq!(r.warm_up_requests().len(), before, "trop tôt : Ctrl+Maj+lettre est un raccourci");
    r.clock.advance(Duration::from_millis(450));
    r.controller.tick();
    assert_eq!(r.warm_up_requests().len(), before + 1);
    assert!(r.warm_up_requests().last().unwrap()["messages"].to_string().contains("anglais"));
    r.controller.tick();
    assert_eq!(r.warm_up_requests().len(), before + 1, "une seule fois");
}

#[test]
fn a_shortcut_right_after_shift_never_loads_the_translation_model() {
    let r = rig_with(local_settings(), "x", "x", true);
    r.key_down();
    r.shift_down();
    let before = r.warm_up_requests().len();
    r.controller.handle_key(KeyEvent::Changed { changed: Key::Vk(0x41), down: true, held: vec![Key::RightControl, Key::Vk(0x41)] });
    r.clock.advance(Duration::from_secs(1));
    r.controller.tick();
    assert_eq!(r.warm_up_requests().len(), before);
}

#[test]
fn without_a_cleanup_engine_shift_does_not_translate() {
    let settings = AppSettings { cleanup_engine: CleanupEngine::None, ..local_settings() };
    let r = rig_with(settings, "bonjour tout le monde on commence", "x", true);
    r.key_down();
    r.shift_down();
    assert!(r.indicator.translations().is_empty());
    r.key_up();
    assert_eq!(r.paster.pasted()[0].0, "bonjour tout le monde on commence");
    assert_eq!(r.history.items()[0].translation, None);
}

#[test]
fn a_failed_translation_pastes_the_french_text_and_says_so() {
    let r = rig("bonjour tout le monde on commence", "x");
    r.cleanup_fails.store(true, Ordering::SeqCst);
    r.key_down();
    r.shift_down();
    r.key_up();
    assert_eq!(r.paster.pasted()[0].0, "bonjour tout le monde on commence");
    let last = r.indicator.last().unwrap();
    assert!(matches!(&last, IndicatorState::Message(m) if m.starts_with("Collé en français, sans traduction")), "{last:?}");
    assert_eq!(r.history.items()[0].timing.as_ref().unwrap().outcome, Cleanup::Failed);
    assert_eq!(r.history.items()[0].translation, None);
    assert_eq!(r.indicator.translations().last(), Some(&None));
}

#[test]
fn a_silent_microphone_is_flagged_after_two_seconds_and_the_flag_clears() {
    let r = rig_with(local_settings(), "x", "x", true);
    r.recorder.set_recording(5.0, 0.0005);
    r.key_down();
    r.clock.advance(Duration::from_millis(1500));
    r.controller.tick();
    assert!(r.indicator.mic_levels().is_empty(), "pas avant 2 s");
    r.clock.advance(Duration::from_millis(600));
    r.controller.tick();
    assert_eq!(r.indicator.mic_levels(), vec![scriibe_core::microphone::MicLevel::Silent]);
    // La voix revient.
    r.recorder.set_recording(5.0, 0.05);
    r.controller.tick();
    r.controller.tick();
    assert_eq!(r.indicator.mic_levels(), vec![scriibe_core::microphone::MicLevel::Silent, scriibe_core::microphone::MicLevel::Ok]);
}

#[test]
fn the_level_meter_follows_the_microphone() {
    let r = rig("x", "x");
    r.key_down();
    r.recorder.emit_level(0.4);
    r.recorder.emit_level(0.9);
    assert_eq!(r.indicator.levels(), vec![0.4, 0.9]);
}

#[test]
fn a_paste_that_cannot_be_done_keeps_the_text_in_the_clipboard_and_says_so() {
    let r = rig("bonjour tout le monde on commence", "Bonjour tout le monde, on commence.");
    r.paster.set_outcome(PasteOutcome::CopiedOnly("fenêtre en administrateur".into()));
    r.dictate();
    assert_eq!(r.indicator.last(), Some(message("Copié, à coller avec Ctrl+V — fenêtre en administrateur")));
    assert_eq!(r.history.items().len(), 1, "la dictée est gardée même si le collage a échoué");
}

#[test]
fn the_pause_media_setting_reaches_the_media_control() {
    let r = rig_with(AppSettings { pause_media: true, ..local_settings() }, "x", "x", true);
    r.key_down();
    r.key_up();
    assert_eq!(r.media.calls(), vec!["pause(true)", "resume"]);
}

#[test]
fn restoring_the_clipboard_follows_the_setting() {
    let r = rig_with(
        AppSettings { restore_clipboard: false, ..local_settings() },
        "bonjour tout le monde on commence",
        "Bonjour tout le monde, on commence.",
        true,
    );
    r.dictate();
    assert!(!r.paster.pasted()[0].1);
}

#[test]
fn a_hotkey_changed_in_the_settings_applies_to_the_next_event() {
    let r = rig("x", "x");
    r.config.change(|s| s.hotkey = Hotkey::RightCommand);
    // Ctrl droit ne dicte plus.
    r.key_down();
    assert!(!r.controller.is_recording());
    // ⌘ droite (Mac) dicte.
    r.controller.handle_key(KeyEvent::Changed { changed: Key::RightCommand, down: true, held: vec![Key::RightCommand] });
    assert!(r.controller.is_recording());
}

/// Les touches choisies dans les réglages ne s'appliquent que sous Windows (les identifiants sont ceux de ses codes de touche).
#[cfg(windows)]
#[test]
fn a_chosen_key_combination_dictates_and_the_default_key_no_longer_does() {
    let r = rig("x", "x");
    r.config.change(|s| s.dictation_keys = vec!["left_control".into(), "f9".into()]);
    r.key_down();
    assert!(!r.controller.is_recording(), "Ctrl droit ne dicte plus");
    let f9 = Key::Vk(0x78);
    r.controller.handle_key(KeyEvent::Changed { changed: Key::LeftControl, down: true, held: vec![Key::LeftControl] });
    assert!(!r.controller.is_recording(), "Ctrl gauche seul non plus");
    r.controller.handle_key(KeyEvent::Changed { changed: f9, down: true, held: vec![Key::LeftControl, f9] });
    assert!(r.controller.is_recording());
}

#[test]
fn a_recording_from_a_file_goes_through_the_same_chain() {
    let r = rig("bonjour tout le monde on commence", "Bonjour tout le monde, on commence.");
    let wav = scriibe_platform::audio::wav(&vec![1000i16; 32_000]);
    let recording = scriibe_platform::audio::read_wav(&wav).unwrap();
    r.controller.process_recording(recording, false, "fichier");
    assert_eq!(r.paster.pasted()[0].0, "Bonjour tout le monde, on commence.");
    assert_eq!(r.history.items().len(), 1);
    assert!(r.recorder.starts().is_empty(), "le micro n'est pas ouvert");
    assert!(r.world.log.lines().join("\n").contains("micro : fichier"));
}
