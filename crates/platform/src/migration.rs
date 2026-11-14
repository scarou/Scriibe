//! Migration vers le rangement par utilisateur (étape 4b ; décision 6 de `docs/CADRAGE_PROFILS.md`, résultat dans `docs/PROFILS.md`).
//!
//! Avant : tout est à la racine du dossier de données (`settings.json` avec les réglages de l'utilisateur, `dictionary.json`,
//! `history.json`, `history-totals.json`). Après : les réglages de l'ordinateur restent dans `settings.json` (`schemaVersion: 2`,
//! `activeProfile`), ceux de l'utilisateur et ses fichiers sont dans `utilisateurs/<id>/` (`profile.json`, le dictionnaire, l'historique).
//!
//! **Quand.** Au lancement de l'application, après la prise du verrou d'instance et avant toute lecture des réglages ou de l'historique
//! (`commands::run`). Les sous-commandes de diagnostic ne migrent jamais (elles lisent seulement [`state`]).
//!
//! **Marque « migré ».** `settings.json` porte `schemaVersion >= 2` ; l'existence de `utilisateurs/` ne compte pas.
//!
//! **Étapes** (aucune n'est un déplacement avant la dernière) :
//! 1. copie de `settings.json` et des trois fichiers de l'utilisateur dans `sauvegarde-avant-utilisateurs-<date>/` (jamais effacé) ;
//!    si elle échoue (disque plein), on s'arrête : rien n'a changé ;
//! 2. construction de l'utilisateur dans `utilisateurs/.migration-<id>/`, à partir de **copies** ;
//! 3. renommage en `utilisateurs/<id>/` (atomique) ;
//! 4. réécriture de `settings.json` sans les champs de l'utilisateur, avec `schemaVersion: 2` et `activeProfile` (atomique) ;
//! 5. seulement après, suppression de la racine des trois fichiers de l'utilisateur (leur copie reste dans la sauvegarde).
//!
//! **Reprises** après un arrêt brutal, une version antérieure ou un dossier de Scribe copié par-dessus : voir [`run`].

use crate::storage::first_run_settings;
use scriibe_core::settings::{is_profile_id, owner_of, split_object, Owner, LEGACY_SCHEMA, SETTINGS_SCHEMA, USER_KEYS};
use serde_json::{Map, Value};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Le dossier des utilisateurs, dans le dossier de données.
pub const USERS_DIR: &str = "utilisateurs";
/// Les réglages d'un utilisateur, dans son dossier.
pub const PROFILE_FILE: &str = "profile.json";
/// Le nom affiché d'un utilisateur, dans `profile.json` (pas un réglage : `AppSettings` ne le voit pas).
pub const NAME_KEY: &str = "name";
/// Le nom affiché quand le prénom est vide (jamais l'identifiant de connexion de l'ordinateur).
pub const DEFAULT_NAME: &str = "Principal";
/// Les fichiers qui suivent l'utilisateur (déplacés de la racine vers son dossier).
pub const USER_FILES: [&str; 3] = ["dictionary.json", "history.json", "history-totals.json"];
/// Début du nom des dossiers de sauvegarde de la migration (`sauvegarde-avant-utilisateurs-2026-11-14`, puis `-2`, `-3`…).
pub const BACKUP_PREFIX: &str = "sauvegarde-avant-utilisateurs-";
const SETTINGS: &str = "settings.json";
/// Dossier de travail de l'étape 2, dans `utilisateurs/` : caché, et jamais un identifiant valide (`is_profile_id` refuse le point).
const WORK_PREFIX: &str = ".migration-";
/// Marque des dossiers temporaires d'une sauvegarde en cours (à la racine : `.sauvegarde-avant-utilisateurs-<date>.tmp-<id>`).
const BACKUP_WORK_MARK: &str = ".tmp-";

/// Ce que la migration a fait, pour le lancement.
#[derive(Debug, Default)]
pub struct Report {
    /// Le dossier de l'utilisateur à rendre actif (`FileConfig::switch_profile`, `FileHistory::switch`). `None` : l'ancien rangement
    /// reste en place pour ce lancement (migration impossible, remise au lancement suivant ; rien n'a changé).
    pub profile: Option<PathBuf>,
    /// Lignes pour le journal (vide : rien à signaler, la migration était déjà faite).
    pub messages: Vec<String>,
}

/// Où en est un dossier de données, sans rien écrire (sous-commandes de diagnostic, qui ne prennent pas le verrou d'instance).
#[derive(Debug, PartialEq, Eq)]
pub enum State {
    /// Pas encore migré (ou aucun utilisateur) : seule l'application, sous son verrou, peut migrer.
    NotMigrated,
    /// Migré : le dossier de l'utilisateur actif.
    Ready(PathBuf),
    /// `settings.json` illisible ou d'une version plus récente : on ne migre pas ; l'utilisateur à lire s'il y en a un.
    Refused(Option<PathBuf>),
}

/// La migration, au lancement de l'application (sous le verrou d'instance). Rejouable : chaque lancement reprend où le précédent
/// s'est arrêté, sans jamais créer un deuxième utilisateur ni effacer une donnée qui n'a pas de copie.
///
/// | Ce que trouve le lancement | Ce qui est fait |
/// |---|---|
/// | Un dossier `utilisateurs/.migration-*` (arrêt avant l'étape 3) | Effacé (sa source est intacte), puis tout recommence |
/// | Un utilisateur complet et un `settings.json` sans `schemaVersion >= 2` (arrêt entre 3 et 4, retour d'une version antérieure, dossier de Scribe copié par-dessus) | Sauvegarde, puis l'utilisateur existant (`activeProfile` s'il est encore là, sinon le dernier utilisé) devient l'actif ; les champs de l'ordinateur sont gardés, ceux de l'utilisateur à la racine ne sont pas fusionnés (ils restent dans la sauvegarde) |
/// | `schemaVersion >= 2` (arrêt après l'étape 4) | Rien, sinon effacer les fichiers de l'étape 5 restés à la racine (un fichier sans copie identique dans une sauvegarde est mis de côté au lieu d'être effacé) |
pub fn run(folder: &Path) -> Report {
    run_until(folder, &chrono::Local::now().format("%Y-%m-%d").to_string(), Stop::Never)
}

/// Où en est le dossier, sans rien écrire.
pub fn state(folder: &Path) -> State {
    match read_settings(&folder.join(SETTINGS)) {
        Source::Unreadable => State::Refused(pick_user(folder, None).map(|u| u.1)),
        Source::Missing => State::NotMigrated,
        Source::Object(o) => {
            let wanted = active_of(&o);
            match version_of(&o) {
                v if v > SETTINGS_SCHEMA => State::Refused(pick_user(folder, wanted.as_deref()).map(|u| u.1)),
                v if v < SETTINGS_SCHEMA => State::NotMigrated,
                _ => pick_user(folder, wanted.as_deref()).map_or(State::NotMigrated, |u| State::Ready(u.1)),
            }
        }
    }
}

/// Où s'arrêter (les tests simulent ainsi un arrêt brutal après chaque étape ; l'application va toujours au bout).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stop {
    Never,
    /// Après l'étape 1.
    AfterBackup,
    /// Pendant l'étape 2 : `profile.json` écrit dans le dossier de travail, pas encore le dictionnaire ni l'historique.
    MidBuild,
    /// Après l'étape 2 (dossier de travail complet, pas encore renommé).
    AfterBuild,
    /// Après l'étape 3.
    AfterRename,
    /// Après l'étape 4.
    AfterRewrite,
    /// Pendant l'étape 5 : un seul fichier effacé de la racine.
    MidCleanup,
}

fn run_until(folder: &Path, today: &str, stop: Stop) -> Report {
    let mut report = Report::default();
    remove_unfinished_work(folder, &mut report);
    let object = match read_settings(&folder.join(SETTINGS)) {
        Source::Unreadable => {
            // Sans les réglages, on ne sait ni ce qui est à l'utilisateur ni où l'on en est : rien n'est touché (la page avertit déjà).
            report.profile = pick_user(folder, None).map(|u| u.1);
            report.messages.push("migration: settings.json est illisible, rien n'est déplacé tant qu'il n'est pas réparé".into());
            return report;
        }
        Source::Missing => None,
        Source::Object(o) => Some(o),
    };
    let version = object.as_ref().map_or(LEGACY_SCHEMA, version_of);
    let wanted = object.as_ref().and_then(active_of);
    if version > SETTINGS_SCHEMA {
        report.profile = pick_user(folder, wanted.as_deref()).map(|u| u.1);
        report.messages.push(format!("migration: settings.json vient d'une version plus récente ({version}), rien n'est touché"));
        return report;
    }
    // Les utilisateurs, lus une seule fois. Une lecture impossible de `utilisateurs/` ne doit jamais faire croire qu'il n'y en a pas
    // (on en créerait un deuxième, vide) : rien n'est touché.
    let scan = match scan_users(folder) {
        Ok(scan) => scan,
        Err(e) => {
            report
                .messages
                .push(format!("migration: {USERS_DIR}/ est illisible ({e}), rien n'est touché ; nouvel essai au prochain lancement"));
            return report;
        }
    };
    let list = repair_users(scan.complete, scan.repairable, &mut report);
    if version >= SETTINGS_SCHEMA {
        if let Some((id, dir)) = pick_from(&list, wanted.as_deref()) {
            if !wanted.as_deref().is_some_and(|w| w.eq_ignore_ascii_case(&id)) {
                // L'utilisateur noté n'existe plus (dossier retiré à la main) : un autre devient l'actif.
                let mut fixed = object.clone().unwrap_or_default();
                fixed.insert("activeProfile".into(), Value::String(id.clone()));
                match write_durable(&folder.join(SETTINGS), &pretty(&fixed)) {
                    Ok(()) => report.messages.push(format!("migration: utilisateur actif introuvable, {id} le remplace")),
                    Err(e) => report.messages.push(format!("migration: utilisateur actif introuvable, {id} le remplace (non noté : {e})")),
                }
            }
            clean_leftovers(folder, today, &mut report);
            report.profile = Some(dir);
            return report;
        }
        // Migré, mais plus aucun utilisateur (dossier `utilisateurs/` retiré à la main) : on en refait un avec ce qui reste. Pas si
        // `utilisateurs/` contient des dossiers que Scriibe ne reconnaît pas : ce sont peut-être des données, on ne décide pas à leur place.
        if scan.unknown > 0 {
            report.messages.push(format!(
                "migration: aucun utilisateur reconnu, mais {} dossier(s) inconnu(s) dans {USERS_DIR}/ : rien n'est touché",
                scan.unknown
            ));
            return report;
        }
        report.messages.push("migration: aucun utilisateur trouvé, un utilisateur est recréé avec ce qui reste à la racine".into());
    }
    let existing = if version < SETTINGS_SCHEMA { pick_from(&list, wanted.as_deref()) } else { None };
    let source = object.unwrap_or_else(first_run_object);

    // Étape 1 : sauvegarde. Si elle échoue, rien n'a changé : l'ancien rangement sert pour ce lancement, on réessaiera.
    match backup(folder, today) {
        Ok(Some((dir, false))) => report.messages.push(format!("migration: copie des fichiers dans {}", file_name(&dir))),
        Ok(Some((dir, true))) => report.messages.push(format!("migration: fichiers déjà copiés à l'identique dans {}", file_name(&dir))),
        Ok(None) => {}
        Err(e) => {
            report.messages.push(format!("migration: sauvegarde impossible ({e}), rien n'a changé ; nouvel essai au prochain lancement"));
            return report;
        }
    }
    if stop == Stop::AfterBackup {
        return report;
    }

    let (id, dir) = match existing {
        Some(user) => {
            report.messages.push(format!(
                "migration: l'utilisateur {} existe déjà, il est repris (pas de nouvel utilisateur ; les réglages de l'utilisateur trouvés à la racine restent dans la sauvegarde)",
                user.0
            ));
            user
        }
        // Étapes 2 et 3.
        None => match build(folder, &source, stop) {
            Ok(Some(user)) => user,
            Ok(None) => return report,
            Err(e) => {
                report.messages.push(format!(
                    "migration: utilisateur impossible à construire ({e}), rien n'a changé ; nouvel essai au prochain lancement"
                ));
                return report;
            }
        },
    };
    // L'utilisateur est complet : il sert dès ce lancement, même si la suite échoue (le lancement suivant la reprendra).
    report.profile = Some(dir);
    if stop == Stop::AfterRename {
        return report;
    }

    // Étape 4.
    let (mut computer, _) = split_object(&source);
    computer.insert("schemaVersion".into(), Value::from(SETTINGS_SCHEMA));
    computer.insert("activeProfile".into(), Value::String(id.clone()));
    if let Err(e) = write_durable(&folder.join(SETTINGS), &pretty(&computer)) {
        report.messages.push(format!("migration: settings.json n'a pas pu être réécrit ({e}) ; repris au prochain lancement"));
        return report;
    }
    if stop == Stop::AfterRewrite {
        return report;
    }

    // Étape 5 : leur copie est dans la sauvegarde de l'étape 1, faite il y a un instant sous le verrou d'instance.
    for name in USER_FILES {
        let path = folder.join(name);
        if !path.exists() {
            continue;
        }
        if let Err(e) = std::fs::remove_file(&path) {
            report.messages.push(format!("migration: {name} n'a pas pu être retiré de la racine ({e}) ; repris au prochain lancement"));
        } else if stop == Stop::MidCleanup {
            return report;
        }
    }
    report.messages.push(format!("migration: données rangées dans {USERS_DIR}/{id}"));
    report
}

/// Ce que contient `settings.json`.
enum Source {
    Missing,
    Object(Map<String, Value>),
    Unreadable,
}

fn read_settings(path: &Path) -> Source {
    match std::fs::read_to_string(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Source::Missing,
        Err(_) => Source::Unreadable,
        // Un fichier vide (coupure de courant) se lit comme un fichier absent, comme `FileConfig` le fait.
        Ok(text) if text.trim().is_empty() => Source::Missing,
        Ok(text) => match serde_json::from_str::<Value>(&text) {
            Ok(Value::Object(o)) => Source::Object(o),
            _ => Source::Unreadable,
        },
    }
}

fn version_of(o: &Map<String, Value>) -> u64 {
    o.get("schemaVersion").and_then(Value::as_u64).unwrap_or(LEGACY_SCHEMA)
}

fn active_of(o: &Map<String, Value>) -> Option<String> {
    o.get("activeProfile").and_then(Value::as_str).filter(|id| is_profile_id(id)).map(str::to_string)
}

fn first_run_object() -> Map<String, Value> {
    match serde_json::to_value(first_run_settings()) {
        Ok(Value::Object(o)) => o,
        _ => Map::new(),
    }
}

fn pretty(o: &Map<String, Value>) -> String {
    serde_json::to_string_pretty(o).unwrap_or_else(|_| "{}".into())
}

fn file_name(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

/// Ce que contient `utilisateurs/`.
#[derive(Default)]
struct Scan {
    /// `utilisateurs/<id>/` : vrai dossier (pas un lien), identifiant valide, avec un `profile.json`. Triés par identifiant.
    complete: Vec<(String, PathBuf)>,
    /// Identifiant valide, sans `profile.json` mais avec un dictionnaire ou un historique (fichier perdu, création interrompue) : réparable.
    repairable: Vec<(String, PathBuf)>,
    /// Dossiers visibles ni complets, ni réparables, ni vides (ou liens) : peut-être des données, jamais traitées comme « pas d'utilisateur ».
    unknown: usize,
}

/// Lit `utilisateurs/` (absent : vide). Une erreur de lecture (antivirus, droits) est réessayée, puis rendue : jamais une liste vide.
fn scan_users(folder: &Path) -> std::io::Result<Scan> {
    let dir = folder.join(USERS_DIR);
    let entries = match retry(|| std::fs::read_dir(&dir)) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Scan::default()),
        Err(e) => return Err(e),
    };
    let mut scan = Scan::default();
    for entry in entries {
        let entry = entry?;
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            scan.unknown += 1;
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            scan.unknown += 1;
            continue;
        }
        if !kind.is_dir() {
            continue;
        }
        let path = entry.path();
        if is_profile_id(&name) && path.join(PROFILE_FILE).is_file() {
            scan.complete.push((name, path));
        } else if is_profile_id(&name) && USER_FILES.iter().any(|f| path.join(f).is_file()) {
            scan.repairable.push((name, path));
        } else if std::fs::read_dir(&path)?.next().is_some() {
            scan.unknown += 1;
        }
    }
    scan.complete.sort();
    scan.repairable.sort();
    Ok(scan)
}

/// Les utilisateurs complets (pour `state` et les tests) ; vide si `utilisateurs/` est illisible.
fn users(folder: &Path) -> Vec<(String, PathBuf)> {
    scan_users(folder).map(|s| s.complete).unwrap_or_default()
}

/// Rend complets les utilisateurs réparables : un `profile.json` neuf (nom « Principal », réglages par défaut) ; leur dictionnaire et leur
/// historique restent. Rend la liste des utilisateurs complets.
fn repair_users(mut complete: Vec<(String, PathBuf)>, repairable: Vec<(String, PathBuf)>, report: &mut Report) -> Vec<(String, PathBuf)> {
    for (id, dir) in repairable {
        let mut profile = Map::new();
        profile.insert(NAME_KEY.into(), Value::String(DEFAULT_NAME.into()));
        match write_durable(&dir.join(PROFILE_FILE), &pretty(&profile)) {
            Ok(()) => {
                report
                    .messages
                    .push(format!("migration: {PROFILE_FILE} manquait pour l'utilisateur {id}, il est refait (données gardées)"));
                complete.push((id, dir));
            }
            Err(e) => {
                report.messages.push(format!("migration: {PROFILE_FILE} manque pour l'utilisateur {id} et n'a pas pu être refait ({e})"))
            }
        }
    }
    complete.sort();
    complete
}

/// La dernière écriture dans le dossier d'un utilisateur (l'historique s'écrit à chaque dictée : c'est « le dernier utilisé »).
fn last_used(dir: &Path) -> Option<SystemTime> {
    std::iter::once(PROFILE_FILE).chain(USER_FILES).filter_map(|n| std::fs::metadata(dir.join(n)).and_then(|m| m.modified()).ok()).max()
}

/// L'utilisateur à rendre actif : `wanted` s'il existe (sans tenir compte des majuscules : macOS et Windows ne les distinguent pas),
/// sinon le dernier utilisé (à égalité, le plus petit identifiant), sinon aucun.
fn pick_from(list: &[(String, PathBuf)], wanted: Option<&str>) -> Option<(String, PathBuf)> {
    if let Some(user) = wanted.and_then(|w| list.iter().find(|(id, _)| id.eq_ignore_ascii_case(w))) {
        return Some(user.clone());
    }
    list.iter().max_by(|a, b| last_used(&a.1).cmp(&last_used(&b.1)).then_with(|| b.0.cmp(&a.0))).cloned()
}

fn pick_user(folder: &Path, wanted: Option<&str>) -> Option<(String, PathBuf)> {
    pick_from(&users(folder), wanted)
}

/// Réessaie une opération sur le disque (5 fois, 200 ms entre deux) : sous Windows, un antivirus ou l'indexeur tient souvent un
/// fichier qu'on vient d'écrire, et un renommage de dossier échoue alors un court instant. `NotFound` n'est pas réessayé.
fn retry<T>(mut op: impl FnMut() -> std::io::Result<T>) -> std::io::Result<T> {
    let mut attempt = 0;
    loop {
        match op() {
            Err(e) if attempt < 4 && e.kind() != std::io::ErrorKind::NotFound => {
                attempt += 1;
                std::thread::sleep(std::time::Duration::from_millis(200));
                let _ = e;
            }
            other => return other,
        }
    }
}

/// Efface ce qu'un arrêt brutal a laissé d'une étape 1 ou 2 inachevée. Leur source est intacte (aucun déplacement n'a eu lieu) :
/// la migration recommence (première ligne du tableau des reprises).
fn remove_unfinished_work(folder: &Path, report: &mut Report) {
    let mut doomed = Vec::new();
    if let Ok(entries) = std::fs::read_dir(folder.join(USERS_DIR)) {
        doomed.extend(entries.flatten().filter(|e| e.file_name().to_string_lossy().starts_with(WORK_PREFIX)).map(|e| e.path()));
    }
    if let Ok(entries) = std::fs::read_dir(folder) {
        doomed.extend(
            entries
                .flatten()
                .filter(|e| {
                    let name = e.file_name().to_string_lossy().into_owned();
                    name.starts_with(&format!(".{BACKUP_PREFIX}")) && name.contains(BACKUP_WORK_MARK)
                })
                .map(|e| e.path()),
        );
    }
    // Le fichier temporaire d'une réécriture de settings.json interrompue (étape 4) : l'ancien fichier est intact.
    if let Ok(entries) = std::fs::read_dir(folder) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with(&format!(".{SETTINGS}.")) && name.ends_with(".tmp") && entry.file_type().is_ok_and(|t| t.is_file()) {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
    for path in doomed {
        match std::fs::remove_dir_all(&path) {
            Ok(()) => report.messages.push(format!("migration: travail inachevé effacé ({})", file_name(&path))),
            Err(e) => report.messages.push(format!("migration: travail inachevé non effacé ({}, {e})", file_name(&path))),
        }
    }
}

/// Un nom de dossier libre à la racine : `<base>`, sinon `<base>-2`, `<base>-3`… (une sauvegarde n'est jamais écrasée).
fn free_folder(folder: &Path, base: &str) -> PathBuf {
    let first = folder.join(base);
    if !first.exists() {
        return first;
    }
    (2..).map(|n| folder.join(format!("{base}-{n}"))).find(|p| !p.exists()).unwrap_or(first)
}

/// Étape 1 : copie des fichiers présents dans un nouveau dossier de sauvegarde. Les copies sont faites dans un dossier temporaire,
/// écrites jusqu'au disque, puis le dossier est renommé : une sauvegarde visible est toujours complète. Si une sauvegarde existante a
/// déjà tous ces fichiers à l'identique (nouvel essai après un échec), elle sert telle quelle : `(dossier, true)`. `Ok(None)` : rien à copier.
fn backup(folder: &Path, today: &str) -> std::io::Result<Option<(PathBuf, bool)>> {
    let present: Vec<&str> = std::iter::once(SETTINGS).chain(USER_FILES).filter(|n| folder.join(n).is_file()).collect();
    if present.is_empty() {
        return Ok(None);
    }
    let originals: Vec<(&str, Vec<u8>)> =
        present.iter().map(|n| std::fs::read(folder.join(n)).map(|b| (*n, b))).collect::<Result<_, _>>()?;
    if let Some(same) = backup_folders(folder)
        .into_iter()
        .find(|b| originals.iter().all(|(name, bytes)| std::fs::read(b.join(name)).is_ok_and(|copy| &copy == bytes)))
    {
        return Ok(Some((same, true)));
    }
    let target = free_folder(folder, &format!("{BACKUP_PREFIX}{today}"));
    let work = folder.join(format!(".{}{BACKUP_WORK_MARK}{}", file_name(&target), scriibe_core::modes::new_id()));
    let result = (|| {
        std::fs::create_dir(&work)?;
        for name in &present {
            copy_durable(&folder.join(name), &work.join(name))?;
        }
        sync_dir(&work);
        retry(|| std::fs::rename(&work, &target))?;
        sync_dir(folder);
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&work);
    }
    result.map(|()| Some((target, false)))
}

/// Les dossiers de sauvegarde de la migration (visibles), à la racine.
fn backup_folders(folder: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(folder)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| e.file_name().to_string_lossy().starts_with(BACKUP_PREFIX) && e.file_type().is_ok_and(|t| t.is_dir()))
                .map(|e| e.path())
                .collect()
        })
        .unwrap_or_default();
    found.sort();
    found
}

/// Le nom affiché d'un utilisateur migré : son prénom, sinon « Principal ».
fn display_name(source: &Map<String, Value>) -> String {
    source.get("firstName").and_then(Value::as_str).map(str::trim).filter(|n| !n.is_empty()).unwrap_or(DEFAULT_NAME).to_string()
}

/// Étapes 2 et 3 : l'utilisateur est construit dans un dossier de travail à partir de copies, puis renommé. `Ok(None)` : arrêt simulé.
fn build(folder: &Path, source: &Map<String, Value>, stop: Stop) -> std::io::Result<Option<(String, PathBuf)>> {
    let users = folder.join(USERS_DIR);
    std::fs::create_dir_all(&users)?;
    let work = users.join(format!("{WORK_PREFIX}{}", scriibe_core::modes::new_id()));
    let result = (|| {
        std::fs::create_dir(&work)?;
        // Les clés de l'utilisateur telles qu'elles sont dans le fichier (rien n'est relu ni normalisé), plus le nom affiché.
        let (_, mut profile) = split_object(source);
        profile.insert(NAME_KEY.into(), Value::String(display_name(source)));
        write_durable(&work.join(PROFILE_FILE), &pretty(&profile))?;
        if stop == Stop::MidBuild {
            return Ok(None);
        }
        for name in USER_FILES {
            if folder.join(name).is_file() {
                copy_durable(&folder.join(name), &work.join(name))?;
            }
        }
        sync_dir(&work);
        if stop == Stop::AfterBuild {
            return Ok(None);
        }
        let id = scriibe_core::modes::new_id();
        let target = users.join(&id);
        retry(|| std::fs::rename(&work, &target))?;
        sync_dir(&users);
        Ok(Some((id, target)))
    })();
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&work);
    }
    result
}

/// Après l'étape 4 : les fichiers de l'étape 5 restés à la racine (arrêt pendant l'étape 5) sont effacés **s'ils ont une copie identique**
/// dans une sauvegarde de la migration. Sinon (écrits à la racine après coup), ils sont mis de côté dans une nouvelle sauvegarde.
fn clean_leftovers(folder: &Path, today: &str, report: &mut Report) {
    let left: Vec<&str> = USER_FILES.into_iter().filter(|n| folder.join(n).is_file()).collect();
    if left.is_empty() {
        return;
    }
    let backups = backup_folders(folder);
    let mut aside: Option<PathBuf> = None;
    for name in left {
        let path = folder.join(name);
        let Ok(bytes) = std::fs::read(&path) else {
            report.messages.push(format!("migration: {name} reste à la racine (illisible)"));
            continue;
        };
        if backups.iter().any(|b| std::fs::read(b.join(name)).is_ok_and(|copy| copy == bytes)) {
            match std::fs::remove_file(&path) {
                Ok(()) => report.messages.push(format!("migration: {name} retiré de la racine (sa copie est dans la sauvegarde)")),
                Err(e) => report.messages.push(format!("migration: {name} reste à la racine ({e})")),
            }
            continue;
        }
        let dir = match &aside {
            Some(dir) => dir.clone(),
            None => {
                let dir = free_folder(folder, &format!("{BACKUP_PREFIX}{today}"));
                if let Err(e) = std::fs::create_dir(&dir) {
                    report.messages.push(format!("migration: {name} reste à la racine (dossier de côté impossible : {e})"));
                    continue;
                }
                aside = Some(dir.clone());
                dir
            }
        };
        match std::fs::rename(&path, dir.join(name)) {
            Ok(()) => report.messages.push(format!("migration: {name} trouvé à la racine, mis de côté dans {}", file_name(&dir))),
            Err(e) => report.messages.push(format!("migration: {name} reste à la racine ({e})")),
        }
    }
    sync_dir(folder);
}

/// Écrit un fichier jusqu'au disque : fichier temporaire à nom unique, `sync_all`, renommage, puis le dossier. Une coupure de courant
/// laisse l'ancien fichier ou le nouveau, jamais un fichier vide (la migration efface ensuite des fichiers : elle ne peut pas s'en contenter).
fn write_durable(path: &Path, text: &str) -> std::io::Result<()> {
    let dir = path.parent().ok_or_else(|| std::io::Error::other("chemin sans dossier"))?;
    let temp = dir.join(format!(".{}.{}.tmp", file_name(path), scriibe_core::modes::new_id()));
    let result = (|| {
        let mut file = std::fs::File::create(&temp)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temp, path)?;
        sync_dir(dir);
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}

/// Copie un fichier jusqu'au disque.
fn copy_durable(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::copy(from, to)?;
    // Sous Windows, `sync_all` (FlushFileBuffers) demande un fichier ouvert en écriture. Une copie en lecture seule (attribut repris de
    // l'original) ne s'ouvre pas ainsi : elle reste valable, seule l'écriture forcée jusqu'au disque est sautée.
    match std::fs::OpenOptions::new().write(true).open(to) {
        Ok(file) => file.sync_all(),
        Err(_) => Ok(()),
    }
}

/// Écrit jusqu'au disque le contenu d'un dossier (noms créés, renommés). Sous Windows, rien à faire (et impossible sur un dossier).
fn sync_dir(dir: &Path) {
    #[cfg(unix)]
    if let Ok(d) = std::fs::File::open(dir) {
        let _ = d.sync_all();
    }
    #[cfg(not(unix))]
    let _ = dir;
}

/// La vue brute d'un utilisateur rangé à part, sous la forme d'un ancien `settings.json` : les clés de l'ordinateur de `settings`, plus
/// celles de l'utilisateur de `profile` (qui fait foi pour elles, comme `AppSettings::from_parts`). Sert à la sauvegarde complète,
/// qui garde ainsi le même format avant et après la migration.
pub fn joined_settings(settings: &Value, profile: &Value) -> Option<Value> {
    let mut joined = settings.as_object()?.clone();
    for key in USER_KEYS {
        joined.remove(*key);
    }
    for (key, value) in profile.as_object()? {
        if owner_of(key) == Some(Owner::User) {
            joined.insert(key.clone(), value.clone());
        }
    }
    Some(Value::Object(joined))
}

/// L'inverse, pour restaurer une sauvegarde complète dans un dossier migré : `(settings.json, profile.json)`. `settings.json` garde la
/// marque de la migration et l'utilisateur actif `id` ; `profile.json` garde ce que l'utilisateur actuel a en plus de ses réglages
/// (son nom affiché).
pub fn split_for_profile(settings: &Value, id: &str, current_profile: Option<&Value>) -> Option<(Value, Value)> {
    let (mut computer, user) = split_object(settings.as_object()?);
    computer.insert("schemaVersion".into(), Value::from(SETTINGS_SCHEMA));
    computer.insert("activeProfile".into(), Value::String(id.to_string()));
    let mut profile: Map<String, Value> = current_profile
        .and_then(Value::as_object)
        .map(|o| o.iter().filter(|(k, _)| owner_of(k) != Some(Owner::User)).map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default();
    profile.entry(NAME_KEY).or_insert_with(|| Value::String(display_name(&user)));
    profile.extend(user);
    Some((Value::Object(computer), Value::Object(profile)))
}

#[cfg(test)]
#[path = "migration_tests.rs"]
mod tests;
