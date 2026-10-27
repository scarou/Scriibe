//! Les réglages propres à Scriibe (absents de Scribe) : appareil et durée de conservation de llama.cpp.

use scriibe_core::settings::{is_device_name, AppSettings, LLAMA_IDLE_DEFAULT};

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
