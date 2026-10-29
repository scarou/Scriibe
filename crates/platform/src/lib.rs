//! Ce que Scriibe demande au système, une interface (trait) par fonction et une implémentation par système.
//!
//! Le cœur (`scriibe-core`) ne connaît pas cette couche : il reçoit du texte et rend du texte. Les implémentations réelles sont
//! choisies à la compilation par [`system`] ; les autres systèmes ont des implémentations « pas encore prises en charge » qui
//! compilent partout et disent clairement ce qui manque. [`testing`] contient des faux pour tester le chef d'orchestre.

pub mod apps;
pub mod audio;
pub mod bench_store;
pub mod indicator;
pub mod keys;
pub mod locale;
pub mod machine;
pub mod media;
pub mod open;
pub mod paste;
#[cfg(any(windows, target_os = "linux"))]
mod recorder;
pub mod secrets;
pub mod sounds;
pub mod storage;
pub mod testing;
pub mod tray_menu;
mod unsupported;

#[cfg(target_os = "linux")]
pub mod linux;
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
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        unsupported::platform()
    }
}
