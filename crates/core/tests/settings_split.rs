//! Répartition des réglages entre l'utilisateur et l'ordinateur (décision 1 de `docs/CADRAGE_PROFILS.md`, étape 4a).

use scriibe_core::settings::{owner_of, AppSettings, Owner, COMPUTER_KEYS, USER_KEYS};
use serde_json::{json, Map, Value};

/// Tous les réglages écrits, y compris les facultatifs (`customModeID`, `historyKeepDays` ne sont écrits que s'ils ont une valeur).
fn every_key_written() -> Map<String, Value> {
    let settings = AppSettings { custom_mode_id: Some("X".into()), history_keep_days: Some(30), ..AppSettings::default() };
    match serde_json::to_value(settings).unwrap() {
        Value::Object(o) => o,
        other => panic!("des réglages ne sont pas un objet : {other}"),
    }
}

#[test]
fn every_settings_field_is_classified_as_user_or_computer() {
    let written = every_key_written();
    let unclassified: Vec<&String> = written.keys().filter(|k| owner_of(k).is_none()).collect();
    assert!(
        unclassified.is_empty(),
        "champ(s) d'AppSettings ni « utilisateur » ni « ordinateur » : {unclassified:?}. Ajoute chaque clé à USER_KEYS ou à COMPUTER_KEYS \
         (crates/core/src/settings.rs) d'après la décision 1 de docs/CADRAGE_PROFILS.md."
    );
}

#[test]
fn the_classification_has_no_overlap_and_no_stale_key() {
    let written = every_key_written();
    for key in USER_KEYS {
        assert!(!COMPUTER_KEYS.contains(key), "{key} est classé des deux côtés");
    }
    for key in USER_KEYS.iter().chain(COMPUTER_KEYS) {
        assert!(written.contains_key(*key), "{key} est classé mais n'est plus un champ d'AppSettings");
    }
}

#[test]
fn the_decision_one_table_is_followed() {
    for key in
        ["firstName", "speakerGender", "language", "translationLanguage", "customModes", "customModeID", "rewriteMode", "simpleFidelity"]
    {
        assert_eq!(owner_of(key), Some(Owner::User), "{key}");
    }
    for key in ["styleOverrides", "customCommands", "voiceCommandsEnabled", "voiceCommands", "listBullet"] {
        assert_eq!(owner_of(key), Some(Owner::User), "{key}");
    }
    // L'historique et les statistiques suivent l'utilisateur, avec leurs trois réglages (relecture C1).
    for key in ["keepDictationText", "historyKeepDays", "typingWPM"] {
        assert_eq!(owner_of(key), Some(Owner::User), "{key}");
    }
    for key in ["hotkey", "dictationKeys", "translateKeys", "formatShortcuts", "microphoneUID", "startSoundName", "transcriptionEngine"] {
        assert_eq!(owner_of(key), Some(Owner::Computer), "{key}");
    }
    for key in
        ["cleanupEngine", "lmStudioURL", "whisperIdleMinutes", "cleanupEnabled", "pauseMedia", "uiLanguage", "theme", "captureOpacity"]
    {
        assert_eq!(owner_of(key), Some(Owner::Computer), "{key}");
    }
    for key in ["trialCandidates", "trialText", "checkUpdates", "setupDone", "schemaVersion"] {
        assert_eq!(owner_of(key), Some(Owner::Computer), "{key}");
    }
    assert_eq!(owner_of("unknownSetting"), None);
}

#[test]
fn split_then_from_parts_gives_back_the_same_settings() {
    let original = AppSettings {
        first_name: "Léa".into(),
        typing_wpm: 55,
        history_keep_days: Some(14),
        hidden_modes: vec![],
        pause_media: true,
        sound_volume: 0.25,
        ..AppSettings::default()
    };
    let (computer, user) = original.split();
    assert!(user.keys().all(|k| owner_of(k) == Some(Owner::User)) && computer.keys().all(|k| owner_of(k) == Some(Owner::Computer)));
    assert!(user.contains_key("firstName") && !computer.contains_key("firstName"));
    assert!(computer.contains_key("pauseMedia") && !user.contains_key("pauseMedia"));
    assert_eq!(user.len() + computer.len(), every_key_written().len() - 1, "customModeID est absent tant qu'il n'a pas de valeur");
    let back = AppSettings::from_parts(&Value::Object(computer), Some(&Value::Object(user))).unwrap();
    assert_eq!(back, original);
}

#[test]
fn without_a_user_file_the_original_settings_json_is_read_as_is() {
    let legacy = json!({"firstName": "Léa", "language": "en", "pauseMedia": true});
    let view = AppSettings::from_parts(&legacy, None).unwrap();
    // Les candidats du banc d'essai par défaut reçoivent un identifiant tiré au hasard à chaque lecture : on les met de côté.
    let direct = AppSettings { trial_candidates: view.trial_candidates.clone(), ..AppSettings::from_value(&legacy).unwrap() };
    assert_eq!(view, direct);
    assert_eq!(view.first_name, "Léa");
}

#[test]
fn a_user_file_decides_every_user_key_and_stray_ones_in_the_computer_file_are_ignored() {
    let computer = json!({"firstName": "Ancien", "language": "en", "historyKeepDays": 7, "pauseMedia": true, "theme": "dark"});
    let user = json!({"firstName": "Léa", "pauseMedia": false, "theme": "light"});
    let view = AppSettings::from_parts(&computer, Some(&user)).unwrap();
    assert_eq!(view.first_name, "Léa", "l'utilisateur fait foi");
    assert_eq!(view.language, AppSettings::default().language, "une clé absente du fichier de l'utilisateur prend sa valeur par défaut");
    assert_eq!(view.history_keep_days, None, "même une clé facultative : pas de retour au fichier de l'ordinateur");
    assert!(view.pause_media, "une clé de l'ordinateur ne se lit pas dans le fichier de l'utilisateur");
    assert_eq!(serde_json::to_value(view.theme).unwrap(), json!("dark"));
}

#[test]
fn a_user_file_that_is_not_an_object_gives_no_view() {
    assert!(AppSettings::from_parts(&json!({}), Some(&json!([1]))).is_none());
}
