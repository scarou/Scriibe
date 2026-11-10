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
use scriibe_pipeline::{CommandSink, Controller, Deps, Executor, InlineExecutor};
use scriibe_platform::apps::AppInfo;
use scriibe_platform::indicator::IndicatorState;
use scriibe_platform::keys::KeyEvent;
use scriibe_platform::paste::{PasteOutcome, SelectedText, Unreadable};
use scriibe_platform::testing::{
    FakeApps, FakeConfig, FakeIndicator, FakeMedia, FakePaster, FakeRecorder, FakeSelected, FakeSounds, MemoryHistory, StaticSecrets,
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

/// Ce que les commandes vocales font de la sélection : elles la gardent ici, pour que les tests la voient arriver.
#[derive(Default)]
struct FakeCommands {
    heard: Mutex<Vec<String>>,
    fail_with: Mutex<Option<&'static str>>,
}

impl CommandSink for FakeCommands {
    fn add_to_dictionary(&self, heard: SelectedText) -> Result<(), &'static str> {
        if let Some(why) = *self.fail_with.lock().unwrap() {
            return Err(why);
        }
        self.heard.lock().unwrap().push(heard.expose().to_string());
        Ok(())
    }
}

struct Rig {
    commands: Arc<FakeCommands>,
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
    /// L'appli au premier plan (modifiable).
    apps: Arc<FakeApps>,
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
    let apps = Arc::new(FakeApps::new(AppInfo { name: "notepad".into(), id: Some("notepad.exe".into()) }));
    let recorder = Arc::new(FakeRecorder::default());
    let paster = Arc::new(FakePaster::default());
    let media = Arc::new(FakeMedia::default());
    let indicator = Arc::new(FakeIndicator::default());
    let sounds = Arc::new(FakeSounds::default());
    let history = Arc::new(MemoryHistory::default());
    let config = Arc::new(FakeConfig::new(settings));
    let executor = Arc::new(ManualExecutor::default());
    let commands = Arc::new(FakeCommands::default());
    let executor_dyn: Arc<dyn Executor> = if inline { Arc::new(InlineExecutor) } else { executor.clone() };
    let wall: Arc<dyn Fn() -> DateTime<Utc> + Send + Sync> = Arc::new(|| DateTime::from_timestamp(1_790_000_000, 0).unwrap());
    let controller = Controller::new(Deps {
        recorder: recorder.clone(),
        paster: paster.clone(),
        media: media.clone(),
        apps: apps.clone(),
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
        commands: commands.clone(),
    });
    Rig { commands, world, clock, controller, recorder, paster, media, indicator, sounds, history, config, executor, cleanup_fails, apps }
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
    assert_eq!(r.paster.pasted(), vec![("Bonjour tout le monde, on se voit demain. ".to_string(), true)]);
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
    assert_eq!(d.mode.as_deref(), Some("Simple"), "le mode en cours est gardé avec la dictée");
    let timing = d.timing.as_ref().unwrap();
    assert_eq!(timing.outcome, Cleanup::Model);
    assert!((timing.transcription - 0.3).abs() < 1e-6, "transcription {}", timing.transcription);
    assert!((timing.cleanup - 0.35).abs() < 1e-6, "mise au propre {}", timing.cleanup);
    assert!((timing.total - 0.65).abs() < 1e-6, "total {}", timing.total);
    let log = r.world.log.lines().join("\n");
    assert!(log.contains("transcription 0.30 s (deepgram)") && log.contains("total après relâche 0.65 s"), "{log}");
    assert!(log.contains("collage: 42 signes envoyés à notepad (à l'appui : notepad)"), "{log}");
}

#[test]
fn text_replacing_a_selection_has_no_capital_and_no_final_period() {
    let r = rig("euh bonjour tout le monde on se voit demain", "Bonjour tout le monde, on se voit demain.");
    r.paster.set_selection(true);
    r.dictate();
    assert_eq!(r.paster.pasted()[0].0, "bonjour tout le monde, on se voit demain");
    assert_eq!(r.history.items()[0].final_text, "bonjour tout le monde, on se voit demain", "l'historique garde ce qui a été collé");
    let log = r.world.log.lines().join("\n");
    assert!(log.contains("sur une sélection"), "{log}");
}

#[test]
fn without_a_selection_the_text_is_pasted_as_written() {
    let r = rig("euh bonjour tout le monde on se voit demain", "Bonjour tout le monde, on se voit demain.");
    r.dictate();
    assert_eq!(r.paster.pasted()[0].0, "Bonjour tout le monde, on se voit demain. ");
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
    assert_eq!(r.paster.pasted()[0].0, "Tu viens ce soir\u{a0}? On mange à 19\u{a0}h. ");
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
    assert_eq!(r.paster.pasted()[0].0, "Bonjour tout le monde on commence. ", "le texte brut est collé");
    let last = r.indicator.last().unwrap();
    assert!(matches!(&last, IndicatorState::Message(m) if m.starts_with("Collé sans mise au propre")), "{last:?}");
    assert_eq!(r.history.items()[0].timing.as_ref().unwrap().outcome, Cleanup::Failed);
    assert!(!r.controller.is_busy());
}

#[test]
fn a_rewrite_the_guard_rejects_leaves_the_raw_text() {
    let r = rig("bonjour tout le monde on commence la réunion", "Je suis un assistant, voici une réponse sans rapport avec la dictée.");
    r.dictate();
    assert_eq!(r.paster.pasted()[0].0, "bonjour tout le monde on commence la réunion ");
    assert_eq!(r.history.items()[0].timing.as_ref().unwrap().outcome, Cleanup::Rejected);
    let last = r.indicator.last().unwrap();
    assert!(matches!(&last, IndicatorState::Message(m) if m.contains("Reformulation écartée")), "{last:?}");
}

#[test]
fn three_words_or_fewer_skip_the_model_in_simple_mode() {
    let r = rig("bonjour à tous", "ne doit pas être utilisé");
    r.dictate();
    assert!(r.cleanup_requests().is_empty(), "aucun appel au modèle");
    assert_eq!(r.paster.pasted()[0].0, "bonjour à tous ");
    assert_eq!(r.history.items()[0].timing.as_ref().unwrap().outcome, Cleanup::Short);
}

#[test]
fn a_spelled_code_cut_by_a_decimal_comma_is_pasted_joined_and_the_history_keeps_what_was_heard() {
    // Dictée réelle du 03/11/2026 : « zéro neuf deux six F A 6 » écrit « 0,926FA6 » par Whisper (docs/ECARTS.md).
    let r = rig("0,926FA6", "ne doit pas être utilisé");
    r.dictate();
    assert_eq!(r.paster.pasted()[0].0, "0926FA6 ");
    assert_eq!(r.history.items()[0].raw_text, "0,926FA6");
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
    assert_eq!(r.paster.pasted()[0].0, "voir le projet Scriibe demain matin ");
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
    assert_eq!(r.paster.pasted()[0].0, "On parle de Scriibe ce matin avec lui. ");
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
fn a_changed_model_or_mode_reloads_the_preload_once_the_change_is_stable() {
    let r = rig_with(local_settings(), "x", "x", true);
    let tick = |millis: u64| {
        r.clock.advance(Duration::from_millis(millis));
        r.controller.tick();
        r.executor.run_all("prechargement");
    };
    tick(600); // premier relevé : celui du lancement
    tick(600);
    assert!(r.warm_up_requests().is_empty(), "rien n'a changé : rien n'est relancé");
    r.config.change(|s| s.rewrite_mode = scriibe_core::modes::RewriteMode::Formal);
    tick(600); // vu pour la première fois
    tick(600);
    assert!(r.warm_up_requests().is_empty(), "le changement doit durer 1,5 s");
    r.config.change(|s| s.rewrite_mode = scriibe_core::modes::RewriteMode::Tech);
    tick(600); // un autre choix repart de zéro
    tick(600);
    assert!(r.warm_up_requests().is_empty(), "un autre choix repart de zéro");
    tick(1000);
    assert_eq!(r.warm_up_requests().len(), 2, "mise au propre puis traduction rechargées, une seule fois");
    tick(600);
    tick(1600);
    assert_eq!(r.warm_up_requests().len(), 2, "plus de changement : plus de rechargement");
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
    assert_eq!(r.paster.pasted()[0].0, "Bonjour tout le monde, on commence. ");
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
    assert_eq!(r.paster.pasted()[0].0, "Hello everyone, let's get started. ");
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
    assert_eq!(r.paster.pasted()[0].0, "Bonjour tout le monde, on commence. ");
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
    assert_eq!(r.paster.pasted()[0].0, "bonjour tout le monde on commence ");
    assert_eq!(r.history.items()[0].translation, None);
}

#[test]
fn a_failed_translation_pastes_the_french_text_and_says_so() {
    let r = rig("bonjour tout le monde on commence", "x");
    r.cleanup_fails.store(true, Ordering::SeqCst);
    r.key_down();
    r.shift_down();
    r.key_up();
    assert_eq!(r.paster.pasted()[0].0, "bonjour tout le monde on commence ");
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
    let shortcut = if cfg!(target_os = "macos") { "⌘V" } else { "Ctrl+V" };
    assert_eq!(r.indicator.last(), Some(message(&format!("Copié, à coller avec {shortcut} — fenêtre en administrateur"))));
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
fn a_capture_notes_the_keys_held_and_does_not_dictate() {
    let r = rig("x", "x");
    r.controller.start_capture();
    assert_eq!(r.controller.capture().map(|c| (c.keys.len(), c.finished)), Some((0, false)));
    // Ctrl droit (la touche par défaut) ne dicte pas pendant la capture.
    r.key_down();
    assert!(!r.controller.is_recording());
    let f9 = Key::Vk(0x78);
    r.controller.handle_key(KeyEvent::Changed { changed: f9, down: true, held: vec![Key::RightControl, f9] });
    r.controller.handle_key(KeyEvent::Changed { changed: f9, down: false, held: vec![Key::RightControl] });
    assert!(!r.controller.capture().unwrap().finished, "une touche est encore tenue");
    r.controller.handle_key(KeyEvent::Changed { changed: Key::RightControl, down: false, held: vec![] });
    let capture = r.controller.capture().unwrap();
    assert!(capture.finished);
    assert_eq!(capture.keys, vec![Key::RightControl, f9]);
    // Ce qui suit ne change plus le résultat, et rien n'a dicté.
    r.controller.handle_key(KeyEvent::Changed { changed: Key::LeftShift, down: true, held: vec![Key::LeftShift] });
    assert_eq!(r.controller.capture().unwrap().keys.len(), 2);
    assert!(!r.controller.is_recording());
    r.controller.stop_capture();
    assert!(r.controller.capture().is_none());
    r.key_down();
    assert!(r.controller.is_recording(), "la touche dicte de nouveau");
}

/// Une combinaison de touches de modification seules n'ouvre le micro qu'après une courte attente (Windows : les touches réglées).
#[cfg(windows)]
#[test]
fn a_modifier_only_combination_waits_before_listening_and_a_shortcut_never_opens_the_microphone() {
    let r = rig("x", "x");
    r.config.change(|s| s.dictation_keys = vec!["left_control".into(), "left_win".into()]);
    let (control, win) = (Key::LeftControl, Key::Vk(0x5B));
    // Un raccourci : une autre touche arrive avant la fin de l'attente.
    r.controller.handle_key(KeyEvent::Changed { changed: control, down: true, held: vec![control] });
    r.controller.handle_key(KeyEvent::Changed { changed: win, down: true, held: vec![control, win] });
    assert!(!r.controller.is_recording(), "pas tout de suite");
    r.controller.handle_key(KeyEvent::Changed { changed: Key::Vk(0x41), down: true, held: vec![control, win, Key::Vk(0x41)] });
    std::thread::sleep(std::time::Duration::from_millis(900));
    assert!(!r.controller.is_recording(), "raccourci : le micro ne s'est pas ouvert");
    // Les touches sont relâchées, puis tenues assez longtemps : l'écoute commence.
    r.controller.handle_key(KeyEvent::Changed { changed: control, down: false, held: vec![win] });
    r.controller.handle_key(KeyEvent::Changed { changed: win, down: false, held: vec![] });
    r.controller.handle_key(KeyEvent::Changed { changed: control, down: true, held: vec![control] });
    r.controller.handle_key(KeyEvent::Changed { changed: win, down: true, held: vec![control, win] });
    std::thread::sleep(std::time::Duration::from_millis(900));
    assert!(r.controller.is_recording(), "tenue assez longtemps : l'écoute a commencé");
    r.controller.handle_key(KeyEvent::Changed { changed: win, down: false, held: vec![control] });
    assert!(!r.controller.is_recording());
}

#[test]
fn a_recording_from_a_file_goes_through_the_same_chain() {
    let r = rig("bonjour tout le monde on commence", "Bonjour tout le monde, on commence.");
    let wav = scriibe_platform::audio::wav(&vec![1000i16; 32_000]);
    let recording = scriibe_platform::audio::read_wav(&wav).unwrap();
    r.controller.process_recording(recording, false, "fichier");
    assert_eq!(r.paster.pasted()[0].0, "Bonjour tout le monde, on commence. ");
    assert_eq!(r.history.items().len(), 1);
    assert!(r.recorder.starts().is_empty(), "le micro n'est pas ouvert");
    assert!(r.world.log.lines().join("\n").contains("micro : fichier"));
}

/// Ctrl droit (la touche par défaut), telle que la page le relaie.
const RIGHT_CONTROL_VK: Key = Key::RightControl;

/// Dans la fenêtre de Scriibe, le crochet du système ne reçoit rien : la page relaie les touches et elles dictent.
#[test]
fn keys_relayed_by_the_page_dictate_when_the_system_hook_is_silent() {
    let r = rig("x", "x");
    assert!(r.controller.handle_page_key(RIGHT_CONTROL_VK, true));
    assert!(r.controller.is_recording());
    // Une répétition de la touche tenue ne change rien.
    assert!(!r.controller.handle_page_key(RIGHT_CONTROL_VK, true));
    assert!(r.controller.handle_page_key(RIGHT_CONTROL_VK, false));
    assert!(!r.controller.is_recording());
    let log = r.world.log.lines().join("\n");
    assert_eq!(log.matches("touches: relayées par la page").count(), 1, "le journal le dit une seule fois");
}

/// Quand le crochet parle (autre fenêtre, ou page ouverte dans un navigateur), la page ne double pas les événements.
#[test]
fn keys_relayed_by_the_page_are_ignored_while_the_system_hook_speaks() {
    let r = rig("x", "x");
    r.key_down();
    assert!(r.controller.is_recording());
    assert!(!r.controller.handle_page_key(RIGHT_CONTROL_VK, true), "doublon du crochet");
    r.key_up();
    assert!(!r.controller.handle_page_key(RIGHT_CONTROL_VK, false));
    assert!(!r.controller.is_recording());
    // Le crochet se tait depuis plus d'une seconde (la personne a changé de fenêtre) : la page reprend.
    r.clock.advance(Duration::from_millis(1100));
    assert!(r.controller.handle_page_key(RIGHT_CONTROL_VK, true));
    assert!(r.controller.is_recording());
}

#[test]
fn keys_relayed_by_the_page_do_not_dictate_during_a_capture() {
    let r = rig("x", "x");
    r.controller.start_capture();
    assert!(!r.controller.handle_page_key(RIGHT_CONTROL_VK, true));
    assert_eq!(r.controller.capture().map(|c| c.keys.len()), Some(0), "la capture a son propre relais");
    assert!(!r.controller.is_recording());
}

/// La fenêtre perd le focus pendant l'appui : le relâchement ne sera jamais vu, la dictée doit se terminer.
#[test]
fn losing_the_focus_releases_the_keys_relayed_by_the_page() {
    let r = rig("x", "x");
    assert!(r.controller.handle_page_key(RIGHT_CONTROL_VK, true));
    assert!(r.controller.is_recording());
    r.controller.release_page_keys();
    assert!(!r.controller.is_recording());
    // Sans touche tenue, ne fait rien.
    r.controller.release_page_keys();
}

// --- Espace entre deux dictées (docs/ESPACEMENT.md). La règle elle-même est testée dans `scriibe_core::spacing` ;
// ici, le branchement : espace après le collage, historique intact, sélection, collage manqué, appli inconnue.

/// Deux dictées à la suite (la seconde `delay` secondes après la fin de la première) : ce qui a été collé.
fn two_dictations(r: &Rig, delay: u64) -> Vec<String> {
    r.dictate();
    r.clock.advance(Duration::from_secs(delay));
    r.dictate();
    r.paster.pasted().into_iter().map(|(text, _)| text).collect()
}

#[test]
fn spacing_every_dictation_is_pasted_with_a_trailing_space_but_the_history_is_not() {
    let r = rig("bonjour tout le monde on commence", "Bonjour tout le monde, on commence.");
    let pasted = two_dictations(&r, 10);
    assert_eq!(pasted, vec!["Bonjour tout le monde, on commence. ", "Bonjour tout le monde, on commence. "]);
    assert!(pasted.iter().all(|p| !p.starts_with(' ')), "rien n'est ajouté devant la dictée suivante");
    let items = r.history.items();
    assert_eq!(items.len(), 2);
    assert!(items.iter().all(|d| d.final_text == "Bonjour tout le monde, on commence."), "l'historique n'a pas l'espace ajoutée");
    let log = r.world.log.lines().join("\n");
    assert!(log.contains("collage: 36 signes"), "{log}");
}

#[test]
fn spacing_the_space_does_not_depend_on_the_app_nor_on_the_delay() {
    // Sous Linux (Wayland) l'appli est « Inconnue » : la règle ne la regarde pas.
    let r = rig("bonjour tout le monde on commence", "Bonjour tout le monde, on commence.");
    r.apps.set(AppInfo { name: "Inconnue".into(), id: None });
    let pasted = two_dictations(&r, 600);
    assert_eq!(pasted, vec!["Bonjour tout le monde, on commence. ", "Bonjour tout le monde, on commence. "]);
}

#[test]
fn spacing_a_translation_into_english_gets_the_space_too() {
    let r = rig("bonjour tout le monde on commence", "Hello everyone, let's get started.");
    r.key_down();
    r.shift_down();
    r.key_up();
    r.key_down();
    r.shift_down();
    r.key_up();
    let pasted = r.paster.pasted();
    assert_eq!(pasted[0].0, "Hello everyone, let's get started. ");
    assert_eq!(pasted[1].0, "Hello everyone, let's get started. ");
    assert_eq!(r.history.items()[1].final_text, "Hello everyone, let's get started.");
}

#[test]
fn spacing_a_selection_replacement_gets_no_space() {
    let r = rig("bonjour tout le monde on commence", "Bonjour tout le monde, on commence.");
    r.paster.set_selection(true);
    r.dictate();
    assert_eq!(r.paster.pasted()[0].0, "bonjour tout le monde, on commence");
}

#[test]
fn spacing_a_paste_that_only_copied_gets_the_same_text_and_the_next_one_is_unchanged() {
    let r = rig("bonjour tout le monde on commence", "Bonjour tout le monde, on commence.");
    r.paster.set_outcome(PasteOutcome::CopiedOnly("fenêtre en administrateur".into()));
    r.dictate();
    r.paster.set_outcome(PasteOutcome::Pasted);
    r.dictate();
    assert_eq!(r.paster.pasted()[1].0, "Bonjour tout le monde, on commence. ");
}

#[test]
fn spacing_a_model_that_writes_no_final_stop_does_not_glue_the_phrases() {
    // Essai de l'auteur : Whisper « small » n'écrit pas toujours de point final.
    let r = rig("bonjour tout le monde on commence", "Bonjour tout le monde on commence");
    let pasted = two_dictations(&r, 10);
    assert_eq!(pasted, vec!["Bonjour tout le monde on commence ", "Bonjour tout le monde on commence "]);
    assert!(r.history.items().iter().all(|d| d.final_text == "Bonjour tout le monde on commence"));
}

// ---- Commandes vocales sur du texte sélectionné (docs/COMMANDES.md) ----

const SECRET: &str = "SECRET-123 mot";

/// Cas 1 et 2 : phrase de commande (avec ponctuation et majuscule) sur une sélection → la commande s'exécute, rien n'est collé ni gardé.
#[test]
fn commands_101_the_phrase_on_a_selection_adds_it_to_the_dictionary_and_pastes_nothing() {
    let r = rig("Ajoute au Dictionnaire !", "ne doit pas servir");
    r.paster.set_selected(FakeSelected::Text("cri be".into()));
    r.dictate();
    assert_eq!(*r.commands.heard.lock().unwrap(), vec!["cri be".to_string()]);
    assert!(r.paster.pasted().is_empty(), "rien n'est collé");
    assert!(r.history.items().is_empty(), "une commande n'entre pas dans l'historique");
    assert!(r.cleanup_requests().is_empty(), "le modèle de mise au propre n'est pas appelé");
    assert!(r.indicator.states().contains(&message("Ajouter au dictionnaire — complète « Écrit »")), "{:?}", r.indicator.states());
    assert!(!r.controller.is_busy());
}

/// Cas 3 : une phrase plus longue n'est pas une commande ; la dictée remplace la sélection, et la sélection n'est même pas lue.
#[test]
fn commands_103_a_longer_sentence_is_dictated_and_the_selection_is_not_read() {
    let r = rig("ajouter au dictionnaire le mot cri be", "Ajouter au dictionnaire le mot cri be.");
    r.paster.set_selected(FakeSelected::Text("cri be".into()));
    r.dictate();
    assert_eq!(r.paster.pasted().len(), 1);
    assert_eq!(r.paster.selected_reads(), 0, "pas de ⌘C pour une dictée qui n'est pas une commande");
    assert!(r.commands.heard.lock().unwrap().is_empty());
}

/// Une dictée ordinaire ne lit jamais la sélection (pas de ⌘C inutile).
#[test]
fn commands_101b_an_ordinary_dictation_never_reads_the_selection() {
    let r = rig("bonjour tout le monde", "Bonjour tout le monde.");
    r.paster.set_selected(FakeSelected::Text("cri be".into()));
    r.dictate();
    assert_eq!(r.paster.selected_reads(), 0);
    assert_eq!(r.paster.pasted().len(), 1);
}

/// Cas 4 : sans sélection, la phrase est collée comme du texte, sans message de commande.
#[test]
fn commands_104_without_a_selection_the_phrase_is_pasted_as_text() {
    let r = rig("Ajouter au dictionnaire", "Ajouter au dictionnaire.");
    r.dictate();
    assert_eq!(r.paster.selected_reads(), 1);
    assert_eq!(
        r.paster.pasted(),
        vec![("Ajouter au dictionnaire ".to_string(), true)],
        "la phrase est collée telle quelle, comme un texte"
    );
    assert_eq!(r.history.items().len(), 1);
    assert!(r.commands.heard.lock().unwrap().is_empty());
    assert_eq!(r.indicator.states().last(), Some(&IndicatorState::Done));
}

/// Cas 5 : en anglais, avec la langue anglaise.
#[test]
fn commands_105_the_english_phrase_works_with_the_english_language() {
    let settings = AppSettings { language: scriibe_core::settings::DictationLanguage::En, ..local_settings() };
    let r = rig_with(settings, "Add to dictionary.", "x", true);
    r.paster.set_selected(FakeSelected::Text("scree bee".into()));
    r.dictate();
    assert_eq!(*r.commands.heard.lock().unwrap(), vec!["scree bee".to_string()]);
}

/// Cas 6 : une phrase choisie dans les réglages remplace celles par défaut ; l'interrupteur général coupe tout.
#[test]
fn commands_106_custom_phrase_and_the_general_switch() {
    let mut settings = local_settings();
    settings.voice_commands.fr.insert("dictionnaire.ajouter".into(), vec!["mémorise ça".into()]);
    let r = rig_with(settings.clone(), "Mémorise ça.", "x", true);
    r.paster.set_selected(FakeSelected::Text("cri be".into()));
    r.dictate();
    assert_eq!(*r.commands.heard.lock().unwrap(), vec!["cri be".to_string()]);

    let r = rig_with(settings, "Ajouter au dictionnaire", "Ajouter au dictionnaire.", true);
    r.paster.set_selected(FakeSelected::Text("cri be".into()));
    r.dictate();
    assert!(r.commands.heard.lock().unwrap().is_empty(), "l'ancienne phrase ne marche plus");

    let off = AppSettings { voice_commands_enabled: false, ..local_settings() };
    let r = rig_with(off, "Ajouter au dictionnaire", "Ajouter au dictionnaire.", true);
    r.paster.set_selected(FakeSelected::Text("cri be".into()));
    r.dictate();
    assert_eq!((r.paster.selected_reads(), r.paster.pasted().len()), (0, 1), "interrupteur coupé : texte ordinaire");
}

/// Cas 7 : presse-papiers non textuel (ou autre lecture impossible) → la commande n'est pas faite, la phrase est collée, la pastille le dit.
#[test]
fn commands_107_an_unreadable_selection_pastes_the_phrase_and_says_why() {
    let r = rig("Ajouter au dictionnaire", "Ajouter au dictionnaire.");
    r.paster.set_selected(FakeSelected::Unreadable(Unreadable::ClipboardNotText));
    r.dictate();
    assert_eq!(r.paster.pasted().len(), 1);
    assert!(r.commands.heard.lock().unwrap().is_empty());
    let states = r.indicator.states();
    assert!(
        states
            .contains(&message("Commande non faite : le presse-papiers contient autre chose que du texte — phrase collée comme du texte")),
        "{states:?}"
    );
}

/// Une sélection trop longue ou sur plusieurs lignes ne va pas au dictionnaire : rien n'est collé (la sélection reste intacte).
#[test]
fn commands_108_a_long_or_multiline_selection_is_refused_without_pasting() {
    for selected in ["un\ndeux", &"a".repeat(81)] {
        let r = rig("Ajouter au dictionnaire", "x");
        r.paster.set_selected(FakeSelected::Text(selected.to_string()));
        r.dictate();
        assert!(r.paster.pasted().is_empty() && r.commands.heard.lock().unwrap().is_empty());
        assert!(r.indicator.states().iter().any(|s| matches!(s, IndicatorState::Message(m) if m.starts_with("Sélection trop longue"))));
    }
}

/// Une erreur de la page (fenêtre impossible à ouvrir) est dite par un texte fixe.
#[test]
fn commands_109_a_failure_of_the_window_is_said_with_a_fixed_message() {
    let r = rig("Ajouter au dictionnaire", "x");
    *r.commands.fail_with.lock().unwrap() = Some("Scriibe n'a pas pu ouvrir sa fenêtre");
    r.paster.set_selected(FakeSelected::Text(SECRET.into()));
    r.dictate();
    assert!(r.paster.pasted().is_empty());
    assert!(r.indicator.states().contains(&message("Scriibe n'a pas pu ouvrir sa fenêtre")));
}

/// Cas 8 : aucune trace de la sélection dans le journal, l'historique, la pastille, ce qui est collé ou les requêtes envoyées aux services.
#[test]
fn commands_110_no_trace_of_the_selection_anywhere() {
    for failure in [None, Some("Scriibe n'a pas pu ouvrir sa fenêtre")] {
        let r = rig("Ajouter au dictionnaire", "x");
        *r.commands.fail_with.lock().unwrap() = failure;
        r.paster.set_selected(FakeSelected::Text(SECRET.into()));
        r.dictate();
        let log = r.world.log.lines().join("\n");
        assert!(log.contains("commande: dictionnaire.ajouter (sélection : 14 signes)"), "le journal ne garde que la longueur : {log}");
        let everything = format!(
            "{log}|{:?}|{:?}|{:?}|{:?}",
            r.history.items(),
            r.indicator.states(),
            r.paster.pasted(),
            r.world.requests().iter().map(|q| String::from_utf8_lossy(&q.body).to_string() + &q.url).collect::<Vec<_>>()
        );
        assert!(!everything.contains("SECRET"), "la sélection a laissé une trace : {everything}");
    }
    // Lecture impossible : la raison est un texte fixe, pas la sélection.
    let r = rig("Ajouter au dictionnaire", "x");
    r.paster.set_selected(FakeSelected::Unreadable(Unreadable::NotTrusted));
    r.dictate();
    assert!(r.world.log.lines().join("\n").contains("autorisation Accessibilité manquante"));
    // Le type lui-même ne montre pas son contenu.
    assert!(!format!("{:?}", SelectedText::new(SECRET)).contains("SECRET"));
}

// ---- Commandes « caractères autour » (tâche 7, docs/COMMANDES.md) ----

/// Ce que la commande colle : un seul collage (donc un seul ⌘Z), qui remplace la sélection restée active.
fn surrounded(raw: &'static str, selected: &str, language: scriibe_core::settings::DictationLanguage) -> (Vec<String>, Rig) {
    let r = rig_with(AppSettings { language, ..local_settings() }, raw, "ne doit pas servir", true);
    r.paster.set_selected(FakeSelected::Text(selected.into()));
    r.dictate();
    (r.paster.pasted().into_iter().map(|(t, _)| t).collect(), r)
}

#[test]
fn commands_201_parentheses_replace_the_selection_with_one_paste() {
    use scriibe_core::settings::DictationLanguage::{En, Fr};
    let (pasted, r) = surrounded("Entre parenthèses.", "cri be", Fr);
    assert_eq!(pasted, vec!["(cri be)".to_string()], "un seul collage : un seul ⌘Z annule");
    assert!(r.history.items().is_empty(), "une commande n'entre pas dans l'historique");
    assert!(r.cleanup_requests().is_empty(), "le modèle de mise au propre n'est pas appelé");
    assert_eq!(r.paster.selected_reads(), 1);
    assert!(r.indicator.states().contains(&IndicatorState::Done), "{:?}", r.indicator.states());
    assert_eq!(surrounded("Put it in parentheses.", "cri be", En).0, vec!["(cri be)".to_string()]);
}

#[test]
fn commands_202_quotes_are_french_guillemets_or_english_marks_by_the_language_spoken() {
    use scriibe_core::settings::DictationLanguage::{En, Fr, Multi};
    assert_eq!(surrounded("Entre guillemets", "cri be", Fr).0, vec!["«\u{a0}cri be\u{a0}»".to_string()]);
    assert_eq!(surrounded("In quotes.", "cri be", En).0, vec!["\"cri be\"".to_string()]);
    // « Plusieurs langues » : la langue de la phrase dite décide, pas celle des réglages.
    assert_eq!(surrounded("Mets entre guillemets", "cri be", Multi).0, vec!["«\u{a0}cri be\u{a0}»".to_string()]);
    assert_eq!(surrounded("Put in quotation marks", "cri be", Multi).0, vec!["\"cri be\"".to_string()]);
}

#[test]
fn commands_203_brackets_and_braces() {
    use scriibe_core::settings::DictationLanguage::{En, Fr};
    assert_eq!(surrounded("Entre crochets", "cri be", Fr).0, vec!["[cri be]".to_string()]);
    assert_eq!(surrounded("In square brackets", "cri be", En).0, vec!["[cri be]".to_string()]);
    assert_eq!(surrounded("Entre accolades", "cri be", Fr).0, vec!["{cri be}".to_string()]);
    assert_eq!(surrounded("In curly brackets", "cri be", En).0, vec!["{cri be}".to_string()]);
}

#[test]
fn commands_204_blanks_at_the_edges_stay_outside_and_lines_are_kept() {
    use scriibe_core::settings::DictationLanguage::Fr;
    assert_eq!(surrounded("Entre parenthèses", "un mot ", Fr).0, vec!["(un mot) ".to_string()]);
    assert_eq!(surrounded("Entre crochets", "\n une ligne\net une autre\n", Fr).0, vec!["\n [une ligne\net une autre]\n".to_string()]);
}

#[test]
fn commands_205_without_a_selection_or_with_a_longer_sentence_the_phrase_is_dictated() {
    use scriibe_core::settings::DictationLanguage::Fr;
    let r = rig("Entre parenthèses", "Entre parenthèses.");
    r.dictate(); // aucune sélection
    assert_eq!(r.paster.pasted().len(), 1);
    assert_eq!(r.paster.pasted()[0].0, "Entre parenthèses ");
    let (pasted, r) = surrounded("Mets le tout entre parenthèses", "cri be", Fr);
    assert_eq!(pasted, vec!["Mets le tout entre parenthèses ".to_string()]);
    assert_eq!(r.paster.selected_reads(), 0, "pas de ⌘C pour une dictée qui n'est pas une commande");
}

#[test]
fn commands_206_a_blank_selection_is_not_wrapped() {
    use scriibe_core::settings::DictationLanguage::Fr;
    let (pasted, r) = surrounded("Entre parenthèses", "  \n ", Fr);
    assert!(pasted.is_empty());
    assert!(r.indicator.states().contains(&message("Rien à entourer (la sélection est vide)")), "{:?}", r.indicator.states());
}

#[test]
fn commands_207_an_unreadable_selection_pastes_the_phrase_and_says_why() {
    let r = rig("Entre guillemets", "Entre guillemets.");
    r.paster.set_selected(FakeSelected::Unreadable(Unreadable::NotTrusted));
    r.dictate();
    assert_eq!(r.paster.pasted().len(), 1);
    assert_eq!(r.paster.pasted()[0].0, "Entre guillemets ");
    assert!(r
        .indicator
        .states()
        .contains(&message("Commande non faite : autorisation Accessibilité manquante — phrase collée comme du texte")));
}

#[test]
fn commands_208_a_paste_that_only_copied_says_so() {
    let r = rig_with(local_settings(), "Entre parenthèses", "x", true);
    r.paster.set_outcome(PasteOutcome::CopiedOnly("fenêtre en administrateur".into()));
    r.paster.set_selected(FakeSelected::Text("cri be".into()));
    r.dictate();
    assert!(r.indicator.states().iter().any(|s| matches!(s, IndicatorState::Message(m) if m.starts_with("Copié, à coller avec"))));
}

/// Cas 8 : le journal ne garde que l'identifiant et la longueur ; ni historique, ni pastille, ni requête ne contiennent la sélection (le collage,
/// lui, la contient forcément : c'est son travail).
#[test]
fn commands_209_no_trace_of_the_selection_outside_the_paste() {
    use scriibe_core::settings::DictationLanguage::Fr;
    let (pasted, r) = surrounded("Entre guillemets", SECRET, Fr);
    assert_eq!(pasted, vec![format!("«\u{a0}{SECRET}\u{a0}»")]);
    let log = r.world.log.lines().join("\n");
    assert!(log.contains("commande: entourer.guillemets (sélection : 14 signes)"), "{log}");
    let everything = format!(
        "{log}|{:?}|{:?}|{:?}",
        r.history.items(),
        r.indicator.states(),
        r.world.requests().iter().map(|q| String::from_utf8_lossy(&q.body).to_string() + &q.url).collect::<Vec<_>>()
    );
    assert!(!everything.contains("SECRET"), "la sélection a laissé une trace : {everything}");
}

#[test]
fn commands_210_custom_phrase_replaces_the_default_for_a_wrapping_command() {
    let mut settings = local_settings();
    settings.voice_commands.fr.insert("entourer.parentheses".into(), vec!["parenthèse autour".into()]);
    let r = rig_with(settings.clone(), "Parenthèse autour.", "x", true);
    r.paster.set_selected(FakeSelected::Text("cri be".into()));
    r.dictate();
    assert_eq!(r.paster.pasted()[0].0, "(cri be)");
    let r = rig_with(settings, "Entre parenthèses", "Entre parenthèses.", true);
    r.paster.set_selected(FakeSelected::Text("cri be".into()));
    r.dictate();
    assert_eq!(r.paster.pasted()[0].0, "Entre parenthèses ", "l'ancienne phrase ne marche plus");
}

// ---- Gras, italique, souligné (tâche 8, docs/COMMANDES.md) ----

/// Ce que la commande fait : aucun collage, un raccourci appuyé (sous sa forme lue, « Ctrl+B » ici : ces tests ne tournent pas sur Mac seulement).
fn formatted(raw: &'static str, language: scriibe_core::settings::DictationLanguage, settings: AppSettings) -> Rig {
    let r = rig_with(AppSettings { language, ..settings }, raw, "ne doit pas servir", true);
    r.paster.set_selected(FakeSelected::Text("cri be".into()));
    r.dictate();
    r
}

fn mod_key(letter: char) -> String {
    scriibe_core::shortcut::Shortcut::parse(&format!("Mod+{letter}")).unwrap().to_string()
}

#[test]
fn commands_301_bold_italic_underline_press_the_app_shortcut_and_paste_nothing() {
    use scriibe_core::settings::DictationLanguage::{En, Fr};
    for (raw, language, letter) in [
        ("Mets en gras.", Fr, 'B'),
        ("Mets en italique", Fr, 'I'),
        ("Mets en souligné !", Fr, 'U'),
        ("Make it bold.", En, 'B'),
        ("Make it italic", En, 'I'),
        ("Underline it", En, 'U'),
    ] {
        let r = formatted(raw, language, local_settings());
        assert_eq!(r.paster.shortcuts(), vec![mod_key(letter)], "{raw}");
        assert!(r.paster.pasted().is_empty(), "rien n'est collé : le raccourci agit sur la sélection restée active ({raw})");
        assert!(r.history.items().is_empty(), "une commande n'entre pas dans l'historique");
        assert!(r.cleanup_requests().is_empty());
        assert!(r.indicator.states().contains(&IndicatorState::Done), "{:?}", r.indicator.states());
    }
}

#[test]
fn commands_302_the_shortcut_is_the_one_in_the_settings() {
    let mut settings = local_settings();
    settings.format_shortcuts.bold = "Ctrl+Alt+G".into();
    let r = formatted("Mets en gras", scriibe_core::settings::DictationLanguage::Fr, settings);
    let expected = scriibe_core::shortcut::Shortcut::parse("Ctrl+Alt+G").unwrap().to_string();
    assert_eq!(r.paster.shortcuts(), vec![expected]);
}

#[test]
fn commands_303_an_unreadable_shortcut_in_the_settings_is_said_and_nothing_is_sent() {
    let mut settings = local_settings();
    settings.format_shortcuts.bold = "G".into(); // construit à la main : le fichier de réglages le corrigerait à la lecture
    let r = formatted("Mets en gras", scriibe_core::settings::DictationLanguage::Fr, settings);
    assert!(r.paster.shortcuts().is_empty() && r.paster.pasted().is_empty());
    assert!(
        r.indicator.states().contains(&message("Raccourci invalide dans les réglages (exemple : Mod+B)")),
        "{:?}",
        r.indicator.states()
    );
}

#[test]
fn commands_304_a_shortcut_that_cannot_be_sent_says_why_with_a_fixed_text() {
    let r = rig_with(local_settings(), "Mets en gras", "x", true);
    r.paster.set_selected(FakeSelected::Text(SECRET.into()));
    r.paster.set_shortcut_error(Some("autorisation Accessibilité manquante"));
    r.dictate();
    assert!(r.paster.pasted().is_empty());
    let states = r.indicator.states();
    assert!(states.contains(&message("Raccourci non envoyé — autorisation Accessibilité manquante")), "{states:?}");
    assert!(!format!("{states:?}{:?}", r.world.log.lines()).contains("SECRET"));
}

#[test]
fn commands_305_without_a_selection_or_with_a_longer_sentence_the_phrase_is_dictated() {
    let r = rig("Mets en gras", "Mets en gras.");
    r.dictate();
    assert!(r.paster.shortcuts().is_empty());
    assert_eq!(r.paster.pasted().len(), 1);
    let r = rig("Mets en gras le titre", "Mets en gras le titre.");
    r.paster.set_selected(FakeSelected::Text("cri be".into()));
    r.dictate();
    assert!(r.paster.shortcuts().is_empty());
    assert_eq!(r.paster.selected_reads(), 0, "pas de ⌘C pour une dictée qui n'est pas une commande");
}

#[test]
fn commands_306_no_trace_of_the_selection_for_a_format_command() {
    let r = rig("Mets en italique", "x");
    r.paster.set_selected(FakeSelected::Text(SECRET.into()));
    r.dictate();
    let log = r.world.log.lines().join("\n");
    assert!(log.contains("commande: format.italique (sélection : 14 signes)"), "{log}");
    let everything = format!("{log}|{:?}|{:?}|{:?}|{:?}", r.history.items(), r.indicator.states(), r.paster.pasted(), r.paster.shortcuts());
    assert!(!everything.contains("SECRET"), "{everything}");
}

#[test]
fn commands_307_the_general_switch_turns_the_format_commands_off() {
    let off = AppSettings { voice_commands_enabled: false, ..local_settings() };
    let r = rig_with(off, "Mets en gras", "Mets en gras.", true);
    r.paster.set_selected(FakeSelected::Text("cri be".into()));
    r.dictate();
    assert_eq!((r.paster.selected_reads(), r.paster.shortcuts().len(), r.paster.pasted().len()), (0, 0, 1));
}

fn custom(id: &str, phrase: &str, shortcut: &str, needs_selection: bool) -> scriibe_core::commands::CustomCommand {
    scriibe_core::commands::CustomCommand {
        id: id.into(),
        name: "Mon raccourci".into(),
        phrases: vec![phrase.into()],
        shortcut: shortcut.into(),
        needs_selection,
    }
}

#[test]
fn commands_309_a_custom_command_presses_its_own_shortcut_on_a_selection() {
    let settings = AppSettings { custom_commands: vec![custom("perso.1", "appliquer le titre", "Mod+Alt+1", true)], ..local_settings() };
    let r = formatted("Appliquer le titre.", scriibe_core::settings::DictationLanguage::Fr, settings);
    let expected = scriibe_core::shortcut::Shortcut::parse("Mod+Alt+1").unwrap().to_string();
    assert_eq!(r.paster.shortcuts(), vec![expected]);
    assert!(r.paster.pasted().is_empty());
    assert!(r.history.items().is_empty());
}

#[test]
fn commands_310_a_custom_command_without_selection_requirement_acts_with_nothing_selected() {
    let settings = AppSettings { custom_commands: vec![custom("perso.2", "enregistrer le fichier", "Mod+S", false)], ..local_settings() };
    let r = rig_with(settings, "Enregistrer le fichier", "ne doit pas servir", true);
    r.paster.set_selected(FakeSelected::NoSelection);
    r.dictate();
    let expected = scriibe_core::shortcut::Shortcut::parse("Mod+S").unwrap().to_string();
    assert_eq!((r.paster.shortcuts(), r.paster.selected_reads(), r.paster.pasted().len()), (vec![expected], 0, 0));
}

#[test]
fn commands_311_a_custom_command_that_needs_a_selection_stays_text_without_one() {
    let settings = AppSettings { custom_commands: vec![custom("perso.1", "appliquer le titre", "Mod+Alt+1", true)], ..local_settings() };
    let r = rig_with(settings, "Appliquer le titre", "Appliquer le titre.", true);
    r.paster.set_selected(FakeSelected::NoSelection);
    r.dictate();
    assert!(r.paster.shortcuts().is_empty());
    assert_eq!(r.paster.pasted().len(), 1);
}

#[test]
fn libreoffice_mac_trailing_new_line_is_a_real_return_key() {
    let r = rig("bonjour retour à la ligne", "Bonjour, retour à la ligne.");
    r.apps.set(AppInfo { name: "LibreOffice".into(), id: Some("org.libreoffice.script".into()) });
    r.dictate();
    assert!(
        r.history.items()[0].final_text.ends_with('\n'),
        "le signe est bien rendu en saut de ligne : {:?}",
        r.history.items()[0].final_text
    );
    let pasted = r.paster.pasted();
    assert!(!pasted[0].0.ends_with('\n'), "Writer ignorerait ce saut de ligne final : {:?}", pasted[0].0);
    assert_eq!(r.paster.shortcuts(), vec!["Entrée".to_string()]);
}

#[test]
fn other_apps_keep_the_trailing_new_line_in_the_pasted_text() {
    let r = rig("bonjour retour à la ligne", "Bonjour, retour à la ligne.");
    r.dictate();
    assert!(r.paster.pasted()[0].0.ends_with('\n'));
    assert!(r.paster.shortcuts().is_empty(), "Entrée enverrait le message dans une messagerie");
}
