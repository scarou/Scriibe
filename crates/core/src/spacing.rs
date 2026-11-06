//! Espace entre deux dictées (S17, propre à Scriibe ; Scribe n'a pas cette fonction, voir `docs/ESPACEMENT.md`).
//!
//! Deux dictées collées à la suite dans la même appli se touchaient (« Bonjour.Comment ça va ? »). Scriibe ne lit pas le curseur de
//! l'appli cible : il retient la dernière dictée collée et, si la suivante arrive dans la même appli peu après, ajoute une espace
//! devant, sauf si la précédente finissait par une espace, un saut de ligne ou un signe qui s'attache au mot suivant. Un modèle
//! Whisper qui n'écrit pas de point final (essai de Christophe avec `ggml-small`) ne colle donc pas les phrases. Fonctions pures :
//! le chef d'orchestre garde la mémoire et l'horloge.

use std::time::Duration;

/// Au-delà de ce délai depuis le dernier collage, on ne suppose plus que le curseur est resté au même endroit.
pub const MAX_GAP: Duration = Duration::from_secs(60);

/// Les derniers signes après lesquels la dictée suivante se colle sans espace : ouvrants (parenthèse, guillemet ouvrant, point d'interrogation
/// ou d'exclamation espagnol) et signes qui relient (tiret, barre oblique, arobase, dièse). Tout autre signe, lettre ou chiffre appelle une espace
/// (les guillemets droits ne figurent pas ici : ils peuvent fermer une phrase).
const JOINING_ENDINGS: &str = "([{«“‘¿¡-–—/@#";

/// Ce qui, au début d'une dictée, se colle au mot précédent : pas d'espace devant.
const ATTACHED_START: &str = ",.;:!?…)]}»%";

/// Ce qu'on retient du dernier collage réussi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastPaste {
    /// Identifiant de l'appli active au moment du collage.
    pub app_id: String,
    /// Heure du chef d'orchestre (son horloge) à la fin du collage.
    pub at: Duration,
    /// Dernier caractère du texte collé (sans l'espace ajouté).
    pub last_char: char,
}

/// Ce qu'il faut retenir après un collage ; `None` si l'appli est inconnue ou le texte vide.
pub fn remember(app_id: Option<&str>, at: Duration, pasted: &str) -> Option<LastPaste> {
    let app_id = app_id.filter(|id| !id.is_empty())?;
    let last_char = pasted.chars().last()?;
    Some(LastPaste { app_id: app_id.to_string(), at, last_char })
}

/// Faut-il une espace devant `text` ? Les six conditions de `docs/ESPACEMENT.md`.
pub fn needs_leading_space(last: Option<&LastPaste>, app_id: Option<&str>, now: Duration, text: &str, replaces_selection: bool) -> bool {
    let Some(last) = last else { return false };
    let Some(app_id) = app_id.filter(|id| !id.is_empty()) else { return false };
    if replaces_selection || last.app_id != app_id {
        return false;
    }
    if now.saturating_sub(last.at) > MAX_GAP || last.last_char.is_whitespace() || JOINING_ENDINGS.contains(last.last_char) {
        return false;
    }
    match text.chars().next() {
        None => false,
        Some(first) => !first.is_whitespace() && !ATTACHED_START.contains(first),
    }
}

/// Le texte à envoyer au collage : `text` précédé d'une espace si la règle le demande. `text` lui-même (historique) ne change pas.
pub fn for_paste(last: Option<&LastPaste>, app_id: Option<&str>, now: Duration, text: &str, replaces_selection: bool) -> String {
    if needs_leading_space(last, app_id, now, text, replaces_selection) {
        format!(" {text}")
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const APP: Option<&str> = Some("notepad.exe");

    fn after(text: &str, secs: u64) -> Option<LastPaste> {
        remember(APP, Duration::from_secs(secs), text)
    }

    fn spaced(last: Option<&LastPaste>, now: u64, text: &str) -> String {
        for_paste(last, APP, Duration::from_secs(now), text, false)
    }

    #[test]
    fn case_1_first_paste_gets_no_space() {
        assert_eq!(spaced(None, 10, "Bonjour."), "Bonjour.");
    }

    #[test]
    fn the_normal_case_adds_one_space_after_each_sentence_ending() {
        for end in ['.', '!', '?', '…', ':', ';'] {
            let last = after(&format!("Fini{end}"), 10);
            assert_eq!(spaced(last.as_ref(), 20, "Suite."), " Suite.", "après « {end} »");
        }
    }

    #[test]
    fn a_dictation_without_a_final_stop_still_gets_a_space() {
        // Whisper « small » n'écrit pas toujours le point final : les phrases ne doivent pas se coller.
        assert_eq!(spaced(after("Bonjour tout le monde", 10).as_ref(), 20, "Suite"), " Suite");
        assert_eq!(spaced(after("Bonjour,", 10).as_ref(), 20, "Suite."), " Suite.");
        assert_eq!(spaced(after("Deux fois 12", 10).as_ref(), 20, "Suite."), " Suite.");
        assert_eq!(spaced(after("Il a dit \"Fini.\"", 10).as_ref(), 20, "Suite."), " Suite.");
        assert_eq!(spaced(after("Il a dit « fini. »", 10).as_ref(), 20, "Suite."), " Suite.");
    }

    #[test]
    fn a_previous_text_ending_with_an_opening_or_joining_sign_gets_no_space() {
        for end in ['(', '[', '{', '«', '“', '‘', '¿', '¡', '-', '–', '—', '/', '@', '#'] {
            let last = after(&format!("Voici {end}"), 10);
            assert_eq!(spaced(last.as_ref(), 20, "suite"), "suite", "après « {end} »");
        }
    }

    #[test]
    fn case_2_previous_text_already_ending_with_a_space_or_new_text_starting_with_one() {
        assert_eq!(spaced(after("Bonjour. ", 10).as_ref(), 20, "Suite."), "Suite.");
        assert_eq!(spaced(after("Bonjour.\u{00A0}", 10).as_ref(), 20, "Suite."), "Suite.");
        assert_eq!(spaced(after("Bonjour.", 10).as_ref(), 20, " Suite."), " Suite.");
        assert_eq!(spaced(after("Bonjour.", 10).as_ref(), 20, "\u{00A0}Suite."), "\u{00A0}Suite.");
    }

    #[test]
    fn case_3_previous_text_ending_with_a_line_break() {
        assert_eq!(spaced(after("- un\n", 10).as_ref(), 20, "Suite."), "Suite.");
        assert_eq!(spaced(after("Bonjour.\r\n", 10).as_ref(), 20, "Suite."), "Suite.");
        assert_eq!(spaced(after("Bonjour.", 10).as_ref(), 20, "\nSuite."), "\nSuite.");
    }

    #[test]
    fn case_4_another_app_gets_no_space() {
        let last = after("Bonjour.", 10);
        assert_eq!(for_paste(last.as_ref(), Some("chrome.exe"), Duration::from_secs(20), "Suite.", false), "Suite.");
        assert_eq!(for_paste(last.as_ref(), None, Duration::from_secs(20), "Suite.", false), "Suite.");
        assert_eq!(for_paste(last.as_ref(), Some(""), Duration::from_secs(20), "Suite.", false), "Suite.");
        assert!(remember(None, Duration::ZERO, "Bonjour.").is_none(), "appli inconnue : rien n'est retenu");
    }

    #[test]
    fn case_5_a_long_delay_gets_no_space() {
        let last = after("Bonjour.", 10);
        assert_eq!(spaced(last.as_ref(), 70, "Suite."), " Suite.", "exactement 60 s : encore dans le délai");
        assert_eq!(spaced(last.as_ref(), 71, "Suite."), "Suite.", "61 s : trop tard");
        assert_eq!(spaced(last.as_ref(), 600, "Suite."), "Suite.");
    }

    #[test]
    fn case_6_a_phrase_starting_with_punctuation_gets_no_space() {
        let last = after("Bonjour.", 10);
        for text in [
            ", et voilà.",
            ". Voilà.",
            "; bref.",
            ": suite.",
            "! Oui.",
            "? Non.",
            "… et puis.",
            ") fin.",
            "] fin.",
            "} fin.",
            "» dit-il.",
            "% de plus.",
        ] {
            assert_eq!(spaced(last.as_ref(), 20, text), text, "« {text} »");
        }
        // Une ouverture (guillemet, parenthèse) ne se colle pas au mot précédent : elle garde son espace.
        assert_eq!(spaced(last.as_ref(), 20, "« Oui »."), " « Oui ».");
        assert_eq!(spaced(last.as_ref(), 20, "(Suite)."), " (Suite).");
    }

    #[test]
    fn case_7_a_translation_into_english_follows_the_same_rule() {
        let last = after("Hello everyone, let's get started.", 10);
        assert_eq!(spaced(last.as_ref(), 25, "Thanks for coming."), " Thanks for coming.");
        assert_eq!(spaced(last.as_ref(), 25, "'Tis fine."), " 'Tis fine.");
    }

    #[test]
    fn text_replacing_a_selection_gets_no_space() {
        let last = after("Bonjour.", 10);
        assert_eq!(for_paste(last.as_ref(), APP, Duration::from_secs(20), "suite", true), "suite");
    }

    #[test]
    fn an_empty_text_gets_no_space() {
        assert_eq!(spaced(after("Bonjour.", 10).as_ref(), 20, ""), "");
    }
}
