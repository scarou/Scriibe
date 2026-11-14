//! Les utilisateurs vus par `FileConfig` (étape 4c) : verrou et contrôle de l'utilisateur actif, changement d'utilisateur, copie, renommage,
//! suppression. Tous sur des dossiers temporaires, avec un dossier de données migré par `migration::run` comme au lancement.

use super::*;
use crate::storage::FileConfig;
use scriibe_core::dictionary::UserDictionary;
use std::sync::Arc;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("scriibe-profiles-config-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Un dossier de données migré (un utilisateur « Principal ») et la configuration positionnée dessus, comme au lancement.
fn migrated(name: &str) -> (PathBuf, FileConfig, ProfileInfo) {
    let dir = scratch(name);
    let report = migration::run(&dir);
    let folder = report.profile.expect("la migration d'un dossier vide donne un utilisateur");
    let config = FileConfig::new(dir.clone());
    config.switch_profile(Some(folder.clone()));
    let id = config.active_profile_id().unwrap();
    (dir, config, ProfileInfo { id, name: DEFAULT_NAME.into(), folder })
}

fn words(list: &[&str]) -> UserDictionary {
    UserDictionary { words: list.iter().map(|w| w.to_string()).collect(), corrections: vec![] }
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn the_lock_accepts_the_active_user_whatever_the_case_and_refuses_any_other_or_none() {
    let (dir, config, principal) = migrated("lock");
    assert!(config.lock_for(Some(&principal.id)).is_ok());
    assert!(config.lock_for(Some(&principal.id.to_lowercase())).is_ok(), "les majuscules ne comptent pas (macOS et Windows)");
    assert!(matches!(config.lock_for(Some("AUTRE")), Err(ProfileError::Stale)));
    assert!(matches!(config.lock_for(None), Err(ProfileError::Stale)), "la page ne connaissait aucun utilisateur : ce n'est pas le bon");
    // Sans utilisateur rangé à part, seul « aucun » convient.
    let bare = FileConfig::new(scratch("lock-bare"));
    assert!(bare.lock_for(None).is_ok());
    assert!(matches!(bare.lock_for(Some("X")), Err(ProfileError::Stale)));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn switching_writes_the_active_user_and_reads_the_other_users_files() {
    let (dir, config, principal) = migrated("switch");
    let mut settings = config.settings();
    settings.first_name = "Alice".into();
    config.save_settings(&settings).unwrap();
    config.save_dictionary(&words(&["alice"])).unwrap();
    let bruno = config.create_profile("Bruno").unwrap();
    let mut ran_with = None;
    let (target, ()) = config.switch_to_profile(&bruno.id, |folder| ran_with = Some(folder.to_path_buf())).unwrap();
    assert_eq!(target.id, bruno.id);
    assert_eq!(ran_with, Some(bruno.folder.clone()), "la suite (l'historique) tourne avec le dossier du nouvel utilisateur");
    assert_eq!(config.active_profile_id().as_deref(), Some(bruno.id.as_str()));
    assert_eq!(config.active_profile_name().as_deref(), Some("Bruno"));
    assert_eq!(config.settings().first_name, "", "les réglages de Bruno, pas ceux d'Alice");
    assert!(config.dictionary().words.is_empty());
    assert_eq!(read_json(&dir.join("settings.json"))["activeProfile"], bruno.id.as_str());
    // Un lancement suivant retrouve Bruno (c'est la migration qui choisit l'utilisateur à l'ouverture).
    assert_eq!(migration::run(&dir).profile, Some(bruno.folder.clone()));
    // Retour à Alice : ses réglages et son dictionnaire sont intacts.
    config.switch_to_profile(&principal.id, |_| ()).unwrap();
    assert_eq!(config.settings().first_name, "Alice");
    assert_eq!(config.dictionary().words, vec!["alice"]);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn an_unknown_user_or_a_refused_settings_file_blocks_the_switch() {
    let (dir, config, principal) = migrated("switch-refused");
    let bruno = config.create_profile("Bruno").unwrap();
    assert!(matches!(config.switch_to_profile("INCONNU", |_| ()), Err(ProfileError::Unknown)));
    assert!(matches!(config.switch_to_profile("../..", |_| ()), Err(ProfileError::Unknown)), "un identifiant n'est jamais un chemin");
    assert_eq!(config.active_profile_id().as_deref(), Some(principal.id.as_str()));
    // `settings.json` illisible : rien n'est écrit, rien ne change.
    std::fs::write(dir.join("settings.json"), "pas du json").unwrap();
    let reread = FileConfig::new(dir.clone());
    reread.switch_profile(Some(principal.folder.clone()));
    assert!(matches!(reread.switch_to_profile(&bruno.id, |_| panic!("ne doit pas tourner")), Err(ProfileError::Io(_))));
    assert_eq!(reread.active_profile_id().as_deref(), Some(principal.id.as_str()));
    assert_eq!(std::fs::read_to_string(dir.join("settings.json")).unwrap(), "pas du json");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_save_of_settings_read_before_a_switch_cannot_put_the_old_user_back() {
    let (dir, config, principal) = migrated("stale-save");
    let before = config.settings();
    assert_eq!(before.active_profile.as_deref(), Some(principal.id.as_str()));
    let bruno = config.create_profile("Bruno").unwrap();
    config.switch_to_profile(&bruno.id, |_| ()).unwrap();
    config.save_settings(&before).unwrap();
    assert_eq!(read_json(&dir.join("settings.json"))["activeProfile"], bruno.id.as_str(), "le pointeur suit le dossier réellement suivi");
    assert_eq!(config.active_profile_id().as_deref(), Some(bruno.id.as_str()));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn creating_renaming_and_deleting_keep_the_lists_and_the_active_name_fresh() {
    let (dir, config, principal) = migrated("lists");
    assert_eq!(config.cached_profiles().len(), 1);
    let bruno = config.create_profile("Bruno").unwrap();
    assert_eq!(config.cached_profiles().len(), 2, "la liste gardée pour le menu est refaite à la création");
    config.rename_profile(&principal.id, "Alice").unwrap();
    assert_eq!(config.active_profile_name().as_deref(), Some("Alice"), "le nom de l'utilisateur actif est relu tout de suite");
    assert_eq!(config.cached_profiles().iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), vec!["Alice", "Bruno"]);
    assert!(matches!(config.delete_profile(&principal.id), Err(ProfileError::Active)), "l'utilisateur actif ne se supprime pas");
    config.delete_profile(&bruno.id).unwrap();
    assert_eq!(config.cached_profiles().len(), 1);
    assert!(!bruno.folder.exists() && dir.join(DELETED_DIR).is_dir(), "mis de côté, pas effacé");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_copy_checks_the_user_the_caller_has_in_mind_and_takes_no_history() {
    let (dir, config, principal) = migrated("copy");
    let mut settings = config.settings();
    settings.first_name = "Alice".into();
    config.save_settings(&settings).unwrap();
    config.save_dictionary(&words(&["alice"])).unwrap();
    std::fs::write(principal.folder.join("history.json"), r#"{"history": []}"#).unwrap();
    assert!(matches!(config.copy_profile(Some("AUTRE"), "Copie"), Err(ProfileError::Stale)));
    assert_eq!(config.list_profiles().unwrap().len(), 1, "rien n'est créé quand l'utilisateur n'est plus le bon");
    let copy = config.copy_profile(Some(&principal.id), "Copie").unwrap();
    assert_eq!(read_json(&copy.folder.join(PROFILE_FILE))["firstName"], "Alice");
    assert_eq!(read_json(&copy.folder.join("dictionary.json"))["words"], serde_json::json!(["alice"]));
    assert!(!copy.folder.join("history.json").exists() && !copy.folder.join("history-totals.json").exists());
    assert_eq!(config.active_profile_id().as_deref(), Some(principal.id.as_str()), "la copie ne devient pas active");
    let _ = std::fs::remove_dir_all(dir);
}

/// Le verrou en action : un fil écrit chez Alice sous `lock_for(Alice)` pendant qu'un autre change d'utilisateur sans arrêt. Aucun mot destiné
/// à Alice ne doit se retrouver chez Bruno (sans verrou, le changement s'intercalerait entre la lecture et l'écriture).
#[test]
fn edits_under_the_lock_never_land_on_the_other_user_while_switching() {
    let (dir, config, alice) = migrated("race");
    let bruno = config.create_profile("Bruno").unwrap();
    let config = Arc::new(config);
    let writer = {
        let (config, alice) = (config.clone(), alice.id.clone());
        std::thread::spawn(move || {
            let mut written = 0;
            for n in 0..300 {
                if let Ok(_guard) = config.lock_for(Some(&alice)) {
                    let mut dictionary = config.dictionary();
                    dictionary.words.push(format!("alice-{n}"));
                    config.save_dictionary(&dictionary).unwrap();
                    written += 1;
                }
            }
            written
        })
    };
    let switcher = {
        let (config, alice, bruno) = (config.clone(), alice.id.clone(), bruno.id.clone());
        std::thread::spawn(move || {
            for n in 0..300 {
                let target = if n % 2 == 0 { &bruno } else { &alice };
                config.switch_to_profile(target, |_| ()).unwrap();
            }
        })
    };
    let written = writer.join().unwrap();
    switcher.join().unwrap();
    assert!(written > 0, "le fil d'écriture a bien passé le contrôle au moins une fois");
    let at_bruno = std::fs::read_to_string(bruno.folder.join("dictionary.json")).unwrap_or_default();
    assert!(!at_bruno.contains("alice-"), "des mots d'Alice chez Bruno : {at_bruno}");
    let at_alice = read_json(&alice.folder.join("dictionary.json"));
    assert_eq!(at_alice["words"].as_array().unwrap().len(), written, "tout ce qui a passé le contrôle est chez Alice, rien n'est perdu");
    let _ = std::fs::remove_dir_all(dir);
}
