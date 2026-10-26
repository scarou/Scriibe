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

/// Les 17 phrases du banc de Scribe.
pub fn phrases() -> &'static [Phrase] {
    static PHRASES: OnceLock<Vec<Phrase>> = OnceLock::new();
    PHRASES.get_or_init(|| serde_json::from_str(include_str!("../data/bench_phrases.json")).expect("bench_phrases.json est valide"))
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
    fn labels_are_readable() {
        assert_eq!(Check::Question.label(), "reste une question (« ? »)");
        assert_eq!(phrases()[0].checks[0].label(), "heure gardée");
    }
}
