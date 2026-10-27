//! L'assemblage : les moteurs, la plateforme et le chef d'orchestre, branchés ensemble.

use scriibe_core::lang::LinguaGuesser;
use scriibe_core::settings::AppSettings;
use scriibe_engines::fs::{FileSystem, RealFs};
use scriibe_engines::http::UreqHttp;
use scriibe_engines::llama::{self, LlamaLauncher};
use scriibe_engines::runtime::{Clock, Log, RealSpawner, Secrets, SystemClock};
use scriibe_engines::server::{EngineServer, Runtime};
use scriibe_engines::services::{Cleaner, Transcriber};
use scriibe_engines::whisper::{self, Whisper, WhisperLauncher};
use scriibe_pipeline::{Controller, Deps, ThreadExecutor};
use scriibe_platform::indicator::Indicator;
use scriibe_platform::paste::Paster;
use scriibe_platform::storage::{data_folder, ConfigSource, FileConfig, FileHistory, FileLog, HistorySink};
use scriibe_platform::Platform;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct App {
    pub controller: Controller,
    pub platform: Platform,
    pub whisper: Arc<Whisper>,
    pub llama: Arc<EngineServer<LlamaLauncher>>,
    pub config: Arc<FileConfig>,
    pub log: Arc<FileLog>,
}

/// Où l'appli range les modèles (Whisper `ggml-*.bin`, GGUF) : `%APPDATA%\Scriibe\models`.
pub fn models_folder(data: &Path) -> PathBuf {
    data.join("models")
}

/// Les fichiers `ggml-*.bin` que `whisper-server` sait charger, le meilleur d'abord (« large-v3-turbo », puis « large », « medium »…).
pub fn detect_whisper_models(fs: &dyn FileSystem, folder: &Path) -> Vec<String> {
    let found: Vec<String> =
        fs.walk(folder).into_iter().map(|p| p.to_string_lossy().into_owned()).filter(|p| whisper::is_whisper_model(p)).collect();
    let mut models = whisper::distinct(&found, |p| std::fs::metadata(p).ok().map(|m| m.len()));
    models.sort_by(|a, b| whisper::score(b, None).cmp(&whisper::score(a, None)).then_with(|| a.cmp(b)));
    models
}

pub fn open_config() -> Arc<FileConfig> {
    Arc::new(FileConfig::new(data_folder()))
}

pub struct Choices {
    pub paster: Option<Arc<dyn Paster>>,
    pub history: Option<Arc<dyn HistorySink>>,
    pub indicator: Arc<dyn Indicator>,
}

pub fn build(config: Arc<FileConfig>, log: Arc<FileLog>, platform: Platform, choices: Choices) -> App {
    let folder = config.folder().to_path_buf();
    let models = models_folder(&folder);
    let fs: Arc<dyn FileSystem> = Arc::new(RealFs);
    let clock: Arc<SystemClock> = Arc::new(SystemClock::default());
    let log_dyn: Arc<dyn Log> = log.clone();
    let runtime =
        Runtime { http: Arc::new(UreqHttp::default()), spawner: Arc::new(RealSpawner), clock: clock.clone(), log: log_dyn.clone() };
    let path_var = std::env::var_os("PATH");

    let whisper = Arc::new(Whisper::new(WhisperLauncher::new(fs.clone(), whisper::PORT, path_var.as_deref()), runtime.clone()));
    // Le journal du moteur s'allonge à chaque requête : on le repart de zéro au lancement s'il est devenu gros.
    let llama_log = folder.join("llama-server.log");
    if std::fs::metadata(&llama_log).is_ok_and(|m| m.len() > 2_000_000) {
        let _ = std::fs::remove_file(&llama_log);
    }
    let llama_launcher = LlamaLauncher::new(fs.clone(), llama::PORT, Some(llama_log), path_var.as_deref());
    llama_launcher.set_device(&read_llama_device(&folder));
    let llama = Arc::new(llama::new_server(llama_launcher, runtime.clone()));

    platform.media.set_log(log_dyn.clone());
    let secrets: Arc<dyn Secrets> = platform.secrets.clone();
    let whisper_models = {
        let (fs, models) = (fs.clone(), models.clone());
        Arc::new(move || detect_whisper_models(fs.as_ref(), &models))
    };
    let transcriber = Arc::new(Transcriber {
        http: runtime.http.clone(),
        secrets: secrets.clone(),
        whisper: whisper.clone(),
        detected_whisper_models: whisper_models,
    });
    let llama_models = {
        let (fs, models) = (fs.clone(), models.clone());
        Arc::new(move || llama::detect_models(fs.as_ref(), std::slice::from_ref(&models), Some(&models)))
    };
    let cleaner = Arc::new(Cleaner {
        http: runtime.http.clone(),
        secrets,
        log: log_dyn.clone(),
        guesser: Arc::new(LinguaGuesser::default()),
        llama: Some(llama.clone()),
        llama_models,
        fs,
        warm_ups: Default::default(),
    });

    let settings_source = config.clone();
    let history: Arc<dyn HistorySink> =
        choices.history.unwrap_or_else(|| Arc::new(FileHistory::open(folder, move || settings_source.settings())));
    let config_dyn: Arc<dyn ConfigSource> = config.clone();
    let controller = Controller::new(Deps {
        recorder: platform.recorder.clone(),
        paster: choices.paster.unwrap_or_else(|| platform.paster.clone()),
        media: platform.media.clone(),
        apps: platform.apps.clone(),
        sounds: platform.sounds.clone(),
        indicator: choices.indicator,
        config: config_dyn,
        history,
        transcriber: transcriber.clone(),
        cleaner: cleaner.clone(),
        clock: clock as Arc<dyn Clock>,
        wall_clock: Arc::new(chrono::Utc::now),
        log: log_dyn,
        executor: Arc::new(ThreadExecutor),
    });
    App { controller, platform, whisper, llama, config, log }
}

/// Ce que `verifier` et `lancer` disent des réglages.
pub fn describe_engines(settings: &AppSettings) -> String {
    format!(
        "transcription : {}, mise au propre : {}, touche : {:?}",
        match settings.transcription_engine {
            scriibe_core::settings::TranscriptionEngine::Deepgram => "Deepgram (cloud)",
            scriibe_core::settings::TranscriptionEngine::Whisper => "Whisper (sur cet ordinateur)",
        },
        settings.cleanup_engine.label(),
        settings.hotkey
    )
}

/// Le choix de l'appareil de `llama-server` est gardé à part des réglages (qui ont le format de Scribe) : `moteurs.json`. Provisoire,
/// en attendant la fenêtre de Réglages.
fn engine_options_path(folder: &Path) -> PathBuf {
    folder.join("moteurs.json")
}

pub fn read_llama_device(folder: &Path) -> String {
    std::fs::read_to_string(engine_options_path(folder))
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|json| json["llamaDevice"].as_str().map(str::to_string))
        .unwrap_or_default()
}

pub fn write_llama_device(folder: &Path, device: &str) -> Result<(), String> {
    let text = serde_json::json!({ "llamaDevice": device }).to_string();
    std::fs::write(engine_options_path(folder), text).map_err(|e| format!("impossible d'écrire moteurs.json ({e})"))
}
