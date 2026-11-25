//! Les réglages propres à Scriibe (absents de Scribe) : appareil et durée de conservation de llama.cpp.

use scriibe_core::settings::{
    is_device_name, is_profile_id, AppSettings, CaptureColor, Theme, UiLanguage, IDLE_CHOICES, LEGACY_SCHEMA, LLAMA_IDLE_DEFAULT,
    SETTINGS_SCHEMA,
};

#[test]
fn llama_options_default_to_scribes_behaviour_and_survive_a_round_trip() {
    let defaults = AppSettings::default();
    assert_eq!((defaults.llama_device.as_str(), defaults.llama_idle_minutes), ("", LLAMA_IDLE_DEFAULT));

    let settings = AppSettings { llama_device: "Vulkan1".into(), llama_idle_minutes: 120, ..AppSettings::default() };
    let back = AppSettings::from_json(&serde_json::to_string(&settings).unwrap()).unwrap();
    assert_eq!((back.llama_device.as_str(), back.llama_idle_minutes), ("Vulkan1", 120));

    // Un fichier de Scribe n'a pas ces champs : valeurs par défaut.
    let from_scribe = AppSettings::from_json(r#"{"language": "fr"}"#).unwrap();
    assert_eq!((from_scribe.llama_device.as_str(), from_scribe.llama_idle_minutes), ("", LLAMA_IDLE_DEFAULT));
}

#[test]
fn llama_options_written_by_hand_are_made_safe() {
    let s = AppSettings::from_json(r#"{"llamaDevice": "Vulkan0; rm -rf", "llamaIdleMinutes": 99999}"#).unwrap();
    assert_eq!(s.llama_device, "", "un nom d'appareil avec autre chose que lettres, chiffres, _ est refusé");
    assert_eq!(s.llama_idle_minutes, 24 * 60);
    assert!(is_device_name("none") && is_device_name("Vulkan1") && !is_device_name("a b"));
}

#[test]
fn interface_language_defaults_to_automatic_and_survives_a_round_trip() {
    assert_eq!(AppSettings::default().ui_language, UiLanguage::Auto);
    let settings = AppSettings { ui_language: UiLanguage::En, ..AppSettings::default() };
    let json = serde_json::to_string(&settings).unwrap();
    assert!(json.contains(r#""uiLanguage":"en""#));
    assert_eq!(AppSettings::from_json(&json).unwrap().ui_language, UiLanguage::En);
    // Un fichier de Scribe, ou une valeur inconnue, laisse l'automatique.
    assert_eq!(AppSettings::from_json(r#"{"language": "fr"}"#).unwrap().ui_language, UiLanguage::Auto);
    assert_eq!(AppSettings::from_json(r#"{"uiLanguage": "klingon"}"#).unwrap().ui_language, UiLanguage::Auto);
}

#[test]
fn the_window_theme_defaults_to_the_system_one_and_survives_a_round_trip() {
    assert_eq!(AppSettings::default().theme, Theme::Auto);
    let settings = AppSettings { theme: Theme::Dark, ..AppSettings::default() };
    let json = serde_json::to_string(&settings).unwrap();
    assert!(json.contains(r#""theme":"dark""#));
    assert_eq!(AppSettings::from_json(&json).unwrap().theme, Theme::Dark);
    assert_eq!(AppSettings::from_json(r#"{"language": "fr"}"#).unwrap().theme, Theme::Auto);
    assert_eq!(AppSettings::from_json(r#"{"theme": "sepia"}"#).unwrap().theme, Theme::Auto);
}

#[test]
fn the_animated_wave_is_on_by_default_apart_from_the_logo_and_is_kept() {
    assert!(AppSettings::default().animated_wave);
    assert!(AppSettings::from_json("{}").unwrap().animated_wave);
    let off = AppSettings::from_json(r#"{"animatedWave": false}"#).unwrap();
    assert!(!off.animated_wave && off.animated_logo, "les deux réglages sont indépendants");
    let back = AppSettings::from_json(&serde_json::to_string(&off).unwrap()).unwrap();
    assert!(!back.animated_wave);
}

#[test]
fn the_animated_logo_is_on_by_default_and_is_kept() {
    assert!(AppSettings::default().animated_logo);
    assert!(AppSettings::from_json("{}").unwrap().animated_logo);
    let off = AppSettings::from_json(r#"{"animatedLogo": false}"#).unwrap();
    assert!(!off.animated_logo);
    let back = AppSettings::from_json(&serde_json::to_string(&off).unwrap()).unwrap();
    assert!(!back.animated_logo);
}

#[test]
fn the_update_check_is_off_by_default_and_can_be_turned_on() {
    assert!(!AppSettings::default().check_updates);
    let on = AppSettings::from_json(r#"{"checkUpdates": true}"#).unwrap();
    assert!(on.check_updates);
    let back = AppSettings::from_json(&serde_json::to_string(&on).unwrap()).unwrap();
    assert!(back.check_updates);
    assert!(!AppSettings::from_json("{}").unwrap().check_updates);
}

#[test]
fn format_shortcuts_default_round_trip_and_are_corrected_when_unreadable() {
    use scriibe_core::settings::AppSettings;
    let d = AppSettings::default();
    assert_eq!(
        (d.format_shortcuts.bold.as_str(), d.format_shortcuts.italic.as_str(), d.format_shortcuts.underline.as_str()),
        ("Mod+B", "Mod+I", "Mod+U")
    );
    let json = serde_json::to_string(&d).unwrap();
    assert!(json.contains("\"formatShortcuts\""));
    assert_eq!(AppSettings::from_json(&json).unwrap().format_shortcuts, d.format_shortcuts);
    let own = AppSettings::from_json(r#"{"formatShortcuts": {"bold": "Ctrl+Alt+G", "italic": "x"}}"#).unwrap();
    assert_eq!(
        (own.format_shortcuts.bold.as_str(), own.format_shortcuts.italic.as_str(), own.format_shortcuts.underline.as_str()),
        ("Ctrl+Alt+G", "Mod+I", "Mod+U")
    );
    assert_eq!(AppSettings::from_json(r#"{"formatShortcuts": 3}"#).unwrap().format_shortcuts, d.format_shortcuts);
}

#[test]
fn one_badly_written_custom_command_is_dropped_alone() {
    use scriibe_core::settings::AppSettings;
    let json = r#"{"customCommands": [
        {"id": "perso.1", "name": "Nouveau", "phrases": ["nouveau document"], "shortcut": "Mod+N"},
        {"id": "perso.2", "name": "Sans raccourci", "phrases": ["sans raccourci ici"]},
        {"id": "perso.3", "name": "Quitter", "phrases": ["quitter tout"], "shortcut": "Mod+Q"}
    ]}"#;
    let s = AppSettings::from_json(json).unwrap();
    let ids: Vec<_> = s.custom_commands.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids, vec!["perso.1"], "la commande sans raccourci et celle qui quitte l'appli sont écartées, pas la liste entière");
}

#[test]
fn the_schema_version_defaults_to_one_is_read_when_present_and_is_written_first() {
    // Version 2 : rangement par utilisateur (étape 4b). Seule la migration écrit 2 : sans numéro, ou par défaut, c'est la version 1.
    assert_eq!((SETTINGS_SCHEMA, LEGACY_SCHEMA), (2, 1));
    assert_eq!(AppSettings::default().schema_version, LEGACY_SCHEMA, "des réglages par défaut ne font pas croire à une migration");
    assert_eq!(AppSettings::from_json("{}").unwrap().schema_version, LEGACY_SCHEMA, "un fichier sans numéro est de la version 1");
    assert_eq!(AppSettings::from_json(r#"{"schemaVersion": 7}"#).unwrap().schema_version, 7);
    assert_eq!(AppSettings::from_json(r#"{"schemaVersion": "deux"}"#).unwrap().schema_version, LEGACY_SCHEMA, "illisible : version 1");
    let json = serde_json::to_string_pretty(&AppSettings::default()).unwrap();
    assert!(json.trim_start_matches("{\n").trim_start().starts_with("\"schemaVersion\": 1"), "le numéro ouvre le fichier");
}

#[test]
fn the_active_profile_is_read_only_if_it_is_a_safe_folder_name() {
    let read = |json: &str| AppSettings::from_json(json).unwrap().active_profile;
    assert_eq!(read(r#"{"activeProfile": "A1B2-C3"}"#).as_deref(), Some("A1B2-C3"));
    for bad in [r#""../x""#, r#"".migration-1""#, r#""a/b""#, r#""a\b""#, r#""""#, "42", r#""-x""#] {
        assert_eq!(read(&format!(r#"{{"activeProfile": {bad}}}"#)), None, "{bad}");
    }
    assert!(
        !serde_json::to_string(&AppSettings::default()).unwrap().contains("activeProfile"),
        "absent tant qu'il n'y a pas d'utilisateur"
    );
    assert!(is_profile_id(&"A".repeat(64)) && !is_profile_id(&"A".repeat(65)));
}

#[test]
fn idle_choices_are_the_same_for_both_engines_and_never_is_zero() {
    assert_eq!(IDLE_CHOICES, [1, 2, 5, 10, 15, 30, 60, 0]);
    for m in IDLE_CHOICES {
        let json = format!(r#"{{"whisperIdleMinutes": {m}, "llamaIdleMinutes": {m}}}"#);
        let s = AppSettings::from_json(&json).unwrap();
        assert_eq!((s.whisper_idle_minutes, s.llama_idle_minutes), (m, m), "{m} min relu tel quel");
    }
}

#[test]
fn old_idle_settings_are_read_as_they_were() {
    // Anciens menus : Whisper 0, 5, 10, 30, 60 ; llama.cpp 30 min, 2 h, 8 h, « tant que Scriibe tourne » (0).
    for m in [0, 5, 10, 30, 60] {
        let s = AppSettings::from_json(&format!(r#"{{"whisperIdleMinutes": {m}}}"#)).unwrap();
        assert_eq!(s.whisper_idle_minutes, m);
    }
    for m in [30, 120, 480, 0] {
        let s = AppSettings::from_json(&format!(r#"{{"llamaIdleMinutes": {m}}}"#)).unwrap();
        assert_eq!(s.llama_idle_minutes, m);
    }
    assert_eq!(AppSettings::default().whisper_idle_minutes, 10, "défauts inchangés");
}

#[test]
fn old_and_new_indicator_colors_are_read() {
    for c in ["scribe", "gray", "red", "blue", "black", "orange", "green", "teal", "purple", "pink", "indigo", "brown"] {
        let s = AppSettings::from_json(&format!(r#"{{"captureColor": "{c}"}}"#)).unwrap();
        assert_eq!(serde_json::to_value(s.capture_color).unwrap(), c);
    }
    let s = AppSettings::from_json(r#"{"captureColor": "fuchsia"}"#).unwrap();
    assert_eq!(s.capture_color, CaptureColor::Scribe, "une couleur inconnue (pas de couleur libre) : celle de Scriibe");
}
