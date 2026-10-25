//! Reconnaissance de la langue d'un texte, pour le garde-fou de la traduction (le texte est-il resté en français ?).
//!
//! Scribe utilise l'analyseur de langue d'Apple ; Scriibe passe par un trait, pour que le cœur ne dépende d'aucune bibliothèque de langue
//! et que chaque système puisse brancher la sienne. [`LinguaGuesser`] (fonction `language-detection`) est l'implémentation portable.

use crate::translation::TranslationLanguage;

/// Part de probabilité (0 à 1) du français et de la langue cible pour un texte.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Odds {
    pub french: f64,
    pub target: f64,
}

pub trait LanguageGuesser {
    fn odds(&self, text: &str, target: TranslationLanguage) -> Odds;
}

/// Ne reconnaît rien : le garde-fou ne pourra pas écarter un texte « resté en français » à partir des seuls mots nouveaux.
pub struct NoGuesser;

impl LanguageGuesser for NoGuesser {
    fn odds(&self, _: &str, _: TranslationLanguage) -> Odds {
        Odds { french: 0.0, target: 0.0 }
    }
}

#[cfg(feature = "language-detection")]
pub use lingua_guesser::LinguaGuesser;

#[cfg(feature = "language-detection")]
mod lingua_guesser {
    use super::{LanguageGuesser, Odds};
    use crate::translation::TranslationLanguage;
    use lingua::{Language, LanguageDetector, LanguageDetectorBuilder};

    /// Reconnaissance par la bibliothèque `lingua` (modèles embarqués pour le français et les cinq langues de traduction).
    pub struct LinguaGuesser {
        detector: LanguageDetector,
    }

    impl Default for LinguaGuesser {
        fn default() -> Self {
            let languages =
                [Language::French, Language::English, Language::German, Language::Spanish, Language::Italian, Language::Portuguese];
            Self { detector: LanguageDetectorBuilder::from_languages(&languages).build() }
        }
    }

    impl LinguaGuesser {
        pub fn new() -> Self {
            Self::default()
        }
    }

    fn language(target: TranslationLanguage) -> Language {
        match target {
            TranslationLanguage::En => Language::English,
            TranslationLanguage::Es => Language::Spanish,
            TranslationLanguage::De => Language::German,
            TranslationLanguage::It => Language::Italian,
            TranslationLanguage::Pt => Language::Portuguese,
        }
    }

    impl LanguageGuesser for LinguaGuesser {
        fn odds(&self, text: &str, target: TranslationLanguage) -> Odds {
            let values = self.detector.compute_language_confidence_values(text);
            let of = |wanted: Language| values.iter().find(|(l, _)| *l == wanted).map(|(_, c)| *c).unwrap_or(0.0);
            Odds { french: of(Language::French), target: of(language(target)) }
        }
    }
}
