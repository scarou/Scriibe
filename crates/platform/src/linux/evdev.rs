//! Les touches vues par X11 (code évdev + 8) ramenées aux codes de touche virtuels de Windows, que tout le reste de Scriibe utilise
//! (`keys::CHOICES`, les réglages, la machine à états). Les touches qu'on ne connaît pas gardent un code à elles (`UNKNOWN_BASE` + code) :
//! elles comptent comme « touche tenue » (une autre touche annule la dictée) sans jamais pouvoir être choisies.

use crate::keys::vk_of_web_code;

/// Les codes de touche de X11 sont ceux du noyau (évdev) décalés de 8.
pub const X11_OFFSET: u32 = 8;

/// Début des codes donnés aux touches inconnues (au-dessus de tout code de Windows).
pub const UNKNOWN_BASE: u32 = 0x1_0000;

const ROW_TOP: &str = "QWERTYUIOP";
const ROW_HOME: &str = "ASDFGHJKL";
const ROW_BOTTOM: &str = "ZXCVBNM";

/// Le nom de la touche dans `KeyboardEvent.code` (emplacement physique, clavier QWERTY de référence), pour un code évdev du noyau.
pub fn web_code(evdev: u32) -> Option<String> {
    let letter = |row: &str, first: u32| row.chars().nth((evdev - first) as usize).map(|c| format!("Key{c}"));
    Some(match evdev {
        1 => "Escape".into(),
        2..=10 => format!("Digit{}", evdev - 1),
        11 => "Digit0".into(),
        14 => "Backspace".into(),
        15 => "Tab".into(),
        16..=25 => return letter(ROW_TOP, 16),
        28 => "Enter".into(),
        29 => "ControlLeft".into(),
        30..=38 => return letter(ROW_HOME, 30),
        42 => "ShiftLeft".into(),
        44..=50 => return letter(ROW_BOTTOM, 44),
        54 => "ShiftRight".into(),
        56 => "AltLeft".into(),
        57 => "Space".into(),
        58 => "CapsLock".into(),
        59..=68 => format!("F{}", evdev - 58),
        70 => "ScrollLock".into(),
        86 => "IntlBackslash".into(),
        87 => "F11".into(),
        88 => "F12".into(),
        96 => "NumpadEnter".into(),
        97 => "ControlRight".into(),
        99 => "PrintScreen".into(),
        100 => "AltRight".into(),
        102 => "Home".into(),
        103 => "ArrowUp".into(),
        104 => "PageUp".into(),
        105 => "ArrowLeft".into(),
        106 => "ArrowRight".into(),
        107 => "End".into(),
        108 => "ArrowDown".into(),
        109 => "PageDown".into(),
        110 => "Insert".into(),
        111 => "Delete".into(),
        119 => "Pause".into(),
        125 => "MetaLeft".into(),
        126 => "MetaRight".into(),
        127 => "ContextMenu".into(),
        183..=194 => format!("F{}", evdev - 170),
        _ => return None,
    })
}

/// Le code de touche virtuel de Windows d'un code de touche de X11.
pub fn vk_of_keycode(keycode: u32) -> u32 {
    let evdev = keycode.saturating_sub(X11_OFFSET);
    web_code(evdev).and_then(|name| vk_of_web_code(&name)).unwrap_or(UNKNOWN_BASE + evdev)
}

/// Le code de touche de X11 des touches de modification dont l'appui fausserait un collage : Maj, Ctrl, Alt (gauche et droite).
pub const MODIFIER_KEYCODES: [u32; 6] =
    [42 + X11_OFFSET, 54 + X11_OFFSET, 29 + X11_OFFSET, 97 + X11_OFFSET, 56 + X11_OFFSET, 100 + X11_OFFSET];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::{vk, CHOICES};

    #[test]
    fn the_modifiers_and_function_keys_become_the_codes_of_windows() {
        assert_eq!(vk_of_keycode(105), vk::RCONTROL, "Ctrl droit : évdev 97 + 8");
        assert_eq!(vk_of_keycode(37), vk::LCONTROL);
        assert_eq!(vk_of_keycode(50), vk::LSHIFT);
        assert_eq!(vk_of_keycode(62), vk::RSHIFT);
        assert_eq!(vk_of_keycode(75), 0x78, "F9");
        assert_eq!(vk_of_keycode(95), 0x7A, "F11");
        assert_eq!(vk_of_keycode(96), 0x7B, "F12");
        assert_eq!(vk_of_keycode(94), 0xE2, "« < > » (évdev 86)");
        assert_eq!(vk_of_keycode(183 + 8), 0x7C, "F13");
        assert_eq!(vk_of_keycode(194 + 8), 0x87, "F24");
    }

    #[test]
    fn every_key_that_can_be_chosen_is_seen_on_linux_except_the_windows_key_names_and_the_menu() {
        // Les touches « Windows » de X11 sont Super : même place, autre nom ; on les reconnaît aussi.
        for choice in CHOICES {
            let seen = (1..256u32).any(|code| vk_of_keycode(code + X11_OFFSET) == choice.vk);
            assert!(seen, "la touche « {} » ({:#x}) n'a pas de code X11", choice.label, choice.vk);
        }
    }

    #[test]
    fn letters_and_digits_keep_their_physical_place_and_unknown_keys_keep_a_code_of_their_own() {
        assert_eq!(vk_of_keycode(38), 0x41, "la touche A (évdev 30) est celle de « KeyA »… à sa place QWERTY");
        assert_eq!(vk_of_keycode(10), 0x31, "chiffre 1");
        assert_eq!(vk_of_keycode(19), 0x30, "chiffre 0");
        let odd = vk_of_keycode(240);
        assert!(odd >= UNKNOWN_BASE);
        assert_ne!(odd, vk_of_keycode(241), "deux touches inconnues ne se confondent pas");
    }
}
