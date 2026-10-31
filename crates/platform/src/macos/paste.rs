//! Collage : le texte va dans le presse-papiers (`arboard`), ⌘V est envoyé par CoreGraphics, puis l'ancien texte est remis (seulement du
//! texte). Même logique que `linux/paste.rs`. Comme sous Windows et Linux, on attend que Maj, Ctrl, Option et Commande soient relâchés :
//! sinon ⌘V arrive avec Option (la touche de dictée par défaut) et ne colle pas ce qu'on croit.
//!
//! Envoyer des touches à une autre appli demande l'autorisation « Accessibilité » (Réglages Système → Confidentialité et sécurité) ; sans
//! elle, le texte reste dans le presse-papiers (« Copié, à coller… »). La touche V est cherchée à sa place QWERTY (code 9), qui est aussi
//! celle de l'AZERTY du Mac ; une disposition Dvorak ou Colemak la déplace (limite connue).

use super::ffi;
use crate::apple::PASTE_BLOCKING_FLAGS;
use crate::paste::{PasteOutcome, Paster};
use arboard::Clipboard;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

const KEY_V: u16 = 9;
/// `kCGEventFlagMaskCommand`.
const FLAG_COMMAND: u64 = 0x0010_0000;

#[derive(Default)]
pub struct MacPaster {
    clipboard: Arc<Mutex<Option<Clipboard>>>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

/// Le presse-papiers de cette appli, ouvert au premier besoin et gardé.
fn with_clipboard<R>(slot: &Mutex<Option<Clipboard>>, f: impl FnOnce(&mut Clipboard) -> Result<R, String>) -> Result<R, String> {
    let mut slot = lock(slot);
    if slot.is_none() {
        *slot = Some(Clipboard::new().map_err(|e| format!("presse-papiers indisponible ({e})"))?);
    }
    f(slot.as_mut().expect("ouvert juste avant"))
}

fn write_text(slot: &Mutex<Option<Clipboard>>, text: &str) -> Result<(), String> {
    with_clipboard(slot, |c| c.set_text(text.to_string()).map_err(|e| format!("écriture dans le presse-papiers refusée ({e})")))
}

fn read_text(slot: &Mutex<Option<Clipboard>>) -> Option<String> {
    with_clipboard(slot, |c| c.get_text().map_err(|e| e.to_string())).ok()
}

/// Attend (au plus 0,3 s) que Maj, Ctrl, Option et Commande soient relâchés.
fn wait_for_modifiers_released() {
    for _ in 0..30 {
        // SAFETY : fonction sans autre condition que son argument.
        let flags = unsafe { ffi::CGEventSourceFlagsState(ffi::SOURCE_COMBINED_SESSION) };
        if flags & PASTE_BLOCKING_FLAGS == 0 {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn post_key(down: bool) -> Result<(), String> {
    // SAFETY : une source nulle veut dire « source par défaut » ; l'événement créé est relâché après l'envoi.
    unsafe {
        let event = ffi::CGEventCreateKeyboardEvent(std::ptr::null(), KEY_V, down);
        if event.is_null() {
            return Err("macOS n'a pas voulu créer la touche V".into());
        }
        ffi::CGEventSetFlags(event, FLAG_COMMAND);
        ffi::CGEventPost(ffi::TAP_HID, event);
        ffi::CFRelease(event);
    }
    Ok(())
}

fn send_command_v() -> Result<(), String> {
    // SAFETY : fonction sans paramètre.
    if !unsafe { ffi::AXIsProcessTrusted() } {
        return Err("collage refusé : autorise Scriibe (ou l'appli qui la lance) dans Réglages Système → Confidentialité et sécurité → Accessibilité".into());
    }
    wait_for_modifiers_released();
    std::thread::sleep(Duration::from_millis(40));
    post_key(true)?;
    std::thread::sleep(Duration::from_millis(10));
    post_key(false)
}

impl Paster for MacPaster {
    fn copy(&self, text: &str) -> bool {
        write_text(&self.clipboard, text).is_ok()
    }

    fn paste(&self, text: &str, restore_clipboard: bool) -> PasteOutcome {
        let saved = if restore_clipboard { read_text(&self.clipboard) } else { None };
        if let Err(e) = write_text(&self.clipboard, text) {
            return PasteOutcome::CopiedOnly(e);
        }
        if let Err(e) = send_command_v() {
            return PasteOutcome::CopiedOnly(e);
        }
        if let (true, Some(saved)) = (restore_clipboard, saved) {
            // En arrière-plan, 1,5 s après : une appli lente doit avoir lu la dictée ; si autre chose a été copié entre-temps, on n'y touche pas.
            let slot = self.clipboard.clone();
            let ours = text.to_string();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(1500));
                if read_text(&slot).as_deref() == Some(ours.as_str()) {
                    let _ = write_text(&slot, &saved);
                }
            });
        }
        PasteOutcome::Pasted
    }
}
