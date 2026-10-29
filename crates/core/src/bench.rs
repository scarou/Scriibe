//! Banc de dictée : les phrases à lire et les contrôles automatiques de chacune. Port de `ReadingBench.phrases` et `BenchCheck` (Scribe).
//!
//! Les phrases sont extraites de `ReadingBench.swift` par `tools/extraire_banc.py` (`data/bench_phrases.json`), jamais retapées.

use crate::re::{cached, is_match};
use serde::Deserialize;
use std::sync::OnceLock;

/// Ce qu'on vérifie dans le texte collé d'une phrase (« onze heures gardé, dix heures retiré »…).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Check {
    /// Au moins une des formes est présente (casse, apostrophes et espaces insécables ignorés ; mots entiers).
    Has { label: String, forms: Vec<String> },
    /// Aucune des formes n'est présente.
    Lacks { label: String, forms: Vec<String> },
    /// Écrit exactement ainsi, majuscules comprises (mots du dictionnaire) ; mot entier aussi.
    Exact { label: String, form: String },
    /// Le texte finit par « ? ».
    Question,
    /// Le texte contient un retour à la ligne.
    NewLine,
}

impl Check {
    pub fn label(&self) -> &str {
        match self {
            Check::Has { label, .. } | Check::Lacks { label, .. } | Check::Exact { label, .. } => label,
            Check::Question => "reste une question (« ? »)",
            Check::NewLine => "retour à la ligne",
        }
    }

    pub fn passes(&self, text: &str) -> bool {
        match self {
            Check::Has { forms, .. } => forms.iter().any(|f| contains(f, text, false)),
            Check::Lacks { forms, .. } => !forms.iter().any(|f| contains(f, text, false)),
            Check::Exact { form, .. } => contains(form, text, true),
            Check::Question => fold(text, true).ends_with('?'),
            Check::NewLine => text.contains('\n'),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Phrase {
    pub category: String,
    pub text: String,
    /// Ce qu'on attend de la mise au propre, en une phrase (pour la personne qui lit).
    pub expect: String,
    pub checks: Vec<Check>,
}

impl Phrase {
    /// Les contrôles qui échouent sur ce texte collé.
    pub fn failures(&self, pasted: &str) -> Vec<&Check> {
        self.checks.iter().filter(|c| !c.passes(pasted)).collect()
    }
}

/// Les 17 phrases du banc de Scribe (en français).
pub fn phrases() -> &'static [Phrase] {
    static PHRASES: OnceLock<Vec<Phrase>> = OnceLock::new();
    PHRASES.get_or_init(|| serde_json::from_str(include_str!("../data/bench_phrases.json")).expect("bench_phrases.json est valide"))
}

/// Les 17 phrases du banc pour une dictée en anglais (écart voulu, `docs/ECARTS.md`) : écrites pour Scriibe, pas extraites de Scribe, avec les mêmes
/// catégories de cas ; les signes dictés, propres au français, y sont remplacés par des noms propres.
pub fn phrases_en() -> &'static [Phrase] {
    static PHRASES: OnceLock<Vec<Phrase>> = OnceLock::new();
    PHRASES.get_or_init(|| serde_json::from_str(include_str!("../data/bench_phrases_en.json")).expect("bench_phrases_en.json est valide"))
}

/// Les phrases à lire pour cette langue dictée : l'anglais pour une dictée en anglais, le français sinon (aussi pour « plusieurs langues »).
pub fn phrases_for(language: crate::settings::DictationLanguage) -> &'static [Phrase] {
    if language == crate::settings::DictationLanguage::En {
        phrases_en()
    } else {
        phrases()
    }
}

/// Minuscules (sauf `lowercase: false`), apostrophe droite, espaces de toute sorte réduits à un seul.
pub fn fold(text: &str, lowercase: bool) -> String {
    let t = if lowercase { text.to_lowercase() } else { text.to_string() }.replace('’', "'");
    t.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// La forme comme mot entier : « euh » ne se trouve pas dans « heureux », « 10 h » pas dans « 110 h ». Entre deux mots de la forme,
/// la ponctuation compte comme un espace : « bon ben » se trouve dans « Bon, ben », « je je » dans « Je… je ».
pub fn contains(form: &str, text: &str, case_sensitive: bool) -> bool {
    let needle = fold(form, !case_sensitive);
    if needle.is_empty() {
        return false;
    }
    // Contrôle exact (dictionnaire) : seulement des espaces entre les mots.
    let separator = if case_sensitive { r"\s+" } else { r"[\s,;:.!?…]+" };
    let mut pattern = needle.split(' ').map(fancy_regex::escape).collect::<Vec<_>>().join(separator);
    let edge = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric());
    if edge(needle.chars().next()) {
        pattern = format!(r"(?<![\p{{L}}\p{{N}}]){pattern}");
    }
    if edge(needle.chars().last()) {
        pattern.push_str(r"(?![\p{L}\p{N}])");
    }
    cached(&pattern).is_some_and(|re| is_match(&re, &fold(text, !case_sensitive)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bench_has_the_17_phrases_of_scribe_each_with_checks() {
        assert_eq!(phrases().len(), 17);
        assert!(phrases().iter().all(|p| !p.checks.is_empty() && !p.text.is_empty()));
        assert_eq!(phrases().iter().map(|p| p.checks.len()).sum::<usize>(), 35);
    }

    #[test]
    fn the_english_bench_mirrors_the_french_one_and_its_checks_tell_good_from_bad() {
        use crate::settings::DictationLanguage;
        let english = phrases_en();
        assert_eq!(english.len(), 17);
        assert!(english.iter().all(|p| !p.checks.is_empty() && !p.text.is_empty() && !p.expect.is_empty()));
        assert!(english.iter().map(|p| &p.category).collect::<std::collections::HashSet<_>>().len() == 17, "17 catégories différentes");
        assert_eq!(phrases_for(DictationLanguage::En)[0].text, english[0].text);
        assert_eq!(phrases_for(DictationLanguage::Fr)[0].text, phrases()[0].text);
        assert_eq!(phrases_for(DictationLanguage::Multi)[0].text, phrases()[0].text, "plusieurs langues : le banc français");
        let pasted = |i: usize, text: &str| english[i].failures(text).is_empty();
        // Une phrase propre passe telle qu'elle est lue ; une reprise non corrigée échoue, corrigée passe.
        assert!(pasted(0, english[0].text.as_str()));
        assert!(pasted(5, "The meeting is at 11 o'clock."));
        assert!(!pasted(5, "The meeting is at ten o'clock, no sorry, at eleven o'clock."));
        assert!(pasted(7, "We leave on Tuesday, depending on the weather."));
        assert!(!pasted(7, "We leave on Monday, I mean Tuesday, depending on the weather."));
        // Les hésitations, un « ? » qui manque, un nom mal écrit.
        assert!(!pasted(3, "Um, I think we could, uh, move the session to Thursday."));
        assert!(pasted(3, "I think we could move the session to Thursday."));
        assert!(!pasted(11, "Why has the server stopped responding since this morning"));
        assert!(pasted(11, "Why has the server stopped responding since this morning?"));
        assert!(!pasted(10, "I forwarded the invoice to Sara Mitchel at Bright Water Logistics."));
        assert!(pasted(10, "I forwarded the invoice to Sarah Mitchell at Brightwater Logistics."));
        // Les nombres, en chiffres ou en lettres.
        assert!(pasted(14, "The invoice comes to $1,250, to be paid before November 15th."));
        assert!(pasted(14, "The invoice is one thousand two hundred and fifty dollars, due before November fifteenth."));
        assert!(!pasted(14, "The invoice comes to $1,520, to be paid before November 5th."));
    }

    #[test]
    fn labels_are_readable() {
        assert_eq!(Check::Question.label(), "reste une question (« ? »)");
        assert_eq!(phrases()[0].checks[0].label(), "heure gardée");
    }
}
