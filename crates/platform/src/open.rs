//! Ouvrir une adresse web ou un dossier avec l'appli par défaut du système.

use std::process::Command;
use std::sync::OnceLock;

/// Ce qui ouvre l'interface de Scriibe dans sa propre fenêtre (rend faux si elle ne peut pas) plutôt que dans le navigateur.
type WindowOpener = Box<dyn Fn(&str) -> bool + Send + Sync>;

static WINDOW_OPENER: OnceLock<WindowOpener> = OnceLock::new();

/// Enregistre la fenêtre de Scriibe : les adresses locales (`http://127.0.0.1:…`) s'ouvrent désormais dedans.
pub fn set_window_opener(opener: impl Fn(&str) -> bool + Send + Sync + 'static) {
    let _ = WINDOW_OPENER.set(Box::new(opener));
}

/// Ouvre `target` (adresse `http://…` ou chemin de dossier). Sans attendre, sans fenêtre de console. L'adresse locale de l'interface
/// s'ouvre dans la fenêtre de Scriibe quand elle existe.
pub fn open(target: &str) {
    if target.starts_with("http://127.0.0.1:") && WINDOW_OPENER.get().is_some_and(|opener| opener(target)) {
        return;
    }
    #[cfg(windows)]
    let mut command = {
        use std::os::windows::process::CommandExt;
        // `start` comprend les adresses comme les dossiers ; le premier argument vide est le titre de fenêtre.
        let mut c = Command::new("cmd");
        c.args(["/C", "start", "", target]).creation_flags(0x0800_0000);
        c
    };
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut c = Command::new("open");
        c.arg(target);
        c
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut c = Command::new("xdg-open");
        c.arg(target);
        c
    };
    let _ = command.spawn();
}
