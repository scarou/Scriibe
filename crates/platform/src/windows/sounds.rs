//! Sons de début et de fin : les sons système de Windows (`MessageBeep`), sans attendre leur fin. Le volume des réglages n'est pas
//! pris en compte (celui de Windows s'applique).

use crate::sounds::Sounds;
use scriibe_core::settings::NO_SOUND;
use windows_sys::Win32::System::Diagnostics::Debug::MessageBeep;
use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONASTERISK, MB_OK};

pub struct WindowsSounds;

impl Sounds for WindowsSounds {
    fn play(&self, name: &str, _volume: f64) {
        if name == NO_SOUND {
            return;
        }
        // « Pop » (fin) et « Tink » (début) sont des sons du Mac : deux sons système différents les remplacent.
        let kind = if name == "Pop" { MB_ICONASTERISK } else { MB_OK };
        // SAFETY : MessageBeep ne prend qu'un entier.
        unsafe {
            MessageBeep(kind);
        }
    }
}
