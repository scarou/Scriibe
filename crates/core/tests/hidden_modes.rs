//! Modes décochés dans Réglages → Les modes (champ `hiddenModes`, propre à Scriibe) : absents du menu de l'icône.

use scriibe_core::settings::AppSettings;

const CUSTOM: &str = r#"{"customModes": [{"id": "4F6D1B7E-0C1A-4E55-9C0B-2A7F3C9D8E11", "name": "SMS", "style": "Court.", "fidelity": "light", "vouvoie": false, "examples": []}]"#;

fn read(extra: &str) -> AppSettings {
    AppSettings::from_json(&format!("{CUSTOM}{extra}}}")).unwrap()
}

#[test]
fn all_modes_are_shown_by_default() {
    let s = AppSettings::default();
    assert!(s.hidden_modes.is_empty());
    assert!(s.menu_mode_ids().iter().all(|id| !s.is_hidden_in_menu(id)));
    // Un fichier de Scribe n'a pas ce champ.
    assert!(read("").hidden_modes.is_empty());
}

#[test]
fn hidden_modes_survive_a_round_trip_and_hide_built_in_and_custom_modes() {
    let s = read(r#", "hiddenModes": ["tech", "4F6D1B7E-0C1A-4E55-9C0B-2A7F3C9D8E11"]"#);
    let back = AppSettings::from_json(&serde_json::to_string(&s).unwrap()).unwrap();
    assert_eq!(back.hidden_modes, vec!["tech", "4F6D1B7E-0C1A-4E55-9C0B-2A7F3C9D8E11"]);
    assert!(back.is_hidden_in_menu("tech") && back.is_hidden_in_menu("4F6D1B7E-0C1A-4E55-9C0B-2A7F3C9D8E11"));
    assert!(!back.is_hidden_in_menu("simple"));
}

#[test]
fn unknown_duplicate_and_unreadable_ids_are_dropped() {
    let s = read(r#", "hiddenModes": ["tech", "tech", "playful", "inconnu", 7]"#);
    assert!(s.hidden_modes.is_empty(), "une liste illisible garde la valeur par défaut");
    let s = read(r#", "hiddenModes": ["tech", "tech", "playful", "inconnu"]"#);
    assert_eq!(s.hidden_modes, vec!["tech"], "Amusant n'est plus proposé : pas de case, donc pas de masquage");
}

#[test]
fn hiding_every_mode_shows_them_all() {
    let mut ids: Vec<String> = AppSettings::default().menu_mode_ids();
    ids.push("4F6D1B7E-0C1A-4E55-9C0B-2A7F3C9D8E11".into());
    let all = serde_json::to_string(&ids).unwrap();
    let s = read(&format!(r#", "hiddenModes": {all}"#));
    assert!(s.hidden_modes.is_empty());
    // Les modes personnels supprimés après coup : le dernier visible l'est toujours.
    let mut s = AppSettings::default();
    s.hidden_modes = s.menu_mode_ids();
    s.hidden_modes.pop();
    s.hidden_modes.push("supprimé".into());
    assert!(s.menu_mode_ids().iter().any(|id| !s.is_hidden_in_menu(id)));
    s.hidden_modes = s.menu_mode_ids();
    assert!(s.menu_mode_ids().iter().all(|id| !s.is_hidden_in_menu(id)));
}
