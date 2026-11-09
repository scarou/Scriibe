//! Le banc de dictée côté serveur : les réglages d'un passage, les passages gardés (`bancs/`) et leur comparaison.

use scriibe_core::bench_archive::{self, BenchRun, BenchSetup, ComparisonRow};
use scriibe_core::cleaner;
use scriibe_core::dictionary::{self, UserDictionary};
use scriibe_core::history::Cleanup;
use scriibe_core::settings::{AppSettings, DictationLanguage, TranscriptionEngine};
use scriibe_core::typography::french_typography;
use serde_json::{json, Value};

/// Le nom d'un fichier de modèle sans dossier ni extension (« ggml-base »).
fn stem(path: &str) -> String {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    name.rsplit_once('.').map_or(name, |(stem, _)| stem).to_string()
}

/// Une empreinte courte (FNV-1a) de la consigne et des exemples du mode : même nom, consigne différente → réglages différents.
fn fingerprint(text: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// Les réglages d'un passage, écrits comme on les lit. `whisper_model` et `llama_model` : le modèle réellement utilisé (le choix, ou
/// à défaut le premier trouvé) ; `microphone` : le nom du micro choisi.
pub fn setup_for(settings: &AppSettings, version: &str, whisper_model: &str, llama_model: &str, microphone: &str) -> BenchSetup {
    let transcription = match settings.transcription_engine {
        TranscriptionEngine::Whisper => {
            let model = stem(whisper_model);
            if model.is_empty() {
                "Whisper".to_string()
            } else {
                format!("Whisper · {model}")
            }
        }
        TranscriptionEngine::Deepgram => format!("Deepgram · {}", settings.transcription_model),
    };
    let model = match settings.cleanup_engine {
        scriibe_core::settings::CleanupEngine::LlamaCpp => {
            settings.cleanup_engine.display_model(if llama_model.is_empty() { settings.active_cleanup_model() } else { llama_model })
        }
        engine => engine.display_model(settings.active_cleanup_model()),
    };
    BenchSetup {
        version: version.to_string(),
        transcription,
        cleanup: if settings.uses_cleanup_model() {
            format!("{} · {model}", settings.cleanup_engine.name())
        } else {
            "Sans modèle".into()
        },
        mode: if settings.rewriting_on() {
            settings.active_mode_label()
        } else if settings.has_cleanup_engine() {
            "Reformulation coupée".into()
        } else {
            "Aucun moteur (littéral)".into()
        },
        mode_detail: if settings.uses_cleanup_model() { fingerprint(&format!("{:?}", settings.active_profile())) } else { String::new() },
        skip_when_clean: settings.skip_applies() && settings.skip_when_clean,
        microphone: if settings.microphone_uid.is_empty() {
            "Micro du système".into()
        } else if settings.microphone_name.is_empty() {
            microphone.to_string()
        } else {
            settings.microphone_name.clone()
        },
    }
}

/// Ce que le banc note en plus de chaque phrase : le texte donné à la mise au propre, la décision qu'aurait prise l'option « Sauter si
/// déjà propre » (activée avec ces réglages), et ce qui serait collé sans modèle. Port de ce que `DictationController` passe à
/// `ReadingBench.record` (`wouldSkip`, `withoutModel`).
pub struct Judgement {
    pub cleanup_input: String,
    pub would_skip: bool,
    pub without_model: String,
}

pub fn judge(settings: &AppSettings, dictionary: &UserDictionary, raw: &str, translating: bool) -> Judgement {
    let cleanup_input = dictionary::apply(dictionary, raw, settings.list_bullet, settings.lists_on()).text;
    let with_option = AppSettings { skip_when_clean: true, ..settings.clone() };
    let would_skip = cleaner::skips_cleanup(&cleanup_input, &with_option, translating);
    let without_model = if settings.language == DictationLanguage::Fr { french_typography(&cleanup_input) } else { cleanup_input.clone() };
    Judgement { cleanup_input, would_skip, without_model }
}

/// L'erreur d'une comparaison avec le modèle, dite comme chez Scribe.
pub fn model_error(rejected: bool, message: &str) -> String {
    if rejected {
        "écarté par le garde-fou : sans l'option, le texte brut aurait été collé".into()
    } else {
        format!("échec ({message})")
    }
}

/// Un passage dans la liste : réglages, score des contrôles, temps moyens.
pub fn run_summary(run: &BenchRun) -> Value {
    let (passed, total) = run.check_score();
    let totals: Vec<f64> = run.entries.iter().map(|e| e.timing.total).filter(|t| *t > 0.0).collect();
    let average_total = (!totals.is_empty()).then(|| totals.iter().sum::<f64>() / totals.len() as f64);
    let by_outcome: Vec<Value> = [Cleanup::Model, Cleanup::Skipped, Cleanup::Short, Cleanup::None, Cleanup::Translated, Cleanup::Fallback]
        .into_iter()
        .filter_map(|outcome| {
            let average = run.average_total(outcome)?;
            let count = run.entries.iter().filter(|e| e.timing.outcome == outcome).count();
            Some(json!({"outcome": outcome, "average": average, "count": count}))
        })
        .collect();
    json!({
        "id": run.id,
        "date": run.date.to_rfc3339(),
        "setup": run.setup,
        "summary": run.setup.summary(),
        "settings_changed": run.settings_changed,
        "phrases": run.entries.len(),
        "checks_passed": passed,
        "checks_total": total,
        "average_total": average_total,
        "average_by_outcome": by_outcome,
    })
}

fn row_json(row: &ComparisonRow) -> Value {
    let (old_words, new_words) = match (&row.older, &row.newer) {
        (Some(a), Some(b)) => {
            let (o, n) = bench_archive::word_diff(&a.pasted, &b.pasted);
            (Some(o), Some(n))
        }
        _ => (None, None),
    };
    json!({
        "category": row.category,
        "text": row.text,
        "in_both": row.in_both(),
        "changed": row.changed(),
        "improvements": row.improvements(),
        "regressions": row.regressions(),
        "older": row.older.as_ref().map(|e| json!({"pasted": e.pasted, "raw": e.raw, "timing": e.timing, "checks": row.older_checks})),
        "newer": row.newer.as_ref().map(|e| json!({"pasted": e.pasted, "raw": e.raw, "timing": e.timing, "checks": row.newer_checks})),
        "older_words": old_words,
        "newer_words": new_words,
    })
}

/// Deux passages, phrase par phrase.
pub fn comparison(older: &BenchRun, newer: &BenchRun) -> Value {
    let rows = bench_archive::compare(older, newer);
    json!({
        "older": run_summary(older),
        "newer": run_summary(newer),
        "common": rows.iter().filter(|r| r.in_both()).count(),
        "improved": rows.iter().filter(|r| !r.improvements().is_empty()).count(),
        "regressed": rows.iter().filter(|r| !r.regressions().is_empty()).count(),
        "rows": rows.iter().map(row_json).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use scriibe_core::settings::{CleanupEngine, TranscriptionEngine};

    #[test]
    fn the_setup_names_the_models_modes_and_microphone_as_they_are_read() {
        let settings = AppSettings {
            transcription_engine: TranscriptionEngine::Whisper,
            cleanup_engine: CleanupEngine::LlamaCpp,
            skip_when_clean: true,
            ..AppSettings::default()
        };
        let setup = setup_for(&settings, "0.1.0 (abc1234)", "C:\\m\\ggml-base.bin", "C:\\m\\qwen2.5-1.5b-instruct-q4_k_m.gguf", "");
        assert_eq!(setup.transcription, "Whisper · ggml-base");
        assert_eq!(setup.cleanup, "llama.cpp · qwen2.5-1.5b-instruct-q4_k_m.gguf");
        assert_eq!(setup.mode, "Simple");
        assert_eq!(setup.microphone, "Micro du système");
        assert!(setup.skip_when_clean, "Simple + Whisper + modèle : l'option « sauter si propre » s'applique");
        assert!(!setup.mode_detail.is_empty());
    }

    #[test]
    fn without_a_model_the_setup_says_so_and_changing_the_style_changes_the_detail() {
        let none = AppSettings { cleanup_engine: CleanupEngine::None, ..AppSettings::default() };
        let setup = setup_for(&none, "v", "", "", "");
        assert_eq!(
            (setup.cleanup.as_str(), setup.mode.as_str(), setup.mode_detail.as_str()),
            ("Sans modèle", "Aucun moteur (littéral)", "")
        );
        let mut formal = AppSettings { cleanup_engine: CleanupEngine::OpenRouter, ..AppSettings::default() };
        formal.set_mode_choice("formal");
        let before = setup_for(&formal, "v", "", "", "").mode_detail;
        formal.style_overrides.insert("formal".into(), "Écris comme un notaire.".into());
        assert_ne!(setup_for(&formal, "v", "", "", "").mode_detail, before, "même nom de mode, consigne différente → réglages différents");
        assert_eq!(setup_for(&formal, "v", "", "", "").cleanup, "OpenRouter · anthropic/claude-haiku-4.5");
    }

    #[test]
    fn a_personal_mode_names_the_run_and_a_new_instruction_makes_it_another_setup() {
        let mut settings = AppSettings {
            transcription_engine: TranscriptionEngine::Whisper,
            cleanup_engine: CleanupEngine::OpenRouter,
            ..AppSettings::default()
        };
        let mut mode =
            scriibe_core::modes::CustomMode::new("Mail au client", "Écris un mail poli.", scriibe_core::modes::Fidelity::Reformulated);
        settings.custom_mode_id = Some(mode.id.clone());
        settings.custom_modes.push(mode.clone());
        let before = setup_for(&settings, "v", "", "", "");
        assert_eq!(before.mode, "Mail au client");
        assert!(!before.mode_detail.is_empty());
        assert!(!before.skip_when_clean, "un mode qui reformule : l'option « sauter si propre » ne s'applique pas");
        mode.style = "Écris un mail très court.".into();
        settings.custom_modes[0] = mode;
        let after = setup_for(&settings, "v", "", "", "");
        assert_eq!(after.mode, before.mode, "même nom");
        assert_ne!(after.mode_detail, before.mode_detail, "consigne différente → réglages différents");
    }

    #[test]
    fn the_bench_judges_a_clean_sentence_even_when_the_option_is_off_and_gives_the_text_without_model() {
        let settings = AppSettings {
            transcription_engine: TranscriptionEngine::Whisper,
            cleanup_engine: CleanupEngine::LlamaCpp,
            skip_when_clean: false,
            ..AppSettings::default()
        };
        assert!(settings.skip_applies(), "Simple + Whisper : l'option s'applique");
        let dictionary = UserDictionary::default();
        let clean = judge(&settings, &dictionary, "Je serai là demain matin vers dix heures.", false);
        assert!(clean.would_skip, "propre : l'option l'aurait sautée, même désactivée");
        assert_eq!(clean.without_model, "Je serai là demain matin vers dix heures.");
        let hesitating = judge(&settings, &dictionary, "Euh je serai là demain matin euh vers dix heures", false);
        assert!(!hesitating.would_skip, "« euh » demande le modèle");
        assert!(!judge(&settings, &dictionary, "Je serai là demain matin vers dix heures.", true).would_skip, "jamais en traduction");
    }

    #[test]
    fn without_the_model_the_french_typography_is_still_applied() {
        let settings = AppSettings::default();
        assert_eq!(settings.language, DictationLanguage::Fr);
        let out = judge(&settings, &UserDictionary::default(), "Tu viens demain matin vers dix heures ?", false);
        assert_eq!(out.without_model, "Tu viens demain matin vers dix heures\u{00A0}?");
    }

    #[test]
    fn a_model_error_says_whether_the_guard_rejected_it() {
        assert!(model_error(true, "x").starts_with("écarté par le garde-fou"));
        assert_eq!(model_error(false, "délai dépassé"), "échec (délai dépassé)");
    }

    #[test]
    fn model_file_names_lose_their_folder_and_extension() {
        assert_eq!(stem("C:\\a\\b\\ggml-base.bin"), "ggml-base");
        assert_eq!(stem("/x/y/ggml-large-v3-turbo.bin"), "ggml-large-v3-turbo");
        assert_eq!(stem(""), "");
    }
}
