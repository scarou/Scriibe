//! Touche globale : le système annonce chaque changement d'état d'une touche (appuyée ou relâchée) avec toutes celles qui sont tenues.
//! La machine à états (appui, relâchement, traduction, annulation) est dans `scriibe_core::hotkey`.

use scriibe_core::hotkey::{Binding, Key};
use scriibe_core::settings::AppSettings;
use std::sync::mpsc::Sender;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyEvent {
    /// Une touche a changé d'état ; `held` : toutes celles qui sont tenues maintenant. Les touches Ctrl et Maj ont leur nom, les autres
    /// (lettres, Alt, touches de fonction…) sont des `Key::Vk`.
    Changed { changed: Key, down: bool, held: Vec<Key> },
}

/// Un abonnement qui s'arrête quand on le lâche.
pub trait Subscription: Send {}

pub trait KeyListener: Send + Sync {
    /// Commence à envoyer les événements dans `sink`. Les touches ne sont jamais retenues : l'appli active les reçoit comme avant.
    /// Le rappel du système doit rester très court : on ne fait que poster l'événement.
    fn start(&self, sink: Sender<KeyEvent>) -> Result<Box<dyn Subscription>, String>;
}

/// Codes de touche virtuels de Windows (`VK_*`) des touches de modification que la machine à états connaît par leur nom.
pub mod vk {
    pub const LSHIFT: u32 = 0xA0;
    pub const RSHIFT: u32 = 0xA1;
    pub const LCONTROL: u32 = 0xA2;
    pub const RCONTROL: u32 = 0xA3;
    pub const LMENU: u32 = 0xA4;
}

fn key_of(vk: u32) -> Key {
    match vk {
        vk::LSHIFT => Key::LeftShift,
        vk::RSHIFT => Key::RightShift,
        vk::LCONTROL => Key::LeftControl,
        vk::RCONTROL => Key::RightControl,
        other => Key::Vk(other),
    }
}

/// Transforme un événement de touche de Windows (`vk` : code de touche, `down` : enfoncée ou relâchée) en événement de l'appli.
/// `held` : les touches déjà tenues, mises à jour ici. Le maintien d'une touche enfoncée (répétition) ne change rien.
pub fn translate_vk(held: &mut Vec<Key>, vk: u32, down: bool) -> Option<KeyEvent> {
    let key = key_of(vk);
    let already = held.contains(&key);
    if down == already {
        return None;
    }
    if down {
        // Une touche dont le relâchement n'a jamais été vu (écran verrouillé…) ne reste pas indéfiniment.
        if held.len() >= MAX_HELD {
            held.remove(0);
        }
        held.push(key);
    } else {
        held.retain(|k| *k != key);
    }
    Some(KeyEvent::Changed { changed: key, down, held: held.clone() })
}

const MAX_HELD: usize = 24;

/// Une touche proposée dans les réglages.
pub struct KeyChoice {
    pub id: &'static str,
    pub label: &'static str,
    /// Code de touche virtuel de Windows.
    pub vk: u32,
    /// Touche de modification : se tient avec une autre ; sinon, touche seule ou à tenir avec un modificateur.
    pub modifier: bool,
}

const fn choice(id: &'static str, label: &'static str, vk: u32, modifier: bool) -> KeyChoice {
    KeyChoice { id, label, vk, modifier }
}

/// Les touches qu'on peut choisir (Windows). AltGr (Alt droit) et la touche Windows n'y sont pas : AltGr sert à taper « @ » ou « € » en AZERTY,
/// et la touche Windows ouvre le menu Démarrer à son relâchement.
pub const CHOICES: [KeyChoice; 36] = [
    choice("right_control", "Ctrl droit", vk::RCONTROL, true),
    choice("left_control", "Ctrl gauche", vk::LCONTROL, true),
    choice("left_alt", "Alt gauche", vk::LMENU, true),
    choice("left_shift", "Maj gauche", vk::LSHIFT, true),
    choice("right_shift", "Maj droit", vk::RSHIFT, true),
    choice("f1", "F1", 0x70, false),
    choice("f2", "F2", 0x71, false),
    choice("f3", "F3", 0x72, false),
    choice("f4", "F4", 0x73, false),
    choice("f5", "F5", 0x74, false),
    choice("f6", "F6", 0x75, false),
    choice("f7", "F7", 0x76, false),
    choice("f8", "F8", 0x77, false),
    choice("f9", "F9", 0x78, false),
    choice("f10", "F10", 0x79, false),
    choice("f11", "F11", 0x7A, false),
    choice("f12", "F12", 0x7B, false),
    choice("f13", "F13", 0x7C, false),
    choice("f14", "F14", 0x7D, false),
    choice("f15", "F15", 0x7E, false),
    choice("f16", "F16", 0x7F, false),
    choice("f17", "F17", 0x80, false),
    choice("f18", "F18", 0x81, false),
    choice("f19", "F19", 0x82, false),
    choice("f20", "F20", 0x83, false),
    choice("f21", "F21", 0x84, false),
    choice("f22", "F22", 0x85, false),
    choice("f23", "F23", 0x86, false),
    choice("f24", "F24", 0x87, false),
    choice("pause", "Pause", 0x13, false),
    choice("scroll_lock", "Verr. défil.", 0x91, false),
    choice("menu", "Menu contextuel", 0x5D, false),
    choice("insert", "Inser", 0x2D, false),
    choice("home", "Début", 0x24, false),
    choice("end", "Fin", 0x23, false),
    choice("page_up", "Page préc.", 0x21, false),
];

/// La touche à tenir en plus pour traduire, quand elle n'est pas réglée : Maj, l'une ou l'autre.
pub const BOTH_SHIFTS: &str = "shift";

fn find(id: &str) -> Option<&'static KeyChoice> {
    CHOICES.iter().find(|c| c.id == id)
}

fn key_for(choice: &KeyChoice) -> Key {
    key_of(choice.vk)
}

/// Vérifie une combinaison de dictée et la touche de traduction. Une touche seule doit être Ctrl droit ou une touche qui ne sert pas à
/// taper (touche de fonction…) : Ctrl gauche, Alt ou Maj seuls se déclencheraient à chaque raccourci clavier.
pub fn validate(dictation: &[String], translate: &[String]) -> Result<(), String> {
    if dictation.is_empty() {
        return if translate.is_empty() { Ok(()) } else { validate_translate(&[], translate) };
    }
    if dictation.len() > 3 {
        return Err("trois touches au plus".into());
    }
    let mut chosen = Vec::new();
    for id in dictation {
        let choice = find(id).ok_or_else(|| format!("touche inconnue : {id}"))?;
        if chosen.iter().any(|c: &&KeyChoice| c.id == choice.id) {
            return Err(format!("{} choisie deux fois", choice.label));
        }
        chosen.push(choice);
    }
    if chosen.iter().filter(|c| !c.modifier).count() > 1 {
        return Err("une seule touche autre que Ctrl, Alt ou Maj dans la combinaison".into());
    }
    if let [only] = chosen.as_slice() {
        if only.modifier && only.id != "right_control" {
            return Err(format!("{} seule se déclencherait à chaque raccourci : ajoute une touche (par exemple F9)", only.label));
        }
    }
    validate_translate(&chosen, translate)
}

fn validate_translate(dictation: &[&KeyChoice], translate: &[String]) -> Result<(), String> {
    match translate {
        [] => {
            if dictation.iter().any(|c| c.id == "left_shift" || c.id == "right_shift") {
                return Err("la dictée utilise Maj : choisis une autre touche pour traduire".into());
            }
            Ok(())
        }
        [id] if id == BOTH_SHIFTS => validate_translate(dictation, &[]),
        [id] => {
            let choice = find(id).ok_or_else(|| format!("touche inconnue : {id}"))?;
            if dictation.iter().any(|c| c.id == choice.id) {
                return Err(format!("{} sert déjà à dicter", choice.label));
            }
            Ok(())
        }
        _ => Err("une seule touche pour traduire".into()),
    }
}

/// Les touches de dictée et de traduction d'une combinaison réglée (vérifiée par `validate`), sinon `None`.
pub fn custom_binding(dictation: &[String], translate: &[String]) -> Option<Binding> {
    if dictation.is_empty() || validate(dictation, translate).is_err() {
        return None;
    }
    let keys = dictation.iter().map(|id| find(id).map(key_for)).collect::<Option<Vec<_>>>()?;
    let translate = match translate {
        [] => vec![Key::LeftShift, Key::RightShift],
        [id] if id == BOTH_SHIFTS => vec![Key::LeftShift, Key::RightShift],
        [id] => vec![find(id).map(key_for)?],
        _ => return None,
    };
    Some(Binding { dictation: keys, translate })
}

/// Les touches en vigueur : celles des réglages (Windows) si elles sont valides, sinon celles par défaut du système (la touche de
/// traduction réglée seule, avec la dictée par défaut, compte aussi).
pub fn binding(settings: &AppSettings) -> Binding {
    let default = Binding::from_hotkey(settings.hotkey);
    if !cfg!(windows) || (settings.dictation_keys.is_empty() && settings.translate_keys.is_empty()) {
        return default;
    }
    let dictation = if settings.dictation_keys.is_empty() { vec!["right_control".to_string()] } else { settings.dictation_keys.clone() };
    custom_binding(&dictation, &settings.translate_keys).unwrap_or(default)
}

/// « Ctrl gauche + F9 » : les touches d'une liste d'identifiants, pour l'affichage.
pub fn label(ids: &[String]) -> String {
    ids.iter().map(|id| find(id).map_or(id.as_str(), |c| c.label)).collect::<Vec<_>>().join(" + ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use scriibe_core::hotkey::{HotkeyEvent, HotkeyMonitor};
    use scriibe_core::settings::Hotkey;

    /// Rejoue des événements Windows dans la machine à états du cœur, comme le fait le chef d'orchestre.
    fn run_with(binding: Binding, events: &[(u32, bool)]) -> String {
        let mut held = Vec::new();
        let mut monitor = HotkeyMonitor::with_binding(binding);
        let mut out = String::new();
        for (code, down) in events {
            if let Some(KeyEvent::Changed { changed, down, held }) = translate_vk(&mut held, *code, *down) {
                out.extend(monitor.handle_change(changed, down, &held).into_iter().map(HotkeyEvent::letter));
            }
        }
        out
    }

    fn run(events: &[(u32, bool)]) -> String {
        run_with(Binding::from_hotkey(Hotkey::RightControl), events)
    }

    fn ids(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    const A: u32 = 0x41;
    const LALT: u32 = 0xA4;
    const F9: u32 = 0x78;

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

    #[test]
    fn a_chosen_combination_dictates_and_a_chosen_key_translates() {
        let binding = custom_binding(&ids(&["left_control", "f9"]), &ids(&["left_alt"])).unwrap();
        assert_eq!(run_with(binding.clone(), &[(vk::LCONTROL, true), (F9, true), (F9, false), (vk::LCONTROL, false)]), "PR");
        // Alt gauche en plus : traduction (et non plus un raccourci qui annule).
        assert_eq!(run_with(binding.clone(), &[(vk::LCONTROL, true), (F9, true), (LALT, true), (F9, false)]), "PCR");
        // Ctrl gauche + C reste un raccourci ordinaire.
        assert_eq!(run_with(binding, &[(vk::LCONTROL, true), (0x43, true), (0x43, false), (vk::LCONTROL, false)]), "");
    }

    #[test]
    fn combinations_are_checked() {
        assert!(validate(&[], &[]).is_ok());
        assert!(validate(&ids(&["right_control"]), &[]).is_ok());
        assert!(validate(&ids(&["f9"]), &[]).is_ok());
        assert!(validate(&ids(&["left_control", "f9"]), &ids(&["left_shift"])).is_ok());
        assert!(validate(&ids(&["left_control", "left_alt"]), &[]).is_ok());
        // Seules, ces touches se déclencheraient à chaque raccourci.
        for id in ["left_control", "left_alt", "left_shift", "right_shift"] {
            assert!(validate(&ids(&[id]), &[]).unwrap_err().contains("seule"), "{id}");
        }
        assert!(validate(&ids(&["f9", "f10"]), &[]).unwrap_err().contains("une seule touche"));
        assert!(validate(&ids(&["a"]), &[]).unwrap_err().contains("inconnue"));
        assert!(validate(&ids(&["left_control", "left_control"]), &[]).unwrap_err().contains("deux fois"));
        assert!(validate(&ids(&["left_control", "f1", "left_alt", "f2"]), &[]).is_err());
        // La traduction ne réutilise pas une touche de dictée, ni Maj si Maj dicte.
        assert!(validate(&ids(&["f9"]), &ids(&["f9"])).unwrap_err().contains("dicter"));
        assert!(validate(&ids(&["left_shift", "f9"]), &[]).unwrap_err().contains("Maj"));
        assert!(validate(&ids(&["f9"]), &ids(&["f1", "f2"])).is_err());
        // Réglée seule, la traduction est vérifiée contre la dictée par défaut (Ctrl droit).
        assert!(validate(&[], &ids(&["left_alt"])).is_ok());
    }

    #[test]
    fn the_effective_binding_falls_back_to_the_default_when_the_settings_are_invalid() {
        let mut settings = AppSettings { hotkey: Hotkey::RightControl, ..AppSettings::default() };
        assert_eq!(binding(&settings), Binding::from_hotkey(Hotkey::RightControl));
        settings.dictation_keys = ids(&["nimporte-quoi"]);
        assert_eq!(binding(&settings), Binding::from_hotkey(Hotkey::RightControl));
        settings.dictation_keys = ids(&["left_control", "f9"]);
        let effective = binding(&settings);
        // Sur Windows la combinaison s'applique ; ailleurs, la touche par défaut du système.
        assert_eq!(effective == Binding::from_hotkey(Hotkey::RightControl), !cfg!(windows));
    }

    #[test]
    fn labels_are_readable() {
        assert_eq!(label(&ids(&["left_control", "f9"])), "Ctrl gauche + F9");
    }

    #[test]
    fn every_choice_has_a_unique_id_and_code() {
        for (i, a) in CHOICES.iter().enumerate() {
            for b in CHOICES.iter().skip(i + 1) {
                assert!(a.id != b.id && a.vk != b.vk, "{} / {}", a.id, b.id);
            }
        }
    }
}
