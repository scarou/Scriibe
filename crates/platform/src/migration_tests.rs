//! Tests de la migration (étape 4b), tous sur des dossiers temporaires : jamais sur le vrai dossier de données. Un test par ligne du
//! tableau des reprises de la décision 6 (`row_1_…`, `row_2_…`, `row_3_…`), plus les cas du retour à une version antérieure, du dossier
//! de Scribe copié après coup, du premier lancement et des refus.

use super::*;
use crate::storage::{FileConfig, FileHistory, HistorySink};
use chrono::Utc;
use scriibe_core::history::Dictation;
use scriibe_core::settings::AppSettings;
use serde_json::json;
use std::collections::BTreeMap;

const TODAY: &str = "2026-11-14";

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("scriibe-migration-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Un dossier de données de l'ancien rangement : réglages de l'utilisateur et de l'ordinateur mêlés, dictionnaire, historique.
fn old_folder(name: &str) -> PathBuf {
    let dir = scratch(name);
    let settings = json!({
        "schemaVersion": 1,
        "firstName": "Léa",
        "language": "en",
        "keepDictationText": false,
        "customCommands": [{"pas": "lisible"}],
        "theme": "dark",
        "pauseMedia": true,
        "scribeOnly": "gardé",
        // Sans cette clé, chaque lecture invente des candidats aux identifiants neufs : la vue ne se comparerait pas.
        "trialCandidates": [],
    });
    std::fs::write(dir.join("settings.json"), serde_json::to_string_pretty(&settings).unwrap()).unwrap();
    std::fs::write(
        dir.join("dictionary.json"),
        r#"{"words":["Scriibe","Whisper"],"corrections":[{"heard":"cri be","written":"Scriibe"}]}"#,
    )
    .unwrap();
    let history = FileHistory::open(dir.clone(), AppSettings::default);
    history.record(Dictation::new(Utc::now(), "Chrome", "un", "un", 12, 4.0, 0));
    history.record(Dictation::new(Utc::now(), "Notes", "deux", "deux", 30, 9.0, 1));
    assert!(dir.join("history.json").is_file() && dir.join("history-totals.json").is_file());
    dir
}

/// Tout le contenu d'un dossier : chemin relatif → octets (les dossiers vides comptent aussi).
fn tree(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            let rel = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
            if path.is_dir() {
                out.insert(format!("{rel}/"), Vec::new());
                walk(root, &path, out);
            } else {
                out.insert(rel, std::fs::read(&path).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn backups(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with(BACKUP_PREFIX))
        .map(|e| e.path())
        .collect();
    found.sort();
    found
}

fn work_folders(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir.join(USERS_DIR))
        .map(|e| e.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).filter(|n| n.starts_with(WORK_PREFIX)).collect())
        .unwrap_or_default()
}

/// Les quatre fichiers de l'ancien rangement, tels qu'ils étaient avant la migration.
fn originals(dir: &Path) -> BTreeMap<&'static str, Vec<u8>> {
    std::iter::once("settings.json").chain(USER_FILES).filter_map(|n| std::fs::read(dir.join(n)).ok().map(|b| (n, b))).collect()
}

/// L'état d'un dossier migré sans rien perdre. Rend l'identifiant de l'utilisateur.
fn assert_migrated(dir: &Path, before: &BTreeMap<&'static str, Vec<u8>>) -> String {
    let list = users(dir);
    assert_eq!(list.len(), 1, "un seul utilisateur : {list:?}");
    let (id, user) = list[0].clone();
    assert!(work_folders(dir).is_empty(), "aucun dossier de travail ne reste");

    // settings.json : la marque, l'utilisateur actif, les champs de l'ordinateur (inconnus compris), aucun champ de l'utilisateur.
    let settings = read_json(&dir.join("settings.json"));
    assert_eq!(settings["schemaVersion"], 2);
    assert_eq!(settings["activeProfile"], id.as_str());
    let old: Value = serde_json::from_slice(&before["settings.json"]).unwrap();
    for (key, value) in old.as_object().unwrap() {
        if owner_of(key) == Some(Owner::User) {
            assert!(settings.get(key).is_none(), "{key} a quitté settings.json");
        } else if key != "schemaVersion" {
            assert_eq!(&settings[key], value, "{key} est resté dans settings.json");
        }
    }

    // profile.json : les champs de l'utilisateur tels qu'ils étaient (rien de relu), et le nom affiché.
    let profile = read_json(&user.join(PROFILE_FILE));
    for (key, value) in old.as_object().unwrap() {
        if owner_of(key) == Some(Owner::User) {
            assert_eq!(&profile[key], value, "{key} est dans profile.json, identique");
        }
    }
    assert!(profile.get("theme").is_none() && profile.get("scribeOnly").is_none());

    // Fichiers de l'utilisateur : déplacés à l'octet près ; plus rien à la racine.
    for name in USER_FILES {
        assert_eq!(std::fs::read(user.join(name)).ok().as_ref(), before.get(name), "{name} identique dans le dossier de l'utilisateur");
        assert!(!dir.join(name).exists(), "{name} a quitté la racine");
    }

    // Une sauvegarde au moins, et l'une d'elles a tous les fichiers d'origine à l'octet près.
    assert!(
        backups(dir).iter().any(|b| before.iter().all(|(name, bytes)| std::fs::read(b.join(name)).ok().as_ref() == Some(bytes))),
        "une sauvegarde complète et fidèle existe"
    );

    // Vu par l'application : mêmes réglages, même dictionnaire, même historique qu'avant.
    let config = FileConfig::new(dir.to_path_buf());
    config.switch_profile(Some(user.clone()));
    let view = config.settings();
    let mut expected = AppSettings::from_value(&old).unwrap();
    expected.schema_version = 2;
    expected.active_profile = Some(id.clone());
    if old.get("trialCandidates").is_none() {
        expected.trial_candidates = view.trial_candidates.clone();
    }
    if !expected.hotkey.is_available() {
        expected.hotkey = scriibe_core::settings::Hotkey::system_default();
    }
    assert_eq!(view, expected, "la vue des réglages n'a pas changé");
    if before.contains_key("dictionary.json") {
        assert_eq!(config.dictionary().words, vec!["Scriibe".to_string(), "Whisper".to_string()]);
    }
    if before.contains_key("history.json") {
        assert_eq!(FileHistory::open(user.clone(), AppSettings::default).dictation_count(), 2);
    }
    assert!(config.problems().is_empty());
    id
}

#[test]
fn a_complete_migration_backs_up_builds_renames_rewrites_then_removes_and_a_second_run_does_nothing() {
    let dir = old_folder("complete");
    let before = originals(&dir);
    let report = run_until(&dir, TODAY, Stop::Never);
    let id = assert_migrated(&dir, &before);
    assert_eq!(report.profile, Some(dir.join(USERS_DIR).join(&id)));
    assert_eq!(backups(&dir), vec![dir.join(format!("{BACKUP_PREFIX}{TODAY}"))]);
    assert_eq!(read_json(&dir.join(USERS_DIR).join(&id).join(PROFILE_FILE))[NAME_KEY], "Léa", "le prénom devient le nom affiché");
    assert!(report.messages.iter().any(|m| m.contains(&id)), "{:?}", report.messages);

    // Deuxième lancement : la migration est faite, rien ne bouge.
    let after = tree(&dir);
    let again = run_until(&dir, TODAY, Stop::Never);
    assert_eq!(tree(&dir), after, "rien n'est réécrit");
    assert_eq!(again.profile, report.profile);
    assert!(again.messages.is_empty(), "{:?}", again.messages);
    assert_eq!(state(&dir), State::Ready(dir.join(USERS_DIR).join(&id)));
    let _ = std::fs::remove_dir_all(dir);
}

/// Tableau des reprises, ligne 1 : un arrêt avant l'étape 3 laisse un dossier `.migration-*` (ou rien), jamais un déplacement.
#[test]
fn row_1_a_stop_before_step_3_changes_nothing_and_the_next_launch_erases_the_work_folder_and_starts_again() {
    for (n, stop) in [Stop::AfterBackup, Stop::MidBuild, Stop::AfterBuild].into_iter().enumerate() {
        let dir = old_folder(&format!("row1-{n}"));
        let before = originals(&dir);
        let report = run_until(&dir, TODAY, stop);
        assert_eq!(report.profile, None, "{stop:?} : l'ancien rangement sert encore");
        assert_eq!(originals(&dir), before, "{stop:?} : aucun fichier de la racine n'a changé");
        assert!(users(&dir).is_empty(), "{stop:?} : aucun utilisateur complet");
        assert_eq!(work_folders(&dir).len(), usize::from(stop != Stop::AfterBackup), "{stop:?}");
        assert_eq!(state(&dir), State::NotMigrated);

        let report = run_until(&dir, TODAY, Stop::Never);
        assert_migrated(&dir, &before);
        assert!(report.profile.is_some());
        if stop != Stop::AfterBackup {
            assert!(report.messages.iter().any(|m| m.contains("travail inachevé effacé")), "{stop:?} : {:?}", report.messages);
        }
        assert_eq!(backups(&dir).len(), 2, "{stop:?} : la première sauvegarde n'est jamais écrasée, une deuxième est faite");
        let _ = std::fs::remove_dir_all(dir);
    }
}

/// Une sauvegarde interrompue (dossier temporaire caché) est effacée elle aussi, sans toucher aux sauvegardes complètes.
#[test]
fn row_1_an_interrupted_backup_is_erased_and_a_finished_one_is_kept() {
    let dir = old_folder("row1-backup");
    let before = originals(&dir);
    let partial = dir.join(format!(".{BACKUP_PREFIX}{TODAY}{BACKUP_WORK_MARK}X"));
    std::fs::create_dir_all(&partial).unwrap();
    std::fs::write(partial.join("settings.json"), "{\"coupé").unwrap();
    run_until(&dir, TODAY, Stop::Never);
    assert!(!partial.exists());
    assert_migrated(&dir, &before);
    let _ = std::fs::remove_dir_all(dir);
}

/// Tableau des reprises, ligne 2 : un utilisateur complet et un `settings.json` sans `schemaVersion >= 2`.
#[test]
fn row_2_a_stop_between_steps_3_and_4_takes_the_existing_user_back_instead_of_making_a_second_one() {
    let dir = old_folder("row2");
    let before = originals(&dir);
    let report = run_until(&dir, TODAY, Stop::AfterRename);
    let (id, user) = users(&dir).pop().expect("l'utilisateur est complet");
    assert_eq!(report.profile, Some(user.clone()), "il sert dès ce lancement");
    assert_eq!(originals(&dir), before, "settings.json n'est pas encore réécrit, rien n'est retiré");
    assert_eq!(state(&dir), State::NotMigrated, "pas de schemaVersion 2 : pas migré");

    let report = run_until(&dir, TODAY, Stop::Never);
    assert_eq!(assert_migrated(&dir, &before), id, "le même utilisateur, pas un deuxième");
    assert_eq!(report.profile, Some(user));
    assert!(report.messages.iter().any(|m| m.contains("existe déjà")), "{:?}", report.messages);
    let _ = std::fs::remove_dir_all(dir);
}

/// Tableau des reprises, ligne 3 : après l'étape 4, la migration est faite ; les fichiers restés à la racine sont effacés.
#[test]
fn row_3_after_step_4_nothing_is_redone_and_the_root_leftovers_are_removed() {
    for (n, stop) in [Stop::AfterRewrite, Stop::MidCleanup].into_iter().enumerate() {
        let dir = old_folder(&format!("row3-{n}"));
        let before = originals(&dir);
        run_until(&dir, TODAY, stop);
        let (id, user) = users(&dir).pop().unwrap();
        assert_eq!(state(&dir), State::Ready(user.clone()), "{stop:?} : migré");
        let left = USER_FILES.iter().filter(|n| dir.join(n).exists()).count();
        assert_eq!(left, if stop == Stop::AfterRewrite { 3 } else { 2 }, "{stop:?}");
        let profile_before = tree(&user);

        let report = run_until(&dir, TODAY, Stop::Never);
        assert_eq!(assert_migrated(&dir, &before), id);
        assert_eq!(tree(&user), profile_before, "{stop:?} : l'utilisateur n'est pas reconstruit");
        assert_eq!(backups(&dir).len(), 1, "{stop:?} : pas de nouvelle sauvegarde");
        assert_eq!(report.profile, Some(user));
        assert_eq!(report.messages.iter().filter(|m| m.contains("retiré de la racine")).count(), left, "{:?}", report.messages);
        let _ = std::fs::remove_dir_all(dir);
    }
}

/// Ligne 3, précision : un fichier écrit à la racine après la migration n'a pas de copie identique ; il est mis de côté, pas effacé.
#[test]
fn row_3_a_root_file_without_an_identical_copy_is_set_aside_instead_of_deleted() {
    let dir = old_folder("row3-aside");
    run_until(&dir, TODAY, Stop::Never);
    std::fs::write(dir.join("dictionary.json"), r#"{"words":["Écrit après"],"corrections":[]}"#).unwrap();
    let report = run_until(&dir, TODAY, Stop::Never);
    assert!(!dir.join("dictionary.json").exists());
    let aside = dir.join(format!("{BACKUP_PREFIX}{TODAY}-2"));
    assert!(std::fs::read_to_string(aside.join("dictionary.json")).unwrap().contains("Écrit après"), "{:?}", report.messages);
    let _ = std::fs::remove_dir_all(dir);
}

/// Retour à une version antérieure (qui ne connaît ni `schemaVersion` ni `activeProfile`), puis retour ici.
#[test]
fn back_from_an_older_version_the_existing_user_is_kept_with_the_computer_fields_it_changed() {
    let dir = old_folder("older-version");
    let before = originals(&dir);
    run_until(&dir, TODAY, Stop::Never);
    let (id, user) = users(&dir).pop().unwrap();
    let profile_before = std::fs::read(user.join(PROFILE_FILE)).unwrap();

    // La version antérieure lit settings.json v2 (champs de l'utilisateur vides), change le thème, enregistre : elle perd la marque et
    // l'utilisateur actif, écrit ses champs d'utilisateur par défaut ; elle crée aussi un dictionnaire et un historique à la racine.
    let mut old = read_json(&dir.join("settings.json"));
    let o = old.as_object_mut().unwrap();
    o.remove("schemaVersion");
    o.remove("activeProfile");
    o.insert("theme".into(), json!("light"));
    o.insert("firstName".into(), json!(""));
    o.insert("language".into(), json!("fr"));
    std::fs::write(dir.join("settings.json"), serde_json::to_string_pretty(&old).unwrap()).unwrap();
    std::fs::write(dir.join("dictionary.json"), r#"{"words":["Ancienne"],"corrections":[]}"#).unwrap();
    std::fs::write(dir.join("history.json"), "[]").unwrap();
    assert_eq!(state(&dir), State::NotMigrated);

    let report = run_until(&dir, TODAY, Stop::Never);
    assert_eq!(users(&dir), vec![(id.clone(), user.clone())], "pas de deuxième « Principal »");
    assert_eq!(report.profile, Some(user.clone()));
    let settings = read_json(&dir.join("settings.json"));
    assert_eq!((settings["schemaVersion"].clone(), settings["activeProfile"].clone()), (json!(2), json!(id)));
    assert_eq!(settings["theme"], "light", "le champ de l'ordinateur changé entre-temps est gardé");
    assert!(settings.get("firstName").is_none() && settings.get("language").is_none());
    assert_eq!(std::fs::read(user.join(PROFILE_FILE)).unwrap(), profile_before, "l'utilisateur n'est pas écrasé par les champs vides");
    assert_eq!(std::fs::read(user.join("dictionary.json")).ok(), before.get("dictionary.json").cloned());
    for name in ["dictionary.json", "history.json"] {
        assert!(!dir.join(name).exists(), "{name} a quitté la racine");
    }
    let second = dir.join(format!("{BACKUP_PREFIX}{TODAY}-2"));
    assert!(std::fs::read_to_string(second.join("dictionary.json")).unwrap().contains("Ancienne"), "rien de perdu : dans la sauvegarde");
    assert!(std::fs::read_to_string(second.join("settings.json")).unwrap().contains("light"));
    let _ = std::fs::remove_dir_all(dir);
}

/// Dossier de Scribe copié par-dessus après la migration : ses champs d'utilisateur ne sont pas fusionnés, ils restent dans la sauvegarde.
#[test]
fn a_scribe_folder_copied_after_the_migration_is_not_merged_and_stays_in_the_backup() {
    let dir = old_folder("scribe-after");
    run_until(&dir, TODAY, Stop::Never);
    let (id, user) = users(&dir).pop().unwrap();
    let user_before = tree(&user);
    std::fs::write(dir.join("settings.json"), r#"{"firstName":"Paul","hotkey":"right_option","restoreClipboard":false}"#).unwrap();
    std::fs::write(dir.join("dictionary.json"), r#"{"words":["Scribe"],"corrections":[]}"#).unwrap();

    run_until(&dir, TODAY, Stop::Never);
    assert_eq!(users(&dir), vec![(id.clone(), user.clone())]);
    assert_eq!(tree(&user), user_before, "l'utilisateur est intact");
    let settings = read_json(&dir.join("settings.json"));
    assert_eq!((settings["activeProfile"].clone(), settings["restoreClipboard"].clone()), (json!(id), json!(false)));
    assert!(settings.get("firstName").is_none());
    let second = dir.join(format!("{BACKUP_PREFIX}{TODAY}-2"));
    assert!(std::fs::read_to_string(second.join("settings.json")).unwrap().contains("Paul"));
    assert!(std::fs::read_to_string(second.join("dictionary.json")).unwrap().contains("Scribe"));
    assert!(!dir.join("dictionary.json").exists());
    let _ = std::fs::remove_dir_all(dir);
}

/// Dossier de Scribe copié **avant** la migration : migré comme l'ancien rangement (conséquences pour les autres chantiers).
#[test]
fn a_scribe_folder_copied_before_the_migration_is_migrated_like_the_old_layout() {
    let dir = scratch("scribe-before");
    std::fs::write(dir.join("settings.json"), r#"{"firstName":"","hotkey":"right_option","startSound":false,"language":"fr"}"#).unwrap();
    std::fs::write(dir.join("dictionary.json"), r#"{"words":["Scriibe","Whisper"],"corrections":[]}"#).unwrap();
    let before = originals(&dir);
    run_until(&dir, TODAY, Stop::Never);
    let id = assert_migrated(&dir, &before);
    assert_eq!(read_json(&dir.join(USERS_DIR).join(id).join(PROFILE_FILE))[NAME_KEY], DEFAULT_NAME, "prénom vide : « Principal »");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_first_launch_makes_one_principal_user_and_no_backup() {
    let dir = scratch("first-launch");
    let report = run_until(&dir, TODAY, Stop::Never);
    let (id, user) = users(&dir).pop().unwrap();
    assert_eq!(report.profile, Some(user.clone()));
    assert!(backups(&dir).is_empty(), "rien à sauvegarder");
    let settings = read_json(&dir.join("settings.json"));
    assert_eq!((settings["schemaVersion"].clone(), settings["activeProfile"].clone()), (json!(2), json!(id)));
    assert_eq!(settings["transcriptionEngine"], "whisper", "les réglages d'un premier lancement");
    assert_eq!(read_json(&user.join(PROFILE_FILE))[NAME_KEY], DEFAULT_NAME);
    assert!(!user.join("dictionary.json").exists(), "pas de dictionnaire inventé");
    // L'application démarre ensuite comme d'habitude : rien à créer, et ses enregistrements vont au bon endroit, nom affiché compris.
    let config = FileConfig::new(dir.clone());
    config.switch_profile(report.profile.clone());
    assert!(!config.create_settings_if_missing().unwrap());
    let mut settings = config.settings();
    settings.first_name = "Léa".into();
    config.save_settings(&settings).unwrap();
    let profile = read_json(&user.join(PROFILE_FILE));
    assert_eq!((profile["firstName"].clone(), profile[NAME_KEY].clone()), (json!("Léa"), json!(DEFAULT_NAME)), "le nom affiché reste");
    assert!(read_json(&dir.join("settings.json")).get("firstName").is_none());
    assert_eq!(read_json(&dir.join("settings.json"))["activeProfile"], id.as_str(), "l'enregistrement garde l'utilisateur actif");
    let _ = std::fs::remove_dir_all(dir);
}

/// Disque plein, dossier en lecture seule… : si la sauvegarde ne peut pas être faite, la migration s'arrête là et rien n'a changé.
#[cfg(unix)]
#[test]
fn a_backup_that_cannot_be_written_stops_everything_and_changes_nothing() {
    use std::os::unix::fs::PermissionsExt;
    // En tant que root, l'écriture n'échoue pas : le test n'a pas de sens.
    // SAFETY : geteuid n'a pas d'effet de bord.
    if unsafe { libc::geteuid() } == 0 {
        return;
    }
    let dir = old_folder("no-room");
    let before = tree(&dir);
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).unwrap();
    let report = run_until(&dir, TODAY, Stop::Never);
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(report.profile, None);
    assert!(report.messages.iter().any(|m| m.contains("rien n'a changé")), "{:?}", report.messages);
    assert_eq!(tree(&dir), before);
    let _ = std::fs::remove_dir_all(dir);
}

/// Si le dossier de l'utilisateur ne peut pas être construit (après la sauvegarde), rien n'a changé non plus, sauf la sauvegarde.
#[cfg(unix)]
#[test]
fn a_user_that_cannot_be_built_changes_nothing_but_the_backup() {
    use std::os::unix::fs::PermissionsExt;
    // SAFETY : geteuid n'a pas d'effet de bord.
    if unsafe { libc::geteuid() } == 0 {
        return;
    }
    let dir = old_folder("no-build");
    let before = originals(&dir);
    std::fs::create_dir_all(dir.join(USERS_DIR)).unwrap();
    std::fs::set_permissions(dir.join(USERS_DIR), std::fs::Permissions::from_mode(0o555)).unwrap();
    let report = run_until(&dir, TODAY, Stop::Never);
    std::fs::set_permissions(dir.join(USERS_DIR), std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(report.profile, None);
    assert_eq!(originals(&dir), before);
    assert!(work_folders(&dir).is_empty());
    let report = run_until(&dir, TODAY, Stop::Never);
    assert!(report.profile.is_some(), "nouvel essai réussi au lancement suivant");
    assert_migrated(&dir, &before);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn an_unreadable_or_newer_settings_file_is_never_migrated_nor_rewritten() {
    for (n, text) in ["{pas du json", r#"{"schemaVersion": 3, "firstName": "Léa"}"#].into_iter().enumerate() {
        let dir = old_folder(&format!("refused-{n}"));
        std::fs::write(dir.join("settings.json"), text).unwrap();
        let before = tree(&dir);
        let report = run_until(&dir, TODAY, Stop::Never);
        assert_eq!(tree(&dir), before, "{text} : rien n'est écrit");
        assert_eq!(report.profile, None, "pas d'utilisateur : l'ancien rangement");
        assert_eq!(report.messages.len(), 1, "{:?}", report.messages);
        assert_eq!(state(&dir), State::Refused(None));
        let _ = std::fs::remove_dir_all(dir);
    }
}

/// Fabrique un utilisateur complet à la main, dont les fichiers datent de `secs` secondes après l'origine.
fn make_user(dir: &Path, id: &str, secs: u64) -> PathBuf {
    let user = dir.join(USERS_DIR).join(id);
    std::fs::create_dir_all(&user).unwrap();
    std::fs::write(user.join(PROFILE_FILE), format!(r#"{{"name":"{id}"}}"#)).unwrap();
    let time = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000 + secs);
    std::fs::File::options().write(true).open(user.join(PROFILE_FILE)).unwrap().set_modified(time).unwrap();
    user
}

/// Ligne 2 avec plusieurs utilisateurs (étape 4c) et sans `activeProfile` (perdu par une version antérieure) : le dernier utilisé.
#[test]
fn row_2_with_several_users_and_no_active_one_the_last_used_is_taken() {
    let dir = scratch("several");
    make_user(&dir, "AAAA", 50);
    let recent = make_user(&dir, "BBBB", 100);
    make_user(&dir, "CCCC", 10);
    std::fs::create_dir_all(dir.join(USERS_DIR).join(".migration-X")).unwrap();
    std::fs::create_dir_all(dir.join(USERS_DIR).join("sans-profil")).unwrap();
    std::fs::write(dir.join("settings.json"), r#"{"theme":"dark"}"#).unwrap();
    let report = run_until(&dir, TODAY, Stop::Never);
    assert_eq!(report.profile, Some(recent));
    assert_eq!(read_json(&dir.join("settings.json"))["activeProfile"], "BBBB");
    assert_eq!(users(&dir).len(), 3, "ni création, ni suppression ; un dossier sans profile.json n'est pas un utilisateur");
    assert!(work_folders(&dir).is_empty());
    let _ = std::fs::remove_dir_all(dir);
}

/// `activeProfile` encore là (arrêt entre 3 et 4 avec plusieurs utilisateurs) : c'est lui, même s'il n'est pas le dernier utilisé.
#[test]
fn row_2_the_noted_active_user_wins_over_the_last_used() {
    let dir = scratch("noted");
    let noted = make_user(&dir, "AAAA", 10);
    make_user(&dir, "BBBB", 100);
    std::fs::write(dir.join("settings.json"), r#"{"activeProfile":"AAAA"}"#).unwrap();
    assert_eq!(run_until(&dir, TODAY, Stop::Never).profile, Some(noted));
    let _ = std::fs::remove_dir_all(dir);
}

/// Migré, mais l'utilisateur actif a disparu (dossier retiré à la main) : un autre le remplace ; s'il n'y en a plus, on en refait un.
#[test]
fn a_missing_active_user_is_replaced_and_with_no_user_left_one_is_made_again() {
    let dir = scratch("missing-active");
    let other = make_user(&dir, "BBBB", 10);
    std::fs::write(dir.join("settings.json"), r#"{"schemaVersion":2,"activeProfile":"AAAA"}"#).unwrap();
    assert_eq!(run_until(&dir, TODAY, Stop::Never).profile, Some(other.clone()));
    assert_eq!(read_json(&dir.join("settings.json"))["activeProfile"], "BBBB");

    std::fs::remove_dir_all(&other).unwrap();
    std::fs::write(dir.join("dictionary.json"), r#"{"words":["Reste"],"corrections":[]}"#).unwrap();
    let report = run_until(&dir, TODAY, Stop::Never);
    let (id, user) = users(&dir).pop().expect("un utilisateur est refait");
    assert_eq!(report.profile, Some(user.clone()));
    assert!(std::fs::read_to_string(user.join("dictionary.json")).unwrap().contains("Reste"), "ce qui restait à la racine est repris");
    assert_eq!(read_json(&dir.join("settings.json"))["activeProfile"], id.as_str());
    assert!(!dir.join("dictionary.json").exists() && backups(&dir).len() == 1);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn state_tells_the_subcommands_where_things_are_without_writing_anything() {
    let dir = scratch("state");
    assert_eq!(state(&dir), State::NotMigrated, "dossier vide");
    std::fs::write(dir.join("settings.json"), r#"{"schemaVersion":2}"#).unwrap();
    assert_eq!(state(&dir), State::NotMigrated, "migré sans utilisateur : seule l'application répare");
    let user = make_user(&dir, "AAAA", 0);
    assert_eq!(state(&dir), State::Ready(user.clone()));
    std::fs::write(dir.join("settings.json"), "{cassé").unwrap();
    assert_eq!(state(&dir), State::Refused(Some(user)));
    let before = tree(&dir);
    let _ = state(&dir);
    assert_eq!(tree(&dir), before);
    let _ = std::fs::remove_dir_all(dir);
}

/// La sauvegarde complète garde son format (un seul objet de réglages) : vue jointe à l'export, partage à la restauration.
#[test]
fn joined_then_split_settings_give_back_both_files() {
    let settings = json!({"schemaVersion": 2, "activeProfile": "AAAA", "theme": "dark", "firstName": "traîne"});
    let profile = json!({"name": "Léa", "firstName": "Léa", "language": "en"});
    let joined = joined_settings(&settings, &profile).unwrap();
    assert_eq!(joined, json!({"schemaVersion": 2, "activeProfile": "AAAA", "theme": "dark", "firstName": "Léa", "language": "en"}));
    let (computer, user) = split_for_profile(&joined, "BBBB", Some(&profile)).unwrap();
    assert_eq!(computer, json!({"schemaVersion": 2, "activeProfile": "BBBB", "theme": "dark"}));
    assert_eq!(user, json!({"name": "Léa", "firstName": "Léa", "language": "en"}));
    let (_, fresh) = split_for_profile(&json!({"firstName": ""}), "BBBB", None).unwrap();
    assert_eq!(fresh, json!({"name": DEFAULT_NAME, "firstName": ""}));
}
