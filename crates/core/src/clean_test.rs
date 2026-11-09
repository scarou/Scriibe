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

/// Phrases tendues ajoutées à l'essai du mode Apaisé (CNV) seulement (S19, écart voulu, `docs/ECARTS.md`) : reproches, jugements et ordres
/// comme on en écrit sur WhatsApp ou les réseaux. La dernière est une question, qui doit le rester.
const CALM_SAMPLES_FR: [&str; 5] = [
    "t'as encore oublié d'aller chercher les enfants c'est vraiment n'importe quoi tu penses jamais à personne",
    "franchement ton message est complètement à côté de la plaque t'as rien compris à ce que j'ai écrit",
    "tu m'as promis de m'appeler hier soir et tu l'as pas fait comme d'habitude on peut jamais compter sur toi",
    "arrête de liker les photos de ton ex ça me rend dingue tu le fais exprès",
    "pourquoi tu réponds jamais à mes messages t'es trop occupé pour moi ?",
];
const CALM_SAMPLES_EN: [&str; 5] = [
    "you forgot to pick up the kids again this is ridiculous you never think about anyone",
    "honestly your message is completely off the mark you didn't understand a single thing I wrote",
    "you promised to call me last night and you didn't, as usual, we can never count on you",
    "stop liking your ex's photos it drives me crazy you're doing it on purpose",
    "why do you never answer my messages are you too busy for me?",
];

/// Les phrases de l'essai : les 12 phrases types, plus (mode Apaisé seulement) cinq phrases tendues aux indices 12 à 16.
pub fn samples_for_mode(language: DictationLanguage, calm: bool) -> Vec<String> {
    let mut list = samples_for(language).to_vec();
    if calm {
        let extra: &[&str] = if language == DictationLanguage::En { &CALM_SAMPLES_EN } else { &CALM_SAMPLES_FR };
        list.extend(extra.iter().map(|s| s.to_string()));
    }
    list
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
            0 | 1 | 2 | 3 | 5 | 7 | 9 | 16 => vec![Check::Question],
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
        0 | 1 | 2 | 3 | 5 | 7 | 9 | 16 => vec![Check::Question],
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
    fn calm_adds_five_tense_samples_and_the_last_one_is_a_question() {
        for language in [DictationLanguage::Fr, DictationLanguage::En] {
            let base = samples_for_mode(language, false);
            let calm = samples_for_mode(language, true);
            assert_eq!(base.len(), 12);
            assert_eq!(calm.len(), 17);
            assert_eq!(&calm[..12], &base[..]);
            assert!(calm[16].ends_with('?') && calm[12..16].iter().all(|s| !s.ends_with('?')));
            assert!(!checks_for_sample_in(16, language).is_empty());
            assert!(checks_for_sample_in(12, language).is_empty());
        }
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
