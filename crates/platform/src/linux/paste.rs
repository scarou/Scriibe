//! Collage : le texte va dans le presse-papiers (`arboard`, qui reste propriétaire de la sélection tant que l'appli tourne), Ctrl+V est envoyé
//! par XTest, puis l'ancien texte est remis (seulement du texte). Comme sous Windows, on attend que Maj, Ctrl et Alt soient relâchés : sinon
//! Ctrl+V arrive avec Maj et devient « coller sans mise en forme ».
//!
//! Limite : les terminaux collent avec Ctrl+Maj+V, pas Ctrl+V ; le texte y est dans le presse-papiers (« Copié, à coller… »).

use super::evdev::MODIFIER_KEYCODES;
use super::x11;
use crate::paste::{PasteOutcome, Paster};
use arboard::{Clipboard, SetExtLinux};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{self, ConnectionExt as _};
use x11rb::protocol::xtest::ConnectionExt as _;

const KEYSYM_CONTROL_L: u32 = 0xFFE3;
const KEYSYM_V: [u32; 2] = [0x76, 0x56];

#[derive(Default)]
pub struct X11Paster {
    clipboard: Arc<Mutex<Option<Clipboard>>>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

/// Le presse-papiers de cette appli, ouvert au premier besoin et gardé : sa fermeture rendrait le texte copié inaccessible.
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

/// Comme `write_text`, mais le texte est marqué « ne pas garder » pour les gestionnaires d'historique du presse-papiers qui lisent cette
/// marque (`x-kde-passwordManagerHint`, que `arboard` pose). Un extrait peut être une adresse, un IBAN, une signature.
fn write_text_transient(slot: &Mutex<Option<Clipboard>>, text: &str) -> Result<(), String> {
    with_clipboard(slot, |c| {
        c.set().exclude_from_history().text(text.to_string()).map_err(|e| format!("écriture dans le presse-papiers refusée ({e})"))
    })
}

fn read_text(slot: &Mutex<Option<Clipboard>>) -> Option<String> {
    with_clipboard(slot, |c| c.get_text().map_err(|e| e.to_string())).ok()
}

/// Attend (au plus 0,3 s) que Maj, Ctrl et Alt soient relâchés.
fn wait_for_modifiers_released(conn: &impl Connection) {
    for _ in 0..30 {
        let held = conn.query_keymap().ok().and_then(|c| c.reply().ok()).is_some_and(|map| {
            MODIFIER_KEYCODES.iter().any(|code| map.keys.get((code / 8) as usize).is_some_and(|byte| byte & (1 << (code % 8)) != 0))
        });
        if !held {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Ctrl+V, avec les touches de la disposition en vigueur (« v » est où l'est la lettre, pas où le QWERTY la met). Sous Wayland, par le
/// portail `RemoteDesktop` de GNOME (`wayland.rs`) : le raccourci de dictée vient d'être relâché, on laisse 40 ms au bureau.
fn send_ctrl_v() -> Result<(), String> {
    if super::wayland::is_wayland() {
        std::thread::sleep(Duration::from_millis(40));
        return super::wayland::send_ctrl_v_blocking();
    }
    let (conn, root) = x11::connect()?;
    let control = x11::keycode_for(&conn, &[KEYSYM_CONTROL_L]).ok_or("touche Ctrl introuvable dans la disposition du clavier")?;
    let v = x11::keycode_for(&conn, &KEYSYM_V).ok_or("touche V introuvable dans la disposition du clavier")?;
    wait_for_modifiers_released(&conn);
    std::thread::sleep(Duration::from_millis(40));
    for (press, code) in [(true, control), (true, v), (false, v), (false, control)] {
        let kind = if press { xproto::KEY_PRESS_EVENT } else { xproto::KEY_RELEASE_EVENT };
        conn.xtest_fake_input(kind, code, 0, root, 0, 0, 0).map_err(|e| format!("collage refusé par X11 ({e})"))?;
    }
    conn.flush().map_err(|e| e.to_string())?;
    // Une requête qui attend sa réponse : les événements sont partis quand elle revient.
    let _ = conn.get_input_focus().map_err(|e| e.to_string())?.reply();
    Ok(())
}

impl Paster for X11Paster {
    fn copy(&self, text: &str) -> bool {
        write_text(&self.clipboard, text).is_ok()
    }

    fn paste(&self, text: &str, restore_clipboard: bool) -> PasteOutcome {
        self.paste_with(text, restore_clipboard, false)
    }

    fn paste_transient(&self, text: &str) -> PasteOutcome {
        self.paste_with(text, true, true)
    }
}

impl X11Paster {
    /// Le collage. `transient` (les extraits) : le texte est marqué « ne pas garder » et un presse-papiers qui était vide est vidé à la
    /// remise (l'extrait ne reste pas dedans).
    fn paste_with(&self, text: &str, restore_clipboard: bool, transient: bool) -> PasteOutcome {
        let saved = if restore_clipboard { read_text(&self.clipboard) } else { None };
        let written = if transient { write_text_transient(&self.clipboard, text) } else { write_text(&self.clipboard, text) };
        if let Err(e) = written {
            return PasteOutcome::CopiedOnly(e);
        }
        if let Err(e) = send_ctrl_v() {
            return PasteOutcome::CopiedOnly(e);
        }
        if restore_clipboard {
            // En arrière-plan : la dictée suivante n'attend pas. 1,5 s : une appli lente doit avoir lu la dictée avant qu'on remette
            // l'ancien contenu ; si autre chose a été copié entre-temps, on n'y touche pas.
            let slot = self.clipboard.clone();
            let ours = text.to_string();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(1500));
                if read_text(&slot).as_deref() == Some(ours.as_str()) {
                    match saved {
                        Some(saved) => drop(write_text(&slot, &saved)),
                        None if transient => drop(with_clipboard(&slot, |c| c.clear().map_err(|e| e.to_string()))),
                        None => {}
                    }
                }
            });
        }
        PasteOutcome::Pasted
    }
}
