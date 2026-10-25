//! Dictionnaire de l'utilisateur : mots à écrire exactement, corrections « entendu → écrit », et application avec les signes dictés.
//! Port de `TextFixer.apply` (Scribe).

use crate::re::{cached, matches_with_group};
use crate::spoken_signs::{self, ListBullet};
use crate::text::word_count;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Correction {
    pub heard: String,
    pub written: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct UserDictionary {
    #[serde(default)]
    pub words: Vec<String>,
    #[serde(default)]
    pub corrections: Vec<Correction>,
}

/// Le texte corrigé et le nombre de corrections du dictionnaire appliquées.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Applied {
    pub text: String,
    pub count: usize,
}

/// Applique les corrections « entendu → écrit », mot entier, sans tenir compte des majuscules.
/// `lists` : les listes dictées (« tiret », « point 2 »…) sont du français ; faux pour une dictée en anglais ou le résultat d'une traduction.
pub fn apply(dictionary: &UserDictionary, text: &str, bullet: ListBullet, lists: bool) -> Applied {
    // Signes dictés de la liste intégrée (« ouvrez la parenthèse » → « ( », « tiret » → « - »), sans compter.
    let mut result = spoken_signs::apply(text, bullet, lists);
    let mut count = 0;
    let mut signs = false;
    for correction in &dictionary.corrections {
        let heard = correction.heard.trim_matches(|c: char| c == ' ' || c == '\t');
        if heard.is_empty() {
            continue;
        }
        let kind = spoken_signs::kind_of(&correction.written);
        // « 3 points d'exclamation » = « trois points d'exclamation » ; mais « deux points » ne doit pas attraper « 2 points » marqués.
        let core = heard_pattern(heard, kind.is_some() && word_count(heard) >= 3);
        let (t, n) = match kind {
            Some(kind) => {
                let (t, n) = spoken_signs::replace(&core, kind, &correction.written, &result);
                signs = signs || n > 0;
                (t, n)
            }
            None => replace(&core, &correction.written, &result),
        };
        result = t;
        count += n;
    }
    // Une correction du dictionnaire peut avoir écrit un signe (« à la ligne » → « retour à la ligne ») ou créé le saut de ligne :
    // signes et listes se refont après elle.
    let signed = spoken_signs::apply(&result, bullet, lists);
    if signed != result {
        result = signed;
        signs = true;
    }
    if signs {
        result = spoken_signs::tidy(&result);
    }
    // Remet l'orthographe exacte des mots du dictionnaire (« claude code » ou « to do » → « Claude Code », « to-do »), sans compter.
    for word in dictionary.words.iter().filter(|w| w.chars().count() > 1) {
        result = replace(&flexible_pattern(word), word, &result).0;
    }
    Applied { text: result, count }
}

/// Ce qui est entendu, tolérant aux apostrophes (' ou ’) et aux espaces multiples que Whisper ou le modèle peuvent produire.
/// Les nombres aussi : Whisper écrit « 3 points » comme « trois points », selon la phrase.
fn heard_pattern(heard: &str, numbers: bool) -> String {
    heard
        .split_whitespace()
        .map(|word| {
            if numbers {
                if let Some(n) = number_variants(word) {
                    return n;
                }
            }
            word.split(['\'', '’']).map(fancy_regex::escape).collect::<Vec<_>>().join("['’]")
        })
        .collect::<Vec<_>>()
        .join(r"\s+")
}

const NUMBERS: [&str; 10] = ["un", "deux", "trois", "quatre", "cinq", "six", "sept", "huit", "neuf", "dix"];

/// « trois » ou « 3 » → motif qui accepte les deux ; `None` pour un mot qui n'est pas un nombre de 1 à 10.
fn number_variants(word: &str) -> Option<String> {
    let w = word.to_lowercase();
    if let Some(i) = NUMBERS.iter().position(|n| *n == w) {
        return Some(format!("(?:{w}|{})", i + 1));
    }
    if w == "une" {
        return Some("(?:une|1)".to_string());
    }
    match w.parse::<usize>() {
        Ok(1) => Some("(?:1|un|une)".to_string()),
        Ok(n @ 2..=10) => Some(format!("(?:{n}|{})", NUMBERS[n - 1])),
        _ => None,
    }
}

/// « to-do » accepte aussi « to do » et « todo » ; « Claude Code » accepte « claude-code ».
fn flexible_pattern(word: &str) -> String {
    word.split([' ', '-']).filter(|p| !p.is_empty()).map(fancy_regex::escape).collect::<Vec<_>>().join(r"[\s\-]?")
}

/// Remplace `core` par `replacement`, mot entier, sans tenir compte des majuscules. Ne compte que les remplacements qui changent le texte.
fn replace(core: &str, replacement: &str, text: &str) -> (String, usize) {
    let pattern = format!(r"(?i)(?<![\p{{L}}\p{{N}}]){core}(?![\p{{L}}\p{{N}}])");
    let Some(regex) = cached(&pattern) else { return (text.to_string(), 0) };
    let found = matches_with_group(&regex, text, 0);
    let changed = found.iter().filter(|(whole, _)| &text[whole.0..whole.1] != replacement).count();
    if changed == 0 {
        return (text.to_string(), 0);
    }
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for (whole, _) in &found {
        out.push_str(&text[last..whole.0]);
        out.push_str(replacement);
        last = whole.1;
    }
    out.push_str(&text[last..]);
    (out, changed)
}
