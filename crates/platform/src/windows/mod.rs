//! Windows : crochet clavier bas niveau, `SendInput`, presse-papiers, micro WASAPI (`cpal`), Gestionnaire d'identifiants,
//! icône de la zone de notification. Rien de ceci n'est exécuté par les tests automatiques (CI sans clavier ni micro) : il est
//! vérifié à la compilation, puis essayé à la main sur un vrai PC (voir `CHANGELOG.md`).

mod audio_sessions;
mod credentials;
pub mod file_dialog;
mod foreground;
mod keyboard;
mod paste;
mod sounds;
pub mod tray;

pub use credentials::{set_credential, CredentialSecrets};

use crate::media::MediaPauser;
use crate::secrets::{ChainedSecrets, EnvSecrets};
use crate::Platform;
use std::sync::Arc;

pub fn platform() -> Platform {
    Platform {
        keys: Arc::new(keyboard::WindowsKeys),
        recorder: Arc::new(crate::recorder::CpalRecorder::default()),
        paster: Arc::new(paste::WindowsPaster),
        media: Arc::new(MediaPauser::new(audio_sessions::WasapiSessions)),
        apps: Arc::new(foreground::WindowsForeground),
        secrets: Arc::new(ChainedSecrets(vec![Box::new(CredentialSecrets), Box::new(EnvSecrets)])),
        sounds: Arc::new(sounds::WindowsSounds),
    }
}

/// Texte UTF-16 terminé par un zéro, pour les fonctions `...W` de Windows.
pub(crate) fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Montre une fenêtre sans l'activer (`SW_SHOWNA`) : la pastille ne doit jamais prendre le focus, même quand Scriibe est déjà au premier plan
/// (Windows laisse alors un processus s'activer lui-même, et la fenêtre de l'interface perdrait la touche tenue).
pub fn show_without_activating(hwnd: isize) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_SHOWNA};
    // SAFETY: `hwnd` vient d'une fenêtre vivante de ce processus ; `ShowWindow` n'a pas d'autre condition.
    unsafe {
        ShowWindow(hwnd as _, SW_SHOWNA);
    }
}
