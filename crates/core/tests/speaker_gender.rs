//! Genre de la personne qui dicte (S21) : écart propre à Scriibe, voir `docs/ECARTS.md`.
#![allow(clippy::field_reassign_with_default)]
use scriibe_core::cleaner::system_prompt;
use scriibe_core::modes::{CustomMode, Fidelity, Gender, RewriteMode};
use scriibe_core::settings::AppSettings;

fn prompt(s: &AppSettings) -> String {
    system_prompt(&[], &s.active_profile())
}

#[test]
fn unspecified_gender_leaves_every_prompt_unchanged() {
    let mut s = AppSettings::default();
    assert_eq!(s.speaker_gender, Gender::Unspecified);
    for mode in RewriteMode::ALL {
        s.rewrite_mode = mode;
        let before = prompt(&s);
        assert!(!before.contains("La personne qui dicte"), "{mode:?}");
    }
}

#[test]
fn a_rewriting_mode_gets_the_gender_line_before_the_language_line() {
    let mut s = AppSettings::default();
    s.rewrite_mode = RewriteMode::Calm;
    let plain = prompt(&s);
    s.speaker_gender = Gender::Masculine;
    let masc = prompt(&s);
    assert!(masc.contains("La personne qui dicte est un homme"));
    assert!(masc.contains("accorde au masculin"));
    assert_ne!(plain, masc);
    s.speaker_gender = Gender::Feminine;
    assert!(prompt(&s).contains("La personne qui dicte est une femme"));
    // Le reste du prompt ne bouge pas : retirer la ligne redonne le prompt sans genre.
    let line = Gender::Masculine.prompt_line().unwrap();
    assert_eq!(masc.replace(&format!("{line}\n"), ""), plain);
}

#[test]
fn a_faithful_mode_keeps_its_prompt_whatever_the_gender() {
    let mut s = AppSettings::default();
    s.rewrite_mode = RewriteMode::Simple;
    let plain = prompt(&s);
    s.speaker_gender = Gender::Feminine;
    assert_eq!(prompt(&s), plain, "Simple garde les mots de la dictée");
}

#[test]
fn a_custom_mode_follows_its_fidelity() {
    let mut s = AppSettings::default();
    s.speaker_gender = Gender::Masculine;
    let mut m = CustomMode::new("Essai", "Écris poliment.", Fidelity::Light);
    s.custom_mode_id = Some(m.id.clone());
    s.custom_modes = vec![m.clone()];
    assert!(!prompt(&s).contains("La personne qui dicte"), "mode fidèle");
    m.fidelity = Fidelity::Free;
    s.custom_modes = vec![m];
    assert!(prompt(&s).contains("La personne qui dicte est un homme"), "mode qui réécrit");
}

#[test]
fn the_gender_survives_a_round_trip_and_unknown_values_fall_back() {
    let mut s = AppSettings::default();
    s.speaker_gender = Gender::Feminine;
    let back = AppSettings::from_json(&serde_json::to_string(&s).unwrap()).unwrap();
    assert_eq!(back.speaker_gender, Gender::Feminine);
    assert_eq!(AppSettings::from_json("{}").unwrap().speaker_gender, Gender::Unspecified);
    assert_eq!(AppSettings::from_json(r#"{"speakerGender":"autre"}"#).unwrap().speaker_gender, Gender::Unspecified);
}
