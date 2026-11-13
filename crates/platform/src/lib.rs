//! Ce que Scriibe demande au système, une interface (trait) par fonction et une implémentation par système.
//!
//! Le cœur (`scriibe-core`) ne connaît pas cette couche : il reçoit du texte et rend du texte. Les implémentations réelles sont
//! choisies à la compilation par [`system`] ; les autres systèmes ont des implémentations « pas encore prises en charge » qui
//! compilent partout et disent clairement ce qui manque. [`testing`] contient des faux pour tester le chef d'orchestre.

mod apple;
pub mod apps;
pub mod audio;
pub mod autostart;
pub mod bench_store;
pub mod indicator;
pub mod instance;
pub mod keys;
pub mod locale;
pub mod machine;
pub mod media;
pub mod open;
pub mod paste;
#[cfg(any(windows, target_os = "linux", target_os = "macos"))]
mod recorder;
pub mod restore;
pub mod secrets;
pub mod sounds;
pub mod storage;
pub mod testing;
pub mod tray_menu;
pub mod uninstall;
mod unsupported;

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(windows)]
pub mod windows;

use std::sync::Arc;

/// Les fonctions du système, branchées pour ce système.
pub struct Platform {
    pub keys: Arc<dyn keys::KeyListener>,
    pub recorder: Arc<dyn audio::Recorder>,
    pub paster: Arc<dyn paste::Paster>,
    pub media: Arc<dyn media::MediaControl>,
    pub apps: Arc<dyn apps::ForegroundApp>,
    pub secrets: Arc<dyn scriibe_engines::runtime::Secrets>,
    pub sounds: Arc<dyn sounds::Sounds>,
}

/// Enregistre une clé d'API dans le coffre du système (Gestionnaire d'identifiants de Windows, trousseau de macOS) ; rend le nom du coffre pour
/// les messages. Elle n'est ni affichée ni journalisée. Linux n'a pas encore de coffre : les clés s'y donnent par l'environnement.
pub fn save_secret(kind: scriibe_engines::runtime::KeyKind, value: &str) -> Result<&'static str, String> {
    #[cfg(windows)]
    {
        windows::set_credential(kind, value).map(|()| "Gestionnaire d'identifiants")
    }
    #[cfg(target_os = "macos")]
    {
        macos::set_credential(kind, value).map(|()| "trousseau")
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = value;
        Err(format!("pas encore pris en charge ici : définis la variable d'environnement {}", secrets::env_name(kind)))
    }
}

/// Les implémentations de ce système.
pub fn system() -> Platform {
    #[cfg(windows)]
    {
        windows::platform()
    }
    #[cfg(target_os = "linux")]
    {
        linux::platform()
    }
    #[cfg(target_os = "macos")]
    {
        macos::platform()
    }
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    {
        unsupported::platform()
    }
}
