//! Export d'un mode (chantier 5, tâche 2) : l'enveloppe de `docs/CADRAGE_PROFILS.md` et la liste blanche.

use scriibe_core::export;
use scriibe_core::modes::{CustomMode, Fidelity, ModeExample};
use serde_json::Value;

fn sms() -> CustomMode {
    let mut m = CustomMode::new("SMS d'été", "Écris comme un SMS court et amical.", Fidelity::Reformulated);
    m.vouvoie = true;
    m.examples = vec![
        ModeExample::new("salut tu viens ce soir", "Salut, tu viens ce soir ?"),
        ModeExample::new("on se voit demain", "On se voit demain !"),
    ];
    m
}

fn keys(v: &Value) -> Vec<&str> {
    let mut k: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
    k.sort_unstable();
    k
}

#[test]
fn the_envelope_has_the_agreed_fields_and_only_the_day() {
    let file = export::modes_export(&[&sms()], true, "0.3.0", "2026-11-12");
    assert_eq!(keys(&file), vec!["app", "content", "exportedAt", "format", "kind", "version"]);
    assert_eq!(file["format"], "scriibe-export");
    assert_eq!(file["version"], 1);
    assert_eq!(file["kind"], "modes");
    assert_eq!(file["app"], "Scriibe 0.3.0");
    assert_eq!(file["exportedAt"], "2026-11-12", "le jour seul, jamais l'heure");
    assert_eq!(keys(&file["content"]), vec!["customModes"]);
}

#[test]
fn a_mode_is_written_field_by_field_from_a_white_list() {
    let mode = sms();
    let file = export::modes_export(&[&mode], true, "0.3.0", "2026-11-12");
    let entry = &file["content"]["customModes"][0];
    assert_eq!(keys(entry), vec!["examples", "fidelity", "id", "name", "rewrites", "style", "vouvoie"]);
    assert_eq!(entry["fidelity"], "reformulated");
    assert_eq!(entry["rewrites"], true, "pour Scribe, qui ne connaît que cette case");
    assert_eq!(entry["vouvoie"], true);
    let examples = entry["examples"].as_array().unwrap();
    assert_eq!(examples.len(), 2);
    assert_eq!(keys(&examples[0]), vec!["dictation", "expected", "id"]);
}

#[test]
fn examples_can_be_left_out() {
    let file = export::modes_export(&[&sms()], false, "0.3.0", "2026-11-12");
    let entry = &file["content"]["customModes"][0];
    assert_eq!(entry["examples"].as_array().unwrap().len(), 0);
    assert_eq!(entry["style"], "Écris comme un SMS court et amical.", "la consigne reste");
    assert!(!file.to_string().contains("salut tu viens"), "aucun texte d'exemple ne part");
}

#[test]
fn an_exported_mode_reads_back_identical_with_the_tolerant_reader() {
    let mode = sms();
    let text = serde_json::to_string_pretty(&export::modes_export(&[&mode], true, "0.3.0", "2026-11-12")).unwrap();
    let back: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(CustomMode::from_value(&back["content"]["customModes"][0]).unwrap(), mode);
}

#[test]
fn nothing_but_the_mode_leaves() {
    let mode = sms();
    let text = export::modes_export(&[&mode], true, "0.3.0", "2026-11-12").to_string();
    for forbidden in ["apiKey", "history", "hotkey", "firstName", "microphone", "whisperModelPath"] {
        assert!(!text.contains(forbidden), "{forbidden} ne doit jamais partir");
    }
}

#[test]
fn the_day_and_the_file_name_carry_no_personal_data() {
    assert_eq!(export::day_of(0), "1970-01-01");
    assert_eq!(export::day_of(1_700_000_000), "2023-11-14");
    assert_eq!(export::mode_file_name("SMS d'été", "2026-11-12"), "scriibe-mode-sms-d-ete-2026-11-12.scriibe.json");
    assert_eq!(export::mode_file_name("  ?? ", "2026-11-12"), "scriibe-mode-sans-nom-2026-11-12.scriibe.json");
    let long = export::mode_file_name(&"a".repeat(100), "2026-11-12");
    assert_eq!(long, format!("scriibe-mode-{}-2026-11-12.scriibe.json", "a".repeat(40)));
    assert_eq!(export::mode_file_name("../../etc/passwd", "2026-11-12"), "scriibe-mode-etc-passwd-2026-11-12.scriibe.json");
}
