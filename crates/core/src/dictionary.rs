//! Dictionnaire de l'utilisateur : mots à écrire exactement, corrections « entendu → écrit », et application avec les signes dictés.
//! Port de `TextFixer.apply` (Scribe).

use crate::re::{cached, matches_with_group};
use crate::spoken_signs::{self, Active, ListBullet};
use crate::text::word_count;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct Correction {
    pub heard: String,
    pub written: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct UserDictionary {
    pub words: Vec<String>,
    pub corrections: Vec<Correction>,
}

impl UserDictionary {
    /// Lecture tolérante : une correction incomplète est ignorée, un champ illisible vaut vide. Échoue si ce n'est pas un objet JSON.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let value: serde_json::Value = serde_json::from_str(json)?;
        let o = value.as_object().ok_or_else(|| serde::de::Error::custom("le dictionnaire n'est pas un objet JSON"))?;
        let words = o.get("words").and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default();
        let corrections = o
            .get("corrections")
            .and_then(serde_json::Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|c| {
                        Some(Correction { heard: c.get("heard")?.as_str()?.to_string(), written: c.get("written")?.as_str()?.to_string() })
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(Self { words, corrections: dedup_corrections(corrections) })
    }
}

/// La clé qui dit que deux « entendu » sont le même : sans tenir compte des majuscules, des espaces multiples ni de l'apostrophe
/// droite ou courbe (comme `heard_pattern` les reconnaît à l'application).
pub fn heard_key(heard: &str) -> String {
    heard.split_whitespace().collect::<Vec<_>>().join(" ").replace('’', "'").to_lowercase()
}

/// Garde la première correction de chaque « entendu » (c'est déjà celle qui s'applique : les corrections se suivent dans l'ordre,
/// la seconde n'aurait plus rien à corriger) ; l'ordre des autres ne change pas.
pub fn dedup_corrections(corrections: Vec<Correction>) -> Vec<Correction> {
    let mut seen = std::collections::HashSet::new();
    corrections.into_iter().filter(|c| seen.insert(heard_key(&c.heard))).collect()
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
    apply_for(dictionary, text, bullet, lists, Active::FR)
}

/// Comme [`apply`], avec les signes dictés des langues `active` (S18 : signes anglais seulement en dictée anglaise ou multilingue).
pub fn apply_for(dictionary: &UserDictionary, text: &str, bullet: ListBullet, lists: bool, active: Active) -> Applied {
    // Signes dictés de la liste intégrée (« ouvrez la parenthèse » → « ( », « tiret » → « - »), sans compter.
    let mut result = spoken_signs::apply_for(text, bullet, lists, active);
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
    let signed = spoken_signs::apply_for(&result, bullet, lists, active);
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
