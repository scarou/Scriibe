//! Sauvegarde complète et restauration (chantier 7 ; `docs/SYSTEME.md` § Cadrage, question 3).
//!
//! Export : le serveur lit les fichiers du dossier de données tels quels, construit l'enveloppe `kind: "backup"` (`scriibe_core::backup`),
//! l'utilisateur choisit où l'enregistrer, puis le fichier écrit est **relu et contrôlé comme pour un import** : s'il ne se relit pas, il
//! est effacé et l'opération échoue (rien ne doit être supprimé ensuite sur la foi d'une sauvegarde illisible).
//!
//! Import : lecture contrôlée par chemin (jamais envoyé par la page), aperçu, accord avec le jeton de l'aperçu, puis préparation de la
//! restauration (`scriibe_platform::restore`) et relance : les fichiers sont échangés au lancement suivant, avant toute lecture.

use super::export::{self, Target};
use super::import::{self, Picked};
use scriibe_core::backup::{self, Backup, BackupFiles};
use scriibe_platform::{migration, restore};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// La dernière sauvegarde écrite **et relue avec succès** : la désinstallation ne se fait que sur elle (ou sans sauvegarde, si
/// l'utilisateur a décoché la case).
static LAST: Mutex<Option<(PathBuf, Instant)>> = Mutex::new(None);
/// La sauvegarde lue, en attente de l'accord, avec le jeton de l'aperçu montré.
static PENDING: Mutex<Option<(String, Backup)>> = Mutex::new(None);

fn last() -> std::sync::MutexGuard<'static, Option<(PathBuf, Instant)>> {
    LAST.lock().unwrap_or_else(|e| e.into_inner())
}

fn pending() -> std::sync::MutexGuard<'static, Option<(String, Backup)>> {
    PENDING.lock().unwrap_or_else(|e| e.into_inner())
}

/// La sauvegarde validée il y a moins de `max_age`, si elle existe encore.
pub(crate) fn recent_backup(max_age: Duration) -> Option<PathBuf> {
    last().as_ref().filter(|(path, at)| at.elapsed() <= max_age && path.is_file()).map(|(path, _)| path.clone())
}

fn read_json(path: &Path) -> Option<Value> {
    let text = import::read_file_max(path, backup::MAX_BYTES, "fichier trop gros").ok()?;
    serde_json::from_str(&text).ok()
}

/// Les fichiers du dossier de données, lus tels quels (un fichier absent ou illisible reste `None`). `profile` : le dossier de
/// l'utilisateur actif après la migration (étape 4b). La sauvegarde garde alors **le même format** qu'avant : un seul objet de réglages
/// (ceux de l'ordinateur et ceux de l'utilisateur joints), son dictionnaire et son historique.
pub(crate) fn read_data_files(folder: &Path, profile: Option<&Path>) -> BackupFiles {
    let user = profile.unwrap_or(folder);
    let settings = read_json(&folder.join("settings.json"));
    let settings = match profile {
        None => settings,
        Some(dir) => settings.zip(read_json(&dir.join(migration::PROFILE_FILE))).and_then(|(s, p)| migration::joined_settings(&s, &p)),
    };
    BackupFiles {
        settings,
        dictionary: read_json(&user.join("dictionary.json")),
        history: read_json(&user.join("history.json")),
        history_totals: read_json(&user.join("history-totals.json")),
    }
}

/// Le nom proposé et le texte de la sauvegarde. `Err` : un message pour la page.
pub(crate) fn prepare(folder: &Path, profile: Option<&Path>, os: &str, now_secs: i64) -> Result<(String, String), String> {
    let files = read_data_files(folder, profile);
    let user = profile.unwrap_or(folder);
    if files.settings.is_none() && profile.is_some_and(|dir| dir.join(migration::PROFILE_FILE).exists()) {
        return Err("profile.json n'a pas pu être lu : la sauvegarde est refusée, rien n'est supprimé".into());
    }
    if files.settings.is_none() {
        return Err("aucun réglage à sauvegarder dans le dossier de données".into());
    }
    // Un fichier présent mais illisible (trop gros, abîmé) ne doit pas disparaître en silence : la désinstallation s'appuie sur la sauvegarde.
    for (name, value) in
        [("dictionary.json", &files.dictionary), ("history.json", &files.history), ("history-totals.json", &files.history_totals)]
    {
        if value.is_none() && user.join(name).exists() {
            return Err(format!("{name} n'a pas pu être lu : la sauvegarde est refusée, rien n'est supprimé"));
        }
    }
    let day = scriibe_core::export::day_of(now_secs);
    let value = backup::backup_export(super::update::current_version(), &day, os, &files);
    let text = serde_json::to_string_pretty(&value).map_err(|e| e.to_string())? + "\n";
    Ok((backup::file_name(&day), text))
}

/// Le chemin `path` est-il dans l'une des racines, ou est-il l'une d'elles ? Comparaison lexicale après avoir rendu le dossier parent
/// canonique (un lien dans le chemin ne contourne pas le refus).
pub(crate) fn is_under(path: &Path, roots: &[PathBuf]) -> bool {
    let resolved = match (path.parent().and_then(|p| p.canonicalize().ok()), path.file_name()) {
        (Some(parent), Some(name)) => parent.join(name),
        _ => path.to_path_buf(),
    };
    roots.iter().any(|root| {
        let root_resolved = root.canonicalize().unwrap_or_else(|_| root.clone());
        resolved.starts_with(&root_resolved) || path.starts_with(root)
    })
}

/// Écrit la sauvegarde puis la relit. Rend le chemin écrit. Un fichier qui ne se relit pas est effacé.
fn write_and_verify(target: Target, name: &str, text: &str) -> Result<PathBuf, String> {
    let written = match target {
        Target::File(path) => std::fs::write(&path, text).map(|()| path),
        Target::Folder(dir) => export::write_without_overwriting(&dir, name, text),
    };
    let path = written.map_err(|_| "enregistrement impossible : l’emplacement choisi n’est pas accessible en écriture".to_string())?;
    let verified =
        import::read_file_max(&path, backup::MAX_BYTES, "sauvegarde trop grosse : 64 Mo au maximum").and_then(|t| backup::parse_backup(&t));
    if let Err(e) = verified {
        let _ = std::fs::remove_file(&path);
        return Err(format!("la sauvegarde écrite ne se relit pas ({e}) : elle a été effacée, rien n'est supprimé"));
    }
    Ok(path)
}

/// `POST api/backup/export` : `{"ok": true, "saved": true, "path": …}`, `{"ok": true, "saved": false}` si l'utilisateur annule, ou
/// `{"ok": false, "error": …}`. `protected` : les racines que la désinstallation supprimera (une sauvegarde rangée dedans partirait avec).
pub(crate) fn export(folder: &Path, profile: Option<&Path>, protected: &[PathBuf]) -> Value {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
    let (name, text) = match prepare(folder, profile, std::env::consts::OS, now) {
        Ok(v) => v,
        Err(e) => return json!({"ok": false, "error": e}),
    };
    let target = match export::choose_target("Enregistrer la sauvegarde", &name) {
        Ok(Some(t)) => t,
        Ok(None) => return json!({"ok": true, "saved": false}),
        Err(e) => return json!({"ok": false, "error": e}),
    };
    if let Target::File(path) = &target {
        if is_under(path, protected) {
            return json!({"ok": false, "error": "cet emplacement est dans un dossier que la désinstallation supprime : choisis un autre dossier (Documents, par exemple)"});
        }
    }
    match write_and_verify(target, &name, &text) {
        Ok(path) => {
            *last() = Some((path.clone(), Instant::now()));
            json!({"ok": true, "saved": true, "path": path.to_string_lossy()})
        }
        Err(e) => json!({"ok": false, "error": e}),
    }
}

/// L'aperçu d'une sauvegarde lue (jamais le contenu des dictées).
fn preview_json(b: &Backup, token: &str) -> Value {
    let s = b.summary();
    json!({
        "ok": true,
        "token": token,
        "app": b.app,
        "exportedAt": b.exported_at,
        "os": b.os,
        "sameOs": b.same_os(std::env::consts::OS),
        "dictations": s.dictations,
        "days": s.days,
        "words": s.words,
        "corrections": s.corrections,
        "modes": s.modes,
    })
}

/// `POST api/backup/import/read` : `{"path"?}` (un chemin tapé n'est honoré que sans boîte, sous Linux), puis aperçu.
pub(crate) fn read(input: &Value) -> Value {
    let typed = input["path"].as_str().filter(|p| !p.trim().is_empty());
    let path = match (cfg!(not(any(windows, target_os = "macos"))), typed) {
        (true, Some(typed)) => import::typed_path(typed),
        _ => match import::choose_file("Restaurer une sauvegarde") {
            Picked::File(path) => path,
            Picked::Cancelled => return json!({"ok": true, "cancelled": true}),
            Picked::Failed(e) => return json!({"ok": false, "error": e}),
            Picked::NoDialog => {
                return json!({"ok": false, "ask_path": true, "error": "le choix d'un fichier n'est pas encore pris en charge sur ce système"})
            }
        },
    };
    match import::read_file_max(&path, backup::MAX_BYTES, "sauvegarde trop grosse : 64 Mo au maximum")
        .and_then(|t| backup::parse_backup(&t))
    {
        Ok(b) => {
            let token = scriibe_core::modes::new_id();
            let out = preview_json(&b, &token);
            *pending() = Some((token, b));
            out
        }
        Err(e) => {
            *pending() = None;
            json!({"ok": false, "error": e})
        }
    }
}

/// Oublie la sauvegarde en attente.
pub(crate) fn cancel() -> Value {
    *pending() = None;
    json!({"ok": true})
}

/// Les fichiers à écrire pour restaurer `backup` sur un ordinateur de système `os` dont les réglages actuels sont `current`.
pub(crate) fn files_to_restore(backup: &Backup, current: Option<&Value>, os: &str) -> Result<Vec<(&'static str, String)>, String> {
    let pretty = |v: &Value| serde_json::to_string_pretty(v).map_err(|e| e.to_string());
    let settings = backup.files.settings.as_ref().ok_or("cette sauvegarde ne contient pas de réglages")?;
    let mut files = vec![("settings.json", pretty(&backup::restored_settings(settings, current, backup.same_os(os)))?)];
    for (name, value) in [
        ("dictionary.json", &backup.files.dictionary),
        ("history.json", &backup.files.history),
        ("history-totals.json", &backup.files.history_totals),
    ] {
        if let Some(value) = value {
            files.push((name, pretty(value)?));
        }
    }
    Ok(files)
}

/// `POST api/backup/import/apply` : vérifie le jeton, prépare la restauration. `Ok(())` : l'appelant relance Scriibe, qui échange les
/// fichiers au lancement suivant. La sauvegarde en attente est gardée tant que la préparation n'a pas réussi.
/// `profile` : le dossier de l'utilisateur actif (dossier migré) ; ses fichiers sont alors remplacés dans ce dossier.
pub(crate) fn apply(folder: &Path, profile: Option<&Path>, input: &Value) -> Result<(), String> {
    let guard = pending();
    let (token, backup) = guard.as_ref().ok_or("aucune sauvegarde en attente : recommence la restauration")?;
    if input["token"].as_str() != Some(token.as_str()) {
        return Err("l'aperçu n'est plus à jour : recommence la restauration".into());
    }
    let current = read_json(&folder.join("settings.json"));
    let mut files = files_to_restore(backup, current.as_ref(), std::env::consts::OS)?;
    let id = match profile {
        None => None,
        Some(dir) => {
            let id = dir.file_name().and_then(|n| n.to_str()).ok_or("dossier d'utilisateur invalide")?;
            files = files_for_profile(files, id, read_json(&dir.join(migration::PROFILE_FILE)).as_ref())?;
            Some(id)
        }
    };
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    restore::stage(folder, &files, &stamp, id)
}

/// Dans un dossier migré, les réglages restaurés sont partagés comme à l'enregistrement : ceux de l'ordinateur dans `settings.json`
/// (avec la marque de la migration et l'utilisateur actif), ceux de l'utilisateur dans son `profile.json` (qui garde son nom affiché).
pub(crate) fn files_for_profile(
    files: Vec<(&'static str, String)>,
    id: &str,
    current_profile: Option<&Value>,
) -> Result<Vec<(&'static str, String)>, String> {
    let pretty = |v: &Value| serde_json::to_string_pretty(v).map_err(|e| e.to_string());
    let mut out = Vec::new();
    for (name, text) in files {
        if name != "settings.json" {
            out.push((name, text));
            continue;
        }
        let settings: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        let (computer, user) = migration::split_for_profile(&settings, id, current_profile).ok_or("réglages illisibles")?;
        out.push(("settings.json", pretty(&computer)?));
        out.push((migration::PROFILE_FILE, pretty(&user)?));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("scriibe-backup-web-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn sample_folder(name: &str) -> PathBuf {
        let dir = scratch(name);
        std::fs::write(dir.join("settings.json"), r#"{"hotkey":"right_option","whisperModelPath":"/Users/a/m.bin","customModes":[]}"#)
            .unwrap();
        std::fs::write(dir.join("dictionary.json"), r#"{"words":["Scriibe"],"corrections":[]}"#).unwrap();
        std::fs::write(dir.join("history.json"), "[]").unwrap();
        std::fs::write(dir.join("scriibe.log"), "journal : pas dans la sauvegarde").unwrap();
        std::fs::create_dir_all(dir.join("bancs")).unwrap();
        std::fs::write(dir.join("bancs").join("banc.json"), "{}").unwrap();
        std::fs::create_dir_all(dir.join("models")).unwrap();
        std::fs::write(dir.join("models").join("ggml-x.bin"), "gros modèle").unwrap();
        dir
    }

    #[test]
    fn the_backup_holds_the_four_files_and_nothing_else() {
        let dir = sample_folder("contenu");
        let (name, text) = prepare(&dir, None, "macos", 1_700_000_000).unwrap();
        assert_eq!(name, "scriibe-sauvegarde-2023-11-14.scriibe.json");
        for forbidden in ["journal", "banc", "gros modèle", "models"] {
            assert!(!text.contains(forbidden), "{forbidden}");
        }
        let b = backup::parse_backup(&text).unwrap();
        assert!(b.files.settings.is_some() && b.files.dictionary.is_some() && b.files.history.is_some());
        assert!(b.files.history_totals.is_none(), "un fichier absent reste absent");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn an_unreadable_history_is_refused_instead_of_silently_left_out() {
        let dir = sample_folder("historique-abime");
        std::fs::write(dir.join("history.json"), "{pas du json").unwrap();
        let err = prepare(&dir, None, "macos", 0).unwrap_err();
        assert!(err.contains("history.json") && err.contains("rien n'est supprimé"), "{err}");
        // Un fichier simplement absent reste permis (jamais d'historique).
        std::fs::remove_file(dir.join("history.json")).unwrap();
        assert!(prepare(&dir, None, "macos", 0).is_ok());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn no_settings_means_no_backup() {
        let dir = scratch("vide");
        assert!(prepare(&dir, None, "macos", 0).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_written_backup_is_read_back_and_a_bad_one_is_deleted() {
        let dir = sample_folder("relecture");
        let (name, text) = prepare(&dir, None, "macos", 0).unwrap();
        let out = dir.join("sortie");
        std::fs::create_dir_all(&out).unwrap();
        let path = write_and_verify(Target::Folder(out.clone()), &name, &text).unwrap();
        assert!(path.is_file());
        // Un contenu qui ne se relit pas : le fichier est effacé et l'opération échoue.
        let err = write_and_verify(Target::Folder(out.clone()), "mauvaise.scriibe.json", "pas une sauvegarde").unwrap_err();
        assert!(err.contains("ne se relit pas") && err.contains("rien n'est supprimé"), "{err}");
        assert!(!out.join("mauvaise.scriibe.json").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_path_under_a_root_to_delete_is_refused() {
        let dir = scratch("racines");
        let root = dir.join("Scriibe");
        std::fs::create_dir_all(root.join("sous")).unwrap();
        let elsewhere = dir.join("Documents");
        std::fs::create_dir_all(&elsewhere).unwrap();
        let roots = vec![root.clone()];
        assert!(is_under(&root.join("sauvegarde.scriibe.json"), &roots));
        assert!(is_under(&root.join("sous").join("s.json"), &roots));
        assert!(!is_under(&elsewhere.join("s.json"), &roots));
        // Un dossier voisin au nom qui commence pareil n'est pas dedans.
        assert!(!is_under(&dir.join("Scriibe-copie").join("s.json"), &roots));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn restoring_replaces_files_after_a_copy_and_keeps_this_computers_fields_from_another_system() {
        let dir = sample_folder("restauration");
        let (_, text) = prepare(&dir, None, "windows", 0).unwrap();
        let backup = backup::parse_backup(&text).unwrap();
        // Cet ordinateur (Mac) a sa propre touche : la sauvegarde vient de Windows.
        let current: Value = serde_json::from_str(r#"{"hotkey":"right_control","whisperModelPath":"/mac/m.bin"}"#).unwrap();
        let files = files_to_restore(&backup, Some(&current), "macos").unwrap();
        let settings: Value = serde_json::from_str(&files[0].1).unwrap();
        assert_eq!(settings["hotkey"], "right_control");
        assert_eq!(settings["whisperModelPath"], "/mac/m.bin");
        let same = files_to_restore(&backup, Some(&current), "windows").unwrap();
        assert!(same[0].1.contains("right_option"), "même système : tout est repris");
        restore::stage(&dir, &files, "20261113-100000", None).unwrap();
        assert!(restore::apply_pending(&dir).is_some());
        assert!(std::fs::read_to_string(dir.join("settings.json")).unwrap().contains("right_control"));
        assert!(dir.join("avant-import-20261113-100000").join("settings.json").is_file());
        assert!(dir.join("models").join("ggml-x.bin").is_file(), "les modèles ne sont jamais touchés");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Après la migration (étape 4b) : la sauvegarde garde son format (réglages joints, fichiers de l'utilisateur actif), et la restaurer
    /// remplace les fichiers de cet utilisateur sans défaire la migration.
    #[test]
    fn in_a_migrated_folder_the_backup_joins_the_user_and_restoring_it_keeps_the_migration() {
        let dir = sample_folder("migre");
        std::fs::write(dir.join("settings.json"), r#"{"hotkey":"right_option","firstName":"Léa","customModes":[]}"#).unwrap();
        let profile = migration::run(&dir).profile.expect("migré");
        let id = profile.file_name().unwrap().to_str().unwrap().to_string();
        assert!(!dir.join("dictionary.json").exists());

        let (_, text) = prepare(&dir, Some(&profile), "macos", 0).unwrap();
        let backup = backup::parse_backup(&text).unwrap();
        let settings = backup.files.settings.clone().unwrap();
        assert_eq!((settings["firstName"].as_str(), settings["hotkey"].as_str()), (Some("Léa"), Some("right_option")), "réglages joints");
        assert!(settings.get(migration::NAME_KEY).is_none(), "le nom affiché n'est pas un réglage");
        assert_eq!(backup.files.dictionary.as_ref().unwrap()["words"][0], "Scriibe", "le dictionnaire de l'utilisateur");

        // Restauration d'une sauvegarde où le prénom a changé.
        let mut changed = backup.clone();
        changed.files.settings.as_mut().unwrap()["firstName"] = json!("Paul");
        changed.files.dictionary = Some(json!({"words": ["Restauré"], "corrections": []}));
        let current = read_json(&dir.join("settings.json"));
        let files = files_to_restore(&changed, current.as_ref(), "macos").unwrap();
        let files = files_for_profile(files, &id, read_json(&profile.join(migration::PROFILE_FILE)).as_ref()).unwrap();
        restore::stage(&dir, &files, "20261114-100000", Some(&id)).unwrap();
        assert!(restore::apply_pending(&dir).unwrap().contains("profile.json"));
        let root: Value = read_json(&dir.join("settings.json")).unwrap();
        assert_eq!((root["schemaVersion"].as_u64(), root["activeProfile"].as_str()), (Some(2), Some(id.as_str())));
        assert!(root.get("firstName").is_none());
        let user: Value = read_json(&profile.join(migration::PROFILE_FILE)).unwrap();
        assert_eq!((user["firstName"].as_str(), user[migration::NAME_KEY].as_str()), (Some("Paul"), Some("Léa")), "le nom affiché reste");
        assert!(std::fs::read_to_string(profile.join("dictionary.json")).unwrap().contains("Restauré"));
        assert!(!dir.join("dictionary.json").exists(), "rien n'est remis à la racine");
        assert_eq!(migration::run(&dir).profile, Some(profile), "la migration n'a rien à refaire");
        assert!(!dir.join("dictionary.json").exists());

        // Un profile.json illisible : la sauvegarde est refusée plutôt que d'oublier les réglages de l'utilisateur.
        std::fs::write(dir.join(migration::USERS_DIR).join(&id).join(migration::PROFILE_FILE), "{cassé").unwrap();
        let err = prepare(&dir, Some(&dir.join(migration::USERS_DIR).join(&id)), "macos", 0).unwrap_err();
        assert!(err.contains("profile.json"), "{err}");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
