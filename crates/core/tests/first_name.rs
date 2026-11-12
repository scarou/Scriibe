//! Prénom de la salutation (chantier 3, tâche 1) : écart propre à Scriibe, voir `docs/PETITS_PLUS.md`.
#![allow(clippy::field_reassign_with_default)]
use scriibe_core::settings::AppSettings;

#[test]
fn by_default_the_greeting_uses_the_computer_account_name() {
    let s = AppSettings::default();
    assert_eq!(s.first_name, "");
    assert_eq!(s.greeting_name("Scarou"), "Scarou");
    assert_eq!(s.greeting_name(""), "");
}

#[test]
fn a_filled_first_name_replaces_the_account_name() {
    let mut s = AppSettings::default();
    s.first_name = "Christophe".into();
    assert_eq!(s.greeting_name("Scarou"), "Christophe");
    s.first_name = "   ".into();
    assert_eq!(s.greeting_name("Scarou"), "Scarou", "des espaces seuls valent vide");
}

#[test]
fn the_first_name_survives_a_round_trip_and_bad_values_fall_back() {
    let mut s = AppSettings::default();
    s.first_name = "Marie-Claire".into();
    let back = AppSettings::from_json(&serde_json::to_string(&s).unwrap()).unwrap();
    assert_eq!(back.first_name, "Marie-Claire");
    assert_eq!(AppSettings::from_json("{}").unwrap().first_name, "");
    assert_eq!(AppSettings::from_json(r#"{"firstName":"  Jean  "}"#).unwrap().first_name, "Jean");
    assert_eq!(AppSettings::from_json(r#"{"firstName":42}"#).unwrap().first_name, "");
}
