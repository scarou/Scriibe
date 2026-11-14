//! Le mode choisi est le même pour la page (Réglages → Modes) et pour le menu de l'icône : une seule source, le fichier de réglages.
//! Ces tests font passer un choix d'un côté à l'autre, puis par un redémarrage (docs/BUGS_AIDE_MODES.md).

use crate::tray_source::{menu_view_in, TraySource};
use crate::web::{data, pick_mode};
use scriibe_core::modes::RewriteMode;
use scriibe_core::settings::{AppSettings, CleanupEngine, UiLang};
use scriibe_platform::storage::{FileConfig, FileHistory};
use scriibe_platform::testing::FakePaster;
use scriibe_platform::tray_menu::TrayMenuSource;
use std::path::PathBuf;
use std::sync::Arc;

/// Un dossier de données à part, avec des réglages où la mise au propre a un moteur mais est coupée (le cas d'un fichier venu de Scribe).
fn folder(cleanup_enabled: bool) -> (PathBuf, Arc<FileConfig>) {
    // Un compteur, pas l'heure : sur macOS l'horloge n'a qu'une précision d'environ une microseconde, et deux tests lancés ensemble partageaient un dossier
    // (le `settings.tmp` de l'un était renommé sous le nez de l'autre : « No such file or directory », une fois sur six environ).
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("scriibe-mode-sync-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let config = Arc::new(FileConfig::new(dir.clone()));
    let settings = AppSettings { cleanup_engine: CleanupEngine::LlamaCpp, cleanup_enabled, ..AppSettings::default() };
    config.save_settings(&settings).unwrap();
    (dir, config)
}

fn tray(dir: &std::path::Path, config: &Arc<FileConfig>) -> TraySource {
    TraySource {
        config: config.clone(),
        history: Arc::new(FileHistory::open(dir.to_path_buf(), AppSettings::default)),
        paster: Arc::new(FakePaster::default()),
        version: "0.0.0".into(),
        busy: Arc::new(|| false),
        log: None,
    }
}

/// Les modes cochés dans le menu pour ces réglages.
fn checked_in_menu(config: &FileConfig) -> Vec<String> {
    menu_view_in(&config.settings(), "0.0.0", UiLang::Fr).groups.iter().flatten().filter(|m| m.checked).map(|m| m.id.clone()).collect()
}

#[test]
fn a_mode_chosen_in_the_page_is_checked_in_the_menu_even_when_rewriting_was_off() {
    let (_, config) = folder(false);
    pick_mode(&config, None, "formal").unwrap();
    assert_eq!(
        checked_in_menu(&config),
        vec!["formal"],
        "le choix de la page rallume la mise au propre, comme dans le menu et dans Scribe"
    );
}

#[test]
fn a_mode_chosen_in_the_menu_is_the_choice_the_page_shows() {
    let (dir, config) = folder(false);
    tray(&dir, &config).choose_mode("", "tech");
    assert_eq!(data::modes(&config.settings())["choice"], "tech");
    assert_eq!(checked_in_menu(&config), vec!["tech"]);
}

#[test]
fn the_same_mode_is_found_by_the_page_and_the_menu_after_a_restart() {
    let (dir, config) = folder(true);
    pick_mode(&config, None, "calm").unwrap();
    tray(&dir, &config).choose_mode("", "letter");
    let restarted = FileConfig::new(dir);
    assert_eq!(data::modes(&restarted.settings())["choice"], "letter");
    assert_eq!(checked_in_menu(&restarted), vec!["letter"]);
}

#[test]
fn a_retired_mode_is_shown_as_simple_by_the_page_and_checked_as_simple_in_the_menu() {
    let (_, config) = folder(true);
    let mut settings = config.settings();
    settings.rewrite_mode = RewriteMode::Playful;
    config.save_settings(&settings).unwrap();
    assert_eq!(data::modes(&config.settings())["choice"], "simple");
    assert_eq!(checked_in_menu(&config), vec!["simple"]);
}

/// Deux utilisateurs (Principal, puis « Bruno ») sur un dossier migré, avec un moteur de mise au propre pour que les modes se choisissent.
/// Rend le dossier, la configuration, le menu et les identifiants (Principal actif, Bruno).
fn two_users(busy: bool) -> (PathBuf, Arc<FileConfig>, TraySource, String, String) {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("scriibe-mode-sync-users-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    FileConfig::new(dir.clone())
        .save_settings(&AppSettings { cleanup_engine: CleanupEngine::LlamaCpp, cleanup_enabled: true, ..AppSettings::default() })
        .unwrap();
    let folder = scriibe_platform::migration::run(&dir).profile.unwrap();
    let config = Arc::new(FileConfig::new(dir.clone()));
    config.switch_profile(Some(folder.clone()));
    let principal = config.active_profile_id().unwrap();
    let bruno = config.create_profile("Bruno").unwrap().id;
    let source = TraySource {
        config: config.clone(),
        history: Arc::new(FileHistory::open(folder, AppSettings::default)),
        paster: Arc::new(FakePaster::default()),
        version: "0.0.0".into(),
        busy: Arc::new(move || busy),
        log: None,
    };
    (dir, config, source, principal, bruno)
}

#[test]
fn a_mode_clicked_in_a_menu_that_showed_another_user_is_not_applied_to_the_active_one() {
    let (dir, config, source, principal, bruno) = two_users(false);
    let before = config.settings().mode_choice();
    // Le menu montrait Bruno ; entre-temps Principal est devenu actif : le clic ne doit rien changer chez lui.
    source.choose_mode(&bruno, "tech");
    assert_eq!(config.settings().mode_choice(), before, "mode refusé : ce n'était plus l'utilisateur du menu");
    source.choose_mode(&principal, "tech");
    assert_eq!(config.settings().mode_choice(), "tech");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_user_chosen_in_the_menu_becomes_active_and_the_menu_follows() {
    let (dir, config, source, principal, bruno) = two_users(false);
    let view = source.view();
    assert_eq!((view.profile_id.as_str(), view.profiles.len()), (principal.as_str(), 2), "deux utilisateurs : le sous-menu existe");
    assert_eq!(view.profile_title, "Utilisateur : Principal");
    source.choose_profile(&bruno);
    assert_eq!(config.active_profile_id().as_deref(), Some(bruno.as_str()));
    let after = source.view();
    assert_eq!(after.profile_id, bruno);
    assert_eq!(after.profile_title, "Utilisateur : Bruno");
    assert!(after.profiles.iter().any(|p| p.id == bruno && p.checked) && after.profiles.iter().all(|p| p.id == bruno || !p.checked));
    // Un mode choisi après le changement, avec l'identifiant que le menu montre maintenant, va chez Bruno et pas chez Principal.
    source.choose_mode(&bruno, "calm");
    assert_eq!(config.settings().mode_choice(), "calm");
    config.switch_to_profile(&principal, |_| ()).unwrap();
    assert_ne!(config.settings().mode_choice(), "calm", "Principal garde son propre mode");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_menu_cannot_change_user_during_a_dictation() {
    let (dir, config, source, principal, bruno) = two_users(true);
    source.choose_profile(&bruno);
    assert_eq!(config.active_profile_id().as_deref(), Some(principal.as_str()));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_menu_has_no_user_submenu_for_a_single_user() {
    let (dir, config, source, _, bruno) = two_users(false);
    config.delete_profile(&bruno).unwrap();
    let view = source.view();
    assert!(view.profiles.is_empty() && view.profile_title.is_empty());
    assert!(!view.profile_id.is_empty());
    let _ = std::fs::remove_dir_all(dir);
}
