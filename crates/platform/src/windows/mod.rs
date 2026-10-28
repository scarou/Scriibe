//! Windows : crochet clavier bas niveau, `SendInput`, presse-papiers, micro WASAPI (`cpal`), Gestionnaire d'identifiants,
//! icône de la zone de notification. Rien de ceci n'est exécuté par les tests automatiques (CI sans clavier ni micro) : il est
//! vérifié à la compilation, puis essayé à la main sur un vrai PC (voir `CHANGELOG.md`).

mod audio_sessions;
mod credentials;
pub mod file_dialog;
mod foreground;
mod keyboard;
mod paste;
mod recorder;
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
        recorder: Arc::new(recorder::CpalRecorder::default()),
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
