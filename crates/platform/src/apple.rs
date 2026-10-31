//! Les parties pures de la couche macOS : table des touches du Mac et lecture de la sortie de `lsappinfo`. Elles ne touchent à rien du système,
//! donc elles compilent et se testent sur tous les systèmes (le reste de la couche est dans `macos`).
#![cfg_attr(not(target_os = "macos"), allow(dead_code))]

use crate::apps::AppInfo;
use crate::keys::vk_of_web_code;
use scriibe_core::hotkey::Key;

/// Début des codes donnés aux touches inconnues du Mac (au-dessus de tout code de Windows et de ceux de Linux) : elles comptent comme
/// « touche tenue » (une autre touche annule la dictée) sans jamais pouvoir être choisies.
pub const UNKNOWN_BASE: u32 = 0x2_0000;

/// Les touches de modification du Mac : code de touche (`kVK_*`), la touche de Scriibe, et le bit de `CGEventFlags` propre à cette touche
/// (indicateurs « dépendants du périphérique » de IOKit, qui distinguent la gauche de la droite). Le bit dit si elle est tenue après l'événement.
pub const MODIFIERS: [(u32, Key, u64); 9] = [
    (59, Key::LeftControl, 0x0000_0001),
    (62, Key::RightControl, 0x0000_2000),
    (56, Key::LeftShift, 0x0000_0002),
    (60, Key::RightShift, 0x0000_0004),
    (55, Key::LeftCommand, 0x0000_0008),
    (54, Key::RightCommand, 0x0000_0010),
    (58, Key::LeftOption, 0x0000_0020),
    (61, Key::RightOption, 0x0000_0040),
    // Fn : un indicateur ordinaire (NX_SECONDARYFNMASK), pas un bit de périphérique.
    (63, Key::Fn, 0x0080_0000),
];

/// Les touches de modification dont l'appui fausserait un collage (Maj, Ctrl, Option, Commande) : leurs bits dans `CGEventFlags`.
pub const PASTE_BLOCKING_FLAGS: u64 = 0x0002_0000 | 0x0004_0000 | 0x0008_0000 | 0x0010_0000;

/// La touche de modification de ce code de touche, avec son bit.
pub fn modifier_of_keycode(code: u32) -> Option<(Key, u64)> {
    MODIFIERS.iter().find(|(c, _, _)| *c == code).map(|(_, key, mask)| (*key, *mask))
}

/// Le nom de la touche dans `KeyboardEvent.code` (emplacement physique, clavier QWERTY de référence) pour un code de touche du Mac.
pub fn web_code(code: u32) -> Option<String> {
    const LETTERS: [(u32, char); 26] = [
        (0, 'A'),
        (11, 'B'),
        (8, 'C'),
        (2, 'D'),
        (14, 'E'),
        (3, 'F'),
        (5, 'G'),
        (4, 'H'),
        (34, 'I'),
        (38, 'J'),
        (40, 'K'),
        (37, 'L'),
        (46, 'M'),
        (45, 'N'),
        (31, 'O'),
        (35, 'P'),
        (12, 'Q'),
        (15, 'R'),
        (1, 'S'),
        (17, 'T'),
        (32, 'U'),
        (9, 'V'),
        (13, 'W'),
        (7, 'X'),
        (16, 'Y'),
        (6, 'Z'),
    ];
    const DIGITS: [(u32, char); 10] =
        [(29, '0'), (18, '1'), (19, '2'), (20, '3'), (21, '4'), (23, '5'), (22, '6'), (26, '7'), (28, '8'), (25, '9')];
    if let Some((_, letter)) = LETTERS.iter().find(|(c, _)| *c == code) {
        return Some(format!("Key{letter}"));
    }
    if let Some((_, digit)) = DIGITS.iter().find(|(c, _)| *c == code) {
        return Some(format!("Digit{digit}"));
    }
    // F1 à F20 : les codes ne se suivent pas.
    const FUNCTION: [(u32, u32); 20] = [
        (122, 1),
        (120, 2),
        (99, 3),
        (118, 4),
        (96, 5),
        (97, 6),
        (98, 7),
        (100, 8),
        (101, 9),
        (109, 10),
        (103, 11),
        (111, 12),
        (105, 13),
        (107, 14),
        (113, 15),
        (106, 16),
        (64, 17),
        (79, 18),
        (80, 19),
        (90, 20),
    ];
    if let Some((_, n)) = FUNCTION.iter().find(|(c, _)| *c == code) {
        return Some(format!("F{n}"));
    }
    Some(
        match code {
            10 | 50 => "IntlBackslash",
            36 => "Enter",
            76 => "NumpadEnter",
            48 => "Tab",
            49 => "Space",
            51 => "Backspace",
            53 => "Escape",
            57 => "CapsLock",
            114 => "Insert",
            115 => "Home",
            116 => "PageUp",
            117 => "Delete",
            119 => "End",
            121 => "PageDown",
            123 => "ArrowLeft",
            124 => "ArrowRight",
            125 => "ArrowDown",
            126 => "ArrowUp",
            _ => return None,
        }
        .to_string(),
    )
}

/// Le code de touche virtuel de Windows d'une touche ordinaire du Mac (celui que tout le reste de Scriibe utilise). Les touches inconnues
/// gardent un code à elles.
pub fn vk_of_keycode(code: u32) -> u32 {
    web_code(code).and_then(|name| vk_of_web_code(&name)).unwrap_or(UNKNOWN_BASE + code)
}

/// L'identifiant de l'application au premier plan donné par `lsappinfo front` (« ASN:0x0-0x1b01b: »).
pub fn parse_front(output: &str) -> Option<String> {
    output.split_whitespace().find(|word| word.starts_with("ASN:")).map(str::to_string)
}

/// L'appli décrite par `lsappinfo info -only name -only bundleid <ASN>`. Sur macOS récent, la première ligne porte le nom et la suivante
/// l'identifiant : `"Notes" ASN:0x0-0xc00c: (in front)` puis `    bundleID="com.apple.Notes"`. D'autres versions écrivent des lignes
/// `"LSDisplayName"="Notes"` et `"CFBundleIdentifier"="com.apple.Notes"` : les deux formes sont lues.
pub fn parse_app_info(output: &str) -> AppInfo {
    let mut name = None;
    let mut id = None;
    for line in output.lines().map(str::trim) {
        if let Some((quoted, after)) = line.strip_prefix('"').and_then(|rest| rest.split_once('"')) {
            if after.trim_start().starts_with("ASN:") {
                name = Some(quoted.to_string()).filter(|n| !n.is_empty());
                continue;
            }
        }
        let Some((key, value)) = line.split_once('=') else { continue };
        let key = key.trim().trim_matches('"').to_ascii_lowercase();
        let value = value.trim().trim_matches('"').trim();
        if value.is_empty() || value.starts_with('[') {
            continue;
        }
        match key.as_str() {
            "lsdisplayname" | "name" => name = Some(value.to_string()),
            "cfbundleidentifier" | "bundleid" => id = Some(value.to_string()),
            _ => {}
        }
    }
    match (name, id) {
        (None, None) => AppInfo::unknown(),
        (name, id) => AppInfo { name: name.or_else(|| id.clone()).unwrap_or_else(|| "Inconnue".into()), id },
    }
}

/// La langue lue par `defaults read -g AppleLocale` (« fr_FR »), ou rien.
pub fn parse_locale(output: &str) -> String {
    output.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::CHOICES;

    #[test]
    fn the_modifier_keys_of_the_mac_have_their_own_name_and_bit() {
        assert_eq!(modifier_of_keycode(61), Some((Key::RightOption, 0x40)));
        assert_eq!(modifier_of_keycode(58), Some((Key::LeftOption, 0x20)));
        assert_eq!(modifier_of_keycode(54), Some((Key::RightCommand, 0x10)));
        assert_eq!(modifier_of_keycode(63), Some((Key::Fn, 0x80_0000)));
        assert_eq!(modifier_of_keycode(62), Some((Key::RightControl, 0x2000)));
        assert_eq!(modifier_of_keycode(0), None, "la touche A n'est pas une touche de modification");
    }

    #[test]
    fn ordinary_keys_become_the_codes_of_windows_at_their_physical_place() {
        assert_eq!(vk_of_keycode(122), 0x70, "F1");
        assert_eq!(vk_of_keycode(101), 0x78, "F9");
        assert_eq!(vk_of_keycode(90), 0x83, "F20");
        assert_eq!(vk_of_keycode(0), 0x41, "la touche A (à sa place QWERTY)");
        assert_eq!(vk_of_keycode(29), 0x30, "chiffre 0");
        assert_eq!(vk_of_keycode(115), 0x24, "Début");
        assert_eq!(vk_of_keycode(10), 0xE2, "« < > » des claviers ISO");
        assert_eq!(vk_of_keycode(49), 0x20, "Espace");
    }

    #[test]
    fn unknown_keys_keep_a_code_of_their_own() {
        let odd = vk_of_keycode(200);
        assert!(odd >= UNKNOWN_BASE);
        assert_ne!(odd, vk_of_keycode(201));
    }

    #[test]
    fn the_function_keys_of_the_list_are_all_seen_on_a_mac_up_to_f20() {
        for (choice, n) in CHOICES.iter().filter_map(|c| Some((c, c.id.strip_prefix('f')?.parse::<u32>().ok()?))) {
            let seen = (0..128u32).any(|code| vk_of_keycode(code) == choice.vk);
            assert_eq!(seen, n <= 20, "{} : vu = {seen}", choice.label);
        }
    }

    #[test]
    fn the_front_app_is_read_from_lsappinfo() {
        assert_eq!(parse_front("ASN:0x0-0x1b01b:\n").as_deref(), Some("ASN:0x0-0x1b01b:"));
        assert_eq!(parse_front(""), None);
        let info = parse_app_info("\"LSDisplayName\"=\"Notes\"\n\"CFBundleIdentifier\"=\"com.apple.Notes\"\n");
        assert_eq!(info, AppInfo { name: "Notes".into(), id: Some("com.apple.Notes".into()) });
        // La vraie sortie d'un Mac (macOS récent), relevée sur le Mac de Christophe.
        let real = "\"TextEdit\" ASN:0x0-0xee0ee: (in front) 
    bundleID=\"com.apple.TextEdit\"
    bundle path=[ NULL ] 
    executable path=[ NULL ] 
 !cgsConnection !signalled type=[ NULL ]  flavor=[ NULL ]  Version=[ NULL ]  Arch=!!none 
";
        assert_eq!(parse_app_info(real), AppInfo { name: "TextEdit".into(), id: Some("com.apple.TextEdit".into()) });
        let old = parse_app_info("\"name\"=\"Safari\"\n\"bundleid\"=\"com.apple.Safari\"\n");
        assert_eq!(old.id.as_deref(), Some("com.apple.Safari"));
        let only_id = parse_app_info("\"CFBundleIdentifier\"=\"org.example.App\"\n");
        assert_eq!(only_id.name, "org.example.App");
        assert_eq!(parse_app_info("\"LSDisplayName\"=[ NULL ]\n"), AppInfo::unknown());
        assert_eq!(parse_app_info(""), AppInfo::unknown());
    }
}
