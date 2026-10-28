//! Les phrases types de l'essai de la mise au propre (`CleanTest.swift`, Scribe) : des questions et des demandes dictées que le modèle ne
//! doit pas exécuter, puis deux dictées témoins. Les phrases sont extraites du Swift par `tools/extraire_phrases_types.py`
//! (`data/clean_samples.json`), jamais retapées ; les contrôles de chacune sont ceux de `CleanTest.checks(forSample:)`.

use crate::bench::Check;
use std::sync::OnceLock;

/// Les 12 phrases types.
pub fn samples() -> &'static [String] {
    static SAMPLES: OnceLock<Vec<String>> = OnceLock::new();
    SAMPLES.get_or_init(|| serde_json::from_str(include_str!("../data/clean_samples.json")).expect("clean_samples.json est valide"))
}

fn forms(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

/// Ce qu'on vérifie en plus du garde-fou dans l'essai d'un mode : les questions restent des questions, la reprise est corrigée, les
/// tics retirés. Les demandes (« Traduis », « Explique-moi ») n'ont que le garde-fou.
pub fn checks_for_sample(index: usize) -> Vec<Check> {
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
