//! Linux (X11) : touche globale (XInput2), collage (presse-papiers + XTest), fenêtre active, micro (`cpal`, donc ALSA, PulseAudio ou
//! PipeWire), sons (lecteur de la session), pause des médias (`pactl` et `playerctl`). Sous Wayland (GNOME), la touche globale et le
//! collage passent par les portails du bureau (`wayland.rs`).
//!
//! Les parties pures (table des touches, lecture de `pactl`) sont testées partout ; le reste est essayé sur un écran virtuel (Xvfb).

mod evdev;
mod foreground;
mod keyboard;
mod media;
mod paste;
mod sounds;
mod wayland;
mod x11;

pub use wayland::{current_translate_trigger as wayland_translate_trigger, current_trigger as wayland_trigger, is_wayland};

pub use keyboard::LinuxKeys;

/// La fenêtre active de X11, pour les essais sur un écran virtuel.
pub fn foreground_for_tests() -> impl crate::apps::ForegroundApp {
    foreground::X11Foreground::default()
}
pub use paste::X11Paster;

use crate::media::MediaPauser;
use crate::secrets::EnvSecrets;
use crate::Platform;
use std::sync::Arc;

pub fn platform() -> Platform {
    Platform {
        // SCRIIBE_X11_ACCEPT_XTEST : prend aussi les touches envoyées par programme (essais sur un écran virtuel avec `xdotool`).
        keys: if wayland::is_wayland() {
            Arc::new(wayland::WaylandKeys)
        } else {
            Arc::new(LinuxKeys { accept_synthetic: std::env::var_os("SCRIIBE_X11_ACCEPT_XTEST").is_some() })
        },
        recorder: Arc::new(crate::recorder::CpalRecorder::default()),
        paster: Arc::new(X11Paster::default()),
        media: Arc::new(MediaPauser::new(media::PulseAudio)),
        apps: Arc::new(foreground::X11Foreground::default()),
        // Les clés se donnent par l'environnement (DEEPGRAM_API_KEY, OPENROUTER_API_KEY) : pas encore de trousseau (voir `ROADMAP.md`).
        secrets: Arc::new(EnvSecrets),
        sounds: Arc::new(sounds::SessionSounds),
    }
}
