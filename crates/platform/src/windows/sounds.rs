//! Sons de début et de fin : synthétisés (`crate::sounds::synth`, volume compris) et joués par `PlaySoundW` depuis la mémoire, sur un fil
//! à part (la lecture ne bloque ni la touche ni la dictée). Un nouveau son remplace celui qui joue encore.

use crate::sounds::{synth, Sounds};
use windows_sys::Win32::Media::Audio::{PlaySoundW, SND_MEMORY, SND_NODEFAULT};

pub struct WindowsSounds;

impl Sounds for WindowsSounds {
    fn play(&self, name: &str, volume: f64) {
        let Some(wav) = synth(name, volume) else { return };
        let _ = std::thread::Builder::new().name("scriibe-son".into()).spawn(move || {
            // SAFETY : sans SND_ASYNC, l'appel rend la main quand le son est fini ; `wav` reste donc valide pendant toute la lecture.
            unsafe {
                PlaySoundW(wav.as_ptr().cast(), std::ptr::null_mut(), SND_MEMORY | SND_NODEFAULT);
            }
        });
    }
}
