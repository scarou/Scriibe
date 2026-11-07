//! Raccourcis clavier d'appli écrits en texte (« Mod+B », « ⌘I », « Ctrl+Maj+U ») : lus ici, envoyés par `crates/platform`.
//! Servent aux commandes vocales gras, italique et souligné (`docs/COMMANDES.md`) : Scriibe n'écrit pas de Markdown, il appuie sur le raccourci
//! de l'appli qui a la sélection.

use std::fmt;

/// Une combinaison : des modificateurs et une lettre ou un chiffre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shortcut {
    /// ⌘ (Mac).
    pub command: bool,
    pub control: bool,
    /// Option sur Mac, Alt ailleurs.
    pub alt: bool,
    pub shift: bool,
    /// Minuscule ou chiffre : `a`–`z`, `0`–`9`.
    pub key: char,
}

impl Shortcut {
    /// Lit un raccourci : modificateurs et touche séparés par `+` (espaces ignorés), dans n'importe quel ordre, sans tenir compte des
    /// majuscules. `Mod` (ou `Cmd`, `⌘`) est ⌘ sur Mac et Ctrl ailleurs ; `Ctrl`, `Alt`, `Option`, `Maj`, `Shift`, `⌃`, `⌥`, `⇧` aussi.
    /// Refuse : pas de ⌘ ni de Ctrl (une lettre seule, ou Option + lettre, s'écrirait dans le texte), pas de touche, deux touches, touche
    /// autre qu'une lettre ou un chiffre, et ⌘Q (Ctrl+Q hors Mac) sous toutes ses formes : il quitte l'appli (avec Maj, il ferme la session).
    pub fn parse(text: &str) -> Option<Shortcut> {
        Self::parse_for(text, cfg!(target_os = "macos"))
    }

    /// Comme [`parse`](Self::parse), pour un système donné (`mac` : `Mod` veut dire ⌘).
    pub fn parse_for(text: &str, mac: bool) -> Option<Shortcut> {
        let mut out = Shortcut { command: false, control: false, alt: false, shift: false, key: ' ' };
        let mut key: Option<char> = None;
        // Les symboles ⌘⌃⌥⇧ peuvent être collés à la lettre (« ⌘B ») : on les sépare d'abord.
        let spaced: String = text.chars().flat_map(|c| if "⌘⌃⌥⇧".contains(c) { vec!['+', c, '+'] } else { vec![c] }).collect();
        for part in spaced.split('+').map(str::trim).filter(|p| !p.is_empty()) {
            match part.to_lowercase().as_str() {
                "mod" if mac => out.command = true,
                "mod" => out.control = true,
                "cmd" | "command" | "⌘" | "meta" => out.command = true,
                "ctrl" | "control" | "ctl" | "⌃" => out.control = true,
                "alt" | "option" | "opt" | "⌥" => out.alt = true,
                "shift" | "maj" | "majuscule" | "⇧" => out.shift = true,
                other => {
                    let mut chars = other.chars();
                    let (Some(c), None) = (chars.next(), chars.next()) else { return None };
                    if key.is_some() || !(c.is_ascii_lowercase() || c.is_ascii_digit()) {
                        return None;
                    }
                    key = Some(c);
                }
            }
        }
        out.key = key?;
        let quits = out.key == 'q' && (out.command || (!mac && out.control));
        ((out.command || out.control) && !quits).then_some(out)
    }
}

impl fmt::Display for Shortcut {
    /// Sous la forme montrée à la personne : « ⌘B » sur Mac, « Ctrl+B » ailleurs.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mac = cfg!(target_os = "macos");
        let key = self.key.to_ascii_uppercase();
        if mac {
            write!(
                f,
                "{}{}{}{}{key}",
                if self.control { "⌃" } else { "" },
                if self.alt { "⌥" } else { "" },
                if self.shift { "⇧" } else { "" },
                if self.command { "⌘" } else { "" }
            )
        } else {
            let mut parts: Vec<String> = Vec::new();
            if self.control || self.command {
                parts.push("Ctrl".into());
            }
            if self.alt {
                parts.push("Alt".into());
            }
            if self.shift {
                parts.push("Maj".into());
            }
            parts.push(key.to_string());
            write!(f, "{}", parts.join("+"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sc(text: &str, mac: bool) -> Option<Shortcut> {
        Shortcut::parse_for(text, mac)
    }

    #[test]
    fn mod_is_command_on_mac_and_control_elsewhere() {
        let mac = sc("Mod+B", true).unwrap();
        assert!(mac.command && !mac.control && mac.key == 'b');
        let other = sc("Mod+B", false).unwrap();
        assert!(!other.command && other.control && other.key == 'b');
    }

    #[test]
    fn symbols_names_order_and_case_are_accepted() {
        let a = sc("⌘B", true).unwrap();
        assert!(a.command && a.key == 'b' && !a.shift);
        let b = sc(" ctrl + SHIFT + u ", false).unwrap();
        assert!(b.control && b.shift && b.key == 'u');
        assert_eq!(sc("U+Ctrl", false), sc("Ctrl+U", false));
        assert!(sc("⌥⇧⌘4", true).is_some_and(|s| s.alt && s.shift && s.command && s.key == '4'));
        assert!(sc("Cmd+I", false).is_some_and(|s| s.command));
    }

    #[test]
    fn option_alone_and_quit_are_refused_but_close_tab_and_option_with_command_are_not() {
        assert_eq!(sc("Ctrl+Q", false), None);
        assert!(sc("Ctrl+W", false).is_some() && sc("Mod+W", true).is_some(), "fermer l'onglet reste un raccourci possible");
        assert!(sc("Mod+Alt+B", true).is_some());
        assert!(sc("Ctrl+Q", true).is_some(), "sur Mac, Ctrl+Q n'est pas « quitter »");
    }

    #[test]
    fn refuses_what_cannot_be_sent_safely() {
        for bad in [
            "",
            "B",
            "Mod",
            "Mod+",
            "Mod+B+I",
            "Mod+Entrée",
            "Shift+B",
            "Mod+é",
            "Mod++",
            "Truc+B",
            "Alt+B",
            "Mod+Q",
            "Mod+Shift+Q",
            "Ctrl+Mod+Q",
        ] {
            assert_eq!(sc(bad, true), None, "{bad}");
        }
    }
}
