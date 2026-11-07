//! L'assemblage : les moteurs, la plateforme et le chef d'orchestre, branchés ensemble.

use scriibe_core::lang::LinguaGuesser;
use scriibe_core::settings::{is_device_name, AppSettings};
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
use std::time::Duration;

pub struct App {
    pub controller: Controller,
    pub platform: Platform,
    pub whisper: Arc<Whisper>,
    pub llama: Arc<EngineServer<LlamaLauncher>>,
    pub cleaner: Arc<Cleaner>,
    pub config: Arc<FileConfig>,
    pub log: Arc<FileLog>,
    /// « Ajouter au dictionnaire » par la voix : le texte sélectionné attend ici que la page le prenne.
    pub prefill: Arc<crate::prefill::PrefillSink>,
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
    let startup = config.settings();
    llama_launcher.set_device(&startup.llama_device);
    let llama = Arc::new(llama::new_server(llama_launcher, runtime.clone()));
    llama.set_idle_limit(idle_limit(startup.llama_idle_minutes));

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
    let prefill = Arc::new(crate::prefill::PrefillSink::default());
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
        commands: prefill.clone(),
    });
    App { controller, platform, whisper, llama, cleaner, config, log, prefill }
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

/// Le délai de libération de `llama-server` pour une durée en minutes (0 : jamais).
pub fn idle_limit(minutes: i64) -> Option<Duration> {
    (minutes > 0).then(|| Duration::from_secs(minutes as u64 * 60))
}

fn legacy_options_path(folder: &Path) -> PathBuf {
    folder.join("moteurs.json")
}

/// Reprend dans les réglages ce que S03 gardait à part dans `moteurs.json` (carte graphique, durée de conservation), puis supprime
/// le fichier : les réglages sont désormais au même endroit.
pub fn migrate_engine_options(config: &FileConfig) {
    let path = legacy_options_path(config.folder());
    let Some(json) = std::fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok()) else {
        return;
    };
    let mut settings = config.settings();
    apply_legacy_options(&mut settings, &json);
    if config.save_settings(&settings).is_ok() {
        let _ = std::fs::remove_file(path);
    }
}

fn apply_legacy_options(settings: &mut AppSettings, json: &serde_json::Value) {
    if let Some(device) = json["llamaDevice"].as_str().filter(|d| is_device_name(d)) {
        settings.llama_device = device.to_string();
    }
    if let Some(minutes) = json["llamaIdleMinutes"].as_u64() {
        settings.llama_idle_minutes = minutes.min(24 * 60) as i64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_engine_file_moves_into_the_settings_and_goes_away() {
        let dir = std::env::temp_dir().join(format!("scriibe-moteurs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let config = FileConfig::new(dir.clone());
        config.create_settings_if_missing().unwrap();
        assert_eq!(config.settings().llama_idle_minutes, 30, "valeur de Scribe par défaut");
        assert_eq!(idle_limit(30), Some(Duration::from_secs(30 * 60)));
        assert_eq!(idle_limit(0), None, "0 : jamais libéré");

        migrate_engine_options(&config);
        assert_eq!(config.settings().llama_device, "", "pas de fichier : rien ne change");

        std::fs::write(dir.join("moteurs.json"), r#"{"llamaDevice": "Vulkan1", "llamaIdleMinutes": 0}"#).unwrap();
        migrate_engine_options(&config);
        let settings = config.settings();
        assert_eq!((settings.llama_device.as_str(), settings.llama_idle_minutes), ("Vulkan1", 0));
        assert!(!dir.join("moteurs.json").exists());

        let mut partial = AppSettings::default();
        apply_legacy_options(&mut partial, &serde_json::json!({"llamaDevice": "a b; rm"}));
        assert_eq!(partial.llama_device, "", "un nom d'appareil invalide est refusé");
        assert_eq!(partial.llama_idle_minutes, 30, "champ absent : la valeur par défaut");
        let _ = std::fs::remove_dir_all(dir);
    }
}
