//! Ouvrir une adresse web ou un dossier avec l'appli par défaut du système.

use std::process::Command;

/// Ouvre `target` (adresse `http://…` ou chemin de dossier). Sans attendre, sans fenêtre de console.
pub fn open(target: &str) {
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
