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
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let dir = std::env::temp_dir().join(format!("scriibe-mode-sync-{}-{nanos}", std::process::id()));
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
    }
}

/// Les modes cochés dans le menu pour ces réglages.
fn checked_in_menu(config: &FileConfig) -> Vec<String> {
    menu_view_in(&config.settings(), "0.0.0", UiLang::Fr).groups.iter().flatten().filter(|m| m.checked).map(|m| m.id.clone()).collect()
}

#[test]
fn a_mode_chosen_in_the_page_is_checked_in_the_menu_even_when_rewriting_was_off() {
    let (_, config) = folder(false);
    pick_mode(&config, "formal").unwrap();
    assert_eq!(
        checked_in_menu(&config),
        vec!["formal"],
        "le choix de la page rallume la mise au propre, comme dans le menu et dans Scribe"
    );
}

#[test]
fn a_mode_chosen_in_the_menu_is_the_choice_the_page_shows() {
    let (dir, config) = folder(false);
    tray(&dir, &config).choose_mode("tech");
    assert_eq!(data::modes(&config.settings())["choice"], "tech");
    assert_eq!(checked_in_menu(&config), vec!["tech"]);
}

#[test]
fn the_same_mode_is_found_by_the_page_and_the_menu_after_a_restart() {
    let (dir, config) = folder(true);
    pick_mode(&config, "calm").unwrap();
    tray(&dir, &config).choose_mode("letter");
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
