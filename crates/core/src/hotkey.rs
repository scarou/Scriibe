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
        if self == Hotkey::RightControl {
            return held.contains(&Key::LeftShift) || held.contains(&Key::RightShift);
        }
        held.contains(&Key::LeftCommand) || (self != Hotkey::RightCommand && held.contains(&Key::RightCommand))
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
    pub hotkey: Hotkey,
    is_down: bool,
}

impl HotkeyMonitor {
    pub fn new(hotkey: Hotkey) -> Self {
        Self { hotkey, is_down: false }
    }

    /// Le système a changé l'état de la touche de modification `changed`; `held` : toutes celles qui sont tenues maintenant.
    pub fn handle(&mut self, changed: Key, held: &[Key]) -> Vec<HotkeyEvent> {
        let key = self.hotkey;
        let mut events = Vec::new();
        if changed != key.key() {
            if self.is_down && key.command_held(held) {
                events.push(HotkeyEvent::Command);
            }
            return events;
        }
        let down = key.is_down(held);
        if down && !self.is_down {
            self.is_down = true;
            events.push(HotkeyEvent::Press);
            if key.command_held(held) {
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

    #[test]
    fn defaults_follow_the_system() {
        assert!(Hotkey::system_default().is_available());
        assert_eq!(Hotkey::RightControl.key(), Key::RightControl);
        assert_eq!(Key::from_name("right_control"), Some(Key::RightControl));
    }
}
