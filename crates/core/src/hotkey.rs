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
}

impl Key {
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "right_option" => Self::RightOption,
            "left_option" => Self::LeftOption,
            "right_command" => Self::RightCommand,
            "left_command" => Self::LeftCommand,
            "fn" => Self::Fn,
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
        }
    }

    pub fn is_down(self, held: &[Key]) -> bool {
        held.contains(&self.key())
    }

    /// Une touche Commande tenue en plus de la touche de dictée : on traduit (⌘ gauche seulement si la dictée est sur ⌘ droite).
    pub fn command_held(self, held: &[Key]) -> bool {
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
