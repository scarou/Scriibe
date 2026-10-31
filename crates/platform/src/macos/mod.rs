//! macOS : touche globale (tap d'événements CoreGraphics), collage (presse-papiers + ⌘V), appli active (`lsappinfo`), micro (`cpal`, donc
//! CoreAudio), sons (`afplay`), clés dans le trousseau, pause des médias (CoreAudio + touche Lecture/Pause). Écrit sans Mac en S09, essayé sur
//! le Mac en S10 (voir `docs/ESSAI_MACOS.md`). L'icône de la barre des menus et la fenêtre sont dans `crates/cli` (Tauri, fil principal) ;
//! pas encore fait : la détection de la carte graphique.
//!
//! Les parties pures (table des touches, lecture de `lsappinfo`) sont dans `crate::apple` et testées partout.

mod credentials;
mod ffi;
pub mod file_dialog;
mod foreground;
mod keyboard;
mod media;
mod paste;
mod sounds;

pub use credentials::{set_credential, KeychainSecrets};

use crate::media::MediaPauser;
use crate::secrets::{ChainedSecrets, EnvSecrets};
use crate::Platform;
use std::sync::Arc;

/// Montre une fenêtre sans l'activer (`orderFrontRegardless`) : la pastille ne doit jamais prendre le focus, sinon le texte dicté ne se
/// collerait pas dans l'appli où la personne écrit. À appeler sur le fil principal, avec le `NSWindow` de la fenêtre.
pub fn order_front_without_activating(ns_window: *mut std::ffi::c_void) {
    use objc2::msg_send;
    use objc2::runtime::AnyObject;
    if ns_window.is_null() {
        return;
    }
    // SAFETY : `ns_window` est le `NSWindow` d'une fenêtre vivante de ce processus (donné par Tauri), utilisé sur le fil principal ;
    // `orderFrontRegardless` n'a pas d'autre condition.
    unsafe {
        let _: () = msg_send![ns_window as *mut AnyObject, orderFrontRegardless];
    }
}

pub fn platform() -> Platform {
    Platform {
        keys: Arc::new(keyboard::MacKeys),
        recorder: Arc::new(crate::recorder::CpalRecorder::default()),
        paster: Arc::new(paste::MacPaster::default()),
        media: Arc::new(MediaPauser::new(media::CoreAudioSource)),
        apps: Arc::new(foreground::MacForeground),
        secrets: Arc::new(ChainedSecrets(vec![Box::new(KeychainSecrets), Box::new(EnvSecrets)])),
        sounds: Arc::new(sounds::AfplaySounds),
    }
}
