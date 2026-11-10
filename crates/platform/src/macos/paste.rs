//! Collage : le texte va dans le presse-papiers (`arboard`), ⌘V est envoyé par CoreGraphics, puis l'ancien texte est remis (seulement du
//! texte). Même logique que `linux/paste.rs`. Comme sous Windows et Linux, on attend que Maj, Ctrl, Option et Commande soient relâchés :
//! sinon ⌘V arrive avec Option (la touche de dictée par défaut) et ne colle pas ce qu'on croit.
//!
//! Envoyer des touches à une autre appli demande l'autorisation « Accessibilité » (Réglages Système → Confidentialité et sécurité) ; sans
//! elle, le texte reste dans le presse-papiers (« Copié, à coller… »). La touche V est cherchée à sa place QWERTY (code 9), qui est aussi
//! celle de l'AZERTY du Mac ; une disposition Dvorak ou Colemak la déplace (limite connue).

use super::ffi;
use crate::apple::PASTE_BLOCKING_FLAGS;
use crate::paste::{
    PasteOutcome, Paster, SelectedRead, SelectedText, Selection, Unreadable, SHORTCUT_KEY_UNKNOWN, SHORTCUT_NOT_SENT, SHORTCUT_NOT_TRUSTED,
};
use arboard::Clipboard;
use scriibe_core::shortcut::Shortcut;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

/// La demande d'autorisation « Accessibilité » a déjà été montrée pendant ce lancement.
static ASKED: AtomicBool = AtomicBool::new(false);

const KEY_V: u16 = 9;
const KEY_C: u16 = 8;
const KEY_COMMAND: u16 = 55;
/// `kCGEventFlagMaskCommand`.
const FLAG_COMMAND: u64 = 0x0010_0000;

#[derive(Default)]
pub struct MacPaster {
    clipboard: Arc<Mutex<Option<Clipboard>>>,
    /// Le détail du dernier collage (route prise, attentes), pour le journal : un collage perdu en silence ne laisse rien d'autre.
    report: Mutex<String>,
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

/// Attend (au plus 0,3 s) que Maj, Ctrl, Option et Commande soient relâchés. Rend le temps attendu en millisecondes et, si la touche
/// restait enfoncée jusqu'au bout, les drapeaux qui la retenaient (0 : tout était relâché).
fn wait_for_modifiers_released() -> (u64, u64) {
    let mut waited = 0;
    for _ in 0..30 {
        // SAFETY : fonction sans autre condition que son argument.
        let flags = unsafe { ffi::CGEventSourceFlagsState(ffi::SOURCE_COMBINED_SESSION) };
        if flags & PASTE_BLOCKING_FLAGS == 0 {
            return (waited, 0);
        }
        std::thread::sleep(Duration::from_millis(10));
        waited += 10;
    }
    // SAFETY : voir ci-dessus.
    (waited, unsafe { ffi::CGEventSourceFlagsState(ffi::SOURCE_COMBINED_SESSION) } & PASTE_BLOCKING_FLAGS)
}

fn post_key(key: u16, down: bool) -> Result<(), String> {
    post_key_flags(key, down, FLAG_COMMAND)
}

fn post_key_flags(key: u16, down: bool, flags: u64) -> Result<(), String> {
    // SAFETY : une source nulle veut dire « source par défaut » ; l'événement créé est relâché après l'envoi.
    unsafe {
        let event = ffi::CGEventCreateKeyboardEvent(std::ptr::null(), key, down);
        if event.is_null() {
            return Err("macOS n'a pas voulu créer la touche".into());
        }
        ffi::CGEventSetFlags(event, flags);
        ffi::CGEventPost(ffi::TAP_HID, event);
        ffi::CFRelease(event);
    }
    Ok(())
}

/// Les navigateurs construits sur Chromium : leur champ de recherche ignore parfois un ⌘V simulé, mais pas Édition → Coller.
const CHROMIUM_BROWSERS: &[&str] = &[
    "com.google.Chrome",
    "com.brave.Browser",
    "com.microsoft.edgemac",
    "com.vivaldi.Vivaldi",
    "org.chromium.Chromium",
    "company.thebrowser.Browser",
];

/// Envoie ⌘V. Rend l'attente des modificateurs (voir `wait_for_modifiers_released`).
fn send_command_v() -> Result<(u64, u64), String> {
    // SAFETY : fonction sans paramètre.
    if !unsafe { ffi::AXIsProcessTrusted() } {
        // Une seule demande par lancement : la fenêtre de macOS ne doit pas revenir à chaque dictée.
        if !ASKED.swap(true, Ordering::SeqCst) {
            ffi::ask_accessibility();
        }
        return Err("collage refusé : autorise Scriibe (ou l'appli qui la lance) dans Réglages Système → Confidentialité et sécurité → Accessibilité".into());
    }
    let waited = wait_for_modifiers_released();
    std::thread::sleep(Duration::from_millis(40));
    // Une vraie frappe de ⌘V : la touche ⌘ s'appuie avant V et se relâche après (certaines pages web ignoraient un V portant seulement le drapeau ⌘).
    post_key_flags(KEY_COMMAND, true, FLAG_COMMAND)?;
    std::thread::sleep(Duration::from_millis(10));
    post_key(KEY_V, true)?;
    std::thread::sleep(Duration::from_millis(10));
    post_key(KEY_V, false)?;
    std::thread::sleep(Duration::from_millis(10));
    post_key_flags(KEY_COMMAND, false, 0)?;
    Ok(waited)
}

/// Code de touche macOS d'une lettre ou d'un chiffre, à sa place QWERTY (même limite que ⌘C et ⌘V : Dvorak et Colemak la déplacent).
/// Une vraie frappe de ⌘C, comme ⌘V : ⌘ enfoncé, C, C relâché, ⌘ relâché. Avec des C portant seulement le drapeau ⌘, le système gardait ⌘ pour
/// enfoncé (journal : « drapeaux retenus 0x100000 ») et le collage suivant attendait 0,3 s que la touche se relâche.
fn send_command_c() -> Result<(), String> {
    post_key_flags(KEY_COMMAND, true, FLAG_COMMAND)?;
    std::thread::sleep(Duration::from_millis(10));
    post_key(KEY_C, true)?;
    std::thread::sleep(Duration::from_millis(10));
    post_key(KEY_C, false)?;
    std::thread::sleep(Duration::from_millis(10));
    post_key_flags(KEY_COMMAND, false, 0)
}

/// Écrit le texte comme une frappe au clavier : des événements « touche » qui portent chacun quelques caractères (CoreGraphics en accepte
/// 20 par événement), un vrai Entrée pour chaque saut de ligne. Aucun presse-papiers : un champ de navigateur qui ignore le collage (le
/// premier après un clic, constaté avec Chrome et YouTube) accepte une saisie.
fn type_text(text: &str) -> Result<(), String> {
    // SAFETY : fonction sans paramètre.
    if !unsafe { ffi::AXIsProcessTrusted() } {
        return Err("Accessibilité non autorisée".into());
    }
    wait_for_modifiers_released();
    std::thread::sleep(Duration::from_millis(40));
    let mut lines = text.split('\n').peekable();
    while let Some(line) = lines.next() {
        let units: Vec<u16> = line.encode_utf16().collect();
        for chunk in units.chunks(16) {
            for down in [true, false] {
                // SAFETY : un événement clavier créé, rempli avec `chunk` (valide pendant l'appel), posté puis relâché.
                unsafe {
                    let event = ffi::CGEventCreateKeyboardEvent(std::ptr::null(), 0, down);
                    if event.is_null() {
                        return Err("macOS n'a pas voulu créer la touche".into());
                    }
                    ffi::CGEventSetFlags(event, 0);
                    ffi::CGEventKeyboardSetUnicodeString(event, chunk.len(), chunk.as_ptr());
                    ffi::CGEventPost(ffi::TAP_HID, event);
                    ffi::CFRelease(event);
                }
                std::thread::sleep(Duration::from_millis(3));
            }
        }
        if lines.peek().is_some() {
            post_key_flags(KEY_RETURN, true, 0)?;
            std::thread::sleep(Duration::from_millis(5));
            post_key_flags(KEY_RETURN, false, 0)?;
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    Ok(())
}

fn key_code(key: char) -> Option<u16> {
    Some(match key {
        'a' => 0,
        's' => 1,
        'd' => 2,
        'f' => 3,
        'h' => 4,
        'g' => 5,
        'z' => 6,
        'x' => 7,
        'c' => 8,
        'v' => 9,
        'b' => 11,
        'q' => 12,
        'w' => 13,
        'e' => 14,
        'r' => 15,
        'y' => 16,
        't' => 17,
        '1' => 18,
        '2' => 19,
        '3' => 20,
        '4' => 21,
        '6' => 22,
        '5' => 23,
        '9' => 25,
        '7' => 26,
        '8' => 28,
        '0' => 29,
        'o' => 31,
        'u' => 32,
        'i' => 34,
        'p' => 35,
        'l' => 37,
        'j' => 38,
        'k' => 40,
        'n' => 45,
        'm' => 46,
        _ => return None,
    })
}

const FLAG_SHIFT: u64 = 0x0002_0000;
const FLAG_CONTROL: u64 = 0x0004_0000;
const FLAG_OPTION: u64 = 0x0008_0000;
const KEY_SHIFT: u16 = 56;
const KEY_RETURN: u16 = 36;
const KEY_CONTROL: u16 = 59;
const KEY_OPTION: u16 = 58;

/// Envoie `shortcut` comme une vraie frappe : chaque modificateur s'appuie avant la touche et se relâche après (comme ⌘V).
fn send_shortcut(shortcut: &Shortcut) -> Result<(), &'static str> {
    let key = key_code(shortcut.key).ok_or(SHORTCUT_KEY_UNKNOWN)?;
    // SAFETY : fonction sans paramètre.
    if !unsafe { ffi::AXIsProcessTrusted() } {
        if !ASKED.swap(true, Ordering::SeqCst) {
            ffi::ask_accessibility();
        }
        return Err(SHORTCUT_NOT_TRUSTED);
    }
    let mut modifiers: Vec<(u16, u64)> = Vec::new();
    if shortcut.control {
        modifiers.push((KEY_CONTROL, FLAG_CONTROL));
    }
    if shortcut.alt {
        modifiers.push((KEY_OPTION, FLAG_OPTION));
    }
    if shortcut.shift {
        modifiers.push((KEY_SHIFT, FLAG_SHIFT));
    }
    if shortcut.command {
        modifiers.push((KEY_COMMAND, FLAG_COMMAND));
    }
    wait_for_modifiers_released();
    std::thread::sleep(Duration::from_millis(40));
    let pause = || std::thread::sleep(Duration::from_millis(10));
    let mut flags = 0;
    let mut result = Ok(());
    for (code, flag) in &modifiers {
        flags |= flag;
        result = result.and_then(|()| post_key_flags(*code, true, flags));
        pause();
    }
    result = result.and_then(|()| post_key_flags(key, true, flags));
    pause();
    result = result.and_then(|()| post_key_flags(key, false, flags));
    pause();
    // Les modificateurs se relâchent toujours, même après une erreur : une touche restée appuyée gênerait tout le reste.
    for (code, flag) in modifiers.iter().rev() {
        flags &= !flag;
        let _ = post_key_flags(*code, false, flags);
        pause();
    }
    result.map_err(|_| SHORTCUT_NOT_SENT)
}

/// Combien de temps on laisse à l'appli pour copier sa sélection après ⌘C.
const COPY_WAIT: Duration = Duration::from_millis(150);
/// Pour une commande vocale (déjà reconnue), on attend plus longtemps : une appli lente (Electron) qui copie après le délai laisserait la
/// sélection dans le presse-papiers.
const COMMAND_COPY_WAIT: Duration = Duration::from_millis(400);

/// Écrase un texte qui a contenu la sélection avant de le laisser tomber.
fn wipe(text: Option<String>) {
    if let Some(text) = text {
        let mut bytes = text.into_bytes();
        bytes.fill(0);
        std::hint::black_box(&bytes);
    }
}

/// Attend (au plus `max`) que le presse-papiers contienne `wanted`.
fn wait_for_text(slot: &Mutex<Option<Clipboard>>, wanted: &Option<String>, max: Duration) -> bool {
    let mut waited = Duration::ZERO;
    loop {
        if read_text(slot) == *wanted {
            return true;
        }
        if waited >= max {
            return false;
        }
        std::thread::sleep(Duration::from_millis(10));
        waited += Duration::from_millis(10);
    }
}

impl MacPaster {
    /// Sonde pour les applis qui n'exposent pas leur sélection à l'Accessibilité (Electron, pages web) : ⌘C, puis on regarde si le
    /// presse-papiers a changé. Si oui, un texte était sélectionné (on rend sa longueur, jamais son contenu) ; le presse-papiers d'avant
    /// est remis, et on s'assure que la remise est faite avant de rendre la main : le collage qui suit écrit à son tour, et une écriture de
    /// l'appli arrivant en retard (Chrome copie en deux temps) ne doit pas passer après la nôtre.
    fn selection_by_copy(&self) -> Option<usize> {
        let before = read_text(&self.clipboard);
        wait_for_modifiers_released();
        std::thread::sleep(Duration::from_millis(40));
        send_command_c().ok()?;
        let mut waited = Duration::ZERO;
        while waited < COPY_WAIT {
            std::thread::sleep(Duration::from_millis(10));
            waited += Duration::from_millis(10);
            let now = read_text(&self.clipboard);
            if now == before {
                continue;
            }
            // L'appli a copié : on la laisse finir (même valeur deux fois de suite), puis on remet l'ancien contenu.
            let mut copied = now;
            for _ in 0..10 {
                std::thread::sleep(Duration::from_millis(15));
                let again = read_text(&self.clipboard);
                if again == copied {
                    break;
                }
                copied = again;
            }
            match &before {
                Some(saved) => drop(write_text(&self.clipboard, saved)),
                None => drop(with_clipboard(&self.clipboard, |c| c.clear().map_err(|e| e.to_string()))),
            }
            wait_for_text(&self.clipboard, &before, Duration::from_millis(200));
            std::thread::sleep(Duration::from_millis(60));
            return Some(copied.map_or(0, |t| t.chars().count()));
        }
        None
    }
}

/// Types de presse-papiers qui ne sont pas du texte (image, fichiers) : le ⌘C d'essai les écraserait pour de bon.
const NON_TEXT_CLIPBOARD_MARKERS: [&str; 7] = ["PNGf", "TIFF", "JPEG", "furl", "GIFf", "public.file-url", "public.png"];

/// Le presse-papiers contient-il autre chose que du texte ? `None` : on ne sait pas le dire (on ne risque alors rien).
fn clipboard_holds_non_text() -> Option<bool> {
    let output = std::process::Command::new("/usr/bin/osascript").args(["-e", "clipboard info"]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(has_non_text_type(&String::from_utf8_lossy(&output.stdout)))
}

/// La sortie de `clipboard info` (« «class utf8», 90, string, 0, … ») cite-t-elle une image ou des fichiers ?
fn has_non_text_type(info: &str) -> bool {
    NON_TEXT_CLIPBOARD_MARKERS.iter().any(|m| info.contains(m))
}

impl MacPaster {
    /// Lecture du texte sélectionné par ⌘C d'essai (applis qui n'exposent pas leur sélection). Le presse-papiers texte est sauvegardé puis
    /// remis. S'il contient une image ou des fichiers (même avec du texte à côté), ou si on ne sait pas le dire, pas de ⌘C : on les
    /// perdrait. Limite : la mise en forme (RTF, HTML) d'un presse-papiers texte n'est pas rendue, comme au collage d'une dictée.
    fn selected_text_by_copy(&self) -> SelectedRead {
        if clipboard_holds_non_text() != Some(false) {
            return SelectedRead::Unreadable(Unreadable::ClipboardNotText);
        }
        let before = read_text(&self.clipboard);
        wait_for_modifiers_released();
        std::thread::sleep(Duration::from_millis(40));
        if send_command_c().is_err() {
            return SelectedRead::Unreadable(Unreadable::Failed);
        }
        let mut waited = Duration::ZERO;
        let mut changed = false;
        let mut found: Option<String> = None;
        while waited < COMMAND_COPY_WAIT {
            std::thread::sleep(Duration::from_millis(10));
            waited += Duration::from_millis(10);
            let now = read_text(&self.clipboard);
            if now == before {
                wipe(now);
                continue;
            }
            // L'appli a copié (du texte ou autre chose) : on la laisse finir (même valeur deux fois de suite).
            changed = true;
            let mut copied = now;
            for _ in 0..10 {
                std::thread::sleep(Duration::from_millis(15));
                let again = read_text(&self.clipboard);
                if again == copied {
                    wipe(again);
                    break;
                }
                wipe(std::mem::replace(&mut copied, again));
            }
            found = copied;
            break;
        }
        // Le presse-papiers d'avant est remis dès qu'il a changé, quoi qu'ait copié l'appli, et la remise est vérifiée avant de rendre la
        // main. Si elle échoue, on vide le presse-papiers : mieux vaut le perdre que laisser la sélection dedans.
        if changed {
            match &before {
                Some(saved) => drop(write_text(&self.clipboard, saved)),
                None => drop(with_clipboard(&self.clipboard, |c| c.clear().map_err(|e| e.to_string()))),
            }
            if !wait_for_text(&self.clipboard, &before, Duration::from_millis(200)) {
                drop(with_clipboard(&self.clipboard, |c| c.clear().map_err(|e| e.to_string())));
            }
            std::thread::sleep(Duration::from_millis(60));
        }
        wipe(before);
        match found {
            Some(text) if !text.is_empty() => SelectedRead::Text(SelectedText::new(text)),
            other => {
                wipe(other);
                SelectedRead::NoSelection
            }
        }
    }
}

impl Paster for MacPaster {
    fn selected_text(&self) -> SelectedRead {
        match ffi::selected_text() {
            ffi::SelectedByAx::Text(text) => SelectedRead::Text(SelectedText::new(text)),
            ffi::SelectedByAx::Empty => SelectedRead::NoSelection,
            ffi::SelectedByAx::NotTrusted => SelectedRead::Unreadable(Unreadable::NotTrusted),
            ffi::SelectedByAx::Unknown => self.selected_text_by_copy(),
        }
    }

    fn selection(&self) -> Selection {
        match ffi::selection_state() {
            ffi::SelectionState::Active => Selection { active: true, report: "active".into() },
            ffi::SelectionState::Empty => Selection { active: false, report: "aucune".into() },
            // L'appli ne dit rien : ⌘C d'essai (pas sans l'autorisation d'envoyer des touches).
            ffi::SelectionState::Unknown(..) => match self.selection_by_copy() {
                Some(len) => Selection { active: true, report: format!("active (⌘C d'essai, {len} signes)") },
                None => Selection { active: false, report: "aucune (⌘C d'essai)".into() },
            },
            ffi::SelectionState::NotTrusted => Selection { active: false, report: ffi::SelectionState::NotTrusted.describe() },
        }
    }

    fn press_shortcut(&self, shortcut: &Shortcut) -> Result<(), &'static str> {
        send_shortcut(shortcut)
    }

    fn press_return(&self) -> bool {
        // Entrée seule, sans ⌘ : on attend que les touches de la dictée soient relâchées, puis Entrée (code 36).
        wait_for_modifiers_released();
        let sent = post_key_flags(KEY_RETURN, true, 0).and_then(|()| {
            std::thread::sleep(Duration::from_millis(10));
            post_key_flags(KEY_RETURN, false, 0)
        });
        std::thread::sleep(Duration::from_millis(30));
        sent.is_ok()
    }
    fn copy(&self, text: &str) -> bool {
        write_text(&self.clipboard, text).is_ok()
    }

    fn last_report(&self) -> String {
        lock(&self.report).clone()
    }

    fn paste(&self, text: &str, restore_clipboard: bool) -> PasteOutcome {
        // Un navigateur Chromium : le texte est « tapé » (sans toucher au presse-papiers), le collage étant parfois ignoré par ses champs.
        let browser = crate::apps::ForegroundApp::current(&super::foreground::MacForeground)
            .id
            .is_some_and(|id| CHROMIUM_BROWSERS.contains(&id.as_str()));
        if browser {
            let started = std::time::Instant::now();
            match type_text(text) {
                Ok(()) => {
                    *lock(&self.report) = format!("route : frappe, {} ms", started.elapsed().as_millis());
                    return PasteOutcome::Pasted;
                }
                Err(why) => *lock(&self.report) = format!("frappe impossible ({why}), repli sur le collage"),
            }
        }
        let saved = if restore_clipboard { read_text(&self.clipboard) } else { None };
        if let Err(e) = write_text(&self.clipboard, text) {
            return PasteOutcome::CopiedOnly(e);
        }
        // Le collage lit le presse-papiers : on s'assure que c'est bien la dictée qu'il contient (0,2 s au plus).
        let clipboard_start = std::time::Instant::now();
        let ready = wait_for_text(&self.clipboard, &Some(text.to_string()), Duration::from_millis(200));
        let clipboard_ms = clipboard_start.elapsed().as_millis();
        let mut report = format!("presse-papiers {}{clipboard_ms} ms", if ready { "prêt en " } else { "NON prêt après " });
        // Frappe impossible dans un navigateur : Édition → Coller par l'Accessibilité (aucune touche simulée), sinon ⌘V.
        let mut by_menu = false;
        if browser {
            match super::foreground::front_pid()
                .ok_or_else(|| "numéro de processus inconnu".to_string())
                .and_then(ffi::press_paste_menu_item)
            {
                Ok(()) => {
                    by_menu = true;
                    report.push_str(", route : menu Coller");
                }
                Err(why) => report.push_str(&format!(", menu Coller impossible ({why})")),
            }
        }
        if !by_menu {
            match send_command_v() {
                Ok((waited, flags)) => {
                    report.push_str(&format!(", route : ⌘V, touches relâchées en {waited} ms"));
                    if flags != 0 {
                        report.push_str(&format!(" (drapeaux retenus {flags:#x})"));
                    }
                }
                Err(e) => {
                    *lock(&self.report) = report;
                    return PasteOutcome::CopiedOnly(e);
                }
            }
        }
        *lock(&self.report) = report;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipboard_info_tells_text_from_images_and_files() {
        // Texte seul, tel que `osascript -e 'clipboard info'` le rend.
        assert!(!has_non_text_type("«class utf8», 90, «class ut16», 176, string, 0, Unicode text, 174"));
        // Image, fichiers copiés dans le Finder, texte accompagné d'une image : pas de ⌘C.
        assert!(has_non_text_type("«class PNGf», 48210, «class TIFF», 99000"));
        assert!(has_non_text_type("«class furl», 120, «class utf8», 14"));
        assert!(has_non_text_type("«class utf8», 12, «class JPEG», 30000"));
    }

    #[test]
    fn wipe_accepts_nothing_and_text() {
        wipe(None);
        wipe(Some("secret".into()));
    }
}
