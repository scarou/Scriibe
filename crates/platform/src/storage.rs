//! Dossier de données de l'appli et fichiers qu'il contient : réglages, dictionnaire, historique, journal. Mêmes noms et mêmes
//! formats que Scribe (`settings.json`, `dictionary.json`, `history.json`, `history-totals.json`, `scribe.log` → `scriibe.log`).

use chrono::{DateTime, Local, Utc};
use scriibe_core::dictionary::UserDictionary;
use scriibe_core::history::{lossy_list, Calendar, DayTotal, Dictation, Store};
use scriibe_core::settings::Hotkey;
use scriibe_core::settings::{AppSettings, CleanupEngine, TranscriptionEngine};
use scriibe_engines::runtime::Log;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

/// `%APPDATA%\Scriibe` (Windows), `~/Library/Application Support/Scriibe` (macOS), `~/.local/share/scriibe` (Linux).
/// `SCRIIBE_DATA` remplace tout (essais).
pub fn data_folder() -> PathBuf {
    if let Some(custom) = std::env::var_os("SCRIIBE_DATA") {
        return PathBuf::from(custom);
    }
    let home = || std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    if cfg!(windows) {
        std::env::var_os("APPDATA").map_or_else(|| PathBuf::from("."), PathBuf::from).join("Scriibe")
    } else if cfg!(target_os = "macos") {
        home().join("Library/Application Support/Scriibe")
    } else {
        std::env::var_os("XDG_DATA_HOME").map_or_else(|| home().join(".local/share"), PathBuf::from).join("scriibe")
    }
}

/// Réglages d'un premier lancement : tout sur cet ordinateur (Whisper et llama.cpp), sans clé ni compte, avec la touche du système.
pub fn first_run_settings() -> AppSettings {
    AppSettings {
        hotkey: Hotkey::system_default(),
        transcription_engine: TranscriptionEngine::Whisper,
        cleanup_engine: CleanupEngine::LlamaCpp,
        ..AppSettings::default()
    }
}

fn write_atomic(path: &Path, content: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let temp = path.with_extension("tmp");
    std::fs::write(&temp, content)?;
    std::fs::rename(&temp, path)
}

/// Un fichier JSON relu quand il change : modifier `settings.json` à la main s'applique à la dictée suivante.
struct Watched<T> {
    path: PathBuf,
    cache: Mutex<Option<(Option<SystemTime>, T)>>,
}

impl<T: Clone> Watched<T> {
    fn new(path: PathBuf) -> Self {
        Self { path, cache: Mutex::new(None) }
    }

    fn get(&self, read: impl Fn(&str) -> Option<T>, default: impl Fn() -> T) -> T {
        let modified = std::fs::metadata(&self.path).and_then(|m| m.modified()).ok();
        let mut cache = self.cache.lock().unwrap_or_else(|p| p.into_inner());
        if let Some((seen, value)) = cache.as_ref() {
            if *seen == modified {
                return value.clone();
            }
        }
        let value = std::fs::read_to_string(&self.path).ok().and_then(|text| read(&text)).unwrap_or_else(default);
        *cache = Some((modified, value.clone()));
        value
    }

    fn forget(&self) {
        *self.cache.lock().unwrap_or_else(|p| p.into_inner()) = None;
    }
}

/// Les réglages et le dictionnaire, lus dans le dossier de données.
pub struct FileConfig {
    folder: PathBuf,
    settings: Watched<AppSettings>,
    dictionary: Watched<UserDictionary>,
}

impl FileConfig {
    pub fn new(folder: PathBuf) -> Self {
        Self { settings: Watched::new(folder.join("settings.json")), dictionary: Watched::new(folder.join("dictionary.json")), folder }
    }

    pub fn folder(&self) -> &Path {
        &self.folder
    }

    pub fn settings_path(&self) -> PathBuf {
        self.folder.join("settings.json")
    }

    /// Écrit les réglages d'un premier lancement s'il n'y en a pas encore. Rend vrai si le fichier vient d'être créé.
    pub fn create_settings_if_missing(&self) -> std::io::Result<bool> {
        if self.settings_path().exists() {
            return Ok(false);
        }
        self.save_settings(&first_run_settings())?;
        Ok(true)
    }

    pub fn save_settings(&self, settings: &AppSettings) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(settings).map_err(std::io::Error::other)?;
        write_atomic(&self.settings_path(), &json)?;
        self.settings.forget();
        Ok(())
    }

    /// Lecture tolérante : un réglage absent ou illisible garde sa valeur par défaut. Une touche de dictée qui n'existe pas sur ce
    /// système (Option droite sur un fichier venu d'un Mac) est remplacée par celle du système.
    pub fn settings(&self) -> AppSettings {
        let mut settings = self.settings.get(|text| AppSettings::from_json(text).ok(), first_run_settings);
        if !settings.hotkey.is_available() {
            settings.hotkey = Hotkey::system_default();
        }
        settings
    }

    pub fn dictionary(&self) -> UserDictionary {
        self.dictionary.get(|text| UserDictionary::from_json(text).ok(), UserDictionary::default)
    }
}

/// D'où le chef d'orchestre lit les réglages et le dictionnaire (relus à chaque dictée : une modification à la main s'applique tout de suite).
pub trait ConfigSource: Send + Sync {
    fn settings(&self) -> AppSettings;
    fn dictionary(&self) -> UserDictionary;
}

impl ConfigSource for FileConfig {
    fn settings(&self) -> AppSettings {
        FileConfig::settings(self)
    }

    fn dictionary(&self) -> UserDictionary {
        FileConfig::dictionary(self)
    }
}

/// Ce que le chef d'orchestre enregistre à la fin d'une dictée.
pub trait HistorySink: Send + Sync {
    fn record(&self, dictation: Dictation);
}

/// Historique dans `history.json` et `history-totals.json`.
pub struct FileHistory {
    folder: PathBuf,
    store: Mutex<Store>,
    settings: Box<dyn Fn() -> AppSettings + Send + Sync>,
}

impl FileHistory {
    /// `settings` : les réglages courants (« garder le texte », durée de conservation).
    pub fn open(folder: PathBuf, settings: impl Fn() -> AppSettings + Send + Sync + 'static) -> Self {
        let read = |name: &str| std::fs::read_to_string(folder.join(name)).ok();
        let history = read("history.json").and_then(|t| lossy_list(&t, Dictation::from_value).ok()).unwrap_or_default();
        let archive = read("history-totals.json").and_then(|t| lossy_list(&t, DayTotal::from_value).ok()).unwrap_or_default();
        let store = Store { history, archive, ..Store::default() };
        Self { folder, store: Mutex::new(store), settings: Box::new(settings) }
    }

    pub fn dictation_count(&self) -> usize {
        self.store.lock().unwrap_or_else(|p| p.into_inner()).history.len()
    }
}

impl HistorySink for FileHistory {
    fn record(&self, dictation: Dictation) {
        let mut store = self.store.lock().unwrap_or_else(|p| p.into_inner());
        store.settings = (self.settings)();
        let now: DateTime<Utc> = Utc::now();
        store.add(&Calendar::new(Local), now, dictation);
        for (name, json) in [
            ("history.json", serde_json::to_string_pretty(&store.history)),
            ("history-totals.json", serde_json::to_string_pretty(&store.archive)),
        ] {
            if let Ok(json) = json {
                let _ = write_atomic(&self.folder.join(name), &json);
            }
        }
    }
}

/// Journal technique (`scriibe.log`) : étapes et durées, jamais le texte dicté ni les clés. Au-delà de 512 Ko, on repart du dernier quart.
pub struct FileLog {
    path: PathBuf,
    echo: bool,
    lock: Mutex<()>,
}

impl FileLog {
    pub fn new(folder: &Path, echo: bool) -> Self {
        Self { path: folder.join("scriibe.log"), echo, lock: Mutex::new(()) }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Log for FileLog {
    fn write(&self, message: &str) {
        let _guard = self.lock.lock().unwrap_or_else(|p| p.into_inner());
        let line = format!("{} {message}\n", Local::now().format("%Y-%m-%d %H:%M:%S%.3f"));
        if self.echo {
            eprint!("{line}");
        }
        if let Some(dir) = self.path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if std::fs::metadata(&self.path).is_ok_and(|m| m.len() > 512 * 1024) {
            if let Ok(old) = std::fs::read_to_string(&self.path) {
                let mut start = old.len() * 3 / 4;
                while !old.is_char_boundary(start) {
                    start += 1;
                }
                let _ = std::fs::write(&self.path, &old[start..]);
            }
        }
        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&self.path) {
            let _ = file.write_all(line.as_bytes());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("scriibe-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn first_run_creates_local_settings_once() {
        let dir = temp("first-run");
        let config = FileConfig::new(dir.clone());
        assert!(config.create_settings_if_missing().unwrap());
        assert!(!config.create_settings_if_missing().unwrap());
        let settings = config.settings();
        assert_eq!(settings.transcription_engine, TranscriptionEngine::Whisper);
        assert_eq!(settings.cleanup_engine, CleanupEngine::LlamaCpp);
        assert_eq!(settings.hotkey, Hotkey::system_default());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_hand_edit_is_picked_up_and_an_unknown_hotkey_falls_back() {
        let dir = temp("edit");
        let config = FileConfig::new(dir.clone());
        config.create_settings_if_missing().unwrap();
        assert_eq!(config.settings().language, scriibe_core::settings::DictationLanguage::Fr);
        // Un fichier de Mac : Option droite n'existe pas ici.
        std::fs::write(config.settings_path(), r#"{"language":"en","hotkey":"rightOption"}"#).unwrap();
        // La date de modification peut être la même à la milliseconde près : on oublie le cache comme le ferait un changement.
        config.settings.forget();
        let settings = config.settings();
        assert_eq!(settings.language, scriibe_core::settings::DictationLanguage::En);
        assert!(settings.hotkey.is_available());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_broken_settings_file_gives_first_run_settings() {
        let dir = temp("broken");
        std::fs::write(dir.join("settings.json"), "pas du json").unwrap();
        // (Les identifiants des candidats d'essai changent à chaque création : on compare les réglages qui comptent.)
        let settings = FileConfig::new(dir.clone()).settings();
        assert_eq!(settings.transcription_engine, TranscriptionEngine::Whisper);
        assert_eq!(settings.cleanup_engine, CleanupEngine::LlamaCpp);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn history_is_written_and_read_back() {
        let dir = temp("history");
        let history = FileHistory::open(dir.clone(), AppSettings::default);
        history.record(Dictation::new(Utc::now(), "Bloc-notes", "bonjour", "Bonjour.", 1, 1.5, 0));
        let again = FileHistory::open(dir.clone(), AppSettings::default);
        assert_eq!(again.dictation_count(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_log_never_grows_without_bound() {
        let dir = temp("log");
        let log = FileLog::new(&dir, false);
        for _ in 0..40 {
            log.write(&"x".repeat(20_000));
        }
        assert!(std::fs::metadata(log.path()).unwrap().len() < 700 * 1024);
        let _ = std::fs::remove_dir_all(dir);
    }
}
