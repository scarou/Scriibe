//! Dossier de données de l'appli et fichiers qu'il contient : réglages, dictionnaire, historique, journal. Mêmes noms et mêmes
//! formats que Scribe (`settings.json`, `dictionary.json`, `history.json`, `history-totals.json`, `scribe.log` → `scriibe.log`).

use crate::migration;
use crate::profiles::{self, ProfileError, ProfileInfo};
use chrono::{DateTime, Local, Utc};
use scriibe_core::dictionary::UserDictionary;
use scriibe_core::history::{lossy_list, Calendar, DayTotal, Dictation, Store};
use scriibe_core::settings::Hotkey;
use scriibe_core::settings::{owner_of, AppSettings, CleanupEngine, Owner, TranscriptionEngine, SETTINGS_SCHEMA};
use scriibe_engines::runtime::Log;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
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
pub(crate) fn write_atomic(path: &Path, content: &str) -> std::io::Result<()> {
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

/// Le chemin surveillé et ce qui en a été lu (sous un même verrou : changer de chemin vide aussitôt la mémoire).
struct WatchedState<T> {
    path: PathBuf,
    cache: Cached<T>,
}

/// Un fichier JSON relu quand il change : modifier `settings.json` à la main s'applique à la dictée suivante. Son chemin peut changer
/// (`retarget`) : c'est ainsi que les fichiers de l'utilisateur suivent l'utilisateur actif.
struct Watched<T> {
    state: Mutex<WatchedState<T>>,
}

impl<T: Clone> Watched<T> {
    fn new(path: PathBuf) -> Self {
        Self { state: Mutex::new(WatchedState { path, cache: None }) }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, WatchedState<T>> {
        self.state.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn path(&self) -> PathBuf {
        self.lock().path.clone()
    }

    /// Surveille désormais un autre fichier : rien de ce qui a été lu avant n'est gardé.
    fn retarget(&self, path: PathBuf) {
        let mut state = self.lock();
        state.path = path;
        state.cache = None;
    }

    /// La valeur lue (celle par défaut si le fichier manque, est vide ou est illisible) et le refus éventuel.
    /// La copie `.bak` d'un fichier illisible se fait ici, à la lecture, et non à la sauvegarde.
    fn read_state(&self, read: impl Fn(&str) -> Parsed<T>, default: impl Fn() -> T) -> (T, Option<ConfigProblem>) {
        let mut state = self.lock();
        let path = state.path.clone();
        let modified = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
        if let Some((seen, value, problem)) = state.cache.as_ref() {
            if *seen == modified {
                return (value.clone(), problem.clone());
            }
        }
        let file = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let unreadable = |path: &Path| ConfigProblem { file: file.clone(), reason: Refusal::Unreadable, backup: set_aside(path) };
        let (value, problem, cacheable) = match std::fs::read_to_string(&path) {
            Ok(text) if text.trim().is_empty() => (default(), None, true),
            Ok(text) => match read(&text) {
                Parsed::Ok(value) => (value, None, true),
                Parsed::Newer(value, version) => (value, Some(ConfigProblem { file, reason: Refusal::Newer(version), backup: None }), true),
                Parsed::Unreadable => (default(), Some(unreadable(&path)), true),
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (default(), None, true),
            // Pas du texte (octets invalides) : illisible pour de bon. Une autre erreur (accès refusé un instant…) se retente à
            // la lecture suivante au lieu de rester refusée jusqu'à la prochaine modification du fichier.
            Err(e) => (default(), Some(unreadable(&path)), e.kind() == std::io::ErrorKind::InvalidData),
        };
        if cacheable {
            state.cache = Some((modified, value.clone(), problem.clone()));
        }
        (value, problem)
    }

    fn forget(&self) {
        self.lock().cache = None;
    }
}

/// `settings.json` lu comme un objet JSON (la valeur brute : la vue des réglages se compose ensuite, voir `FileConfig::view`).
fn read_settings_object(text: &str) -> Parsed<Value> {
    match serde_json::from_str::<Value>(text) {
        Ok(value) if value.is_object() => {
            let version = value.get("schemaVersion").and_then(Value::as_u64).unwrap_or(1);
            if version > SETTINGS_SCHEMA {
                Parsed::Newer(value, version)
            } else {
                Parsed::Ok(value)
            }
        }
        _ => Parsed::Unreadable,
    }
}

/// `profile.json` lu comme un objet JSON. Pas de numéro de version propre (étape 4b) : ses clés sont celles de `settings.json`, dont le
/// numéro fait foi pour tout le dossier ; un `profile.json` n'est donc jamais « trop récent », seulement lisible ou non.
fn read_profile_object(text: &str) -> Parsed<Value> {
    match serde_json::from_str::<Value>(text) {
        Ok(value) if value.is_object() => Parsed::Ok(value),
        _ => Parsed::Unreadable,
    }
}

/// Les réglages et le dictionnaire, lus dans le dossier de données.
///
/// **Étape 4a (profils).** `settings()` est toujours une vue unique : les clés de l'ordinateur viennent de `settings.json`, celles de
/// l'utilisateur de `profile.json` et `dictionary.json` **du dossier de l'utilisateur actif** (`switch_profile`). Tant qu'aucun
/// utilisateur n'est rangé à part (pas de migration, étape 4b), il n'y a pas de dossier d'utilisateur : tout reste dans `settings.json`
/// et `dictionary.json` à la racine, comme avant, et rien ne change sur le disque.
pub struct FileConfig {
    folder: PathBuf,
    /// Dossier de l'utilisateur actif ; `None` : rangement d'origine (tout à la racine).
    profile_folder: Mutex<Option<PathBuf>>,
    settings: Watched<Value>,
    profile: Watched<Value>,
    dictionary: Watched<UserDictionary>,
    /// Verrou des suites « lire, modifier, écrire » (étape 4c) : deux requêtes de la page, un clic du menu de l'icône et un changement
    /// d'utilisateur ne s'entremêlent pas. Voir `lock_for`.
    edit: Mutex<()>,
    /// Les utilisateurs lus il y a peu (le menu de l'icône relit sa vue deux fois par seconde : pas de lecture du disque à chaque fois).
    profiles_seen: Mutex<Option<(std::time::Instant, Vec<ProfileInfo>)>>,
}

/// Combien de temps la liste des utilisateurs lue pour le menu de l'icône reste valable.
const PROFILES_FRESH: std::time::Duration = std::time::Duration::from_secs(2);

impl FileConfig {
    pub fn new(folder: PathBuf) -> Self {
        Self {
            settings: Watched::new(folder.join("settings.json")),
            profile: Watched::new(folder.join("profile.json")),
            dictionary: Watched::new(folder.join("dictionary.json")),
            profile_folder: Mutex::new(None),
            edit: Mutex::new(()),
            profiles_seen: Mutex::new(None),
            folder,
        }
    }

    pub fn folder(&self) -> &Path {
        &self.folder
    }

    pub fn settings_path(&self) -> PathBuf {
        self.folder.join("settings.json")
    }

    /// Le dossier de l'utilisateur actif ; `None` tant que rien n'est rangé par utilisateur.
    pub fn profile_folder(&self) -> Option<PathBuf> {
        self.profile_folder.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }

    /// Change d'utilisateur : `profile.json` et `dictionary.json` sont relus dans ce dossier. `None` : retour au rangement d'origine.
    /// Le cadrage appelle cela depuis la route de changement d'utilisateur (4c) ; la page doit alors recharger ses données.
    pub fn switch_profile(&self, folder: Option<PathBuf>) {
        let mut current = self.profile_folder.lock().unwrap_or_else(|p| p.into_inner());
        let root = folder.clone().unwrap_or_else(|| self.folder.clone());
        self.profile.retarget(root.join("profile.json"));
        self.dictionary.retarget(root.join("dictionary.json"));
        *current = folder;
    }

    /// L'identifiant de l'utilisateur actif (le nom de son dossier) ; `None` tant que rien n'est rangé par utilisateur.
    pub fn active_profile_id(&self) -> Option<String> {
        self.profile_folder().and_then(|f| f.file_name().map(|n| n.to_string_lossy().into_owned()))
    }

    /// Le nom affiché de l'utilisateur actif (« Principal » s'il n'en a pas) ; `None` sans utilisateur.
    pub fn active_profile_name(&self) -> Option<String> {
        self.profile_folder()?;
        let (profile, _) = self.profile.read_state(read_profile_object, || Value::Object(Default::default()));
        let name = profile.get(migration::NAME_KEY).and_then(Value::as_str).map(str::trim).filter(|n| !n.is_empty());
        Some(name.unwrap_or(migration::DEFAULT_NAME).to_string())
    }

    /// Prend le verrou des modifications (réglages, `profile.json`, dictionnaire, changement d'utilisateur). Celui qui le tient lit,
    /// modifie et écrit sans qu'un autre s'intercale ; `save_settings` et `save_dictionary` ne le prennent pas eux-mêmes (le verrou
    /// n'est pas réentrant) : c'est à l'appelant de le tenir autour de sa suite « lire, modifier, écrire ».
    pub fn lock_edit(&self) -> MutexGuard<'_, ()> {
        self.edit.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Comme `lock_edit`, mais refuse (`ProfileError::Stale`) si l'utilisateur actif n'est plus celui que l'appelant avait en tête :
    /// la page et le menu de l'icône envoient l'identifiant de l'utilisateur avec ce qu'ils écrivent, pour ne jamais écraser les
    /// données d'un autre (garde-fou 409 de la décision 3). `given` : l'identifiant reçu (`None` : l'appelant n'en avait pas, ce qui
    /// ne convient que s'il n'y a pas d'utilisateur actif). Les majuscules ne comptent pas.
    pub fn lock_for(&self, given: Option<&str>) -> Result<MutexGuard<'_, ()>, ProfileError> {
        let guard = self.lock_edit();
        let active = self.active_profile_id();
        if active.as_deref().map(str::to_ascii_lowercase) == given.map(str::to_ascii_lowercase) {
            Ok(guard)
        } else {
            Err(ProfileError::Stale)
        }
    }

    fn forget_profiles(&self) {
        *self.profiles_seen.lock().unwrap_or_else(|p| p.into_inner()) = None;
    }

    /// Les utilisateurs, lus sur le disque à l'instant (la page des Réglages). Erreur de lecture : rendue, jamais une liste vide.
    pub fn list_profiles(&self) -> std::io::Result<Vec<ProfileInfo>> {
        let found = profiles::list(&self.folder)?;
        *self.profiles_seen.lock().unwrap_or_else(|p| p.into_inner()) = Some((std::time::Instant::now(), found.clone()));
        Ok(found)
    }

    /// Les utilisateurs lus il y a moins de deux secondes (menu de l'icône, état de la page). Illisible : la liste vide, sans la garder.
    pub fn cached_profiles(&self) -> Vec<ProfileInfo> {
        if let Some((at, found)) = self.profiles_seen.lock().unwrap_or_else(|p| p.into_inner()).as_ref() {
            if at.elapsed() < PROFILES_FRESH {
                return found.clone();
            }
        }
        self.list_profiles().unwrap_or_default()
    }

    /// Crée un utilisateur vide. Il ne devient pas actif.
    pub fn create_profile(&self, name: &str) -> Result<ProfileInfo, ProfileError> {
        let _guard = self.lock_edit();
        let made = profiles::create(&self.folder, name, None);
        self.forget_profiles();
        made
    }

    /// Crée un utilisateur copié de l'utilisateur actif (réglages et dictionnaire, jamais l'historique ni les statistiques). `given` :
    /// l'utilisateur que la page croit actif (voir `lock_for`). Il ne devient pas actif.
    pub fn copy_profile(&self, given: Option<&str>, name: &str) -> Result<ProfileInfo, ProfileError> {
        let _guard = self.lock_for(given)?;
        let source = self.profile_folder().ok_or(ProfileError::Unknown)?;
        let made = profiles::create(&self.folder, name, Some(&source));
        self.forget_profiles();
        made
    }

    pub fn rename_profile(&self, id: &str, name: &str) -> Result<ProfileInfo, ProfileError> {
        let _guard = self.lock_edit();
        let renamed = profiles::rename(&self.folder, id, name);
        if renamed.is_ok() {
            self.profile.forget();
        }
        self.forget_profiles();
        renamed
    }

    /// Met un utilisateur de côté (`utilisateurs-supprimes/`, jamais effacé). Pas l'utilisateur actif. Rend où il se trouve maintenant.
    pub fn delete_profile(&self, id: &str) -> Result<PathBuf, ProfileError> {
        let _guard = self.lock_edit();
        let today = Local::now().format("%Y-%m-%d").to_string();
        let moved = profiles::delete(&self.folder, id, self.active_profile_id().as_deref(), &today);
        self.forget_profiles();
        moved
    }

    /// Passe à un autre utilisateur : `activeProfile` est écrit dans `settings.json`, puis les fichiers de l'utilisateur sont relus dans son
    /// dossier. `also` tourne dans la foulée, sous le même verrou, avec ce dossier (l'historique change de dossier là : aucune dictée
    /// enregistrée entre les deux ne va chez le mauvais utilisateur). Rien ne change si `settings.json` est refusé (illisible, plus récent).
    pub fn switch_to_profile<R>(&self, id: &str, also: impl FnOnce(&Path) -> R) -> Result<(ProfileInfo, R), ProfileError> {
        let _guard = self.lock_edit();
        let target = profiles::find(&self.folder, id)?.ok_or(ProfileError::Unknown)?;
        let (mut settings, problem) = self.settings.read_state(read_settings_object, || {
            serde_json::to_value(first_run_settings()).unwrap_or_else(|_| Value::Object(Default::default()))
        });
        if let Some(problem) = problem {
            return Err(ProfileError::Io(problem.message()));
        }
        let object = settings.as_object_mut().ok_or(ProfileError::Unreadable)?;
        object.insert("activeProfile".into(), Value::String(target.id.clone()));
        let text = serde_json::to_string_pretty(&settings).map_err(std::io::Error::other)?;
        write_atomic(&self.settings_path(), &text)?;
        self.settings.forget();
        self.switch_profile(Some(target.folder.clone()));
        let after = also(&target.folder);
        Ok((target, after))
    }

    /// Écrit les réglages d'un premier lancement s'il n'y en a pas encore. Rend vrai si le fichier vient d'être créé.
    pub fn create_settings_if_missing(&self) -> std::io::Result<bool> {
        if self.settings_path().exists() {
            return Ok(false);
        }
        self.save_settings(&first_run_settings())?;
        Ok(true)
    }

    /// Écrit les réglages, sauf si `settings.json` (ou `profile.json`) est refusé (illisible ou plus récent) : alors rien n'est écrit
    /// et l'erreur dit pourquoi. Sans utilisateur rangé à part, tout va dans `settings.json`. Avec un utilisateur, chaque clé va dans le
    /// fichier de son propriétaire (`settings.json` : l'ordinateur ; `profile.json` : l'utilisateur), selon la décision 1.
    pub fn save_settings(&self, settings: &AppSettings) -> std::io::Result<()> {
        let (_, problems) = self.view();
        if let Some(problem) = problems.first() {
            return Err(problem.refuse_write());
        }
        let Some(profile_folder) = self.profile_folder() else {
            let json = serde_json::to_string_pretty(settings).map_err(std::io::Error::other)?;
            write_atomic(&self.settings_path(), &json)?;
            self.settings.forget();
            return Ok(());
        };
        let (mut computer, user) = settings.split();
        // L'utilisateur actif est celui dont le dossier est suivi : une copie des réglages lue avant un changement d'utilisateur ne peut
        // pas remettre l'ancien dans `settings.json` (le changement passe par `switch_to_profile`, pas par une sauvegarde).
        if let Some(id) = self.active_profile_id() {
            computer.insert("activeProfile".into(), Value::String(id));
        }
        // `profile.json` garde ce qui n'est pas un réglage (le nom affiché, étape 4b) : seules les clés de l'utilisateur sont remplacées.
        let (current, _) = self.profile.read_state(read_profile_object, || Value::Object(Default::default()));
        let mut profile: serde_json::Map<String, Value> = current
            .as_object()
            .map(|o| o.iter().filter(|(k, _)| owner_of(k) != Some(Owner::User)).map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();
        profile.extend(user);
        let computer = serde_json::to_string_pretty(&computer).map_err(std::io::Error::other)?;
        let user = serde_json::to_string_pretty(&profile).map_err(std::io::Error::other)?;
        write_atomic(&self.settings_path(), &computer)?;
        self.settings.forget();
        write_atomic(&profile_folder.join("profile.json"), &user)?;
        self.profile.forget();
        Ok(())
    }

    /// Lecture tolérante : un réglage absent ou illisible garde sa valeur par défaut. Une touche de dictée qui n'existe pas sur ce
    /// système (Option droite sur un fichier venu d'un Mac) est remplacée par celle du système.
    pub fn settings(&self) -> AppSettings {
        self.view().0
    }

    /// La vue unique des réglages et les fichiers refusés (dans l'ordre : `settings.json`, puis `profile.json`).
    fn view(&self) -> (AppSettings, Vec<ConfigProblem>) {
        let in_a_profile = self.profile_folder().is_some();
        let (computer, computer_problem) = self.settings.read_state(read_settings_object, || {
            serde_json::to_value(first_run_settings()).unwrap_or_else(|_| Value::Object(Default::default()))
        });
        let mut problems: Vec<ConfigProblem> = computer_problem.into_iter().collect();
        let user = in_a_profile.then(|| {
            let (profile, problem) = self.profile.read_state(read_profile_object, || Value::Object(Default::default()));
            problems.extend(problem);
            profile
        });
        let mut settings = AppSettings::from_parts(&computer, user.as_ref()).unwrap_or_else(first_run_settings);
        if !settings.hotkey.is_available() {
            settings.hotkey = Hotkey::system_default();
        }
        (settings, problems)
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
        write_atomic(&self.dictionary.path(), &json)?;
        self.dictionary.forget();
        Ok(())
    }

    /// Les fichiers que Scriibe refuse en ce moment (la page l'affiche tant que ce n'est pas réparé).
    pub fn problems(&self) -> Vec<ConfigProblem> {
        let mut problems = self.view().1;
        problems.extend(self.dictionary_state().1);
        problems
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

/// L'historique en mémoire et le dossier où il s'écrit.
struct HistoryState {
    folder: PathBuf,
    store: Store,
}

/// Historique dans `history.json` et `history-totals.json`. Le dossier peut changer (`switch`) : c'est celui de l'utilisateur actif.
pub struct FileHistory {
    state: Mutex<HistoryState>,
    settings: Box<dyn Fn() -> AppSettings + Send + Sync>,
}

/// Lit l'historique d'un dossier. Un fichier qui existe mais ne se lit pas est mis de côté (`.bak`) avant que l'historique vide ne l'écrase.
fn load_store(folder: &Path) -> Store {
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
    Store { history, archive, ..Store::default() }
}

impl FileHistory {
    /// `settings` : les réglages courants (« garder le texte », durée de conservation).
    pub fn open(folder: PathBuf, settings: impl Fn() -> AppSettings + Send + Sync + 'static) -> Self {
        let store = load_store(&folder);
        Self { state: Mutex::new(HistoryState { folder, store }), settings: Box::new(settings) }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HistoryState> {
        self.state.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Passe à l'historique d'un autre dossier (l'utilisateur actif change). Sous le verrou : une dictée qui s'enregistre ou une page qui
    /// lit voit l'ancien historique en entier ou le nouveau en entier, jamais un mélange. L'ancien est déjà sur le disque (chaque
    /// changement s'écrit tout de suite) ; l'état propre à l'ancien (dernier texte dicté) n'est pas repris.
    pub fn switch(&self, folder: PathBuf) {
        let mut state = self.lock();
        if state.folder == folder {
            return;
        }
        let store = load_store(&folder);
        *state = HistoryState { folder, store };
    }

    pub fn dictation_count(&self) -> usize {
        self.lock().store.history.len()
    }

    /// Lit l'historique (la durée de conservation est d'abord appliquée si elle est due : « à l'ouverture des pages »).
    pub fn read<R>(&self, f: impl FnOnce(&Store, &Calendar<Local>, DateTime<Utc>) -> R) -> R {
        self.update(|store, calendar, now| f(store, calendar, now))
    }

    /// Change l'historique puis l'écrit sur le disque. `f` reçoit l'état, le calendrier local et l'heure.
    pub fn update<R>(&self, f: impl FnOnce(&mut Store, &Calendar<Local>, DateTime<Utc>) -> R) -> R {
        let mut state = self.lock();
        state.store.settings = (self.settings)();
        let (calendar, now) = (Calendar::new(Local), Utc::now());
        state.store.apply_retention_if_due(&calendar, now);
        let result = f(&mut state.store, &calendar, now);
        Self::persist(&state);
        result
    }

    fn persist(state: &HistoryState) {
        for (name, json) in [
            ("history.json", serde_json::to_string_pretty(&state.store.history)),
            ("history-totals.json", serde_json::to_string_pretty(&state.store.archive)),
        ] {
            if let Ok(json) = json {
                let _ = write_atomic(&state.folder.join(name), &json);
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
        // Seule la migration écrit la version 2 (étape 4b) : des réglages par défaut, ou sans numéro, restent en version 1.
        assert_eq!(written["schemaVersion"], scriibe_core::settings::LEGACY_SCHEMA);
        std::fs::write(config.settings_path(), r#"{"language":"en"}"#).unwrap();
        config.settings.forget();
        assert_eq!(config.settings().schema_version, scriibe_core::settings::LEGACY_SCHEMA);
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
        let original = r#"{"schemaVersion": 3, "language": "en", "champFutur": {"a": 1}}"#;
        std::fs::write(dir.join("settings.json"), original).unwrap();
        let config = FileConfig::new(dir.clone());
        assert_eq!(config.settings().language, scriibe_core::settings::DictationLanguage::En, "ce que Scriibe comprend sert encore");
        assert_eq!(config.problems()[0].reason, Refusal::Newer(3));
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

    fn user_json(config: &FileConfig, name: &str) -> Value {
        serde_json::from_str(&std::fs::read_to_string(config.profile_folder().unwrap().join(name)).unwrap()).unwrap()
    }

    #[test]
    fn without_a_profile_folder_everything_stays_in_the_original_files() {
        let dir = temp("no-profile");
        let config = FileConfig::new(dir.clone());
        assert_eq!(config.profile_folder(), None);
        let settings = AppSettings { first_name: "Léa".into(), pause_media: true, ..first_run_settings() };
        config.save_settings(&settings).unwrap();
        config.save_dictionary(&UserDictionary { words: vec!["Scriibe".into()], corrections: vec![] }).unwrap();
        // Comme avant l'étape 4a : un seul fichier de réglages avec toutes les clés, pas de profile.json.
        let written: Value = serde_json::from_str(&std::fs::read_to_string(config.settings_path()).unwrap()).unwrap();
        assert_eq!(written["firstName"], "Léa");
        assert_eq!(written["pauseMedia"], true);
        assert_eq!(std::fs::read_to_string(config.settings_path()).unwrap(), serde_json::to_string_pretty(&settings).unwrap());
        assert!(!dir.join("profile.json").exists());
        assert!(dir.join("dictionary.json").is_file());
        assert_eq!(config.settings().first_name, "Léa");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn with_a_profile_folder_each_key_goes_to_its_owner_and_the_view_joins_them() {
        let dir = temp("profile-split");
        let config = FileConfig::new(dir.clone());
        // Un fichier d'origine complet : les clés de l'utilisateur y sont déjà.
        let before =
            AppSettings { first_name: "Ancien".into(), language: scriibe_core::settings::DictationLanguage::En, ..first_run_settings() };
        config.save_settings(&before).unwrap();
        let user = dir.join("utilisateurs").join("A");
        config.switch_profile(Some(user.clone()));
        // Le profil n'existe pas encore : l'utilisateur est neuf, ses clés ont leur valeur par défaut (celles du fichier d'origine ne comptent plus).
        assert_eq!(config.settings().first_name, "");
        assert_eq!(config.settings().language, AppSettings::default().language);

        let after = AppSettings {
            first_name: "Léa".into(),
            typing_wpm: 55,
            pause_media: true,
            theme: scriibe_core::settings::Theme::Dark,
            ..config.settings()
        };
        config.save_settings(&after).unwrap();
        let (computer, profile) = (
            user_json(&config, "profile.json"),
            serde_json::from_str::<Value>(&std::fs::read_to_string(config.settings_path()).unwrap()).unwrap(),
        );
        assert_eq!((computer["firstName"].as_str(), computer["typingWPM"].as_i64()), (Some("Léa"), Some(55)));
        assert!(computer.get("pauseMedia").is_none() && computer.get("theme").is_none(), "rien de l'ordinateur dans le profil");
        assert_eq!(profile["pauseMedia"], true);
        assert!(profile.get("firstName").is_none() && profile.get("typingWPM").is_none(), "rien de l'utilisateur dans settings.json");
        // Depuis 4c, une sauvegarde faite dans le dossier d'un utilisateur note aussi qui est actif (`activeProfile`, le nom de son dossier).
        assert_eq!(
            config.settings(),
            AppSettings { trial_candidates: config.settings().trial_candidates, active_profile: Some("A".into()), ..after }
        );
        assert!(!dir.join("profile.json").exists(), "le profil est dans le dossier de l'utilisateur, pas à la racine");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn switching_user_switches_the_settings_and_the_dictionary_and_none_reads_the_root_again() {
        let dir = temp("profile-switch");
        let config = FileConfig::new(dir.clone());
        config.save_settings(&AppSettings { first_name: "Racine".into(), ..first_run_settings() }).unwrap();
        config.save_dictionary(&UserDictionary { words: vec!["racine".into()], corrections: vec![] }).unwrap();
        let (a, b) = (dir.join("utilisateurs").join("A"), dir.join("utilisateurs").join("B"));

        config.switch_profile(Some(a.clone()));
        assert!(config.dictionary().words.is_empty(), "un utilisateur neuf n'a pas le dictionnaire de la racine");
        config.save_settings(&AppSettings { first_name: "Alice".into(), ..config.settings() }).unwrap();
        config.save_dictionary(&UserDictionary { words: vec!["alice".into()], corrections: vec![] }).unwrap();
        assert!(a.join("dictionary.json").is_file());

        config.switch_profile(Some(b.clone()));
        assert_eq!((config.settings().first_name.as_str(), config.dictionary().words.len()), ("", 0), "B ne voit rien de A");
        config.save_settings(&AppSettings { first_name: "Bruno".into(), ..config.settings() }).unwrap();
        config.save_dictionary(&UserDictionary { words: vec!["bruno".into()], corrections: vec![] }).unwrap();

        config.switch_profile(Some(a));
        assert_eq!(
            (config.settings().first_name.as_str(), config.dictionary().words.as_slice()),
            ("Alice", ["alice".to_string()].as_slice())
        );
        config.switch_profile(Some(b));
        assert_eq!(
            (config.settings().first_name.as_str(), config.dictionary().words.as_slice()),
            ("Bruno", ["bruno".to_string()].as_slice())
        );
        // Les clés de l'ordinateur sont les mêmes pour tous.
        config.save_settings(&AppSettings { pause_media: true, ..config.settings() }).unwrap();
        config.switch_profile(None);
        assert!(config.settings().pause_media);
        // Sans dossier d'utilisateur, le dictionnaire de la racine n'a pas été touché ; en revanche `settings.json` n'a plus les clés de
        // l'utilisateur (la première sauvegarde avec un utilisateur les en a retirées) : le retour en arrière n'est pas une fonction.
        assert_eq!(config.dictionary().words, vec!["racine".to_string()]);
        assert_eq!(config.settings().first_name, "");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_hand_edit_of_the_profile_is_picked_up_and_an_unreadable_one_blocks_only_the_save() {
        let dir = temp("profile-edit");
        let config = FileConfig::new(dir.clone());
        config.save_settings(&first_run_settings()).unwrap();
        let user = dir.join("utilisateurs").join("A");
        config.switch_profile(Some(user.clone()));
        config.save_settings(&config.settings()).unwrap();
        std::fs::write(user.join("profile.json"), r#"{"firstName":"Main"}"#).unwrap();
        config.profile.forget();
        assert_eq!(config.settings().first_name, "Main");

        std::fs::write(user.join("profile.json"), "{pas du json").unwrap();
        config.profile.forget();
        let problems = config.problems();
        assert_eq!(problems.len(), 1);
        assert_eq!((problems[0].file.as_str(), &problems[0].reason), ("profile.json", &Refusal::Unreadable));
        assert!(problems[0].backup.is_some(), "copie .bak du profil illisible");
        let err = config.save_settings(&config.settings()).unwrap_err();
        assert!(err.to_string().contains("profile.json"), "{err}");
        assert_eq!(std::fs::read_to_string(user.join("profile.json")).unwrap(), "{pas du json", "rien n'a été écrasé");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn switching_history_swaps_the_whole_history_under_the_lock_and_keeps_each_on_its_own_disk() {
        let dir = temp("history-switch");
        let (a, b) = (dir.join("A"), dir.join("B"));
        let history = FileHistory::open(a.clone(), AppSettings::default);
        history.record(Dictation::new(Utc::now(), "Bloc-notes", "un", "Un.", 1, 1.0, 0));
        history.record(Dictation::new(Utc::now(), "Bloc-notes", "deux", "Deux.", 1, 1.0, 0));
        assert_eq!(history.dictation_count(), 2);

        history.switch(b.clone());
        assert_eq!(history.dictation_count(), 0, "B est vide, rien de A");
        assert_eq!(history.read(|store, _, _| store.last_dictation_text.clone()), None, "le dernier texte dicté de A n'est pas repris");
        history.record(Dictation::new(Utc::now(), "Chrome", "trois", "Trois.", 1, 1.0, 0));
        assert_eq!(FileHistory::open(b.clone(), AppSettings::default).dictation_count(), 1);
        assert_eq!(FileHistory::open(a.clone(), AppSettings::default).dictation_count(), 2, "A est resté sur son disque");

        history.switch(a.clone());
        assert_eq!(history.dictation_count(), 2);
        history.switch(a);
        assert_eq!(history.dictation_count(), 2, "changer pour le même dossier ne recharge rien");
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
