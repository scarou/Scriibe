//! Touche de dictée : machine à états (appui, relâchement, ⌘ ajoutée, autre touche tapée). Port de `HotkeyMonitor.handle` (Scribe).
//!
//! Le cœur ne lit pas le clavier : chaque système lui envoie « telle touche de modification a changé, voici celles qui sont tenues ».
//! La machine à états est la même partout ; ce qui change d'un système à l'autre, ce sont les touches (voir `docs/ECARTS.md`).

use crate::settings::Hotkey;

/// Une touche de modification physique (gauche et droite distinguées).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    RightOption,
    LeftOption,
    RightCommand,
    LeftCommand,
    Fn,
    /// Windows et Linux : la touche Ctrl de droite (distincte de AltGr, que Windows émet comme Ctrl gauche + Alt droite).
    RightControl,
    LeftControl,
    LeftShift,
    RightShift,
    /// Toute autre touche, par son code du système (Alt gauche, touches de fonction, Pause…) : une touche de dictée réglée à la main, ou une
    /// touche ordinaire tapée pendant l'appui.
    Vk(u32),
}

impl Key {
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "right_option" => Self::RightOption,
            "left_option" => Self::LeftOption,
            "right_command" => Self::RightCommand,
            "left_command" => Self::LeftCommand,
            "fn" => Self::Fn,
            "right_control" => Self::RightControl,
            "left_control" => Self::LeftControl,
            "left_shift" => Self::LeftShift,
            "right_shift" => Self::RightShift,
            _ => return None,
        })
    }
}

impl Hotkey {
    /// La touche physique qui dicte.
    pub fn key(self) -> Key {
        match self {
            Hotkey::RightOption => Key::RightOption,
            Hotkey::LeftOption => Key::LeftOption,
            Hotkey::RightCommand => Key::RightCommand,
            Hotkey::Fn => Key::Fn,
            Hotkey::RightControl => Key::RightControl,
        }
    }

    /// La touche par défaut du système (écart voulu avec Scribe : Option droite n'existe pas sous Windows, voir `docs/ECARTS.md`).
    pub fn system_default() -> Self {
        if cfg!(target_os = "macos") {
            Hotkey::RightOption
        } else {
            Hotkey::RightControl
        }
    }

    /// Les touches de la machine à états qui existent sur ce système (Option, Commande et Fn sont propres au Mac).
    pub fn is_available(self) -> bool {
        match self {
            Hotkey::RightControl => !cfg!(target_os = "macos"),
            _ => cfg!(target_os = "macos"),
        }
    }

    pub fn is_down(self, held: &[Key]) -> bool {
        held.contains(&self.key())
    }

    /// Une touche de traduction tenue en plus de la touche de dictée : on traduit. Sur Mac, ⌘ (la gauche seulement si la dictée est
    /// sur ⌘ droite) ; sous Windows et Linux, Maj (l'une ou l'autre).
    pub fn command_held(self, held: &[Key]) -> bool {
        Binding::from_hotkey(self).translate_held(held)
    }
}

/// Les touches de dictée et de traduction en vigueur. Par défaut celles du système (`from_hotkey`) ; réglables à la main sous Windows
/// (une touche ou une combinaison, par exemple Ctrl gauche + F9).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    /// Toutes ces touches tenues ensemble : la dictée.
    pub dictation: Vec<Key>,
    /// L'une de ces touches tenue en plus : la traduction.
    pub translate: Vec<Key>,
    /// Millisecondes à tenir la combinaison avant que l'écoute commence (0 : tout de suite). Pour une combinaison de touches de
    /// modification seules (Maj gauche, Ctrl + Alt…), que les raccourcis clavier appuient aussi : un appui bref n'ouvre pas le micro.
    pub hold_ms: u32,
}

impl Binding {
    pub fn from_hotkey(hotkey: Hotkey) -> Self {
        let translate = if hotkey == Hotkey::RightControl {
            vec![Key::LeftShift, Key::RightShift]
        } else if hotkey == Hotkey::RightCommand {
            vec![Key::LeftCommand]
        } else {
            vec![Key::LeftCommand, Key::RightCommand]
        };
        Self { dictation: vec![hotkey.key()], translate, hold_ms: 0 }
    }

    pub fn is_down(&self, held: &[Key]) -> bool {
        !self.dictation.is_empty() && self.dictation.iter().all(|key| held.contains(key))
    }

    pub fn translate_held(&self, held: &[Key]) -> bool {
        self.translate.iter().any(|key| held.contains(key))
    }
}

/// Ce que la machine à états annonce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyEvent {
    /// La touche de dictée est appuyée.
    Press,
    /// La touche de dictée est relâchée.
    Release,
    /// ⌘ tenue pendant l'appui (avant ou après la touche de dictée) ; peut arriver plusieurs fois par dictée.
    Command,
    /// Une autre touche tapée pendant l'appui : c'est un raccourci clavier, pas une dictée.
    OtherKey,
}

impl HotkeyEvent {
    /// Lettre du journal des tests : P, R, C, X.
    pub fn letter(self) -> char {
        match self {
            Self::Press => 'P',
            Self::Release => 'R',
            Self::Command => 'C',
            Self::OtherKey => 'X',
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct HotkeyMonitor {
    pub binding: Binding,
    is_down: bool,
}

impl Default for Binding {
    fn default() -> Self {
        Self::from_hotkey(Hotkey::default())
    }
}

impl HotkeyMonitor {
    pub fn new(hotkey: Hotkey) -> Self {
        Self::with_binding(Binding::from_hotkey(hotkey))
    }

    pub fn with_binding(binding: Binding) -> Self {
        Self { binding, is_down: false }
    }

    /// Une touche (quelconque) a changé d'état ; `held` : toutes celles qui sont tenues maintenant. Une touche qui n'est ni de dictée ni de
    /// traduction et qui n'est pas une touche de modification Ctrl ou Maj est une frappe ordinaire : un raccourci, qui annule la dictée.
    pub fn handle_change(&mut self, changed: Key, down: bool, held: &[Key]) -> Vec<HotkeyEvent> {
        let known = self.binding.dictation.contains(&changed) || self.binding.translate.contains(&changed);
        if matches!(changed, Key::Vk(_)) && !known {
            return if down { self.handle_key_down() } else { Vec::new() };
        }
        self.handle(changed, held)
    }

    /// Le système a changé l'état de la touche de modification `changed`; `held` : toutes celles qui sont tenues maintenant.
    pub fn handle(&mut self, changed: Key, held: &[Key]) -> Vec<HotkeyEvent> {
        let mut events = Vec::new();
        if !self.binding.dictation.contains(&changed) {
            if self.is_down && self.binding.translate_held(held) {
                events.push(HotkeyEvent::Command);
            }
            return events;
        }
        let down = self.binding.is_down(held);
        if down && !self.is_down {
            self.is_down = true;
            events.push(HotkeyEvent::Press);
            if self.binding.translate_held(held) {
                events.push(HotkeyEvent::Command);
            }
        } else if !down && self.is_down {
            self.is_down = false;
            events.push(HotkeyEvent::Release);
        }
        events
    }

    /// Une touche ordinaire a été tapée.
    pub fn handle_key_down(&mut self) -> Vec<HotkeyEvent> {
        if self.is_down {
            vec![HotkeyEvent::OtherKey]
        } else {
            Vec::new()
        }
    }

    /// La combinaison de dictée est tenue en ce moment.
    pub fn is_down(&self) -> bool {
        self.is_down
    }

    /// Remet la machine à zéro (changement de touche, arrêt de la surveillance).
    pub fn reset(&mut self) {
        self.is_down = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(hotkey: Hotkey, steps: &[(Key, &[Key])]) -> String {
        let mut monitor = HotkeyMonitor::new(hotkey);
        steps.iter().flat_map(|(key, held)| monitor.handle(*key, held)).map(HotkeyEvent::letter).collect()
    }

    #[test]
    fn right_control_dictates() {
        use Key::*;
        assert_eq!(run(Hotkey::RightControl, &[(RightControl, &[RightControl]), (RightControl, &[])]), "PR");
    }

    #[test]
    fn shift_translates_with_right_control_either_side() {
        use Key::*;
        // Maj après Ctrl.
        let steps: [(Key, &[Key]); 3] =
            [(RightControl, &[RightControl]), (LeftShift, &[RightControl, LeftShift]), (RightControl, &[LeftShift])];
        assert_eq!(run(Hotkey::RightControl, &steps), "PCR");
        // Maj avant Ctrl : la traduction est annoncée dès l'appui. Maj seule ne dicte pas.
        let steps: [(Key, &[Key]); 3] =
            [(RightShift, &[RightShift]), (RightControl, &[RightShift, RightControl]), (RightControl, &[RightShift])];
        assert_eq!(run(Hotkey::RightControl, &steps), "PCR");
        assert_eq!(run(Hotkey::RightControl, &[(LeftShift, &[LeftShift]), (LeftShift, &[])]), "");
    }

    #[test]
    fn ctrl_gauche_from_altgr_does_not_dictate() {
        use Key::*;
        // AltGr arrive comme Ctrl gauche (fictif) + Alt droite : Ctrl gauche ne déclenche rien.
        assert_eq!(run(Hotkey::RightControl, &[(LeftControl, &[LeftControl]), (LeftControl, &[])]), "");
    }

    fn run_changes(binding: Binding, steps: &[(Key, bool, &[Key])]) -> String {
        let mut monitor = HotkeyMonitor::with_binding(binding);
        steps.iter().flat_map(|(key, down, held)| monitor.handle_change(*key, *down, held)).map(HotkeyEvent::letter).collect()
    }

    const F9: Key = Key::Vk(0x78);
    const LEFT_ALT: Key = Key::Vk(0xA4);
    const A: Key = Key::Vk(0x41);

    fn control_f9() -> Binding {
        Binding { dictation: vec![Key::LeftControl, F9], translate: vec![Key::LeftShift, Key::RightShift], hold_ms: 0 }
    }

    #[test]
    fn a_combination_dictates_while_all_its_keys_are_held_in_any_order() {
        use Key::*;
        // Ctrl gauche puis F9, relâchés dans l'ordre inverse.
        let steps: [(Key, bool, &[Key]); 4] =
            [(LeftControl, true, &[LeftControl]), (F9, true, &[LeftControl, F9]), (LeftControl, false, &[F9]), (F9, false, &[])];
        assert_eq!(run_changes(control_f9(), &steps), "PR");
        // F9 puis Ctrl gauche.
        let steps: [(Key, bool, &[Key]); 4] =
            [(F9, true, &[F9]), (LeftControl, true, &[F9, LeftControl]), (F9, false, &[LeftControl]), (LeftControl, false, &[])];
        assert_eq!(run_changes(control_f9(), &steps), "PR");
    }

    #[test]
    fn a_part_of_the_combination_alone_does_nothing_and_ctrl_shortcuts_are_untouched() {
        use Key::*;
        assert_eq!(run_changes(control_f9(), &[(LeftControl, true, &[LeftControl]), (LeftControl, false, &[])]), "");
        assert_eq!(run_changes(control_f9(), &[(F9, true, &[F9]), (F9, false, &[])]), "");
        // Ctrl gauche + C : un raccourci, jamais une dictée.
        assert_eq!(run_changes(control_f9(), &[(LeftControl, true, &[LeftControl]), (A, true, &[LeftControl, A])]), "");
    }

    #[test]
    fn another_key_during_a_combination_cancels_and_the_translation_key_adds_the_command() {
        use Key::*;
        let steps: [(Key, bool, &[Key]); 4] = [
            (LeftControl, true, &[LeftControl]),
            (F9, true, &[LeftControl, F9]),
            (A, true, &[LeftControl, F9, A]),
            (F9, false, &[LeftControl, A]),
        ];
        assert_eq!(run_changes(control_f9(), &steps), "PXR");
        let steps: [(Key, bool, &[Key]); 3] =
            [(LeftControl, true, &[LeftControl]), (F9, true, &[LeftControl, F9]), (LeftShift, true, &[LeftControl, F9, LeftShift])];
        assert_eq!(run_changes(control_f9(), &steps), "PC");
    }

    #[test]
    fn a_single_non_modifier_key_can_dictate_and_alt_is_a_normal_key_otherwise() {
        use Key::*;
        let alt = Binding { dictation: vec![LEFT_ALT], translate: vec![LeftShift], hold_ms: 0 };
        assert_eq!(run_changes(alt.clone(), &[(LEFT_ALT, true, &[LEFT_ALT]), (LEFT_ALT, false, &[])]), "PR");
        // Alt gauche n'est pas la touche de dictée : avec Ctrl droit tenu, c'est un raccourci qui annule.
        let steps: [(Key, bool, &[Key]); 2] = [(RightControl, true, &[RightControl]), (LEFT_ALT, true, &[RightControl, LEFT_ALT])];
        assert_eq!(run_changes(Binding::from_hotkey(Hotkey::RightControl), &steps), "PX");
    }

    #[test]
    fn defaults_follow_the_system() {
        assert!(Hotkey::system_default().is_available());
        assert_eq!(Hotkey::RightControl.key(), Key::RightControl);
        assert_eq!(Key::from_name("right_control"), Some(Key::RightControl));
    }
}
