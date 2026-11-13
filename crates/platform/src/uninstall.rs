//! Désinstallation de Scriibe (chantier 7 ; `docs/SYSTEME.md` § Cadrage, questions 1, 2, 4, 5, 6 et relecture).
//!
//! **Une suppression de trop est irréversible** (Corbeille vidée), d'où l'organisation :
//! - la liste ([`Plan`]) est construite **uniquement** à partir des emplacements connus de Scriibe, reconstruits un à un depuis des
//!   variables d'environnement absolues ; aucun chemin venu d'un fichier de réglages ni de la page n'y entre ;
//! - chaque racine passe par des garde-fous de nom et de structure, les liens symboliques ne sont jamais suivis ;
//! - l'exécution se fait **après la sortie** du programme, par une copie de lui-même ([`start_helper`] → [`finish`]), qui **reconstruit la
//!   liste elle-même** : le fichier de travail ne contient aucun chemin à supprimer ;
//! - tout part à la Corbeille ; si elle est indisponible, on s'arrête (jamais de suppression définitive, sauf les sons temporaires).
//!
//! Tout le code qui décide quoi supprimer prend ses dossiers en paramètres ([`Env`]) : les tests travaillent sur des dossiers factices.

use std::path::{Path, PathBuf};
use std::time::Duration;

/// Le système dont on applique les emplacements (paramètre pour tester les trois dispositions depuis n'importe quel système).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Mac,
    Windows,
    Linux,
}

impl Os {
    /// Le système sur lequel tourne ce programme.
    pub fn current() -> Os {
        if cfg!(windows) {
            Os::Windows
        } else if cfg!(target_os = "macos") {
            Os::Mac
        } else {
            Os::Linux
        }
    }
}

/// Tout ce dont dépendent les emplacements : variables d'environnement, programme en cours, option « garder mes modèles ».
#[derive(Debug, Clone)]
pub struct Env {
    pub os: Os,
    pub home: Option<PathBuf>,
    pub appdata: Option<PathBuf>,
    pub local_appdata: Option<PathBuf>,
    pub xdg_data_home: Option<PathBuf>,
    pub xdg_cache_home: Option<PathBuf>,
    pub tmp: PathBuf,
    /// Le programme à supprimer (`Scriibe.app`, `scriibe.exe`, `scriibe`) : l'exécutable de Scriibe, pas la copie de l'assistant.
    pub exe: PathBuf,
    /// `SCRIIBE_DATA` est défini (essais) : cette variable déplace les données mais pas les moteurs ni les clés.
    pub scriibe_data_set: bool,
    pub keep_models: bool,
}

impl Env {
    /// L'environnement réel de ce programme. `exe` : le programme de Scriibe (`current_exe()` dans l'application, ou celui noté dans le
    /// fichier de travail pour l'assistant).
    pub fn real(exe: PathBuf, keep_models: bool) -> Env {
        let var = |name: &str| std::env::var_os(name).map(PathBuf::from).filter(|p| !p.as_os_str().is_empty());
        Env {
            os: Os::current(),
            home: if cfg!(windows) { var("USERPROFILE") } else { var("HOME") },
            appdata: var("APPDATA"),
            local_appdata: var("LOCALAPPDATA"),
            xdg_data_home: var("XDG_DATA_HOME"),
            xdg_cache_home: var("XDG_CACHE_HOME"),
            tmp: std::env::temp_dir(),
            exe,
            scriibe_data_set: var("SCRIIBE_DATA").is_some(),
            keep_models,
        }
    }

    fn data_base_linux(&self) -> Option<PathBuf> {
        self.xdg_data_home.clone().or_else(|| self.home.as_ref().map(|h| h.join(".local").join("share")))
    }
}

/// Le dossier de données (mêmes formules que `storage::data_folder`, sans `SCRIIBE_DATA`), ou `None` si l'environnement manque.
pub fn data_root(env: &Env) -> Option<PathBuf> {
    match env.os {
        Os::Mac => env.home.as_ref().map(|h| h.join("Library").join("Application Support").join("Scriibe")),
        Os::Windows => env.appdata.as_ref().map(|a| a.join("Scriibe")),
        Os::Linux => env.data_base_linux().map(|b| b.join("scriibe")),
    }
}

/// Le dossier entier des moteurs téléchargés. Mac : dans le dossier de données (rien à ajouter). Windows : `%LOCALAPPDATA%\Scriibe`.
/// Linux : `<données>/Scriibe` (S majuscule). On vise le parent `Scriibe`, jamais `engines` seul.
pub fn engines_root(env: &Env) -> Option<PathBuf> {
    match env.os {
        Os::Mac => None,
        Os::Windows => env.local_appdata.as_ref().map(|l| l.join("Scriibe")),
        Os::Linux => env.data_base_linux().map(|b| b.join("Scriibe")),
    }
}

/// Ce qu'est un élément de la liste.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Data,
    Engines,
    Cache,
    Launcher,
    Leftover,
    TempSound,
    Program,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Data => "data",
            Kind::Engines => "engines",
            Kind::Cache => "cache",
            Kind::Launcher => "launcher",
            Kind::Leftover => "leftover",
            Kind::TempSound => "sound",
            Kind::Program => "program",
        }
    }
}

/// Un élément à supprimer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub kind: Kind,
    pub path: PathBuf,
    pub size: u64,
    /// L'élément est un lien symbolique : il part comme lien, sa cible reste en place.
    pub link: bool,
}

/// Ce que la désinstallation fera et ne fera pas.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Plan {
    pub items: Vec<Item>,
    /// Ce qui reste volontairement : `(nature, chemin, taille)` (`models` : « Garder mes modèles »).
    pub kept: Vec<(String, PathBuf, u64)>,
    /// Ce que la sauvegarde ne contient pas : `(nature, taille)` (`models`, `benches`, `logs`, `engines`).
    pub not_backed_up: Vec<(String, u64)>,
    /// Phrases pour la fenêtre (programme exclu, structure inattendue…).
    pub warnings: Vec<String>,
    /// Désinstallation interdite (`SCRIIBE_DATA`) : la raison.
    pub blocked: Option<String>,
    /// Une entrée de démarrage existe : elle sera retirée.
    pub startup: bool,
}

impl Plan {
    /// Les racines que la désinstallation supprimera (une sauvegarde rangée dedans partirait avec elle).
    pub fn roots(&self) -> Vec<PathBuf> {
        self.items.iter().filter(|i| i.kind != Kind::TempSound).map(|i| i.path.clone()).collect()
    }

    /// La taille totale de ce qui part à la Corbeille.
    pub fn trash_size(&self) -> u64 {
        self.items.iter().filter(|i| i.kind != Kind::TempSound).map(|i| i.size).sum()
    }
}

// ───────────────────────────── Garde-fous ─────────────────────────────

fn name_of(path: &Path) -> Option<String> {
    path.file_name().map(|n| n.to_string_lossy().into_owned())
}

/// Le dernier élément du chemin est exactement l'un de `names` (casse ignorée sous Windows et macOS).
fn has_name(path: &Path, names: &[&str], os: Os) -> bool {
    let Some(name) = name_of(path) else { return false };
    names.iter().any(|n| if os == Os::Linux { name == *n } else { name.eq_ignore_ascii_case(n) })
}

fn is_dir_nofollow(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|m| m.is_dir())
}

fn is_file_nofollow(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|m| m.is_file())
}

/// Taille d'un élément sans suivre aucun lien.
pub fn size_of(path: &Path) -> u64 {
    let Ok(meta) = std::fs::symlink_metadata(path) else { return 0 };
    if !meta.is_dir() {
        return meta.len();
    }
    let Ok(entries) = std::fs::read_dir(path) else { return 0 };
    entries.flatten().map(|e| size_of(&e.path())).sum()
}

fn is_link(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink())
}

fn item(kind: Kind, path: PathBuf) -> Item {
    Item { kind, size: size_of(&path), link: is_link(&path), path }
}

/// Une racine est acceptable si elle est absolue, au bon nom et existe (sans suivre de lien : un lien seul est accepté comme lien).
fn root_ok(path: &Path, names: &[&str], os: Os) -> bool {
    path.is_absolute() && has_name(path, names, os) && std::fs::symlink_metadata(path).is_ok()
}

/// Le programme est dans un dossier de développement (`target/`) ou dans un dépôt git : on n'y touche pas.
pub fn is_dev_location(exe: &Path) -> bool {
    exe.ancestors().skip(1).any(|a| a.file_name().is_some_and(|n| n == "target") || a.join(".git").exists())
}

/// Le programme (et son dossier) peuvent-ils être supprimés par l'utilisateur qui lance Scriibe ? Programme partagé (`root`, autre compte,
/// dossier non modifiable) : non.
pub fn program_removable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        use std::os::unix::fs::MetadataExt;
        let Some(parent) = path.parent() else { return false };
        let Ok(meta) = std::fs::symlink_metadata(path) else { return false };
        // SAFETY : geteuid n'a pas d'effet de bord.
        let euid = unsafe { libc::geteuid() };
        if meta.uid() != euid && euid != 0 {
            return false;
        }
        let Ok(c) = std::ffi::CString::new(parent.as_os_str().as_bytes()) else { return false };
        // SAFETY : `c` est une chaîne C valide pendant l'appel.
        unsafe { libc::access(c.as_ptr(), libc::W_OK) == 0 }
    }
    #[cfg(not(unix))]
    {
        std::fs::symlink_metadata(path).is_ok_and(|m| !m.permissions().readonly())
    }
}

/// `scriibe-son-<chiffres>-…​.wav` : le nom des sons temporaires de macOS et Linux (`macos/sounds.rs`, `linux/sounds.rs`).
pub fn is_temp_sound_name(name: &str) -> bool {
    let Some(rest) = name.strip_prefix("scriibe-son-") else { return false };
    let Some(rest) = rest.strip_suffix(".wav") else { return false };
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    !digits.is_empty() && rest[digits.len()..].starts_with('-')
}

fn temp_sounds(env: &Env) -> Vec<Item> {
    if env.os == Os::Windows || !env.tmp.is_absolute() {
        return Vec::new();
    }
    let Ok(entries) = std::fs::read_dir(&env.tmp) else { return Vec::new() };
    let mut out: Vec<Item> = entries
        .flatten()
        .filter(|e| e.file_name().to_str().is_some_and(is_temp_sound_name))
        // Seulement les fichiers ordinaires (ni lien, ni dossier) qui appartiennent à l'utilisateur.
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()) && owned_by_user(&e.path()))
        .map(|e| item(Kind::TempSound, e.path()))
        .collect();
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

fn owned_by_user(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        // SAFETY : geteuid n'a pas d'effet de bord.
        std::fs::symlink_metadata(path).is_ok_and(|m| m.uid() == unsafe { libc::geteuid() })
    }
    #[cfg(not(unix))]
    {
        std::fs::symlink_metadata(path).is_ok()
    }
}

fn caches(env: &Env) -> Vec<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    match env.os {
        Os::Mac => {
            if let Some(h) = &env.home {
                let lib = h.join("Library");
                for dir in ["Caches", "WebKit", "HTTPStorages"] {
                    candidates.push(lib.join(dir).join("fr.scriibe.app"));
                }
                candidates.push(lib.join("HTTPStorages").join("fr.scriibe.app.binarycookies"));
                candidates.push(lib.join("Preferences").join("fr.scriibe.app.plist"));
                candidates.push(lib.join("Saved Application State").join("fr.scriibe.app.savedState"));
            }
        }
        Os::Windows => {
            if let Some(l) = &env.local_appdata {
                candidates.push(l.join("fr.scriibe.app"));
            }
        }
        Os::Linux => {
            if let Some(c) = env.xdg_cache_home.clone().or_else(|| env.home.as_ref().map(|h| h.join(".cache"))) {
                candidates.push(c.join("fr.scriibe.app"));
            }
            if let Some(d) = env.data_base_linux() {
                candidates.push(d.join("fr.scriibe.app"));
            }
        }
    }
    const NAMES: [&str; 4] = ["fr.scriibe.app", "fr.scriibe.app.binarycookies", "fr.scriibe.app.plist", "fr.scriibe.app.savedState"];
    candidates.into_iter().filter(|p| p.is_absolute() && has_name(p, &NAMES, env.os) && std::fs::symlink_metadata(p).is_ok()).collect()
}

fn launcher_files(env: &Env) -> Vec<PathBuf> {
    if env.os != Os::Linux {
        return Vec::new();
    }
    let Some(base) = env.data_base_linux() else { return Vec::new() };
    const NAMES: [&str; 2] = ["fr.scriibe.Scriibe.desktop", "fr.scriibe.Scriibe.svg"];
    [
        base.join("applications").join("fr.scriibe.Scriibe.desktop"),
        base.join("icons").join("hicolor").join("scalable").join("apps").join("fr.scriibe.Scriibe.svg"),
    ]
    .into_iter()
    .filter(|p| p.is_absolute() && has_name(p, &NAMES, Os::Linux) && std::fs::symlink_metadata(p).is_ok())
    .collect()
}

/// Le programme de Scriibe d'après `exe` : `Scriibe.app` sur Mac, le fichier ailleurs.
pub fn program_of(env: &Env) -> Option<PathBuf> {
    match env.os {
        Os::Mac => env.exe.ancestors().find(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("app"))).map(Path::to_path_buf),
        _ => Some(env.exe.clone()),
    }
}

fn leftovers(env: &Env, program: &Path) -> Vec<PathBuf> {
    let Some(dir) = program.parent() else { return Vec::new() };
    let names: &[&str] = match env.os {
        Os::Mac => &["Scriibe.app.old", ".Scriibe-mise-a-jour"],
        Os::Windows => &["scriibe.exe.old", "scriibe.exe.new"],
        Os::Linux => &["scriibe.old", "scriibe.new"],
    };
    names.iter().map(|n| dir.join(n)).filter(|p| has_name(p, names, env.os) && std::fs::symlink_metadata(p).is_ok()).collect()
}

/// Construit la liste. `external_models` : chemins de modèles choisis hors du dossier de Scriibe (cités dans « ce qui reste », jamais supprimés).
pub fn build_plan(env: &Env, external_models: &[String], startup_present: bool) -> Plan {
    let mut plan = Plan { startup: startup_present, ..Plan::default() };
    if env.scriibe_data_set {
        plan.blocked = Some("La variable SCRIIBE_DATA est définie (essais) : elle déplace les données mais pas les moteurs ni les clés, la désinstallation est désactivée.".into());
        return plan;
    }
    const DATA_NAMES: [&str; 2] = ["Scriibe", "scriibe"];
    let mut roots: Vec<PathBuf> = Vec::new();

    // Données : structure reconnue (`settings.json` ou `instance.lock`).
    if let Some(data) = data_root(env) {
        if root_ok(&data, &DATA_NAMES, env.os) {
            if is_dir_nofollow(&data) && (is_file_nofollow(&data.join("settings.json")) || is_file_nofollow(&data.join("instance.lock"))) {
                let models = data.join("models");
                for (name, folder) in [("models", models.clone()), ("benches", data.join("bancs"))] {
                    if std::fs::symlink_metadata(&folder).is_ok() {
                        plan.not_backed_up.push((name.to_string(), size_of(&folder)));
                    }
                }
                let logs: u64 = ["scriibe.log", "llama-server.log"].iter().map(|n| size_of(&data.join(n))).sum();
                if logs > 0 {
                    plan.not_backed_up.push(("logs".into(), logs));
                }
                if env.keep_models && std::fs::symlink_metadata(&models).is_ok() {
                    plan.kept.push(("models".into(), models.clone(), size_of(&models)));
                    // Chaque enfant du dossier de données (sauf `models`) part tel quel, lien compris, sans descendre dedans.
                    let mut children: Vec<PathBuf> =
                        std::fs::read_dir(&data).map(|d| d.flatten().map(|e| e.path()).collect()).unwrap_or_default();
                    children.sort();
                    for child in children.into_iter().filter(|c| c != &models) {
                        plan.items.push(item(Kind::Data, child));
                    }
                } else {
                    plan.items.push(item(Kind::Data, data.clone()));
                }
                roots.push(data);
            } else {
                plan.warnings
                    .push("Un dossier de Scriibe n'a pas la structure attendue (réglages absents) : il est laissé en place.".into());
            }
        }
    }

    // Moteurs : le dossier `Scriibe` entier, reconnu par la présence de `engines` (jamais `engines` seul).
    if let Some(engines) = engines_root(env) {
        if root_ok(&engines, &DATA_NAMES, env.os)
            && !roots.iter().any(|r| r == &engines || engines.starts_with(r) || r.starts_with(&engines))
        {
            if is_dir_nofollow(&engines) && is_dir_nofollow(&engines.join("engines")) {
                plan.not_backed_up.push(("engines".into(), size_of(&engines)));
                plan.items.push(item(Kind::Engines, engines.clone()));
                roots.push(engines);
            } else {
                plan.warnings
                    .push("Un dossier de Scriibe n'a pas la structure attendue (moteurs absents) : il est laissé en place.".into());
            }
        }
    }

    for cache in caches(env) {
        plan.items.push(item(Kind::Cache, cache));
    }
    for file in launcher_files(env) {
        plan.items.push(item(Kind::Launcher, file));
    }

    // Programme : exclu d'un dossier de développement, d'un programme partagé et d'un autre nom.
    let program_names: &[&str] = match env.os {
        Os::Mac => &["Scriibe.app"],
        Os::Windows => &["scriibe.exe"],
        Os::Linux => &["scriibe"],
    };
    match program_of(env) {
        Some(program) if program.is_absolute() && std::fs::symlink_metadata(&program).is_ok() => {
            if is_dev_location(&program) {
                plan.warnings.push("Scriibe est lancée depuis un dossier de développement (target/ ou dépôt git) : seules les données partent, le programme reste.".into());
            } else if !has_name(&program, program_names, env.os) {
                plan.warnings
                    .push("Le programme porte un autre nom que celui de Scriibe : il n'est pas supprimé, à retirer à la main.".into());
            } else if !program_removable(&program) {
                plan.warnings
                    .push("Le programme (ou son dossier) n'est pas modifiable par votre compte : seules vos données partent.".into());
            } else {
                for left in leftovers(env, &program) {
                    plan.items.push(item(Kind::Leftover, left));
                }
                plan.items.push(item(Kind::Program, program));
            }
        }
        None if is_dev_location(&env.exe) => plan.warnings.push("Scriibe est lancée depuis un dossier de développement (target/ ou dépôt git) : seules les données partent, le programme reste.".into()),
        _ => plan.warnings.push("Le programme de Scriibe n'a pas été retrouvé : seules les données partent.".into()),
    }

    plan.items.extend(temp_sounds(env));

    for path in external_models {
        let p = Path::new(path);
        // Un modèle hors des dossiers de Scriibe reste ; un chemin dans un dossier qui part est supprimé avec lui (modèle téléchargé).
        if !path.is_empty() && !roots.iter().any(|r| p.starts_with(r)) {
            plan.kept.push(("external-model".into(), p.to_path_buf(), 0));
        }
    }
    plan
}

/// L'ordre des étapes (relecture n° 15) : données, moteurs, caches, lanceur ; sons ; programme en dernier.
fn step_order(kind: Kind) -> u8 {
    match kind {
        Kind::Data | Kind::Engines | Kind::Cache | Kind::Launcher | Kind::Leftover => 0,
        Kind::TempSound => 1,
        Kind::Program => 2,
    }
}

// ───────────────────────────── Exécution ─────────────────────────────

/// Ce que l'exécution fait réellement (remplacé par un faux dans les tests, qui ne touchent jamais aux vrais dossiers).
pub trait Effects {
    fn remove_startup(&mut self) -> Result<(), String>;
    /// Met à la Corbeille. Jamais de suppression définitive.
    fn trash(&mut self, path: &Path) -> Result<(), String>;
    /// Efface un fichier jetable (sons temporaires) directement.
    fn remove_file(&mut self, path: &Path) -> Result<(), String>;
    /// Efface les clés d'API : `(nom, résultat)` ; `Ok(false)` : absente. Vide si le système n'a pas de coffre.
    fn delete_keys(&mut self) -> Vec<(String, Result<bool, String>)>;
    fn reset_permissions(&mut self) -> Result<(), String>;
}

/// Ce qui s'est passé.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    pub startup_removed: bool,
    pub startup_error: Option<String>,
    pub trashed: Vec<(PathBuf, u64)>,
    pub sounds_removed: usize,
    pub failed: Vec<(PathBuf, String)>,
    pub keys_deleted: Vec<String>,
    pub keys_failed: Vec<(String, String)>,
    pub permissions_error: Option<String>,
    /// L'exécution s'est arrêtée après un échec de la Corbeille : la raison. Clés, autorisations et programme sont alors intacts.
    pub stopped: Option<String>,
}

/// Exécute `plan` dans l'ordre fixé. Un échec de la Corbeille (après `retries` reprises) arrête tout avant les clés et le programme.
pub fn execute(plan: &Plan, effects: &mut dyn Effects, retries: u32, delay: Duration) -> Report {
    let mut report = Report::default();
    if plan.blocked.is_some() {
        report.stopped = plan.blocked.clone();
        return report;
    }
    // 2. Entrée de démarrage retirée avant le reste : jamais une entrée vers un programme absent.
    if plan.startup {
        match effects.remove_startup() {
            Ok(()) => report.startup_removed = true,
            Err(e) => report.startup_error = Some(e),
        }
    }
    let mut items: Vec<&Item> = plan.items.iter().collect();
    items.sort_by_key(|i| step_order(i.kind));
    for it in items {
        if it.kind == Kind::Program {
            // 5. Clés puis 6. autorisations, avant le programme (7), après les fichiers.
            finish_keys(effects, &mut report);
            report.permissions_error = effects.reset_permissions().err();
        }
        if it.kind == Kind::TempSound {
            match effects.remove_file(&it.path) {
                Ok(()) => report.sounds_removed += 1,
                Err(e) => report.failed.push((it.path.clone(), e)),
            }
            continue;
        }
        let mut last = Err(String::new());
        for attempt in 0..=retries {
            last = effects.trash(&it.path);
            if last.is_ok() {
                break;
            }
            if attempt < retries {
                std::thread::sleep(delay);
            }
        }
        match last {
            Ok(()) => report.trashed.push((it.path.clone(), it.size)),
            Err(e) => {
                report.failed.push((it.path.clone(), e.clone()));
                report.stopped = Some(e);
                return report;
            }
        }
    }
    if !plan.items.iter().any(|i| i.kind == Kind::Program) {
        // Pas de programme dans la liste (exclu) : clés et autorisations après les fichiers quand même.
        finish_keys(effects, &mut report);
        report.permissions_error = effects.reset_permissions().err();
    }
    report
}

fn finish_keys(effects: &mut dyn Effects, report: &mut Report) {
    for (name, result) in effects.delete_keys() {
        match result {
            Ok(true) => report.keys_deleted.push(name),
            Ok(false) => {}
            Err(e) => report.keys_failed.push((name, e)),
        }
    }
}

/// « 3,2 Go », « 14 Mo »…
pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["octets", "Ko", "Mo", "Go"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1000.0 && unit < UNITS.len() - 1 {
        value /= 1000.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} octets")
    } else if value >= 10.0 {
        format!("{} {}", value.round() as u64, UNITS[unit])
    } else {
        format!("{:.1} {}", value, UNITS[unit]).replace('.', ",")
    }
}

/// Le dernier message : ce qui est parti, ce qui reste, la Corbeille à vider.
pub fn final_message(plan: &Plan, report: &Report, os: Os) -> String {
    let mut lines: Vec<String> = Vec::new();
    let total: u64 = report.trashed.iter().map(|(_, s)| s).sum();
    if let Some(reason) = &report.stopped {
        lines.push("La désinstallation s'est arrêtée : la Corbeille n'a pas pu recevoir un élément.".into());
        lines.push(format!("Raison : {reason}"));
        lines.push(
            "Rien d'autre n'a été supprimé définitivement. Relance « scriibe desinstaller » ou le bouton de Scriibe quand c'est réglé."
                .into(),
        );
    } else {
        lines.push("Scriibe est désinstallée.".into());
    }
    if !report.trashed.is_empty() {
        lines.push(format!("Partis à la Corbeille ({}) :", human_size(total)));
        lines.extend(report.trashed.iter().map(|(p, s)| format!("  • {} ({})", p.display(), human_size(*s))));
    }
    if report.sounds_removed > 0 {
        lines.push(format!("{} son(s) temporaire(s) effacé(s).", report.sounds_removed));
    }
    if report.startup_removed {
        lines.push("L'entrée de démarrage est retirée.".into());
    }
    if let Some(e) = &report.startup_error {
        lines.push(format!("L'entrée de démarrage n'a pas pu être retirée ({e}) : retire-la dans les réglages du système."));
    }
    if !report.keys_deleted.is_empty() {
        lines.push(format!("Clés d'API effacées : {}.", report.keys_deleted.join(", ")));
    }
    for (name, e) in &report.keys_failed {
        let how = match os {
            Os::Mac => "Trousseaux d'accès › rechercher « Scriibe »",
            Os::Windows => "Gestionnaire d'identifiants › Informations d'identification Windows › « Scriibe »",
            Os::Linux => "retire la variable d'environnement de ton profil",
        };
        lines.push(format!("La clé {name} n'a pas pu être effacée ({e}) : {how}."));
    }
    if os == Os::Linux {
        lines.push("Linux : retire DEEPGRAM_API_KEY et OPENROUTER_API_KEY de ton profil de shell si tu les y avais mises.".into());
    }
    if let Some(e) = &report.permissions_error {
        lines.push(format!("Les autorisations n'ont pas pu être retirées ({e}) : Réglages Système › Confidentialité et sécurité."));
    }
    if os == Os::Mac {
        lines.push(
            "Autorisations de Scriibe (micro, accessibilité, surveillance de l'entrée) : retirées ; une réinstallation les redemandera."
                .into(),
        );
    }
    for (p, e) in &report.failed {
        lines.push(format!("Reste : {} ({e})", p.display()));
    }
    for (kind, path, _) in &plan.kept {
        if kind == "models" {
            lines.push(format!("Gardés à ta demande : tes modèles téléchargés ({}).", path.display()));
        } else {
            lines.push(format!("Reste en place (hors de Scriibe) : {}", path.display()));
        }
    }
    if !report.trashed.is_empty() {
        lines.push(format!("Vide la Corbeille pour libérer {}.", human_size(total)));
    }
    lines.join("\n")
}

// ───────────────────────────── Corbeille et système réels ─────────────────────────────

/// Met `path` à la Corbeille du système. Un lien symbolique part comme lien, sa cible reste en place (documenté par la crate).
pub fn trash_path(path: &Path) -> Result<(), String> {
    #[allow(unused_mut)]
    let mut context = trash::TrashContext::default();
    // macOS : `NSFileManager`, jamais le Finder en AppleScript (qui déclencherait une demande « Automatisation »).
    #[cfg(target_os = "macos")]
    {
        use trash::macos::{DeleteMethod, TrashContextExtMacos};
        context.set_delete_method(DeleteMethod::NsFileManager);
    }
    context.delete(path).map_err(|e| e.to_string())
}

/// Windows : la Corbeille d'un volume a une capacité ; un élément plus gros est supprimé définitivement par le système. Règle
/// prudente, sans lire le registre : ce qui part ne doit pas dépasser 5 % de la taille du volume (sinon on s'arrête avant).
pub fn windows_trash_ok(total_to_trash: u64, volume_total: u64) -> bool {
    volume_total > 0 && u128::from(total_to_trash) * 20 <= u128::from(volume_total)
}

/// Un message si la Corbeille risque de ne pas pouvoir tout recevoir (Windows seulement).
pub fn capacity_problem(plan: &Plan) -> Option<String> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
        let first = plan.items.iter().find(|i| i.kind != Kind::TempSound)?;
        let wide: Vec<u16> = first.path.to_string_lossy().encode_utf16().chain(std::iter::once(0)).collect();
        let (mut free, mut total, mut total_free) = (0u64, 0u64, 0u64);
        // SAFETY : pointeurs vers des variables locales valides pendant l'appel.
        let ok = unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut free, &mut total, &mut total_free) } != 0;
        if ok && !windows_trash_ok(plan.trash_size(), total) {
            return Some(format!(
                "Ce qui doit partir ({}) est trop gros pour la Corbeille de ce disque : Windows le supprimerait définitivement. Coche « Garder mes modèles téléchargés », ou efface les modèles à la main, puis recommence.",
                human_size(plan.trash_size())
            ));
        }
        None
    }
    #[cfg(not(windows))]
    {
        let _ = plan;
        None
    }
}

const KEY_ACCOUNTS: [&str; 3] = ["deepgram", "openrouter", "github"];

/// Efface les clés d'API du coffre du système : `(nom, résultat)`, `Ok(false)` si absente. Linux : aucun coffre.
pub fn delete_credentials() -> Vec<(String, Result<bool, String>)> {
    #[cfg(target_os = "macos")]
    {
        KEY_ACCOUNTS
            .iter()
            .map(|account| {
                let result = match security_framework::passwords::delete_generic_password("Scriibe", account) {
                    Ok(()) => Ok(true),
                    // -25300 : errSecItemNotFound.
                    Err(e) if e.code() == -25300 => Ok(false),
                    // Une entrée créée par une autre appli ou une autre version (ancien compte github, constaté à l'essai réel) refuse la
                    // suppression directe (« Invalid attempt to change the owner of this item ») : l'outil `security` du système, qui passe
                    // par le même trousseau avec ses propres droits, a pu la supprimer.
                    Err(e) => match std::process::Command::new("/usr/bin/security")
                        .args(["delete-generic-password", "-s", "Scriibe", "-a", account])
                        .output()
                    {
                        Ok(out) if out.status.success() => Ok(true),
                        _ => Err(e.to_string()),
                    },
                };
                ((*account).to_string(), result)
            })
            .collect()
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::Security::Credentials::{CredDeleteW, CRED_TYPE_GENERIC};
        KEY_ACCOUNTS
            .iter()
            .map(|account| {
                let name: Vec<u16> = format!("Scriibe/{account}").encode_utf16().chain(std::iter::once(0)).collect();
                // SAFETY : chaîne large terminée par zéro, valide pendant l'appel.
                let result = if unsafe { CredDeleteW(name.as_ptr(), CRED_TYPE_GENERIC, 0) } != 0 {
                    Ok(true)
                } else {
                    // SAFETY : lecture du dernier code d'erreur du fil.
                    match unsafe { windows_sys::Win32::Foundation::GetLastError() } {
                        1168 => Ok(false), // ERROR_NOT_FOUND
                        code => Err(format!("code {code}")),
                    }
                };
                ((*account).to_string(), result)
            })
            .collect()
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        let _ = KEY_ACCOUNTS;
        Vec::new()
    }
}

/// Les effets réels, pour l'assistant.
pub struct RealEffects;

impl Effects for RealEffects {
    fn remove_startup(&mut self) -> Result<(), String> {
        crate::autostart::set(false)
    }
    fn trash(&mut self, path: &Path) -> Result<(), String> {
        trash_path(path)
    }
    fn remove_file(&mut self, path: &Path) -> Result<(), String> {
        std::fs::remove_file(path).map_err(|e| e.to_string())
    }
    fn delete_keys(&mut self) -> Vec<(String, Result<bool, String>)> {
        delete_credentials()
    }
    fn reset_permissions(&mut self) -> Result<(), String> {
        if cfg!(target_os = "macos") {
            // Ne vise que Scriibe ; un échec n'arrête rien.
            let out = std::process::Command::new("/usr/bin/tccutil")
                .args(["reset", "All", "fr.scriibe.app"])
                .output()
                .map_err(|e| e.to_string())?;
            if !out.status.success() {
                return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
            }
        }
        Ok(())
    }
}

// ───────────────────────────── L'assistant ─────────────────────────────

/// Le fichier de travail de l'assistant : **aucun chemin à supprimer**, seulement de quoi reconstruire la liste.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job {
    pub exe: PathBuf,
    pub keep_models: bool,
    pub pid: u32,
    pub external_models: Vec<String>,
    pub startup_present: bool,
}

impl Job {
    fn to_json(&self) -> String {
        serde_json::json!({"exe": self.exe.to_string_lossy(), "keepModels": self.keep_models, "pid": self.pid, "externalModels": self.external_models, "startup": self.startup_present}).to_string()
    }
    fn from_json(text: &str) -> Option<Job> {
        let v: serde_json::Value = serde_json::from_str(text).ok()?;
        Some(Job {
            exe: PathBuf::from(v["exe"].as_str()?),
            keep_models: v["keepModels"].as_bool()?,
            pid: u32::try_from(v["pid"].as_u64()?).ok()?,
            external_models: v["externalModels"].as_array()?.iter().filter_map(|m| m.as_str().map(str::to_string)).collect(),
            startup_present: v["startup"].as_bool().unwrap_or(false),
        })
    }
}

/// Lance l'assistant : une copie du programme dans un dossier temporaire, avec le fichier de travail. Le processus qui l'appelle doit
/// quitter tout de suite après ; l'assistant attend sa fin avant de toucher à quoi que ce soit.
pub fn start_helper(job: &Job) -> Result<(), String> {
    let dir = std::env::temp_dir().join(format!("scriibe-desinstallation-{}", job.pid));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| format!("dossier temporaire : {e}"))?;
    let me = std::env::current_exe().map_err(|e| e.to_string())?;
    let copy = dir.join(if cfg!(windows) { "scriibe.exe" } else { "scriibe" });
    std::fs::copy(&me, &copy).map_err(|e| format!("copie de l'assistant : {e}"))?;
    let job_file = dir.join("travail.json");
    std::fs::write(&job_file, job.to_json()).map_err(|e| format!("fichier de travail : {e}"))?;
    let mut command = std::process::Command::new(&copy);
    command.arg("desinstaller-fin").arg(&job_file);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW | DETACHED_PROCESS : l'assistant survit à la fin de Scriibe, sans console.
        command.creation_flags(0x0800_0000 | 0x0000_0008);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // Nouvelle session : l'assistant ne meurt pas avec le terminal ou le groupe de Scriibe.
        command.process_group(0);
    }
    command.stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    command.spawn().map(|_| ()).map_err(|e| format!("lancement de l'assistant : {e}"))
}

/// Attend la fin du processus `pid` (au plus `timeout`). Vrai s'il a fini. Windows : attente sur le handle ; Mac et Linux : `kill -0`.
pub fn wait_for_exit(pid: u32, timeout: Duration) -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
        use windows_sys::Win32::System::Threading::{OpenProcess, WaitForSingleObject};
        const SYNCHRONIZE: u32 = 0x0010_0000;
        // SAFETY : le handle est refermé avant de sortir.
        unsafe {
            let handle = OpenProcess(SYNCHRONIZE, 0, pid);
            if handle.is_null() {
                return true; // plus de processus de ce numéro
            }
            let done = WaitForSingleObject(handle, u32::try_from(timeout.as_millis()).unwrap_or(u32::MAX)) == WAIT_OBJECT_0;
            CloseHandle(handle);
            done
        }
    }
    #[cfg(unix)]
    {
        let start = std::time::Instant::now();
        loop {
            // SAFETY : signal 0 : simple test d'existence.
            let alive = unsafe { libc::kill(pid as libc::pid_t, 0) } == 0;
            if !alive {
                return true;
            }
            if start.elapsed() >= timeout {
                return false;
            }
            std::thread::sleep(Duration::from_millis(200));
        }
    }
    #[cfg(not(any(windows, unix)))]
    {
        let _ = (pid, timeout);
        true
    }
}

/// Montre le message final (boîte du système) et le garde dans un fichier du dossier temporaire. Au mieux : un échec n'arrête rien.
pub fn show_message(text: &str) {
    let report = std::env::temp_dir().join("scriibe-desinstallation.txt");
    let _ = std::fs::write(&report, text);
    #[cfg(target_os = "macos")]
    {
        let escaped = text.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n");
        let script = format!("display dialog \"{escaped}\" with title \"Scriibe\" buttons {{\"OK\"}} default button \"OK\"");
        let _ = std::process::Command::new("/usr/bin/osascript").args(["-e", &script]).output();
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONINFORMATION, MB_OK};
        let wide = |s: &str| -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() };
        // SAFETY : chaînes larges terminées par zéro, valides pendant l'appel.
        unsafe {
            MessageBoxW(std::ptr::null_mut(), wide(text).as_ptr(), wide("Scriibe").as_ptr(), MB_OK | MB_ICONINFORMATION);
        }
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let shown = std::process::Command::new("zenity")
            .args(["--info", "--title=Scriibe", "--no-markup", &format!("--text={text}")])
            .status()
            .is_ok_and(|s| s.success())
            || std::process::Command::new("kdialog").args(["--title", "Scriibe", "--msgbox", text]).status().is_ok_and(|s| s.success());
        if !shown {
            let _ = std::process::Command::new("notify-send").args(["Scriibe", text]).status();
        }
    }
}

/// Windows : la commande qui efface le dossier de l'assistant après `seconds` secondes (le temps que l'assistant se termine). Les guillemets
/// du chemin sont écrits tels quels (`raw_arg`) : par `args`, Rust les échappe en `\"`, que `cmd` ne comprend pas (constaté à l'essai
/// réel : le dossier restait dans `%TEMP%`).
#[cfg(windows)]
fn deferred_cleanup(dir: &Path, seconds: u32) -> std::process::Command {
    use std::os::windows::process::CommandExt;
    let mut command = std::process::Command::new("cmd");
    command
        .raw_arg(format!("/C ping -n {} 127.0.0.1 >nul & rmdir /S /Q \"{}\"", seconds + 1, dir.display()))
        .creation_flags(0x0800_0000 | 0x0000_0008);
    command
}

/// Le point d'entrée de l'assistant (`scriibe desinstaller-fin <fichier de travail>`). Rend le code de sortie.
pub fn finish(job_file: &Path) -> i32 {
    let Some(job) = std::fs::read_to_string(job_file).ok().and_then(|t| Job::from_json(&t)) else {
        return 2;
    };
    let work_dir = job_file.parent().map(Path::to_path_buf);
    if !wait_for_exit(job.pid, Duration::from_secs(60)) {
        show_message("Scriibe ne s'est pas fermée : rien n'a été supprimé. Ferme Scriibe depuis son icône, puis recommence.");
        return 3;
    }
    // La liste est reconstruite ici, avec les mêmes garde-fous : le fichier de travail ne désigne rien à supprimer.
    let env = Env::real(job.exe.clone(), job.keep_models);
    let plan = build_plan(&env, &job.external_models, job.startup_present);
    let report = execute(&plan, &mut RealEffects, 5, Duration::from_millis(1500));
    show_message(&final_message(&plan, &report, env.os));
    // Windows ne vide pas `%TEMP%` de lui-même : le dossier de l'assistant s'efface par un `cmd` différé. Ailleurs, la copie s'efface seule.
    if let Some(dir) = work_dir.filter(|d| name_of(d).is_some_and(|n| n.starts_with("scriibe-desinstallation-"))) {
        #[cfg(windows)]
        {
            let _ = deferred_cleanup(&dir, 4).spawn();
        }
        #[cfg(not(windows))]
        {
            let _ = std::fs::remove_dir_all(&dir);
        }
    }
    i32::from(report.stopped.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Un dossier factice propre à ce test.
    struct Scratch(PathBuf);
    impl Scratch {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("scriibe-uninstall-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            // Chemin canonique : sous macOS, /var est un lien vers /private/var.
            Self(dir.canonicalize().unwrap())
        }
        fn path(&self, rel: &str) -> PathBuf {
            self.0.join(rel)
        }
        fn file(&self, rel: &str, content: &str) -> PathBuf {
            let p = self.path(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, content).unwrap();
            p
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn env(os: Os, root: &Scratch, exe: PathBuf, keep_models: bool) -> Env {
        Env {
            os,
            home: Some(root.path("home")),
            appdata: Some(root.path("home/AppData/Roaming")),
            local_appdata: Some(root.path("home/AppData/Local")),
            xdg_data_home: None,
            xdg_cache_home: None,
            tmp: root.path("tmp"),
            exe,
            scriibe_data_set: false,
            keep_models,
        }
    }

    /// Un faux ordinateur Mac : données, caches, app, restes, sons temporaires, et des voisins qui ne doivent jamais bouger.
    fn fake_mac(root: &Scratch) -> PathBuf {
        root.file("home/Library/Application Support/Scriibe/settings.json", "{}");
        root.file("home/Library/Application Support/Scriibe/history.json", "[]");
        root.file("home/Library/Application Support/Scriibe/models/ggml-x.bin", "modèle");
        root.file("home/Library/Application Support/Scriibe/bancs/b.json", "{}");
        root.file("home/Library/Caches/fr.scriibe.app/cache.db", "c");
        root.file("home/Library/WebKit/fr.scriibe.app/x", "w");
        root.file("home/Library/Preferences/fr.scriibe.app.plist", "p");
        root.file("home/Applications/Scriibe.app/Contents/MacOS/scriibe", "binaire");
        root.file("home/Applications/Scriibe.app.old/Contents/x", "ancien");
        root.file("tmp/scriibe-son-123-Tink-0.5.wav", "son");
        root.file("tmp/scriibe-son-abc.wav", "pas le bon nom");
        root.file("tmp/autre.wav", "autre");
        // Les voisins : jamais touchés.
        root.file("home/Library/Application Support/Autre/données.txt", "a");
        root.file("home/Library/Application Support/Scriibe-copie/settings.json", "{}");
        root.file("home/Library/Caches/com.autre.app/c", "c");
        root.file("home/Documents/important.txt", "important");
        root.file("home/Applications/Autre.app/Contents/MacOS/autre", "autre");
        root.path("home/Applications/Scriibe.app/Contents/MacOS/scriibe")
    }

    /// Faux effets : « Corbeille » = un dossier du test, les clés et le démarrage sont notés sans rien toucher.
    struct Fake {
        trash_dir: PathBuf,
        log: Vec<String>,
        fail_on: Option<PathBuf>,
        keys: HashMap<String, bool>,
    }
    impl Fake {
        fn new(root: &Scratch) -> Self {
            let trash_dir = root.path("CORBEILLE");
            std::fs::create_dir_all(&trash_dir).unwrap();
            Fake {
                trash_dir,
                log: Vec::new(),
                fail_on: None,
                keys: HashMap::from([("deepgram".into(), true), ("openrouter".into(), true), ("github".into(), false)]),
            }
        }
    }
    impl Effects for Fake {
        fn remove_startup(&mut self) -> Result<(), String> {
            self.log.push("démarrage".into());
            Ok(())
        }
        fn trash(&mut self, path: &Path) -> Result<(), String> {
            if self.fail_on.as_deref() == Some(path) {
                return Err("Corbeille indisponible".into());
            }
            self.log.push(format!("corbeille {}", path.file_name().unwrap().to_string_lossy()));
            let n = std::fs::read_dir(&self.trash_dir).unwrap().count();
            std::fs::rename(path, self.trash_dir.join(format!("{n}-{}", path.file_name().unwrap().to_string_lossy())))
                .map_err(|e| e.to_string())
        }
        fn remove_file(&mut self, path: &Path) -> Result<(), String> {
            self.log.push(format!("effacé {}", path.file_name().unwrap().to_string_lossy()));
            std::fs::remove_file(path).map_err(|e| e.to_string())
        }
        fn delete_keys(&mut self) -> Vec<(String, Result<bool, String>)> {
            self.log.push("clés".into());
            let mut names: Vec<_> = self.keys.iter().map(|(k, v)| (k.clone(), Ok(*v))).collect();
            names.sort_by(|a, b| a.0.cmp(&b.0));
            names
        }
        fn reset_permissions(&mut self) -> Result<(), String> {
            self.log.push("tccutil".into());
            Ok(())
        }
    }

    #[test]
    fn the_mac_plan_lists_exactly_scriibe_things_and_nothing_else() {
        let root = Scratch::new("mac-plan");
        let exe = fake_mac(&root);
        let plan = build_plan(&env(Os::Mac, &root, exe, false), &[], true);
        let mut paths: Vec<String> =
            plan.items.iter().map(|i| i.path.strip_prefix(&root.0).unwrap().to_string_lossy().replace('\\', "/")).collect();
        paths.sort();
        assert_eq!(
            paths,
            [
                "home/Applications/Scriibe.app",
                "home/Applications/Scriibe.app.old",
                "home/Library/Application Support/Scriibe",
                "home/Library/Caches/fr.scriibe.app",
                "home/Library/Preferences/fr.scriibe.app.plist",
                "home/Library/WebKit/fr.scriibe.app",
                "tmp/scriibe-son-123-Tink-0.5.wav",
            ]
        );
        assert!(plan.startup && plan.blocked.is_none() && plan.warnings.is_empty());
        assert!(plan.not_backed_up.iter().any(|(k, s)| k == "models" && *s > 0));
        assert!(plan.not_backed_up.iter().any(|(k, _)| k == "benches"));
    }

    #[test]
    fn nothing_outside_the_list_is_touched_and_the_order_is_the_agreed_one() {
        let root = Scratch::new("mac-run");
        let exe = fake_mac(&root);
        let before_neighbours = [
            "home/Library/Application Support/Autre/données.txt",
            "home/Library/Application Support/Scriibe-copie/settings.json",
            "home/Library/Caches/com.autre.app/c",
            "home/Documents/important.txt",
            "home/Applications/Autre.app/Contents/MacOS/autre",
            "tmp/scriibe-son-abc.wav",
            "tmp/autre.wav",
        ];
        let plan = build_plan(&env(Os::Mac, &root, exe, false), &[], true);
        let mut fake = Fake::new(&root);
        let report = execute(&plan, &mut fake, 0, Duration::ZERO);
        assert_eq!(report.stopped, None);
        for n in before_neighbours {
            assert!(root.path(n).exists(), "{n} doit rester");
        }
        for gone in [
            "home/Library/Application Support/Scriibe",
            "home/Library/Caches/fr.scriibe.app",
            "home/Applications/Scriibe.app",
            "home/Applications/Scriibe.app.old",
            "tmp/scriibe-son-123-Tink-0.5.wav",
        ] {
            assert!(!root.path(gone).exists(), "{gone} doit être parti");
        }
        // Ordre : démarrage, fichiers (Corbeille), sons effacés, clés, autorisations, programme en dernier.
        let log = fake.log.join(" | ");
        let pos = |needle: &str| log.find(needle).unwrap_or_else(|| panic!("{needle} absent de {log}"));
        assert!(log.starts_with("démarrage"), "{log}");
        assert!(pos("corbeille fr.scriibe.app") < pos("effacé scriibe-son"));
        assert!(pos("effacé scriibe-son") < pos("clés"));
        assert!(pos("clés") < pos("tccutil"));
        assert!(log.ends_with("tccutil | corbeille Scriibe.app"), "le programme part en dernier : {log}");
        assert_eq!(report.keys_deleted, ["deepgram", "openrouter"], "une clé absente n'est pas une erreur");
        assert!(report.startup_removed);
    }

    #[test]
    fn a_failing_trash_stops_before_keys_and_program_and_never_deletes_for_good() {
        let root = Scratch::new("echec");
        let exe = fake_mac(&root);
        let plan = build_plan(&env(Os::Mac, &root, exe, false), &[], false);
        let mut fake = Fake::new(&root);
        fake.fail_on = Some(root.path("home/Library/Application Support/Scriibe"));
        let report = execute(&plan, &mut fake, 2, Duration::ZERO);
        assert_eq!(report.stopped.as_deref(), Some("Corbeille indisponible"));
        assert!(root.path("home/Library/Application Support/Scriibe/settings.json").exists(), "rien n'est supprimé");
        assert!(root.path("home/Applications/Scriibe.app").exists(), "le programme reste");
        assert!(!fake.log.iter().any(|l| l == "clés" || l == "tccutil"), "clés et autorisations après les fichiers : {:?}", fake.log);
        let message = final_message(&plan, &report, Os::Mac);
        assert!(message.contains("s'est arrêtée") && message.contains("Corbeille indisponible"), "{message}");
    }

    #[test]
    fn a_model_link_goes_away_and_its_target_is_left_intact() {
        let root = Scratch::new("lien");
        let exe = fake_mac(&root);
        // Un faux modèle ailleurs (comme dans ~/.lmstudio/models) et un lien vers lui dans models/.
        let target = root.file("home/.lmstudio/models/faux-modele.gguf", "le vrai modèle");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, root.path("home/Library/Application Support/Scriibe/models/lien.gguf")).unwrap();
        let plan = build_plan(&env(Os::Mac, &root, exe, false), &[], false);
        let mut fake = Fake::new(&root);
        execute(&plan, &mut fake, 0, Duration::ZERO);
        assert!(!root.path("home/Library/Application Support/Scriibe").exists());
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "le vrai modèle", "la cible est intacte");
        assert!(std::fs::symlink_metadata(root.path("home/Library/Application Support/Scriibe-copie/settings.json")).is_ok());
    }

    #[test]
    fn keeping_models_removes_each_other_child_one_by_one_and_leaves_models_alone() {
        let root = Scratch::new("garder");
        let exe = fake_mac(&root);
        let data = root.path("home/Library/Application Support/Scriibe");
        let target = root.file("home/.lmstudio/models/ext.gguf", "ext");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, data.join("lien-vers-ailleurs")).unwrap();
        let plan = build_plan(
            &env(Os::Mac, &root, exe, true),
            &["/Users/a/ext.gguf".into(), data.join("models/ggml-x.bin").to_string_lossy().into_owned()],
            false,
        );
        let data_items: Vec<_> = plan.items.iter().filter(|i| i.kind == Kind::Data).map(|i| name_of(&i.path).unwrap()).collect();
        assert!(!data_items.contains(&"models".to_string()), "models/ reste : {data_items:?}");
        assert!(
            data_items.contains(&"settings.json".to_string())
                && data_items.contains(&"history.json".to_string())
                && data_items.contains(&"bancs".to_string())
        );
        #[cfg(unix)]
        assert!(plan.items.iter().any(|i| i.link && name_of(&i.path).as_deref() == Some("lien-vers-ailleurs")), "un lien part comme lien");
        assert!(plan.kept.iter().any(|(k, _, _)| k == "models"));
        // Un modèle hors de Scriibe est cité ; un modèle dans models/ (téléchargé) n'est pas cité comme « hors ».
        assert_eq!(plan.kept.iter().filter(|(k, _, _)| k == "external-model").count(), 1);
        let mut fake = Fake::new(&root);
        execute(&plan, &mut fake, 0, Duration::ZERO);
        assert!(data.join("models/ggml-x.bin").exists(), "les modèles sont gardés");
        assert!(!data.join("settings.json").exists());
        assert!(target.exists());
    }

    #[test]
    fn windows_layout_engines_folder_is_a_different_whole_folder() {
        let root = Scratch::new("windows");
        root.file("home/AppData/Roaming/Scriibe/settings.json", "{}");
        root.file("home/AppData/Local/Scriibe/engines/whisper-server.exe", "w");
        root.file("home/AppData/Local/fr.scriibe.app/EBWebView/x", "c");
        let exe = root.file("home/scriibe/scriibe.exe", "exe");
        root.file("home/scriibe/scriibe.exe.old", "old");
        root.file("home/AppData/Local/Autre/x", "a");
        let plan = build_plan(&env(Os::Windows, &root, exe, false), &[], false);
        let mut paths: Vec<String> =
            plan.items.iter().map(|i| i.path.strip_prefix(&root.0).unwrap().to_string_lossy().replace('\\', "/")).collect();
        paths.sort();
        assert_eq!(
            paths,
            [
                "home/AppData/Local/Scriibe",
                "home/AppData/Local/fr.scriibe.app",
                "home/AppData/Roaming/Scriibe",
                "home/scriibe/scriibe.exe",
                "home/scriibe/scriibe.exe.old"
            ]
        );
        assert!(plan.items.iter().all(|i| i.kind != Kind::TempSound));
    }

    #[test]
    fn linux_layout_has_two_scriibe_folders_a_launcher_and_xdg_variables() {
        let root = Scratch::new("linux");
        root.file("home/.local/share/scriibe/settings.json", "{}");
        root.file("home/.local/share/Scriibe/engines/llama-server", "l");
        root.file("home/.local/share/applications/fr.scriibe.Scriibe.desktop", "d");
        root.file("home/.local/share/icons/hicolor/scalable/apps/fr.scriibe.Scriibe.svg", "s");
        root.file("home/.local/share/applications/autre.desktop", "a");
        root.file("home/.cache/fr.scriibe.app/c", "c");
        let exe = root.file("home/scriibe/scriibe", "exe");
        let plan = build_plan(&env(Os::Linux, &root, exe.clone(), false), &[], false);
        let mut paths: Vec<String> =
            plan.items.iter().map(|i| i.path.strip_prefix(&root.0).unwrap().to_string_lossy().replace('\\', "/")).collect();
        paths.sort();
        assert_eq!(
            paths,
            [
                "home/.cache/fr.scriibe.app",
                "home/.local/share/Scriibe",
                "home/.local/share/applications/fr.scriibe.Scriibe.desktop",
                "home/.local/share/icons/hicolor/scalable/apps/fr.scriibe.Scriibe.svg",
                "home/.local/share/scriibe",
                "home/scriibe/scriibe",
            ]
        );
        // XDG_DATA_HOME déplace les deux dossiers ; un chemin relatif est refusé.
        let mut moved = env(Os::Linux, &root, exe, false);
        moved.xdg_data_home = Some(PathBuf::from("relatif/partage"));
        assert!(build_plan(&moved, &[], false).items.iter().all(|i| i.kind == Kind::Program || i.kind == Kind::Cache));
    }

    #[test]
    fn guards_refuse_unknown_structure_wrong_names_dev_builds_and_relative_paths() {
        let root = Scratch::new("garde-fous");
        // Dossier de données sans settings.json ni verrou : laissé en place avec un avertissement.
        root.file("home/Library/Application Support/Scriibe/notes.txt", "a moi");
        let plan = build_plan(&env(Os::Mac, &root, root.path("home/x/Scriibe.app/Contents/MacOS/scriibe"), false), &[], false);
        assert!(plan.items.iter().all(|i| i.kind != Kind::Data), "{:?}", plan.items);
        assert!(plan.warnings.iter().any(|w| w.contains("structure")));
        // Programme dans target/ : exclu. Dans un dépôt git : exclu. Autre nom : exclu.
        for (rel, expect_warning) in [
            ("dev/Scriibe/target/release/Scriibe.app", "développement"),
            ("depot/Scriibe.app", "développement"),
            ("apps/Renommee.app", "autre nom"),
        ] {
            root.file(&format!("{rel}/Contents/MacOS/scriibe"), "x");
            if rel.starts_with("depot") {
                std::fs::create_dir_all(root.path("depot/.git")).unwrap();
            }
            let exe = root.path(&format!("{rel}/Contents/MacOS/scriibe"));
            let plan = build_plan(&env(Os::Mac, &root, exe, false), &[], false);
            assert!(plan.items.iter().all(|i| i.kind != Kind::Program), "{rel}");
            assert!(plan.warnings.iter().any(|w| w.contains(expect_warning)), "{rel} : {:?}", plan.warnings);
        }
        // Mac : un programme hors de tout .app (compilé) n'a pas de programme à supprimer.
        let plan = build_plan(&env(Os::Mac, &root, root.path("bin/scriibe"), false), &[], false);
        assert!(plan.items.iter().all(|i| i.kind != Kind::Program));
        // Variables relatives : rien.
        let mut rel = env(Os::Windows, &root, PathBuf::from("scriibe.exe"), false);
        rel.appdata = Some(PathBuf::from("."));
        rel.local_appdata = None;
        assert!(build_plan(&rel, &[], false).items.is_empty());
    }

    #[test]
    fn scriibe_data_blocks_everything() {
        let root = Scratch::new("bloque");
        let exe = fake_mac(&root);
        let mut e = env(Os::Mac, &root, exe, false);
        e.scriibe_data_set = true;
        let plan = build_plan(&e, &[], true);
        assert!(plan.items.is_empty() && plan.blocked.as_ref().is_some_and(|b| b.contains("SCRIIBE_DATA")));
        let mut fake = Fake::new(&root);
        let report = execute(&plan, &mut fake, 0, Duration::ZERO);
        assert!(fake.log.is_empty() && report.stopped.is_some(), "rien n'est fait");
    }

    #[cfg(unix)]
    #[test]
    fn a_shared_program_is_left_out_of_the_list() {
        use std::os::unix::fs::PermissionsExt;
        // En tant que root, tout est modifiable : le test n'a pas de sens.
        // SAFETY : geteuid n'a pas d'effet de bord.
        if unsafe { libc::geteuid() } == 0 {
            return;
        }
        let root = Scratch::new("partage");
        let exe = root.file("opt/Scriibe.app/Contents/MacOS/scriibe", "x");
        let dir = root.path("opt");
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).unwrap();
        let plan = build_plan(&env(Os::Mac, &root, exe, false), &[], false);
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(plan.items.iter().all(|i| i.kind != Kind::Program));
        assert!(plan.warnings.iter().any(|w| w.contains("modifiable")), "{:?}", plan.warnings);
    }

    #[test]
    fn only_real_temporary_sounds_match() {
        for ok in ["scriibe-son-123-Tink-0.5.wav", "scriibe-son-1-a.wav"] {
            assert!(is_temp_sound_name(ok), "{ok}");
        }
        for bad in [
            "scriibe-son-abc.wav",
            "scriibe-son-.wav",
            "scriibe-son-12.wav",
            "scriibe-son-12-x.mp3",
            "autre-son-12-x.wav",
            "scriibe-son-12-x.wav.bak",
        ] {
            assert!(!is_temp_sound_name(bad), "{bad}");
        }
    }

    #[test]
    fn temp_sounds_that_are_links_or_folders_are_never_listed() {
        let root = Scratch::new("sons");
        root.file("tmp/scriibe-son-1-a.wav", "ok");
        std::fs::create_dir_all(root.path("tmp/scriibe-son-2-dossier.wav")).unwrap();
        let outside = root.file("autre/precieux.wav", "précieux");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, root.path("tmp/scriibe-son-3-lien.wav")).unwrap();
        let sounds = temp_sounds(&env(Os::Mac, &root, root.path("x"), false));
        let names: Vec<String> = sounds.iter().map(|i| name_of(&i.path).unwrap()).collect();
        assert_eq!(names, ["scriibe-son-1-a.wav"]);
        assert!(outside.exists());
    }

    #[test]
    fn roots_follow_the_formulas_of_the_real_folders() {
        // Lecture seule : les formules de la liste sont celles de `data_folder` et `engines_folder` (sans SCRIIBE_DATA).
        if std::env::var_os("SCRIIBE_DATA").is_some() {
            return;
        }
        let real = Env::real(PathBuf::from("/x"), false);
        assert_eq!(data_root(&real), Some(crate::storage::data_folder()));
        match real.os {
            Os::Mac => assert_eq!(crate::storage::data_folder().join("engines"), scriibe_engines::engines_folder().unwrap()),
            _ => assert_eq!(engines_root(&real).map(|p| p.join("engines")), scriibe_engines::engines_folder()),
        }
    }

    #[test]
    fn the_helper_job_names_no_path_to_delete_and_roundtrips() {
        let job = Job {
            exe: PathBuf::from("/Applications/Scriibe.app/Contents/MacOS/scriibe"),
            keep_models: true,
            pid: 4242,
            external_models: vec!["/m/a.gguf".into()],
            startup_present: true,
        };
        let text = job.to_json();
        assert_eq!(Job::from_json(&text), Some(job));
        // Seuls le programme et les modèles cités (jamais supprimés) y figurent.
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        let keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
        assert_eq!(keys.len(), 5);
        assert_eq!(Job::from_json("pas du json"), None);
    }

    /// Essai manuel (`cargo test -p scriibe-platform real_trash -- --ignored`) : met à la **vraie** Corbeille un dossier factice qui contient
    /// un lien vers un faux fichier extérieur. Le lien part, la cible reste. Ignoré par défaut : il écrit dans la Corbeille de l'utilisateur.
    #[cfg(unix)]
    #[test]
    #[ignore = "écrit dans la vraie Corbeille (un petit dossier factice)"]
    fn real_trash_removes_a_link_and_leaves_its_target() {
        let root = Scratch::new("vraie-corbeille");
        let target = root.file("dehors/faux-modele.gguf", "cible à garder");
        let folder = root.path("a-jeter-scriibe-essai");
        std::fs::create_dir_all(folder.join("models")).unwrap();
        std::os::unix::fs::symlink(&target, folder.join("models/lien.gguf")).unwrap();
        std::fs::write(folder.join("settings.json"), "{}").unwrap();
        trash_path(&folder).unwrap();
        assert!(!folder.exists(), "le dossier est parti");
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "cible à garder");
        // Un lien seul, au premier niveau : il part comme lien.
        let link = root.path("lien-seul");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        trash_path(&link).unwrap();
        assert!(std::fs::symlink_metadata(&link).is_err());
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "cible à garder");
    }

    /// Windows : le dossier de l'assistant, dont le nom contient une espace, est bien effacé par la commande différée.
    #[cfg(windows)]
    #[test]
    fn the_deferred_cleanup_removes_a_folder_whose_path_has_spaces() {
        let root = Scratch::new("nettoyage differe");
        let folder = root.file("scriibe-desinstallation-1/travail.json", "{}").parent().unwrap().to_path_buf();
        let mut child = deferred_cleanup(&folder, 1).spawn().unwrap();
        child.wait().unwrap();
        for _ in 0..40 {
            if !folder.exists() {
                return;
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        panic!("le dossier est toujours là : {}", folder.display());
    }

    #[test]
    fn windows_recycle_bin_capacity_rule_is_prudent() {
        assert!(windows_trash_ok(10 << 30, 500 << 30));
        assert!(!windows_trash_ok(40 << 30, 500 << 30));
        assert!(!windows_trash_ok(1, 0));
    }

    #[test]
    fn final_message_says_what_left_what_stays_and_what_to_empty() {
        let root = Scratch::new("message");
        let exe = fake_mac(&root);
        let data = root.path("home/Library/Application Support/Scriibe");
        let plan = build_plan(&env(Os::Mac, &root, exe, true), &["/Users/a/ext.gguf".into()], true);
        let mut fake = Fake::new(&root);
        fake.keys.insert("openrouter".into(), true);
        let report = execute(&plan, &mut fake, 0, Duration::ZERO);
        let message = final_message(&plan, &report, Os::Mac);
        assert!(message.contains("Scriibe est désinstallée") && message.contains("Partis à la Corbeille"), "{message}");
        assert!(message.contains("Gardés à ta demande") && message.contains(&data.join("models").display().to_string()), "{message}");
        assert!(message.contains("/Users/a/ext.gguf") && message.contains("Vide la Corbeille"), "{message}");
        assert!(message.contains("deepgram"), "{message}");
        assert_eq!(human_size(3_200_000_000), "3,2 Go");
        assert_eq!(human_size(14_000_000), "14 Mo");
        assert_eq!(human_size(12), "12 octets");
    }
}
