//! Touche globale : le système annonce les changements des touches de modification et les touches ordinaires tapées.
//! La machine à états (appui, relâchement, traduction, annulation) est dans `scriibe_core::hotkey`.

use scriibe_core::hotkey::Key;
use std::sync::mpsc::Sender;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyEvent {
    /// Une touche de modification a changé ; `held` : toutes celles qui sont tenues maintenant.
    Modifier { changed: Key, held: Vec<Key> },
    /// Une touche ordinaire (ou Alt, Windows…) est enfoncée.
    Typed,
}

/// Un abonnement qui s'arrête quand on le lâche.
pub trait Subscription: Send {}

pub trait KeyListener: Send + Sync {
    /// Commence à envoyer les événements dans `sink`. Les touches ne sont jamais retenues : l'appli active les reçoit comme avant.
    /// Le rappel du système doit rester très court : on ne fait que poster l'événement.
    fn start(&self, sink: Sender<KeyEvent>) -> Result<Box<dyn Subscription>, String>;
}

/// Codes de touche virtuels de Windows (`VK_*`) des touches de modification que la machine à états connaît.
pub mod vk {
    pub const LSHIFT: u32 = 0xA0;
    pub const RSHIFT: u32 = 0xA1;
    pub const LCONTROL: u32 = 0xA2;
    pub const RCONTROL: u32 = 0xA3;
}

fn modifier(vk: u32) -> Option<Key> {
    match vk {
        vk::LSHIFT => Some(Key::LeftShift),
        vk::RSHIFT => Some(Key::RightShift),
        vk::LCONTROL => Some(Key::LeftControl),
        vk::RCONTROL => Some(Key::RightControl),
        _ => None,
    }
}

/// Transforme un événement de touche de Windows (`vk` : code de touche, `down` : enfoncée ou relâchée) en événement de l'appli.
/// `held` : les touches de modification déjà tenues, mises à jour ici. Le maintien d'une touche enfoncée (répétition) ne change rien.
/// Alt, la touche Windows et toutes les lettres comptent comme « autre touche » : Ctrl droit + une lettre est un raccourci clavier.
pub fn translate_vk(held: &mut Vec<Key>, vk: u32, down: bool) -> Option<KeyEvent> {
    match modifier(vk) {
        Some(key) => {
            let already = held.contains(&key);
            if down == already {
                return None;
            }
            if down {
                held.push(key);
            } else {
                held.retain(|k| *k != key);
            }
            Some(KeyEvent::Modifier { changed: key, held: held.clone() })
        }
        None => down.then_some(KeyEvent::Typed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scriibe_core::hotkey::{HotkeyEvent, HotkeyMonitor};
    use scriibe_core::settings::Hotkey;

    /// Rejoue des événements Windows dans la machine à états du cœur, comme le fait le chef d'orchestre.
    fn run(events: &[(u32, bool)]) -> String {
        let mut held = Vec::new();
        let mut monitor = HotkeyMonitor::new(Hotkey::RightControl);
        let mut out = String::new();
        for (code, down) in events {
            let produced = match translate_vk(&mut held, *code, *down) {
                Some(KeyEvent::Modifier { changed, held }) => monitor.handle(changed, &held),
                Some(KeyEvent::Typed) => monitor.handle_key_down(),
                None => Vec::new(),
            };
            out.extend(produced.into_iter().map(HotkeyEvent::letter));
        }
        out
    }

    const A: u32 = 0x41;
    const LALT: u32 = 0xA4;

    #[test]
    fn right_control_alone_is_a_dictation() {
        assert_eq!(run(&[(vk::RCONTROL, true), (vk::RCONTROL, false)]), "PR");
    }

    #[test]
    fn key_repeat_does_not_press_twice() {
        assert_eq!(run(&[(vk::RCONTROL, true), (vk::RCONTROL, true), (vk::RCONTROL, true), (vk::RCONTROL, false)]), "PR");
    }

    #[test]
    fn shift_added_translates() {
        assert_eq!(run(&[(vk::RCONTROL, true), (vk::LSHIFT, true), (vk::LSHIFT, false), (vk::RCONTROL, false)]), "PCR");
    }

    #[test]
    fn a_shortcut_cancels() {
        // Ctrl droit + A : un raccourci, pas une dictée.
        assert_eq!(run(&[(vk::RCONTROL, true), (A, true), (A, false), (vk::RCONTROL, false)]), "PXR");
        // Ctrl droit + Alt (hors AltGr) aussi.
        assert_eq!(run(&[(vk::RCONTROL, true), (LALT, true), (LALT, false), (vk::RCONTROL, false)]), "PXR");
    }

    #[test]
    fn typing_without_the_hotkey_is_ignored() {
        assert_eq!(run(&[(A, true), (A, false), (vk::LCONTROL, true), (A, true), (A, false), (vk::LCONTROL, false)]), "");
    }

    #[test]
    fn altgr_is_not_right_control() {
        // Sous Windows, AltGr arrive comme Ctrl gauche puis Alt droite : jamais Ctrl droit.
        const RALT: u32 = 0xA5;
        assert_eq!(run(&[(vk::LCONTROL, true), (RALT, true), (RALT, false), (vk::LCONTROL, false)]), "");
    }
}
