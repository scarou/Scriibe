//! Tests du fichier d'un utilisateur entier (étape 5) : aucun secret ni historique à l'export, aller-retour identique, refus de ce qui est mal
//! formé, règles de collision de l'import (identifiants neufs, titres numérotés, raccourcis contrôlés).

use super::*;
use crate::commands::{CustomCommand, VoicePhrases};
use crate::dictionary::Correction;
use crate::modes::{Fidelity, ModeExample};
use crate::settings::{DictationLanguage, USER_KEYS};

const DAY: &str = "2026-11-14";
/// Des identifiants de la forme des vrais (UUID) : un identifiant d'une autre forme est remplacé dès la lecture du fichier.
const MODE_A: &str = "4F6D1B7E-0C1A-4E55-9C0B-2A7F3C9D8E11";
const MODE_B: &str = "7B2E9A40-5D13-4C86-8F1A-6E0D4B3C2A99";

fn mode(id: &str, name: &str, style: &str, examples: usize) -> CustomMode {
    CustomMode {
        id: id.into(),
        name: name.into(),
        style: style.into(),
        fidelity: Fidelity::Reformulated,
        vouvoie: true,
        examples: (0..examples)
            .map(|n| ModeExample { id: format!("EX-{id}-{n}"), dictation: format!("dictée {n}"), expected: format!("attendu {n}") })
            .collect(),
    }
}

fn command(name: &str, phrase: &str) -> CustomCommand {
    CustomCommand {
        id: "custom-1".into(),
        name: name.into(),
        phrases: vec![phrase.into()],
        shortcut: "Mod+Shift+X".into(),
        needs_selection: false,
    }
}

fn dictionary() -> UserDictionary {
    UserDictionary {
        words: vec!["Scriibe".into(), "Claude Code".into()],
        corrections: vec![Correction { heard: "cri be".into(), written: "Scriibe".into() }],
    }
}

/// Un utilisateur complet, avec des valeurs reconnaissables dans les champs de l'ordinateur et de l'historique.
fn full_user() -> AppSettings {
    let mut voice_commands = VoicePhrases::default();
    voice_commands.fr.insert("bold".into(), vec!["gras".into()]);
    AppSettings {
        first_name: "Léa".into(),
        language: DictationLanguage::En,
        speaker_gender: crate::modes::Gender::Feminine,
        list_bullet: crate::spoken_signs::ListBullet::Star,
        voice_commands_enabled: false,
        voice_commands,
        custom_modes: vec![mode(MODE_A, "SMS", "Court.", 2), mode(MODE_B, "Compte rendu", "Formel.", 1)],
        custom_mode_id: Some(MODE_B.into()),
        hidden_modes: vec!["tech".into(), MODE_A.into()],
        style_overrides: [("formal".to_string(), "Plus soutenu.".to_string())].into_iter().collect(),
        custom_commands: vec![command("Barrer", "barre ça")],
        // Ce qui ne doit jamais partir.
        lm_studio_url: "http://hote-secret:1234".into(),
        whisper_model_path: "/chemin/secret/modele.bin".into(),
        microphone_name: "Micro secret".into(),
        trial_text: "texte d'essai secret".into(),
        typing_wpm: 99,
        keep_dictation_text: false,
        history_keep_days: Some(7),
        ..AppSettings::default()
    }
}

fn content_of(file: &Value) -> &Map<String, Value> {
    file["content"].as_object().unwrap()
}

#[test]
fn the_export_has_the_agreed_envelope_and_only_the_day() {
    let file = export_profile("Léa", &full_user(), &dictionary(), Categories::export_default(), "0.5.0", DAY);
    assert_eq!(
        (file["format"].as_str(), file["version"].as_u64(), file["kind"].as_str()),
        (Some("scriibe-export"), Some(1), Some("profile"))
    );
    assert_eq!((file["app"].as_str(), file["exportedAt"].as_str()), (Some("Scriibe 0.5.0"), Some(DAY)));
    assert_eq!(file_name(DAY), "scriibe-utilisateur-2026-11-14.scriibe.json", "pas de nom d'utilisateur dans le nom du fichier");
}

#[test]
fn nothing_secret_nothing_of_the_history_and_nothing_of_the_computer_leaves() {
    let file =
        export_profile("Léa", &full_user(), &dictionary(), Categories { identity: true, ..Categories::export_default() }, "0.5.0", DAY);
    let text = serde_json::to_string(&file).unwrap();
    for secret in
        ["hote-secret", "chemin/secret", "Micro secret", "texte d'essai secret", "keepDictationText", "historyKeepDays", "typingWPM"]
    {
        assert!(!text.contains(secret), "{secret} ne doit pas partir : {text}");
    }
    // Les clés du contenu sont toutes dans la liste blanche : rien d'autre n'est écrit.
    let allowed: Vec<&str> = [
        &PREFERENCE_KEYS[..],
        &["name", "firstName", "hiddenModes", "dictionary", "customModes", "customModeID", "styleOverrides", "customCommands"],
    ]
    .concat();
    for key in content_of(&file).keys() {
        assert!(allowed.contains(&key.as_str()), "clé hors liste blanche : {key}");
    }
    // Aucune clé d'ordinateur ni d'API.
    for key in content_of(&file).keys() {
        assert!(!["hotkey", "lmStudioURL", "microphoneUID", "theme", "openRouterKey", "deepgramKey"].contains(&key.as_str()));
    }
}

#[test]
fn the_display_name_and_the_first_name_are_off_by_default_and_leave_only_when_ticked() {
    assert!(!Categories::export_default().identity, "décision 5 : décochée par défaut");
    let without = export_profile("Léa", &full_user(), &dictionary(), Categories::export_default(), "0.5.0", DAY);
    assert!(content_of(&without).get("name").is_none() && content_of(&without).get("firstName").is_none());
    assert!(!serde_json::to_string(&without).unwrap().contains("Léa"), "le prénom ne figure nulle part ailleurs dans le fichier");
    let with =
        export_profile("Léa", &full_user(), &dictionary(), Categories { identity: true, ..Categories::export_default() }, "0.5.0", DAY);
    assert_eq!((content_of(&with)["name"].as_str(), content_of(&with)["firstName"].as_str()), (Some("Léa"), Some("Léa")));
}

#[test]
fn an_unticked_category_has_no_key_and_examples_follow_their_own_box() {
    let none = Categories {
        identity: false,
        preferences: false,
        words: false,
        corrections: false,
        modes: false,
        examples: false,
        styles: false,
        commands: false,
    };
    assert!(content_of(&export_profile("Léa", &full_user(), &dictionary(), none, "0.5.0", DAY)).is_empty());
    let only_words = export_profile("Léa", &full_user(), &dictionary(), Categories { words: true, ..none }, "0.5.0", DAY);
    assert_eq!(content_of(&only_words).keys().collect::<Vec<_>>(), vec!["dictionary"]);
    assert!(only_words["content"]["dictionary"].get("corrections").is_none());
    let no_examples =
        export_profile("Léa", &full_user(), &dictionary(), Categories { examples: false, ..Categories::export_default() }, "0.5.0", DAY);
    assert!(no_examples["content"]["customModes"].as_array().unwrap().iter().all(|m| m["examples"].as_array().unwrap().is_empty()));
    let with_examples = export_profile("Léa", &full_user(), &dictionary(), Categories::export_default(), "0.5.0", DAY);
    assert_eq!(with_examples["content"]["customModes"][0]["examples"].as_array().unwrap().len(), 2);
}

#[test]
fn hidden_custom_modes_and_the_chosen_one_do_not_leave_without_their_modes() {
    let file =
        export_profile("Léa", &full_user(), &dictionary(), Categories { modes: false, ..Categories::export_default() }, "0.5.0", DAY);
    let content = content_of(&file);
    assert!(content.get("customModes").is_none() && content.get("customModeID").is_none());
    assert_eq!(content["hiddenModes"], json!(["tech"]), "l'identifiant du mode personnel masqué est retiré, le mode livré reste");
}

#[test]
fn the_command_ids_and_the_computer_time_in_them_do_not_leave() {
    let file = export_profile("Léa", &full_user(), &dictionary(), Categories::export_default(), "0.5.0", DAY);
    let command = &file["content"]["customCommands"][0];
    assert!(command.get("id").is_none());
    assert_eq!(command["name"], "Barrer");
}

/// L'essentiel d'un utilisateur, sans les identifiants (qui changent à l'import).
fn essence(s: &AppSettings) -> Value {
    json!({
        "firstName": s.first_name, "language": s.language.raw(), "gender": format!("{:?}", s.speaker_gender), "bullet": format!("{:?}", s.list_bullet),
        "voiceOn": s.voice_commands_enabled, "voice": s.voice_commands,
        "modes": s.custom_modes.iter().map(|m| json!({"name": m.name, "style": m.style, "fidelity": m.fidelity.raw(), "vouvoie": m.vouvoie,
            "examples": m.examples.iter().map(|e| json!([e.dictation, e.expected])).collect::<Vec<_>>()})).collect::<Vec<_>>(),
        "chosen": s.custom_mode_id.as_ref().and_then(|id| s.custom_modes.iter().find(|m| &m.id == id)).map(|m| m.name.clone()),
        "hidden": s.hidden_modes.iter().map(|h| s.custom_modes.iter().find(|m| &m.id == h).map_or(h.clone(), |m| format!("custom:{}", m.name))).collect::<Vec<_>>(),
        "styles": s.style_overrides,
        "commands": s.custom_commands.iter().map(|c| json!([c.name, c.phrases, c.shortcut, c.needs_selection])).collect::<Vec<_>>(),
    })
}

fn read_back(built: &Built, name: &str) -> AppSettings {
    let mut profile = built.profile.clone();
    profile.insert("name".into(), json!(name));
    AppSettings::from_value(&Value::Object(profile)).unwrap()
}

#[test]
fn an_export_read_back_gives_the_same_user_with_new_identifiers() {
    let source = full_user();
    let file = export_profile("Léa", &source, &dictionary(), Categories { identity: true, ..Categories::export_default() }, "0.5.0", DAY);
    let incoming = parse_profile(&serde_json::to_string(&file).unwrap()).unwrap();
    let built = build(&incoming, Categories::import_default());
    let back = read_back(&built, "Léa");
    assert_eq!(essence(&back), essence(&source), "l'essentiel est identique");
    assert_eq!(built.dictionary, dictionary());
    // Tous les identifiants sont neufs, le mode choisi et le mode masqué suivent la table.
    let old: Vec<&str> =
        source.custom_modes.iter().flat_map(|m| std::iter::once(m.id.as_str()).chain(m.examples.iter().map(|e| e.id.as_str()))).collect();
    for m in &back.custom_modes {
        assert!(!old.contains(&m.id.as_str()) && m.examples.iter().all(|e| !old.contains(&e.id.as_str())));
    }
    assert_eq!(
        back.custom_mode_id,
        Some(back.custom_modes[1].id.clone()),
        "« Compte rendu » reste le mode choisi sous son nouvel identifiant"
    );
    assert!(back.hidden_modes.contains(&back.custom_modes[0].id) && back.hidden_modes.contains(&"tech".to_string()));
    assert!(
        back.custom_commands.iter().all(|c| c.id != "custom-1" && c.id.starts_with(crate::commands::CUSTOM_PREFIX)),
        "identifiant de commande neuf"
    );
    // Le fichier réexporté dit la même chose (aux identifiants près).
    let again =
        export_profile("Léa", &back, &built.dictionary, Categories { identity: true, ..Categories::export_default() }, "0.5.0", DAY);
    let strip = |mut v: Value| {
        fn walk(v: &mut Value) {
            match v {
                Value::Object(o) => {
                    o.remove("id");
                    o.remove("customModeID");
                    o.remove("hiddenModes");
                    o.values_mut().for_each(walk);
                }
                Value::Array(a) => a.iter_mut().for_each(walk),
                _ => {}
            }
        }
        walk(&mut v);
        v
    };
    assert_eq!(strip(again), strip(file));
}

#[test]
fn what_the_import_writes_is_only_user_keys_and_never_a_computer_field_from_the_file() {
    let mut file = export_profile("Léa", &full_user(), &dictionary(), Categories::export_default(), "0.5.0", DAY);
    // Un fichier d'un tiers qui glisse des clés d'ordinateur, d'historique et de clé d'API dans le contenu.
    let content = file["content"].as_object_mut().unwrap();
    for (key, value) in [
        ("lmStudioURL", json!("http://pirate:1")),
        ("hotkey", json!("fn")),
        ("openRouterKey", json!("sk-vol")),
        ("keepDictationText", json!(false)),
        ("typingWPM", json!(1)),
        ("history", json!([{"text": "x"}])),
        ("activeProfile", json!("AAAA")),
        ("schemaVersion", json!(99)),
    ] {
        content.insert(key.into(), value);
    }
    let built = build(&parse_profile(&file.to_string()).unwrap(), Categories::import_default());
    for key in built.profile.keys() {
        assert!(USER_KEYS.contains(&key.as_str()), "clé qui n'est pas à l'utilisateur : {key}");
    }
    let text = serde_json::to_string(&built.profile).unwrap();
    assert!(!text.contains("pirate") && !text.contains("sk-vol") && !text.contains("activeProfile") && !text.contains("schemaVersion"));
    for key in ["keepDictationText", "historyKeepDays", "typingWPM"] {
        assert!(built.profile.get(key).is_none(), "{key} (statistiques) ne vient jamais d'un fichier");
    }
}

#[test]
fn the_identity_box_decides_the_first_name_at_import() {
    let file =
        export_profile("Léa", &full_user(), &dictionary(), Categories { identity: true, ..Categories::export_default() }, "0.5.0", DAY);
    let incoming = parse_profile(&file.to_string()).unwrap();
    assert_eq!((incoming.name.as_deref(), incoming.first_name.as_deref()), (Some("Léa"), Some("Léa")));
    assert_eq!(build(&incoming, Categories::import_default()).profile["firstName"], "Léa");
    assert!(build(&incoming, Categories { identity: false, ..Categories::import_default() }).profile.get("firstName").is_none());
}

#[test]
fn an_unticked_category_is_not_imported() {
    let file = export_profile("Léa", &full_user(), &dictionary(), Categories::export_default(), "0.5.0", DAY);
    let incoming = parse_profile(&file.to_string()).unwrap();
    let no_modes = build(&incoming, Categories { modes: false, ..Categories::import_default() });
    assert!(no_modes.profile.get("customModes").is_none() && no_modes.profile.get("customModeID").is_none());
    let hidden = no_modes.profile["hiddenModes"].as_array().unwrap();
    assert!(hidden.iter().all(|h| h == "tech"), "l'identifiant du mode masqué n'existe plus : il est retiré à la lecture");
    let no_words = build(&incoming, Categories { words: false, corrections: false, ..Categories::import_default() });
    assert_eq!(no_words.dictionary, UserDictionary::default());
    let nothing = build(
        &incoming,
        Categories {
            identity: false,
            preferences: false,
            words: false,
            corrections: false,
            modes: false,
            examples: false,
            styles: false,
            commands: false,
        },
    );
    assert!(nothing.profile.is_empty() && nothing.dictionary == UserDictionary::default());
    let no_examples = build(&incoming, Categories { examples: false, ..Categories::import_default() });
    assert_eq!(no_examples.outcome.examples, 0);
}

#[test]
fn modes_with_the_same_name_are_numbered_and_commands_are_checked() {
    let s = AppSettings {
        custom_modes: vec![mode("A", "SMS", "un", 0), mode("B", "SMS", "deux", 0)],
        custom_commands: vec![
            command("Barrer", "barre ça"),
            CustomCommand { shortcut: "pas un raccourci".into(), ..command("Cassée", "phrase cassée") },
            command("Autre", "barre ça"),
        ],
        ..AppSettings::default()
    };
    let file = export_profile("x", &s, &UserDictionary::default(), Categories::export_default(), "0.5.0", DAY);
    let built = build(&parse_profile(&file.to_string()).unwrap(), Categories::import_default());
    assert_eq!(built.outcome.modes, vec!["SMS", "SMS 2"], "titres numérotés comme à l'import de modes");
    assert_eq!(built.outcome.commands, vec!["Barrer"], "raccourci illisible écarté, phrase déjà prise par une autre commande écartée");
    assert_eq!(built.outcome.commands_skipped, 2);
}

#[test]
fn styles_that_do_not_belong_to_a_built_in_mode_are_dropped() {
    let mut file = export_profile("x", &full_user(), &dictionary(), Categories::export_default(), "0.5.0", DAY);
    file["content"]["styleOverrides"] = json!({"formal": "Plus soutenu.", "inconnu": "x", "tech": "   "});
    let incoming = parse_profile(&file.to_string()).unwrap();
    assert_eq!(preview(&incoming).styles, vec!["formal"]);
    assert_eq!(build(&incoming, Categories::import_default()).profile["styleOverrides"], json!({"formal": "Plus soutenu."}));
}

#[test]
fn hiding_every_mode_is_undone_at_reading() {
    let all: Vec<&str> = RewriteMode::ALL.iter().map(|m| m.raw()).collect();
    let mut file = export_profile("x", &AppSettings::default(), &dictionary(), Categories::export_default(), "0.5.0", DAY);
    file["content"]["hiddenModes"] = json!(all);
    let built = build(&parse_profile(&file.to_string()).unwrap(), Categories::import_default());
    assert!(built.profile.get("hiddenModes").is_none_or(|h| h.as_array().unwrap().is_empty()), "jamais tous les modes masqués");
}

#[test]
fn the_dictionary_import_keeps_its_limits() {
    let mut file = export_profile("x", &full_user(), &dictionary(), Categories::export_default(), "0.5.0", DAY);
    file["content"]["dictionary"]["words"] = json!((0..501).map(|n| format!("mot{n}")).collect::<Vec<_>>());
    assert!(parse_profile(&file.to_string()).unwrap_err().contains("trop de mots"));
    file["content"]["dictionary"]["words"] = json!(["x".repeat(101)]);
    assert!(parse_profile(&file.to_string()).unwrap_err().contains("trop long"));
    file["content"]["dictionary"]["words"] = json!(["Scriibe", "scriibe", " Scriibe "]);
    let incoming = parse_profile(&file.to_string()).unwrap();
    assert_eq!(incoming.dictionary.words, vec!["Scriibe"], "doublons du fichier écartés comme à l'étape 2");
}

#[test]
fn files_that_are_not_a_user_or_are_malformed_are_refused_without_effect() {
    let good = export_profile("x", &full_user(), &dictionary(), Categories::export_default(), "0.5.0", DAY);
    let with = |edit: &dyn Fn(&mut Value)| {
        let mut file = good.clone();
        edit(&mut file);
        parse_profile(&file.to_string())
    };
    assert!(parse_profile("pas du json").unwrap_err().contains("JSON"));
    assert!(parse_profile(&"x".repeat(MAX_BYTES as usize + 1)).unwrap_err().contains("trop gros"));
    assert!(with(&|f| f["kind"] = json!("dictionary")).unwrap_err().contains("autre type"));
    assert!(with(&|f| f["version"] = json!(2)).unwrap_err().contains("plus récente"));
    assert!(with(&|f| f["format"] = json!("autre")).unwrap_err().contains("pas un fichier d'export"));
    assert!(with(&|f| f["content"] = json!([1])).unwrap_err().contains("pas d'utilisateur"));
    assert!(with(&|f| f["content"] = json!({})).unwrap_err().contains("aucun contenu"));
    assert!(
        with(&|f| f["content"] = json!({"lmStudioURL": "x"})).unwrap_err().contains("aucun contenu"),
        "des clés inconnues ne sont pas un contenu"
    );
    assert!(with(&|f| f["content"]["name"] = json!("n".repeat(41))).unwrap_err().contains("40 caractères"));
    assert!(with(&|f| f["content"]["firstName"] = json!(5)).unwrap_err().contains("mal formé"));
    assert!(with(&|f| f["content"]["voiceCommands"] = json!("x")).unwrap_err().contains("mal formées"));
    assert!(with(&|f| f["content"]["voiceCommands"] = json!({"fr": {"bold": ["x".repeat(201)]}})).unwrap_err().contains("trop longue"));
    assert!(with(&|f| f["content"]["voiceCommands"] = json!({"fr": {"bold": (0..21).map(|n| format!("p{n}")).collect::<Vec<_>>()}}))
        .unwrap_err()
        .contains("trop de phrases"));
    assert!(with(&|f| f["content"]["hiddenModes"] = json!("tech")).unwrap_err().contains("mal formée"));
    assert!(with(&|f| f["content"]["hiddenModes"] = json!((0..101).map(|n| n.to_string()).collect::<Vec<_>>()))
        .unwrap_err()
        .contains("trop"));
    assert!(with(
        &|f| f["content"]["customModes"] = json!((0..51).map(|n| json!({"name": n.to_string(), "fidelity": "light"})).collect::<Vec<_>>())
    )
    .unwrap_err()
    .contains("50 au maximum"));
    assert!(with(&|f| f["content"]["customModes"] = json!([{"name": "x", "fidelity": "inconnu"}])).unwrap_err().contains("fidélité"));
    assert!(with(&|f| f["content"]["customCommands"] = json!((0..101).map(|n| json!({"name": n.to_string()})).collect::<Vec<_>>()))
        .unwrap_err()
        .contains("100 au maximum"));
    assert!(with(&|f| f["content"]["styleOverrides"] = json!({"formal": 3})).unwrap_err().contains("mal formées"));
    assert!(with(&|f| f["content"]["styleOverrides"] = json!({"formal": "x".repeat(20_001)})).unwrap_err().contains("trop longue"));
}

#[test]
fn the_preview_counts_each_category_and_lists_modes_and_commands() {
    let file =
        export_profile("Léa", &full_user(), &dictionary(), Categories { identity: true, ..Categories::export_default() }, "0.5.0", DAY);
    let p = preview(&parse_profile(&file.to_string()).unwrap());
    assert_eq!((p.name.as_deref(), p.first_name.as_deref()), (Some("Léa"), Some("Léa")));
    assert_eq!((p.words, p.corrections, p.hidden_modes), (2, 1, 2));
    assert_eq!(p.modes.iter().map(|m| m.name.as_str()).collect::<Vec<_>>(), vec!["SMS", "Compte rendu"]);
    assert_eq!(p.modes[0].examples, 2);
    assert_eq!(p.commands.len(), 1);
    assert_eq!(p.styles, vec!["formal"]);
    assert!(p.preferences.contains(&"langue dictée".to_string()));
}

#[test]
fn the_inventory_of_a_user_lists_what_would_leave() {
    let c = contents("Léa", &full_user(), &dictionary());
    assert_eq!((c.words.len(), c.corrections.len(), c.examples), (2, 1, 3));
    assert_eq!(c.modes, vec!["SMS", "Compte rendu"]);
    assert_eq!(c.commands, vec!["Barrer"]);
    assert_eq!(c.styles, vec!["formal"]);
    assert!(c.preferences.contains(&"modes masqués du menu".to_string()) && c.preferences.contains(&"genre".to_string()));
    let blank = contents("Principal", &AppSettings::default(), &UserDictionary::default());
    assert!(blank.words.is_empty() && blank.modes.is_empty() && blank.commands.is_empty() && blank.styles.is_empty());
}
