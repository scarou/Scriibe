//! Implémentations des systèmes qui n'ont pas encore leur code (Wayland, BSD…) : elles compilent et disent ce qui manque.

use crate::apps::{AppInfo, ForegroundApp};
use crate::audio::{LevelListener, Recorder, Recording, StartInfo};
use crate::keys::{KeyEvent, KeyListener, Subscription};
use crate::media::NoMedia;
use crate::paste::{PasteOutcome, Paster};
use crate::secrets::EnvSecrets;
use crate::sounds::Silent;
use crate::Platform;
use scriibe_core::microphone::Microphone;
use std::sync::mpsc::Sender;
use std::sync::Arc;

const MESSAGE: &str = "pas encore pris en charge sur ce système (Windows d'abord)";

struct NoKeys;

impl KeyListener for NoKeys {
    fn start(&self, _: Sender<KeyEvent>) -> Result<Box<dyn Subscription>, String> {
        Err(format!("touche globale : {MESSAGE}"))
    }
}

struct NoRecorder;

impl Recorder for NoRecorder {
    fn microphones(&self) -> Vec<Microphone> {
        Vec::new()
    }

    fn start(&self, _: &str) -> Result<StartInfo, String> {
        Err(format!("micro : {MESSAGE}"))
    }

    fn stop(&self) -> Recording {
        Recording { wav: crate::audio::wav(&[]), duration: 0.0, level: 0.0, gain: 1.0 }
    }

    fn loudest_so_far(&self) -> f32 {
        0.0
    }

    fn input_description(&self) -> String {
        String::new()
    }

    fn set_level_listener(&self, _: Option<LevelListener>) {}
}

struct NoPaster;

impl Paster for NoPaster {
    fn paste(&self, _: &str, _: bool) -> PasteOutcome {
        PasteOutcome::CopiedOnly(format!("collage : {MESSAGE}"))
    }
}

struct NoApp;

impl ForegroundApp for NoApp {
    fn current(&self) -> AppInfo {
        AppInfo::unknown()
    }
}

#[cfg_attr(any(windows, target_os = "linux", target_os = "macos"), allow(dead_code))]
pub fn platform() -> Platform {
    Platform {
        keys: Arc::new(NoKeys),
        recorder: Arc::new(NoRecorder),
        paster: Arc::new(NoPaster),
        media: Arc::new(NoMedia),
        apps: Arc::new(NoApp),
        secrets: Arc::new(EnvSecrets),
        sounds: Arc::new(Silent),
    }
}
