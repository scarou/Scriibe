//! Corrections du dictionnaire : un « entendu » n'y figure qu'une fois (chantier 4, étape 1).

use scriibe_core::dictionary::{apply, dedup_corrections, heard_key, Correction, UserDictionary};
use scriibe_core::spoken_signs::ListBullet;

fn correction(heard: &str, written: &str) -> Correction {
    Correction { heard: heard.into(), written: written.into() }
}

#[test]
fn two_heard_phrases_are_the_same_when_only_case_spaces_or_apostrophe_shape_differ() {
    assert_eq!(heard_key("  Cloud   CODE "), "cloud code");
    assert_eq!(heard_key("l’avion"), heard_key("L'avion"));
    assert_ne!(heard_key("cloud code"), heard_key("cloud codes"));
}

#[test]
fn the_first_correction_of_each_heard_phrase_stays_and_the_order_is_kept() {
    let kept = dedup_corrections(vec![
        correction("cloud code", "Claude Code"),
        correction("cri be", "Scriibe"),
        correction("Cloud  Code", "Claude code"),
        correction("l’avion", "l'avion"),
        correction("L'avion", "L'Avion"),
    ]);
    assert_eq!(kept, vec![correction("cloud code", "Claude Code"), correction("cri be", "Scriibe"), correction("l’avion", "l'avion")]);
}

#[test]
fn a_dictionary_file_with_repeated_heard_phrases_is_read_without_the_repeats() {
    let dictionary = UserDictionary::from_json(
        r#"{"words": [], "corrections": [
            {"heard": "cloud code", "written": "Claude Code"},
            {"heard": "CLOUD CODE", "written": "autre"},
            {"heard": "tauri", "written": "Tauri"}]}"#,
    )
    .unwrap();
    assert_eq!(dictionary.corrections, vec![correction("cloud code", "Claude Code"), correction("tauri", "Tauri")]);
    // Le résultat est celui d'avant : la première correction gagnait déjà, la seconde ne servait à rien.
    let text = apply(&dictionary, "j'utilise cloud code et tauri", ListBullet::Dash, true).text;
    assert_eq!(text, "j'utilise Claude Code et Tauri");
}
