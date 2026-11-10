//! Collage : le texte va dans le presse-papiers, Ctrl+V est envoyé avec `SendInput`, puis l'ancien texte est remis (seulement du texte).
//! Comme dans Scribe : on attend que les touches de modification soient relâchées, sinon Ctrl+V arrive avec Maj et devient
//! « coller sans mise en forme ». Bloqué par Windows dans une fenêtre lancée en administrateur (Scriibe ne l'est pas).

use super::wide;
use crate::paste::{PasteOutcome, Paster, SelectedRead, SelectedText, Selection, Unreadable, SHORTCUT_KEY_UNKNOWN};
use scriibe_core::shortcut::Shortcut;
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
use windows_sys::Win32::UI::WindowsAndMessaging::{GetClassNameW, GetForegroundWindow};

const CF_UNICODETEXT: u32 = 13;
const VK_CONTROL: u16 = 0x11;
const VK_V: u16 = 0x56;
const VK_C: u16 = 0x43;
const VK_SHIFT: u16 = 0x10;
const VK_MENU: u16 = 0x12;
const VK_RETURN: u16 = 0x0D;
/// Raison à texte fixe d'un raccourci non envoyé (pastille et journal).
const SHORTCUT_NOT_SENT: &str = "Windows a bloqué la touche (fenêtre lancée en administrateur ?)";
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

/// Appuie `modifiers` puis `vk`, relâche `vk` puis les modificateurs (dans l'ordre inverse), d'un seul appel : Windows ne glisse rien entre
/// deux. Un code de touche virtuel suit la disposition du clavier : « V » marche en AZERTY comme en QWERTY.
fn send_chord(modifiers: &[u16], vk: u16) -> Result<(), ()> {
    let mut inputs: Vec<INPUT> = modifiers.iter().map(|m| key(*m, false)).collect();
    inputs.push(key(vk, false));
    inputs.push(key(vk, true));
    inputs.extend(modifiers.iter().rev().map(|m| key(*m, true)));
    // SAFETY : tableau d'entrées valide pour la durée de l'appel.
    let sent = unsafe { SendInput(inputs.len() as u32, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32) };
    if sent as usize == inputs.len() {
        Ok(())
    } else {
        Err(())
    }
}

/// Ctrl+V.
fn send_ctrl_v() -> Result<(), String> {
    send_chord(&[VK_CONTROL], VK_V).map_err(|()| "Windows a bloqué le collage (fenêtre lancée en administrateur ?)".into())
}

/// Le code de touche virtuel d'une lettre ou d'un chiffre (`a`–`z`, `0`–`9`) : celui de la majuscule ASCII.
fn vk_of(key: char) -> Option<u16> {
    (key.is_ascii_lowercase() || key.is_ascii_digit()).then(|| key.to_ascii_uppercase() as u16)
}

/// Les fenêtres de console : un Ctrl+C y interrompt le programme au lieu de copier, donc jamais de Ctrl+C d'essai dedans.
const CONSOLE_CLASSES: [&str; 4] = ["ConsoleWindowClass", "CASCADIA_HOSTING_WINDOW_CLASS", "mintty", "VirtualConsoleClass"];

fn foreground_is_console() -> bool {
    // SAFETY : lecture de la fenêtre active et de sa classe, dans un tampon assez grand.
    let class = unsafe {
        let window = GetForegroundWindow();
        if window.is_null() {
            return false;
        }
        let mut buffer = [0u16; 256];
        let length = GetClassNameW(window, buffer.as_mut_ptr(), buffer.len() as i32);
        String::from_utf16_lossy(&buffer[..length.max(0) as usize])
    };
    CONSOLE_CLASSES.contains(&class.as_str())
}

/// Le presse-papiers contient-il autre chose que du texte (image, fichiers) ? Le Ctrl+C d'essai l'écraserait pour de bon. Windows fabrique
/// les formats image équivalents (bitmap, DIB) à partir d'un PNG : on les voit tous.
fn clipboard_holds_non_text() -> bool {
    const NON_TEXT: [u32; 6] = [2, 6, 8, 14, 15, 17]; // CF_BITMAP, CF_TIFF, CF_DIB, CF_ENHMETAFILE, CF_HDROP, CF_DIBV5
                                                      // SAFETY : simple interrogation des formats disponibles.
    NON_TEXT.iter().any(|format| unsafe { IsClipboardFormatAvailable(*format) } != 0)
}

/// Écrase un texte qui a contenu la sélection avant de le laisser tomber.
fn wipe(text: Option<String>) {
    if let Some(text) = text {
        let mut bytes = text.into_bytes();
        bytes.fill(0);
        std::hint::black_box(&bytes);
    }
}

/// Le résultat d'un Ctrl+C d'essai.
enum Probe {
    /// L'appli a copié ce texte (peut être vide si elle a copié autre chose).
    Copied(Option<String>),
    /// L'appli n'a rien copié dans le délai : rien n'est sélectionné.
    Nothing,
    Unreadable(Unreadable),
}

/// Sonde la sélection de l'appli active : Ctrl+C, puis on regarde si le presse-papiers a changé (numéro de séquence : même un texte
/// identique à l'ancien compte). L'ancien contenu texte est remis dans tous les cas où il a changé, et la remise est faite avant de rendre
/// la main (le collage qui suit écrit à son tour). Jamais dans une console, jamais sur un presse-papiers qui n'est pas du texte.
/// Limite : l'appli copie dans l'historique du presse-papiers de Windows (Win+V) ; Scriibe ne peut pas l'en empêcher.
fn probe_selection(wait: Duration) -> Probe {
    if foreground_is_console() {
        return Probe::Unreadable(Unreadable::Failed);
    }
    if clipboard_holds_non_text() {
        return Probe::Unreadable(Unreadable::ClipboardNotText);
    }
    let before = read_text();
    // SAFETY : lecture du numéro de séquence du presse-papiers.
    let sequence = unsafe { GetClipboardSequenceNumber() };
    wait_for_modifiers_released();
    std::thread::sleep(Duration::from_millis(40));
    if send_chord(&[VK_CONTROL], VK_C).is_err() {
        wipe(before);
        return Probe::Unreadable(Unreadable::Failed);
    }
    let mut waited = Duration::ZERO;
    let mut changed = false;
    while waited < wait {
        std::thread::sleep(Duration::from_millis(10));
        waited += Duration::from_millis(10);
        // SAFETY : lecture du numéro de séquence.
        if unsafe { GetClipboardSequenceNumber() } != sequence {
            changed = true;
            break;
        }
    }
    if !changed {
        wipe(before);
        return Probe::Nothing;
    }
    // L'appli a copié : on la laisse finir (numéro de séquence stable 30 ms, au plus 300 ms), puis on lit.
    let mut last = unsafe { GetClipboardSequenceNumber() };
    for _ in 0..10 {
        std::thread::sleep(Duration::from_millis(30));
        // SAFETY : lecture du numéro de séquence.
        let now = unsafe { GetClipboardSequenceNumber() };
        if now == last {
            break;
        }
        last = now;
    }
    let copied = read_text();
    // Remise de l'ancien contenu. S'il n'y en avait pas (ou pas de texte), on vide : mieux vaut le perdre que laisser la sélection dedans.
    match &before {
        Some(saved) => drop(write_text(saved, false)),
        None => {
            if open_clipboard() {
                // SAFETY : presse-papiers ouvert par ce fil.
                unsafe {
                    EmptyClipboard();
                    CloseClipboard();
                }
            }
        }
    }
    wipe(before);
    Probe::Copied(copied)
}

impl Paster for WindowsPaster {
    fn selection(&self) -> Selection {
        match probe_selection(Duration::from_millis(150)) {
            Probe::Copied(text) => {
                let length = text.as_ref().map_or(0, |t| t.chars().count());
                wipe(text);
                if length > 0 {
                    Selection { active: true, report: format!("active (Ctrl+C d'essai, {length} signes)") }
                } else {
                    Selection { active: false, report: "aucune (Ctrl+C d'essai)".into() }
                }
            }
            Probe::Nothing => Selection { active: false, report: "aucune (Ctrl+C d'essai)".into() },
            Probe::Unreadable(why) => Selection { active: false, report: format!("inconnue ({})", why.reason()) },
        }
    }

    fn selected_text(&self) -> SelectedRead {
        // Pour une commande vocale (déjà reconnue) on attend plus longtemps : une appli lente (Electron) qui copie après le délai
        // laisserait la sélection dans le presse-papiers.
        match probe_selection(Duration::from_millis(400)) {
            Probe::Copied(Some(text)) if !text.is_empty() => SelectedRead::Text(SelectedText::new(text)),
            Probe::Copied(other) => {
                wipe(other);
                SelectedRead::NoSelection
            }
            Probe::Nothing => SelectedRead::NoSelection,
            Probe::Unreadable(why) => SelectedRead::Unreadable(why),
        }
    }

    fn press_shortcut(&self, shortcut: &Shortcut) -> Result<(), &'static str> {
        let vk = vk_of(shortcut.key).ok_or(SHORTCUT_KEY_UNKNOWN)?;
        let mut modifiers = Vec::new();
        if shortcut.control || shortcut.command {
            modifiers.push(VK_CONTROL);
        }
        if shortcut.alt {
            modifiers.push(VK_MENU);
        }
        if shortcut.shift {
            modifiers.push(VK_SHIFT);
        }
        wait_for_modifiers_released();
        std::thread::sleep(Duration::from_millis(40));
        send_chord(&modifiers, vk).map_err(|()| SHORTCUT_NOT_SENT)
    }

    fn press_return(&self) -> bool {
        wait_for_modifiers_released();
        let sent = send_chord(&[], VK_RETURN).is_ok();
        std::thread::sleep(Duration::from_millis(30));
        sent
    }

    fn copy(&self, text: &str) -> bool {
        write_text(text, false).is_ok()
    }

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
