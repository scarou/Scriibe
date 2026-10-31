//! macOS : touche globale (tap d'événements CoreGraphics), collage (presse-papiers + ⌘V), appli active (`lsappinfo`), micro (`cpal`, donc
//! CoreAudio), sons (`afplay`), clés dans le trousseau, pause des médias (CoreAudio + touche Lecture/Pause). Écrit sans Mac en S09, essayé sur
//! le Mac en S10 (voir `docs/ESSAI_MACOS.md`). Pas encore faits : l'icône de la barre des menus, la fenêtre
//! de Scriibe (l'interface s'ouvre dans le navigateur), le verrou d'instance, la détection de la carte graphique.
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
