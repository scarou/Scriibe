//! Texte dicté par-dessus une sélection (S14, propre à Scriibe) : il remplace un mot ou un groupe de mots au milieu d'une phrase, donc
//! sans majuscule au début ni point à la fin. Les noms propres du dictionnaire, les sigles et les mots comme « iPhone » gardent leur
//! majuscule ; les autres noms propres (« Paris ») perdent la leur, c'est la limite de la règle.

use crate::dictionary::UserDictionary;

/// Ce qui peut précéder le premier mot sans l'empêcher d'être « le début » : guillemets, parenthèse, espaces.
const OPENERS: &str = "«“\"'‘(¿¡ \u{00A0}\u{202F}";

/// Le texte tel qu'il s'écrit au milieu d'une phrase : première lettre en minuscule (sauf nom du dictionnaire, sigle, « I », « iPhone »…)
/// et point final retiré (« … » et « ... » restent, ainsi que `?` et `!`).
pub fn inside_a_sentence(text: &str, dictionary: &UserDictionary) -> String {
    let without_period = strip_final_period(text);
    lower_first_word(&without_period, dictionary)
}

fn strip_final_period(text: &str) -> String {
    let body = text.trim_end();
    let tail = &text[body.len()..];
    match body.strip_suffix('.') {
        Some(rest) if !rest.ends_with('.') && !rest.is_empty() => format!("{rest}{tail}"),
        _ => text.to_string(),
    }
}

fn lower_first_word(text: &str, dictionary: &UserDictionary) -> String {
    let start = match text.char_indices().find(|(_, c)| !OPENERS.contains(*c)) {
        Some((i, _)) => i,
        None => return text.to_string(),
    };
    let rest = &text[start..];
    let first = rest.chars().next().expect("trouvé juste avant");
    if !first.is_uppercase() {
        return text.to_string();
    }
    let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
    let word = &rest[..end];
    let letters = word.trim_end_matches(|c: char| !c.is_alphanumeric());
    let tail: String = letters.chars().skip(1).collect();
    // « iPhone », « NASA », « Jean-Pierre », « AI-OS » : une autre majuscule dans le mot ; « I » seul : le pronom anglais.
    if tail.chars().any(char::is_uppercase) || letters == "I" || is_in_dictionary(letters, dictionary) {
        return text.to_string();
    }
    let lowered: String = first.to_lowercase().chain(rest.chars().skip(1)).collect();
    format!("{}{lowered}", &text[..start])
}

/// Le mot est-il le premier mot d'une entrée du dictionnaire (« Claude » pour « Claude Code »), ou d'une écriture imposée ?
fn is_in_dictionary(word: &str, dictionary: &UserDictionary) -> bool {
    let first_word = |entry: &str| entry.split_whitespace().next().map(str::to_lowercase);
    let wanted = word.to_lowercase();
    dictionary.words.iter().any(|w| first_word(w).as_deref() == Some(wanted.as_str()))
        || dictionary.corrections.iter().any(|c| first_word(&c.written).as_deref() == Some(wanted.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dictionary::Correction;

    fn dictionary() -> UserDictionary {
        UserDictionary {
            words: vec!["Claude Code".into(), "to-do".into()],
            corrections: vec![Correction { heard: "cri be".into(), written: "Scriibe".into() }],
        }
    }

    fn fix(text: &str) -> String {
        inside_a_sentence(text, &dictionary())
    }

    #[test]
    fn the_capital_and_the_final_period_go() {
        assert_eq!(fix("Bonjour tout le monde."), "bonjour tout le monde");
        assert_eq!(fix("Demain."), "demain");
        assert_eq!(fix("Je viendrai demain."), "je viendrai demain");
    }

    #[test]
    fn question_exclamation_and_ellipsis_are_kept() {
        assert_eq!(fix("Tu viens ?"), "tu viens ?");
        assert_eq!(fix("Super !"), "super !");
        assert_eq!(fix("Peut-être..."), "peut-être...");
        assert_eq!(fix("Peut-être…"), "peut-être…");
    }

    #[test]
    fn names_acronyms_and_dictionary_words_keep_their_capital() {
        assert_eq!(fix("Claude fait le travail."), "Claude fait le travail");
        assert_eq!(fix("Scriibe écoute."), "Scriibe écoute");
        assert_eq!(fix("iPhone neuf."), "iPhone neuf");
        assert_eq!(fix("NASA annonce."), "NASA annonce");
        assert_eq!(fix("Jean-Pierre arrive."), "Jean-Pierre arrive");
        assert_eq!(fix("I think so."), "I think so");
        assert_eq!(fix("To-do du jour."), "To-do du jour");
    }

    #[test]
    fn quotes_and_trailing_spaces_are_respected() {
        assert_eq!(fix("« Bonjour » dit-il."), "« bonjour » dit-il");
        assert_eq!(fix("Bonjour.\n"), "bonjour\n");
    }

    #[test]
    fn lists_numbers_and_empty_text_are_left_alone() {
        assert_eq!(fix("1. Du pain\n2. Du lait"), "1. Du pain\n2. Du lait");
        assert_eq!(fix("- Du pain"), "- Du pain");
        assert_eq!(fix(""), "");
        assert_eq!(fix("."), ".");
        assert_eq!(fix("déjà minuscule."), "déjà minuscule");
    }
}
