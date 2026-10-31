//! macOS : touche globale (tap d'événements CoreGraphics), collage (presse-papiers + ⌘V), appli active (`lsappinfo`), micro (`cpal`, donc
//! CoreAudio), sons (`afplay`), clés dans le trousseau. **Premier jalon, écrit sans Mac sous la main** : il compile sur le Mac ou dans la CI, pas
//! encore essayé (voir `docs/ESSAI_MACOS.md`). Pas encore faits : la pause des médias (`NoMedia`), l'icône de la barre des menus, la fenêtre
//! de Scriibe (l'interface s'ouvre dans le navigateur), le verrou d'instance, la détection de la carte graphique.
//!
//! Les parties pures (table des touches, lecture de `lsappinfo`) sont dans `crate::apple` et testées partout.

mod credentials;
mod ffi;
pub mod file_dialog;
mod foreground;
mod keyboard;
mod paste;
mod sounds;

pub use credentials::{set_credential, KeychainSecrets};

use crate::media::NoMedia;
use crate::secrets::{ChainedSecrets, EnvSecrets};
use crate::Platform;
use std::sync::Arc;

pub fn platform() -> Platform {
    Platform {
        keys: Arc::new(keyboard::MacKeys),
        recorder: Arc::new(crate::recorder::CpalRecorder::default()),
        paster: Arc::new(paste::MacPaster::default()),
        // Pause des médias : à faire (CoreAudio pour savoir qui joue, touche Lecture/Pause du clavier pour l'interrompre).
        media: Arc::new(NoMedia),
        apps: Arc::new(foreground::MacForeground),
        secrets: Arc::new(ChainedSecrets(vec![Box::new(KeychainSecrets), Box::new(EnvSecrets)])),
        sounds: Arc::new(sounds::AfplaySounds),
    }
}
