//! Dossier de données de l'appli et fichiers qu'il contient : réglages, dictionnaire, historique, journal. Mêmes noms et mêmes
//! formats que Scribe (`settings.json`, `dictionary.json`, `history.json`, `history-totals.json`, `scribe.log` → `scriibe.log`).

use chrono::{DateTime, Local, Utc};
use scriibe_core::dictionary::UserDictionary;
use scriibe_core::history::{lossy_list, Calendar, DayTotal, Dictation, Store};
use scriibe_core::settings::Hotkey;
use scriibe_core::settings::{AppSettings, CleanupEngine, TranscriptionEngine, SETTINGS_SCHEMA};
use scriibe_engines::runtime::Log;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

/// Désinstallation en cours : plus aucune écriture dans le dossier de données (le processus vivant réécrirait l'historique ou le journal
/// dans un dossier qui part à la Corbeille). Posé juste avant le lancement de l'assistant, jamais levé par la suite.
static FROZEN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Coupe (ou rétablit, si le lancement de l'assistant échoue) toute écriture dans le dossier de données.
pub fn set_frozen(frozen: bool) {
    FROZEN.store(frozen, std::sync::atomic::Ordering::SeqCst);
}

pub fn is_frozen() -> bool {
    FROZEN.load(std::sync::atomic::Ordering::SeqCst)
}

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
        std::env::var_os("XDG_DATA_HOME")
            .filter(|v| !v.is_empty())
            .map_or_else(|| home().join(".local/share"), PathBuf::from)
            .join("scriibe")
    }
}

/// Réglages d'un premier lancement : tout sur cet ordinateur (Whisper et llama.cpp), sans clé ni compte, avec la touche du système.
pub fn first_run_settings() -> AppSettings {
    let defaults = AppSettings::default();
    // Hors du Mac, des sons doux et discrets (Funk au début, Bottle à la fin : choix de l'auteur, S06) ; Tink et Pop sont ceux de Scribe.
    let (start_sound_name, end_sound_name) = if cfg!(target_os = "macos") {
        (defaults.start_sound_name.clone(), defaults.end_sound_name.clone())
    } else {
        ("Funk".into(), "Bottle".into())
    };
    AppSettings {
        hotkey: Hotkey::system_default(),
        transcription_engine: TranscriptionEngine::Whisper,
        cleanup_engine: CleanupEngine::LlamaCpp,
        start_sound_name,
        end_sound_name,
        ..defaults
    }
}

/// Numéro du prochain fichier temporaire de ce processus (avec son identifiant, il rend le nom unique).
static TEMP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Écrit dans un fichier temporaire à nom unique (`.<nom>.<pid>-<n>.tmp`, dans le même dossier) puis le renomme : deux écritures
/// à la fois (ou deux Scriibe) ne se marchent plus dessus, et un arrêt en pleine écriture laisse l'ancien fichier intact.
fn write_atomic(path: &Path, content: &str) -> std::io::Result<()> {
    if is_frozen() {
        return Ok(());
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let n = TEMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let temp = path.with_file_name(format!(".{name}.{}-{n}.tmp", std::process::id()));
    let result = std::fs::write(&temp, content).and_then(|()| std::fs::rename(&temp, path));
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}

/// Pourquoi Scriibe refuse un fichier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// Le fichier existe mais n'est pas du JSON que Scriibe sait lire.
    Unreadable,
    /// Le fichier vient d'une version plus récente de Scriibe (numéro de version du format).
    Newer(u64),
}

/// Un fichier de données refusé : Scriibe continue avec ce qu'il sait lire, mais n'y écrit plus rien (la sauvegarde suivante
/// l'aurait écrasé par les valeurs par défaut, ou amputé de ce qu'elle ne comprend pas).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigProblem {
    /// Nom du fichier (`settings.json`).
    pub file: String,
    pub reason: Refusal,
    /// Nom de la copie `<nom>.<date>.bak` faite à côté d'un fichier illisible, si elle a pu l'être.
    pub backup: Option<String>,
}

impl ConfigProblem {
    /// Le message d'une sauvegarde refusée (rendu par le programme, traduit par la page).
    pub fn message(&self) -> String {
        match self.reason {
            Refusal::Unreadable => format!(
                "Le fichier {} est illisible : Scriibe n'y écrit plus rien tant qu'il n'est pas réparé ou supprimé.",
                self.file
            ),
            Refusal::Newer(_) => format!(
                "Le fichier {} vient d'une version plus récente de Scriibe : Scriibe n'y écrit rien pour ne pas l'abîmer. Mets Scriibe à jour.",
                self.file
            ),
        }
    }

    fn refuse_write(&self) -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::PermissionDenied, self.message())
    }
}

/// Met de côté un fichier que Scriibe ne sait pas lire : copie `<nom>.<date>.bak` à côté, comme Scribe. Une seule fois : si une
/// copie identique existe déjà (relance avec le même fichier cassé), c'est elle qui est rendue. Rend le nom de la copie.
pub(crate) fn set_aside(path: &Path) -> Option<String> {
    use std::io::Write;
    let bytes = std::fs::read(path).ok()?;
    let name = path.file_name()?.to_string_lossy().into_owned();
    let dir = path.parent()?;
    let prefix = format!("{name}.");
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let found = entry.file_name().to_string_lossy().into_owned();
        if found.starts_with(&prefix) && found.ends_with(".bak") && std::fs::read(entry.path()).is_ok_and(|b| b == bytes) {
            return Some(found);
        }
    }
    if is_frozen() {
        return None;
    }
    let stamp = Local::now().format("%Y%m%d-%H%M%S");
    for n in 0..100 {
        let candidate = if n == 0 { format!("{name}.{stamp}.bak") } else { format!("{name}.{stamp}-{n}.bak") };
        match std::fs::OpenOptions::new().write(true).create_new(true).open(dir.join(&candidate)) {
            Ok(mut file) => return file.write_all(&bytes).ok().map(|()| candidate),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(_) => return None,
        }
    }
    None
}

/// Ce que donne la lecture d'un fichier JSON.
enum Parsed<T> {
    Ok(T),
    /// Lisible, mais d'une version plus récente : la valeur lue sert à l'affichage, jamais à une réécriture.
    Newer(T, u64),
    Unreadable,
}

/// Dernière lecture d'un fichier : sa date de modification, la valeur lue et le refus éventuel.
type Cached<T> = Option<(Option<SystemTime>, T, Option<ConfigProblem>)>;

/// Un fichier JSON relu quand il change : modifier `settings.json` à la main s'applique à la dictée suivante.
struct Watched<T> {
    path: PathBuf,
    cache: Mutex<Cached<T>>,
}

impl<T: Clone> Watched<T> {
    fn new(path: PathBuf) -> Self {
        Self { path, cache: Mutex::new(None) }
    }

    /// La valeur lue (celle par défaut si le fichier manque, est vide ou est illisible) et le refus éventuel.
    /// La copie `.bak` d'un fichier illisible se fait ici, à la lecture, et non à la sauvegarde.
    fn read_state(&self, read: impl Fn(&str) -> Parsed<T>, default: impl Fn() -> T) -> (T, Option<ConfigProblem>) {
        let modified = std::fs::metadata(&self.path).and_then(|m| m.modified()).ok();
        let mut cache = self.cache.lock().unwrap_or_else(|p| p.into_inner());
        if let Some((seen, value, problem)) = cache.as_ref() {
            if *seen == modified {
                return (value.clone(), problem.clone());
            }
        }
        let file = self.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let unreadable = |path: &Path| ConfigProblem { file: file.clone(), reason: Refusal::Unreadable, backup: set_aside(path) };
        let (value, problem, cacheable) = match std::fs::read_to_string(&self.path) {
            Ok(text) if text.trim().is_empty() => (default(), None, true),
            Ok(text) => match read(&text) {
                Parsed::Ok(value) => (value, None, true),
                Parsed::Newer(value, version) => (value, Some(ConfigProblem { file, reason: Refusal::Newer(version), backup: None }), true),
                Parsed::Unreadable => (default(), Some(unreadable(&self.path)), true),
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (default(), None, true),
            // Pas du texte (octets invalides) : illisible pour de bon. Une autre erreur (accès refusé un instant…) se retente à
            // la lecture suivante au lieu de rester refusée jusqu'à la prochaine modification du fichier.
            Err(e) => (default(), Some(unreadable(&self.path)), e.kind() == std::io::ErrorKind::InvalidData),
        };
        if cacheable {
            *cache = Some((modified, value.clone(), problem.clone()));
        }
        (value, problem)
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

    /// Écrit les réglages, sauf si `settings.json` est refusé (illisible ou plus récent) : alors rien n'est écrit et l'erreur dit pourquoi.
    pub fn save_settings(&self, settings: &AppSettings) -> std::io::Result<()> {
        if let Some(problem) = self.settings_state().1 {
            return Err(problem.refuse_write());
        }
        let json = serde_json::to_string_pretty(settings).map_err(std::io::Error::other)?;
        write_atomic(&self.settings_path(), &json)?;
        self.settings.forget();
        Ok(())
    }

    /// Lecture tolérante : un réglage absent ou illisible garde sa valeur par défaut. Une touche de dictée qui n'existe pas sur ce
    /// système (Option droite sur un fichier venu d'un Mac) est remplacée par celle du système.
    pub fn settings(&self) -> AppSettings {
        self.settings_state().0
    }

    fn settings_state(&self) -> (AppSettings, Option<ConfigProblem>) {
        let (mut settings, problem) = self.settings.read_state(
            |text| match AppSettings::from_json(text) {
                Ok(s) if s.schema_version > SETTINGS_SCHEMA => {
                    let version = s.schema_version;
                    Parsed::Newer(s, version)
                }
                Ok(s) => Parsed::Ok(s),
                Err(_) => Parsed::Unreadable,
            },
            first_run_settings,
        );
        if !settings.hotkey.is_available() {
            settings.hotkey = Hotkey::system_default();
        }
        (settings, problem)
    }

    pub fn dictionary(&self) -> UserDictionary {
        self.dictionary_state().0
    }

    fn dictionary_state(&self) -> (UserDictionary, Option<ConfigProblem>) {
        self.dictionary.read_state(|text| UserDictionary::from_json(text).map_or(Parsed::Unreadable, Parsed::Ok), UserDictionary::default)
    }

    /// Écrit le dictionnaire, sauf si `dictionary.json` est illisible : alors rien n'est écrit et l'erreur dit pourquoi.
    pub fn save_dictionary(&self, dictionary: &UserDictionary) -> std::io::Result<()> {
        if let Some(problem) = self.dictionary_state().1 {
            return Err(problem.refuse_write());
        }
        let json = serde_json::to_string_pretty(dictionary).map_err(std::io::Error::other)?;
        write_atomic(&self.folder.join("dictionary.json"), &json)?;
        self.dictionary.forget();
        Ok(())
    }

    /// Les fichiers que Scriibe refuse en ce moment (la page l'affiche tant que ce n'est pas réparé).
    pub fn problems(&self) -> Vec<ConfigProblem> {
        [self.settings_state().1, self.dictionary_state().1].into_iter().flatten().collect()
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
        // Un fichier qui existe mais ne se lit pas est mis de côté (`.bak`) avant que l'historique vide ne l'écrase.
        let read = |name: &str| {
            let path = folder.join(name);
            match std::fs::read_to_string(&path) {
                Ok(text) if text.trim().is_empty() => None,
                Ok(text) => Some(text),
                Err(e) => {
                    if e.kind() != std::io::ErrorKind::NotFound {
                        set_aside(&path);
                    }
                    None
                }
            }
        };
        let history = read("history.json")
            .and_then(|t| lossy_list(&t, Dictation::from_value).map_err(|_| set_aside(&folder.join("history.json"))).ok())
            .unwrap_or_default();
        let archive = read("history-totals.json")
            .and_then(|t| lossy_list(&t, DayTotal::from_value).map_err(|_| set_aside(&folder.join("history-totals.json"))).ok())
            .unwrap_or_default();
        let store = Store { history, archive, ..Store::default() };
        Self { folder, store: Mutex::new(store), settings: Box::new(settings) }
    }

    pub fn dictation_count(&self) -> usize {
        self.store.lock().unwrap_or_else(|p| p.into_inner()).history.len()
    }

    /// Lit l'historique (la durée de conservation est d'abord appliquée si elle est due : « à l'ouverture des pages »).
    pub fn read<R>(&self, f: impl FnOnce(&Store, &Calendar<Local>, DateTime<Utc>) -> R) -> R {
        self.update(|store, calendar, now| f(store, calendar, now))
    }

    /// Change l'historique puis l'écrit sur le disque. `f` reçoit l'état, le calendrier local et l'heure.
    pub fn update<R>(&self, f: impl FnOnce(&mut Store, &Calendar<Local>, DateTime<Utc>) -> R) -> R {
        let mut store = self.store.lock().unwrap_or_else(|p| p.into_inner());
        store.settings = (self.settings)();
        let (calendar, now) = (Calendar::new(Local), Utc::now());
        store.apply_retention_if_due(&calendar, now);
        let result = f(&mut store, &calendar, now);
        self.persist(&store);
        result
    }

    fn persist(&self, store: &Store) {
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

impl HistorySink for FileHistory {
    fn record(&self, dictation: Dictation) {
        self.update(|store, calendar, now| store.add(calendar, now, dictation));
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
        if is_frozen() {
            return;
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
        let (start, end) = if cfg!(target_os = "macos") { ("Tink", "Pop") } else { ("Funk", "Bottle") };
        assert_eq!((settings.start_sound_name.as_str(), settings.end_sound_name.as_str()), (start, end), "sons doux hors du Mac");
        assert!(crate::sounds::NAMES.contains(&start) && crate::sounds::NAMES.contains(&end));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn history_is_kept_on_disk_deleted_and_purged_by_age() {
        let dir = temp("history-by-age");
        let history = FileHistory::open(dir.clone(), AppSettings::default);
        let now = Utc::now();
        let old = Dictation::new(now - chrono::Duration::days(40), "Chrome", "vieux", "vieux", 20, 8.0, 0);
        let fresh = Dictation::new(now, "Bloc-notes", "frais", "frais", 10, 4.0, 1);
        let fresh_id = fresh.id.clone();
        history.record(old);
        history.record(fresh);
        assert_eq!(FileHistory::open(dir.clone(), AppSettings::default).dictation_count(), 2, "écrit sur le disque");

        // Garder 30 jours : la dictée de il y a 40 jours part, ses totaux restent dans l'archive.
        let kept = history.update(|store, calendar, now| {
            store.apply_history_change(calendar, now, true, Some(30));
            (store.history.len(), store.archive.iter().map(|d| d.words).sum::<i64>())
        });
        assert_eq!(kept, (1, 20));
        assert_eq!(history.read(|store, _, _| store.total_words()), 30, "les chiffres comptent l'archive");

        history.update(|store, _, _| store.delete(&fresh_id));
        assert_eq!(history.dictation_count(), 0);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_dictionary_is_saved_and_read_back() {
        let dir = temp("dictionary");
        let config = FileConfig::new(dir.clone());
        assert!(config.dictionary().words.is_empty());
        let dictionary = UserDictionary {
            words: vec!["Scriibe".into()],
            corrections: vec![scriibe_core::dictionary::Correction { heard: "cri be".into(), written: "Scriibe".into() }],
        };
        config.save_dictionary(&dictionary).unwrap();
        assert_eq!(config.dictionary(), dictionary);
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

    fn files_in(dir: &Path, suffix: &str) -> Vec<String> {
        let mut found: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(suffix))
            .collect();
        found.sort();
        found
    }

    #[test]
    fn saved_settings_carry_the_schema_version_and_a_file_without_one_counts_as_version_one() {
        let dir = temp("schema-version");
        let config = FileConfig::new(dir.clone());
        config.save_settings(&AppSettings::default()).unwrap();
        let written: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(config.settings_path()).unwrap()).unwrap();
        assert_eq!(written["schemaVersion"], SETTINGS_SCHEMA);
        std::fs::write(config.settings_path(), r#"{"language":"en"}"#).unwrap();
        config.settings.forget();
        assert_eq!(config.settings().schema_version, SETTINGS_SCHEMA);
        assert!(config.problems().is_empty());
        config.save_settings(&config.settings()).unwrap();
        assert!(
            std::fs::read_to_string(config.settings_path()).unwrap().contains("\"schemaVersion\": 1"),
            "le numéro est écrit à la sauvegarde"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn an_unreadable_settings_file_blocks_every_save_until_it_is_repaired() {
        let dir = temp("refused-unreadable");
        std::fs::write(dir.join("settings.json"), "pas du json").unwrap();
        let config = FileConfig::new(dir.clone());
        assert_eq!(
            config.settings().transcription_engine,
            TranscriptionEngine::Whisper,
            "l'appli continue avec les réglages du premier lancement"
        );
        let problems = config.problems();
        assert_eq!((problems.len(), problems[0].file.as_str(), &problems[0].reason), (1, "settings.json", &Refusal::Unreadable));

        let error = config.save_settings(&AppSettings::default()).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
        assert!(error.to_string().contains("settings.json"));
        assert_eq!(std::fs::read_to_string(dir.join("settings.json")).unwrap(), "pas du json", "le fichier n'est pas écrasé");

        // Réparé à la main : le refus tombe à la lecture suivante, la sauvegarde repart.
        std::fs::write(dir.join("settings.json"), "{}").unwrap();
        config.settings.forget();
        assert!(config.problems().is_empty());
        config.save_settings(&AppSettings::default()).unwrap();
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_settings_file_from_a_newer_version_is_read_but_never_rewritten() {
        let dir = temp("refused-newer");
        let original = r#"{"schemaVersion": 2, "language": "en", "champFutur": {"a": 1}}"#;
        std::fs::write(dir.join("settings.json"), original).unwrap();
        let config = FileConfig::new(dir.clone());
        assert_eq!(config.settings().language, scriibe_core::settings::DictationLanguage::En, "ce que Scriibe comprend sert encore");
        assert_eq!(config.problems()[0].reason, Refusal::Newer(2));
        assert!(config.save_settings(&config.settings()).is_err());
        assert_eq!(std::fs::read_to_string(dir.join("settings.json")).unwrap(), original, "les champs inconnus restent intacts");
        assert!(files_in(&dir, ".bak").is_empty(), "un fichier lisible n'a pas besoin de copie");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn an_unreadable_dictionary_blocks_its_save_but_not_the_settings() {
        let dir = temp("refused-dictionary");
        std::fs::write(dir.join("dictionary.json"), "[1, 2").unwrap();
        let config = FileConfig::new(dir.clone());
        assert_eq!(config.dictionary(), UserDictionary::default());
        assert_eq!(config.problems().iter().map(|p| p.file.as_str()).collect::<Vec<_>>(), vec!["dictionary.json"]);
        let words = UserDictionary { words: vec!["Scriibe".into()], corrections: vec![] };
        assert_eq!(config.save_dictionary(&words).unwrap_err().kind(), std::io::ErrorKind::PermissionDenied);
        assert_eq!(std::fs::read_to_string(dir.join("dictionary.json")).unwrap(), "[1, 2");
        config.save_settings(&AppSettings::default()).unwrap();
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn an_unreadable_file_is_copied_aside_once_at_reading_time() {
        let dir = temp("bak-once");
        std::fs::write(dir.join("settings.json"), "pas du json").unwrap();
        let config = FileConfig::new(dir.clone());
        config.settings();
        let copies = files_in(&dir, ".bak");
        assert_eq!(copies.len(), 1);
        assert!(copies[0].starts_with("settings.json.") && copies[0].ends_with(".bak"), "{copies:?}");
        assert_eq!(std::fs::read_to_string(dir.join(&copies[0])).unwrap(), "pas du json");
        assert_eq!(config.problems()[0].backup.as_deref(), Some(copies[0].as_str()), "la page peut dire où est la copie");

        // Lectures répétées, puis relance avec le même fichier cassé : toujours une seule copie.
        config.settings.forget();
        config.settings();
        FileConfig::new(dir.clone()).settings();
        assert_eq!(files_in(&dir, ".bak"), copies);

        // Un autre contenu cassé, c'est une autre copie.
        std::fs::write(dir.join("settings.json"), "encore cassé").unwrap();
        FileConfig::new(dir.clone()).settings();
        assert_eq!(files_in(&dir, ".bak").len(), 2);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn an_empty_file_is_not_a_problem() {
        let dir = temp("empty-file");
        std::fs::write(dir.join("settings.json"), "  \n").unwrap();
        let config = FileConfig::new(dir.clone());
        config.settings();
        assert!(config.problems().is_empty() && files_in(&dir, ".bak").is_empty());
        config.save_settings(&AppSettings::default()).unwrap();
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn an_unreadable_history_is_copied_aside_before_the_empty_one_overwrites_it() {
        let dir = temp("history-bak");
        std::fs::write(dir.join("history.json"), "pas du json").unwrap();
        let history = FileHistory::open(dir.clone(), AppSettings::default);
        history.record(Dictation::new(Utc::now(), "Bloc-notes", "bonjour", "Bonjour.", 1, 1.5, 0));
        let copies = files_in(&dir, ".bak");
        assert_eq!(copies.len(), 1);
        assert!(copies[0].starts_with("history.json."));
        assert_eq!(std::fs::read_to_string(dir.join(&copies[0])).unwrap(), "pas du json");
        assert_eq!(FileHistory::open(dir.clone(), AppSettings::default).dictation_count(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn simultaneous_writes_use_their_own_temporary_files_and_leave_none_behind() {
        let dir = temp("unique-temp");
        let config = std::sync::Arc::new(FileConfig::new(dir.clone()));
        let threads: Vec<_> = (0..8)
            .map(|i| {
                let config = config.clone();
                std::thread::spawn(move || {
                    for n in 0..25 {
                        let settings = AppSettings { first_name: format!("Prénom {i}-{n}"), ..AppSettings::default() };
                        config.save_settings(&settings).expect("une écriture ne marche pas sur le temporaire d'une autre");
                    }
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
        assert!(AppSettings::from_json(&std::fs::read_to_string(config.settings_path()).unwrap()).is_ok());
        assert!(files_in(&dir, ".tmp").is_empty(), "{:?}", files_in(&dir, ".tmp"));
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
