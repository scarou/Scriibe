//! Collage : le texte va dans le presse-papiers, Ctrl+V est envoyé avec `SendInput`, puis l'ancien texte est remis (seulement du texte).
//! Comme dans Scribe : on attend que les touches de modification soient relâchées, sinon Ctrl+V arrive avec Maj et devient
//! « coller sans mise en forme ». Bloqué par Windows dans une fenêtre lancée en administrateur (Scriibe ne l'est pas).

use super::wide;
use crate::paste::{PasteOutcome, Paster};
use std::time::Duration;
use windows_sys::Win32::Foundation::{GlobalFree, HANDLE};
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, GetClipboardSequenceNumber, IsClipboardFormatAvailable, OpenClipboard,
    RegisterClipboardFormatW, SetClipboardData,
};
use windows_sys::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
};

const CF_UNICODETEXT: u32 = 13;
const VK_CONTROL: u16 = 0x11;
const VK_V: u16 = 0x56;
/// Les touches dont l'appui fausserait le collage : Maj, Ctrl, Alt (gauche et droite).
const MODIFIERS: [i32; 6] = [0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5];

pub struct WindowsPaster;

fn open_clipboard() -> bool {
    for _ in 0..20 {
        // SAFETY : sans fenêtre propriétaire.
        if unsafe { OpenClipboard(std::ptr::null_mut()) } != 0 {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    false
}

/// Texte actuel du presse-papiers (rien si c'est une image, des fichiers…).
fn read_text() -> Option<String> {
    if !open_clipboard() {
        return None;
    }
    // SAFETY : le presse-papiers est ouvert ; la mémoire est verrouillée le temps de la copie.
    let text = unsafe {
        let mut found = None;
        if IsClipboardFormatAvailable(CF_UNICODETEXT) != 0 {
            let handle = GetClipboardData(CF_UNICODETEXT);
            if !handle.is_null() {
                let pointer = GlobalLock(handle) as *const u16;
                if !pointer.is_null() {
                    let capacity = GlobalSize(handle) / 2;
                    let slice = std::slice::from_raw_parts(pointer, capacity);
                    let length = slice.iter().position(|c| *c == 0).unwrap_or(capacity);
                    found = Some(String::from_utf16_lossy(&slice[..length]));
                    GlobalUnlock(handle);
                }
            }
        }
        found
    };
    // SAFETY : fermeture du presse-papiers ouvert plus haut.
    unsafe {
        CloseClipboard();
    }
    text
}

/// Une donnée du presse-papiers : mémoire globale déplaçable, que le presse-papiers prend en charge si l'appel réussit.
///
/// SAFETY : le presse-papiers doit être ouvert par ce fil.
unsafe fn set_global(format: u32, bytes: &[u8]) -> bool {
    let memory: HANDLE = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes.len()) };
    if memory.is_null() {
        return false;
    }
    let target = unsafe { GlobalLock(memory) } as *mut u8;
    if target.is_null() {
        unsafe { GlobalFree(memory) };
        return false;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), target, bytes.len());
        GlobalUnlock(memory);
    }
    if unsafe { SetClipboardData(format, memory) }.is_null() {
        unsafe { GlobalFree(memory) };
        return false;
    }
    true
}

/// Remplace le presse-papiers par `text`. `transient` : demande à l'historique du presse-papiers de Windows (Win+V) et au presse-papiers
/// du cloud de ne pas le garder, comme Scribe le demande aux gestionnaires du Mac.
fn write_text(text: &str, transient: bool) -> Result<(), String> {
    if !open_clipboard() {
        return Err("presse-papiers occupé".into());
    }
    let bytes: Vec<u8> = wide(text).iter().flat_map(|c| c.to_le_bytes()).collect();
    // SAFETY : presse-papiers ouvert par ce fil.
    let ok = unsafe {
        EmptyClipboard();
        let ok = set_global(CF_UNICODETEXT, &bytes);
        if ok && transient {
            for name in ["ExcludeClipboardContentFromMonitorProcessing", "CanIncludeInClipboardHistory", "CanUploadToCloudClipboard"] {
                let format = RegisterClipboardFormatW(wide(name).as_ptr());
                if format != 0 {
                    set_global(format, &0u32.to_le_bytes());
                }
            }
        }
        CloseClipboard();
        ok
    };
    if ok {
        Ok(())
    } else {
        Err("écriture dans le presse-papiers refusée".into())
    }
}

fn wait_for_modifiers_released() {
    for _ in 0..30 {
        // SAFETY : lecture de l'état d'une touche ; le bit de poids fort dit qu'elle est enfoncée.
        let held = MODIFIERS.iter().any(|vk| unsafe { GetAsyncKeyState(*vk) } < 0);
        if !held {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn key(vk: u16, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: vk, wScan: 0, dwFlags: if up { KEYEVENTF_KEYUP } else { 0 }, time: 0, dwExtraInfo: 0 } },
    }
}

/// Ctrl+V. Un code de touche virtuel suit la disposition du clavier : « V » marche en AZERTY comme en QWERTY.
fn send_ctrl_v() -> Result<(), String> {
    let inputs = [key(VK_CONTROL, false), key(VK_V, false), key(VK_V, true), key(VK_CONTROL, true)];
    // SAFETY : tableau d'entrées valide pour la durée de l'appel.
    let sent = unsafe { SendInput(inputs.len() as u32, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32) };
    if sent as usize == inputs.len() {
        Ok(())
    } else {
        Err("Windows a bloqué le collage (fenêtre lancée en administrateur ?)".into())
    }
}

impl Paster for WindowsPaster {
    fn paste(&self, text: &str, restore_clipboard: bool) -> PasteOutcome {
        let saved = if restore_clipboard { read_text() } else { None };
        if let Err(e) = write_text(text, restore_clipboard) {
            return PasteOutcome::CopiedOnly(e);
        }
        // SAFETY : lecture du numéro de séquence du presse-papiers.
        let ours = unsafe { GetClipboardSequenceNumber() };
        wait_for_modifiers_released();
        std::thread::sleep(Duration::from_millis(40));
        if let Err(e) = send_ctrl_v() {
            return PasteOutcome::CopiedOnly(e);
        }
        if let (true, Some(saved)) = (restore_clipboard, saved) {
            // En arrière-plan : la dictée suivante n'attend pas. 1,5 s : une appli lente (Electron, bureau à distance) doit avoir
            // lu la dictée avant qu'on remette l'ancien contenu ; si autre chose a été copié entre-temps, on n'y touche pas.
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(1500));
                // SAFETY : lecture du numéro de séquence.
                if unsafe { GetClipboardSequenceNumber() } == ours {
                    let _ = write_text(&saved, false);
                }
            });
        }
        PasteOutcome::Pasted
    }
}
