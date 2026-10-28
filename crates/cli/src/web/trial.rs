//! Les essais de la mise au propre : une phrase type passée à un mode (Modes → « Essayer sur les phrases types », `CleanTest.trial`) et une
//! phrase passée à un modèle (Banc d'essai de la mise au propre, `TrialBench`). Un appel par phrase ou par modèle : la page les enchaîne,
//! car en local les lancer ensemble fausserait les temps.

use super::Context;
use scriibe_core::dictionary;
use scriibe_core::modes::RewriteMode;
use scriibe_core::settings::{AppSettings, CleanupEngine, DictationLanguage};
use scriibe_core::typography::french_typography;
use serde_json::{json, Value};
use std::time::Instant;

/// Le texte tel que le modèle le reçoit : dictionnaire et signes dictés appliqués avant lui, comme en vraie dictée.
fn fixed(ctx: &Context, settings: &AppSettings, text: &str) -> (String, Vec<String>) {
    let dictionary = ctx.app.config.dictionary();
    let applied = dictionary::apply(&dictionary, text, settings.list_bullet, true);
    (applied.text, dictionary.words)
}

/// Une phrase type passée au mode `mode` (le mode en cours si absent) : sortie, temps, mots en commun, verdict du garde-fou et contrôles.
pub fn sample(ctx: &Context, input: &Value) -> Value {
    let samples = scriibe_core::clean_test::samples();
    let Some(index) = input["index"].as_u64().map(|i| i as usize).filter(|i| *i < samples.len()) else {
        return json!({"ok": false, "error": "phrase inconnue"});
    };
    let mut settings = ctx.app.config.settings();
    if let Some(choice) = input["mode"].as_str() {
        settings.set_mode_choice(choice);
        if settings.mode_choice() != choice {
            return json!({"ok": false, "error": "mode inconnu"});
        }
    }
    settings.cleanup_enabled = true;
    if !settings.has_cleanup_engine() {
        return json!({"ok": false, "error": "Aucun moteur de mise au propre : choisis-en un dans Réglages → Moteurs"});
    }
    if settings.rewrite_mode == RewriteMode::Literal && settings.active_custom_mode().is_none() {
        return json!({"ok": false, "error": "Mode Littéral : aucun modèle n'est appelé, rien à essayer"});
    }
    // Amusant (abandonné) : jugé comme Simple.
    if settings.rewrite_mode == RewriteMode::Playful {
        settings.rewrite_mode = RewriteMode::Simple;
    }
    let (line, words) = fixed(ctx, &settings, &samples[index]);
    // Modèle chargé avant le chrono (llama.cpp lance son serveur) : les temps sont ceux de la mise au propre.
    if index == 0 {
        ctx.app.cleaner.warm_up(&settings, true, &words, None);
    }
    let start = Instant::now();
    let raw = match ctx.app.cleaner.rewrite(&line, &settings, &words) {
        Ok(raw) => raw,
        Err(e) => return json!({"ok": false, "error": e.message}),
    };
    let seconds = start.elapsed().as_secs_f64();
    let kept = scriibe_core::cleaner::accepts(&line, &raw, settings.active_profile().rewrites);
    let overlap = scriibe_core::text::overlap(&line, &raw);
    let dictionary = ctx.app.config.dictionary();
    let mut out = dictionary::apply(&dictionary, &raw, settings.list_bullet, true).text;
    if settings.language == DictationLanguage::Fr {
        out = french_typography(&out);
    }
    // Les contrôles jugent le texte tel qu'il serait collé (typographie comprise : « 11h » → « 11 h »).
    let checks: Vec<Value> =
        scriibe_core::clean_test::checks_for_sample(index).iter().map(|c| json!({"label": c.label(), "ok": c.passes(&out)})).collect();
    json!({
        "ok": true,
        "output": out,
        "sent": (line != samples[index]).then_some(line),
        "seconds": seconds,
        "kept_words": overlap.kept,
        "own_words": overlap.own,
        "accepted": kept,
        "checks": checks,
    })
}

/// Une phrase passée à un modèle : texte rendu et temps. En Littéral (aucun modèle), on compare les modèles en Simple.
pub fn candidate(ctx: &Context, input: &Value) -> Value {
    let Ok(engine) = serde_json::from_value::<CleanupEngine>(input["engine"].clone()) else {
        return json!({"ok": false, "error": "moteur inconnu"});
    };
    if engine == CleanupEngine::None {
        return json!({"ok": false, "error": "choisis un moteur"});
    }
    let mut settings = ctx.app.config.settings();
    settings.cleanup_engine = engine;
    settings.set_active_cleanup_model(input["model"].as_str().unwrap_or(""));
    settings.cleanup_enabled = true;
    if settings.rewrite_mode == RewriteMode::Literal || settings.rewrite_mode == RewriteMode::Playful {
        settings.rewrite_mode = RewriteMode::Simple;
        settings.custom_mode_id = None;
    }
    let (line, words) = fixed(ctx, &settings, input["text"].as_str().unwrap_or(""));
    if line.trim().is_empty() {
        return json!({"ok": false, "error": "écris une phrase de test"});
    }
    ctx.app.cleaner.warm_up(&settings, true, &words, None);
    let start = Instant::now();
    match ctx.app.cleaner.cleanup(&line, &settings, &words) {
        Ok(text) => {
            let seconds = start.elapsed().as_secs_f64();
            let text = if settings.language == DictationLanguage::Fr { french_typography(&text) } else { text };
            json!({"ok": true, "text": text, "seconds": seconds})
        }
        Err(e) => json!({"ok": false, "error": e.message}),
    }
}
