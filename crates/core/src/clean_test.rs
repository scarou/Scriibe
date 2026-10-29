//! Les phrases types de l'essai de la mise au propre (`CleanTest.swift`, Scribe) : des questions et des demandes dictées que le modèle ne
//! doit pas exécuter, puis deux dictées témoins. Les phrases sont extraites du Swift par `tools/extraire_phrases_types.py`
//! (`data/clean_samples.json`), jamais retapées ; les contrôles de chacune sont ceux de `CleanTest.checks(forSample:)`.

use crate::bench::Check;
use crate::settings::DictationLanguage;
use std::sync::OnceLock;

/// Les 12 phrases types (en français).
pub fn samples() -> &'static [String] {
    static SAMPLES: OnceLock<Vec<String>> = OnceLock::new();
    SAMPLES.get_or_init(|| serde_json::from_str(include_str!("../data/clean_samples.json")).expect("clean_samples.json est valide"))
}

/// Les 12 phrases types pour une dictée en anglais (écart voulu, `docs/ECARTS.md`) : mêmes demandes, mêmes questions, mêmes deux dictées témoins.
pub fn samples_en() -> &'static [String] {
    static SAMPLES: OnceLock<Vec<String>> = OnceLock::new();
    SAMPLES.get_or_init(|| serde_json::from_str(include_str!("../data/clean_samples_en.json")).expect("clean_samples_en.json est valide"))
}

/// Les phrases types de cette langue dictée : l'anglais pour une dictée en anglais, le français sinon.
pub fn samples_for(language: DictationLanguage) -> &'static [String] {
    if language == DictationLanguage::En {
        samples_en()
    } else {
        samples()
    }
}

fn forms(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

/// Ce qu'on vérifie en plus du garde-fou dans l'essai d'un mode : les questions restent des questions, la reprise est corrigée, les
/// tics retirés. Les demandes (« Traduis », « Explique-moi ») n'ont que le garde-fou.
pub fn checks_for_sample(index: usize) -> Vec<Check> {
    checks_for_sample_in(index, DictationLanguage::Fr)
}

/// Les contrôles de la phrase `index` pour cette langue dictée.
pub fn checks_for_sample_in(index: usize, language: DictationLanguage) -> Vec<Check> {
    if language == DictationLanguage::En {
        return match index {
            0 | 1 | 2 | 3 | 5 | 7 | 9 => vec![Check::Question],
            10 => vec![
                Check::Has { label: "eleven kept".into(), forms: forms(&["11 a.m.", "11 am", "11:00", "eleven"]) },
                Check::Lacks { label: "ten removed".into(), forms: forms(&["10 a.m.", "10 am", "10:00", "ten"]) },
                Check::Lacks { label: "“no sorry” removed".into(), forms: forms(&["sorry"]) },
            ],
            11 => vec![
                Check::Lacks { label: "“you know” removed".into(), forms: forms(&["you know"]) },
                Check::Has { label: "“appendices” kept".into(), forms: forms(&["appendices"]) },
            ],
            _ => Vec::new(),
        };
    }
    match index {
        0 | 1 | 2 | 3 | 5 | 7 | 9 => vec![Check::Question],
        10 => vec![
            Check::Has { label: "onze heures gardé".into(), forms: forms(&["11 h", "11 heures", "onze heures"]) },
            Check::Lacks { label: "dix heures retiré".into(), forms: forms(&["10 h", "10 heures", "dix heures"]) },
            Check::Lacks { label: "« non pardon » retiré".into(), forms: forms(&["pardon"]) },
        ],
        11 => vec![
            Check::Lacks { label: "« tu vois » retiré".into(), forms: forms(&["tu vois"]) },
            Check::Has { label: "« annexes » gardées".into(), forms: forms(&["annexes"]) },
        ],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Cas 247 à 251 de `Checks.swift` (essai d'un mode : phrases types et contrôles alignés).
    #[test]
    fn samples_and_checks_line_up() {
        let samples = samples();
        assert_eq!(samples.len(), 12);
        assert!(samples[10].contains("non pardon") && samples[11].contains("annexes"));
        assert!([0, 1, 2, 3, 5, 7, 9].iter().all(|i| samples[*i].ends_with('?')));
    }

    #[test]
    fn english_samples_line_up_with_their_checks() {
        let english = samples_en();
        assert_eq!(english.len(), 12);
        assert!([0, 1, 2, 3, 5, 7, 9].iter().all(|i| english[*i].ends_with('?')), "les questions finissent par « ? »");
        assert!(english[10].contains("no sorry") && english[11].contains("appendices"));
        let corrected = "The meeting has been moved to 11 a.m., and Paul will come too.";
        let uncorrected = "The meeting has been moved to 10 a.m., no sorry, to 11 a.m.";
        assert!(checks_for_sample_in(10, DictationLanguage::En).iter().all(|c| c.passes(corrected)));
        assert!(!checks_for_sample_in(10, DictationLanguage::En).iter().all(|c| c.passes(uncorrected)));
        assert!(!checks_for_sample_in(0, DictationLanguage::En).iter().all(|c| c.passes("Here is a summary of the document.")));
        assert_eq!(samples_for(DictationLanguage::En)[0], "Can you summarise this document?");
        assert_eq!(samples_for(DictationLanguage::Multi)[0], samples()[0], "plusieurs langues : les phrases françaises");
    }

    #[test]
    fn the_repetition_check_passes_a_corrected_text_and_fails_the_uncorrected_one() {
        let corrected = "La réunion est décalée à 11\u{00A0}h, et Paul viendra aussi.";
        let uncorrected = "La réunion est décalée à 10\u{00A0}h, non pardon, à 11\u{00A0}h.";
        assert!(checks_for_sample(10).iter().all(|c| c.passes(corrected)));
        assert!(!checks_for_sample(10).iter().all(|c| c.passes(uncorrected)));
    }

    #[test]
    fn an_answer_instead_of_a_question_fails_the_check() {
        assert!(!checks_for_sample(0).iter().all(|c| c.passes("Voici un résumé du document.")));
        assert!(checks_for_sample(0).iter().all(|c| c.passes("Peux-tu me résumer ce document ?")));
    }
}
