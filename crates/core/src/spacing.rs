//! Espace entre deux dictées (S17, refaite en S20 ; propre à Scriibe, Scribe n'a pas cette fonction, voir `docs/ESPACEMENT.md`).
//!
//! Deux dictées collées à la suite se touchaient (« Bonjour.Comment ça va ? »). Scriibe ne lit pas le curseur de l'appli cible et, sous
//! Linux (Wayland), ne sait même pas quelle appli est devant. Règle unique, la même sur les trois systèmes : chaque dictée collée est
//! **suivie** d'une espace, sauf si elle finit déjà par une espace ou un saut de ligne, ou si elle remplace une sélection (elle s'écrit
//! alors au milieu d'une phrase : l'espace qui suit est déjà dans le texte). Rien n'est donc ajouté devant la dictée suivante (pas de
//! double espace). Fonction pure : le texte de l'historique ne change pas.

/// Le texte à envoyer au collage : `text` suivi d'une espace si la règle le demande.
pub fn for_paste(text: &str, replaces_selection: bool) -> String {
    match text.chars().last() {
        Some(last) if !replaces_selection && !last.is_whitespace() => format!("{text} "),
        _ => text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dictation_gets_a_trailing_space() {
        assert_eq!(for_paste("Bonjour, ceci est un essai.", false), "Bonjour, ceci est un essai. ");
    }

    #[test]
    fn a_dictation_without_a_final_stop_still_gets_a_trailing_space() {
        // Whisper « small » n'écrit pas toujours de point final.
        assert_eq!(for_paste("Bonjour tout le monde on commence", false), "Bonjour tout le monde on commence ");
    }

    #[test]
    fn every_kind_of_last_sign_gets_the_space() {
        for text in ["fin.", "fin !", "fin ?", "fin…", "fin :", "fin)", "fin»", "fin", "fin 12", "fin-"] {
            assert_eq!(for_paste(text, false), format!("{text} "), "{text}");
        }
    }

    #[test]
    fn a_dictation_ending_with_a_line_break_gets_no_space() {
        assert_eq!(for_paste("Bonjour.\n", false), "Bonjour.\n");
        assert_eq!(for_paste("Bonjour.\r\n", false), "Bonjour.\r\n");
        assert_eq!(for_paste("Bonjour.\n\n", false), "Bonjour.\n\n");
    }

    #[test]
    fn a_dictation_already_ending_with_a_space_gets_no_second_one() {
        assert_eq!(for_paste("Bonjour. ", false), "Bonjour. ");
        assert_eq!(for_paste("Bonjour.\u{a0}", false), "Bonjour.\u{a0}");
    }

    #[test]
    fn a_selection_replacement_gets_no_space() {
        assert_eq!(for_paste("bonjour tout le monde", true), "bonjour tout le monde");
    }

    #[test]
    fn an_empty_text_stays_empty() {
        assert_eq!(for_paste("", false), "");
    }

    #[test]
    fn two_dictations_in_a_row_never_make_a_double_space() {
        let first = for_paste("Bonjour.", false);
        let second = for_paste("Comment ça va ?", false);
        assert_eq!(format!("{first}{second}"), "Bonjour. Comment ça va ? ");
        assert!(!format!("{first}{second}").contains("  "));
    }

    #[test]
    fn a_dictation_never_starts_with_an_added_space() {
        for text in ["Bonjour", ", la suite", ".", "(entre parenthèses)"] {
            assert!(!for_paste(text, false).starts_with(' '), "{text}");
        }
    }
}
