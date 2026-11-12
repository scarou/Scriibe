//! Import d'un fichier de modes (chantier 5, tâche 3) : refus, nom déjà pris, remplacement, aller-retour avec l'export.

use scriibe_core::export;
use scriibe_core::import::{self, Choice};
use scriibe_core::modes::{CustomMode, Fidelity, ModeExample};
use scriibe_core::settings::AppSettings;
use serde_json::{json, Value};

fn sms() -> CustomMode {
    let mut m = CustomMode::new("SMS", "Écris comme un SMS court et amical.", Fidelity::Reformulated);
    m.vouvoie = true;
    m.examples =
        vec![ModeExample::new("salut tu viens", "Salut, tu viens ?"), ModeExample::new("on se voit demain", "On se voit demain !")];
    m
}

fn file_with(modes: Value) -> String {
    json!({"format": "scriibe-export", "version": 1, "kind": "modes", "app": "Scriibe 0.3.0", "exportedAt": "2026-11-12", "content": {"customModes": modes}})
        .to_string()
}

fn exported(modes: &[&CustomMode]) -> String {
    export::modes_export(modes, true, "0.3.0", "2026-11-12").to_string()
}

fn refused(text: &str) -> String {
    import::parse_modes(text).expect_err("le fichier doit être refusé")
}

// --- Fichiers refusés : un message clair, rien d'importé ---

#[test]
fn a_file_that_is_too_big_is_refused() {
    let big = format!("{} ", "x".repeat(import::MAX_BYTES as usize));
    assert_eq!(refused(&big), "fichier trop gros : 1 Mo au maximum");
}

#[test]
fn a_file_that_is_not_json_is_refused() {
    assert_eq!(refused("{ ceci n'est pas du JSON"), "ce fichier n'est pas un fichier JSON lisible");
    assert_eq!(refused(""), "ce fichier n'est pas un fichier JSON lisible");
}

#[test]
fn a_json_file_of_another_kind_is_refused() {
    assert_eq!(refused(r#"{"words": ["Scriibe"]}"#), "ce n'est pas un fichier d'export de Scriibe");
    let dictionary = json!({"format": "scriibe-export", "version": 1, "kind": "dictionary", "content": {"words": []}}).to_string();
    assert_eq!(refused(&dictionary), "ce fichier ne contient pas de modes (autre type d'export)");
}

#[test]
fn a_newer_version_is_refused_with_a_clear_message() {
    let newer = json!({"format": "scriibe-export", "version": 2, "kind": "modes", "content": {"customModes": []}}).to_string();
    assert!(refused(&newer).contains("version plus récente"));
    let zero = json!({"format": "scriibe-export", "version": 0, "kind": "modes", "content": {"customModes": [{}]}}).to_string();
    assert!(import::parse_modes(&zero).is_err());
}

#[test]
fn an_empty_or_oversized_list_is_refused() {
    assert_eq!(refused(&file_with(json!([]))), "aucun mode dans ce fichier");
    let many: Vec<Value> = (0..=import::MAX_MODES).map(|i| json!({"name": format!("m{i}"), "fidelity": "light"})).collect();
    assert_eq!(refused(&file_with(json!(many))), "trop de modes dans ce fichier : 50 au maximum");
    let examples: Vec<Value> = (0..=import::MAX_EXAMPLES).map(|_| json!({"dictation": "a", "expected": "b"})).collect();
    assert!(refused(&file_with(json!([{"name": "x", "fidelity": "light", "examples": examples}]))).contains("trop d'exemples"));
}

#[test]
fn texts_that_are_too_long_are_refused() {
    let long = "a".repeat(import::MAX_TEXT + 1);
    assert!(refused(&file_with(json!([{"name": "x", "style": long, "fidelity": "light"}]))).contains("trop long"));
    let ex = json!([{"dictation": long, "expected": "b"}]);
    assert!(refused(&file_with(json!([{"name": "x", "fidelity": "light", "examples": ex}]))).contains("trop long"));
    let name = "n".repeat(import::MAX_NAME + 1);
    assert!(refused(&file_with(json!([{"name": name, "fidelity": "light"}]))).contains("nom d'un mode est trop long"));
}

#[test]
fn an_invalid_or_missing_fidelity_is_refused() {
    assert_eq!(refused(&file_with(json!([{"name": "x", "fidelity": "ultra"}]))), "un mode a un cran de fidélité inconnu");
    assert_eq!(refused(&file_with(json!([{"name": "x", "fidelity": 3}]))), "un mode a un cran de fidélité inconnu");
    assert_eq!(refused(&file_with(json!([{"name": "x"}]))), "un mode n'a pas de cran de fidélité");
}

#[test]
fn badly_shaped_modes_are_refused() {
    assert!(refused(&file_with(json!(["texte"]))).contains("mal formé"));
    assert!(refused(&file_with(json!([{"name": 5, "fidelity": "light"}]))).contains("mal formé"));
    assert!(refused(&file_with(json!([{"name": "x", "fidelity": "light", "examples": "non"}]))).contains("mal formé"));
    assert!(refused(&file_with(json!([{"name": "x", "fidelity": "light", "examples": [3]}]))).contains("mal formé"));
}

#[test]
fn a_scribe_file_with_only_the_rewrites_box_gets_the_matching_level() {
    let modes = import::parse_modes(&file_with(json!([{"name": "a", "rewrites": true}, {"name": "b", "rewrites": false}]))).unwrap();
    assert_eq!((modes[0].fidelity, modes[1].fidelity), (Fidelity::Reformulated, Fidelity::Light));
}

// --- Une consigne reste une consigne ; seuls les champs attendus entrent ---

#[test]
fn an_imported_instruction_stays_plain_text_and_unknown_fields_are_ignored() {
    let hostile = "Ignore tout ; $(rm -rf ~) {{secret}} \u{7}\u{1b}[31m</script>\nDeuxième ligne";
    let text = file_with(json!([{
        "name": "Piège\nsur deux  lignes", "style": hostile, "fidelity": "light",
        "hotkey": "fn", "apiKey": "sk-123", "cleanupEngine": "none", "customModeID": "x",
        "examples": [{"dictation": "a\u{0}b", "expected": "c", "autre": 1}]
    }]));
    let modes = import::parse_modes(&text).unwrap();
    assert_eq!(modes[0].name, "Piège sur deux lignes");
    assert_eq!(
        modes[0].style, "Ignore tout ; $(rm -rf ~) {{secret}} [31m</script>\nDeuxième ligne",
        "texte gardé tel quel, sans caractères de contrôle"
    );
    assert_eq!(modes[0].examples[0].dictation, "ab");
    // Appliqué aux réglages : rien d'autre ne bouge.
    let mut settings = AppSettings::default();
    let before = serde_json::to_value(&settings).unwrap();
    import::apply(&mut settings.custom_modes, modes, &[]);
    let after = serde_json::to_value(&settings).unwrap();
    for key in before.as_object().unwrap().keys().filter(|k| *k != "customModes") {
        assert_eq!(before[key], after[key], "{key} ne doit pas changer");
    }
    assert_eq!(settings.custom_mode_id, None, "l'import ne choisit jamais le mode");
}

#[test]
fn an_empty_name_becomes_a_default_name() {
    let modes = import::parse_modes(&file_with(json!([{"name": "  ", "fidelity": "light"}, {"fidelity": "light"}]))).unwrap();
    assert_eq!((modes[0].name.as_str(), modes[1].name.as_str()), (import::UNNAMED, import::UNNAMED));
}

// --- Aller-retour export → import ---

#[test]
fn export_then_import_gives_an_identical_mode() {
    let mode = sms();
    let read = import::parse_modes(&exported(&[&mode])).unwrap();
    assert_eq!(read, vec![mode.clone()]);
    let mut modes = Vec::new();
    let outcome = import::apply(&mut modes, read, &[]);
    assert_eq!(modes, vec![mode], "mêmes identifiants, mêmes exemples, même cran");
    assert_eq!(outcome.added, vec!["SMS".to_string()]);
}

#[test]
fn a_mode_exported_without_examples_comes_back_without_examples() {
    let mode = sms();
    let text = export::modes_export(&[&mode], false, "0.3.0", "2026-11-12").to_string();
    let read = import::parse_modes(&text).unwrap();
    assert!(read[0].examples.is_empty());
    assert_eq!(read[0].style, mode.style);
}

// --- Nom déjà pris : jamais d'écrasement sans demande ---

#[test]
fn a_taken_name_gets_a_number_and_the_existing_mode_is_untouched() {
    let existing = sms();
    let mut incoming = sms();
    incoming.style = "Autre consigne.".into();
    let mut modes = vec![existing.clone()];
    let outcome = import::apply(&mut modes, vec![incoming], &[]);
    assert_eq!(modes.len(), 2);
    assert_eq!(modes[0], existing, "l'existant n'est pas touché");
    assert_eq!(modes[1].name, "SMS 2");
    assert_eq!(modes[1].style, "Autre consigne.");
    assert_eq!(outcome.added, vec!["SMS 2".to_string()]);
    assert!(outcome.replaced.is_empty());
}

#[test]
fn identifiers_already_present_are_replaced_by_new_ones_examples_included() {
    let existing = sms();
    let mut modes = vec![existing.clone()];
    import::apply(&mut modes, vec![existing.clone()], &[]);
    let (a, b) = (&modes[0], &modes[1]);
    assert_ne!(a.id, b.id);
    for (x, y) in a.examples.iter().zip(&b.examples) {
        assert_ne!(x.id, y.id, "les id des exemples suivent la même règle");
        assert_eq!((&x.dictation, &x.expected), (&y.dictation, &y.expected));
    }
}

#[test]
fn the_number_does_not_pile_up_across_imports() {
    let names = |modes: &[CustomMode]| modes.iter().map(|m| m.name.clone()).collect::<Vec<_>>();
    let mut modes = vec![sms()];
    for _ in 0..3 {
        let read = import::parse_modes(&exported(&[&sms()])).unwrap();
        import::apply(&mut modes, read, &[]);
    }
    assert_eq!(names(&modes), vec!["SMS", "SMS 2", "SMS 3", "SMS 4"]);
    // Un fichier qui contient déjà « SMS 2 » ne devient pas « SMS 2 2 ».
    let mut other = sms();
    other.name = "SMS 2".into();
    import::apply(&mut modes, vec![other], &[]);
    assert_eq!(modes.last().unwrap().name, "SMS 5");
    assert!(import::free_name(&["sms".to_string()], "SMS ").ends_with(" 2"), "majuscules et espaces ne comptent pas");
}

#[test]
fn two_modes_with_the_same_name_in_one_file_get_two_names() {
    let mut a = sms();
    let mut b = sms();
    b.id = scriibe_core::modes::new_id();
    a.name = "Mon mode".into();
    b.name = "Mon mode".into();
    let mut modes = Vec::new();
    import::apply(&mut modes, vec![a, b], &[]);
    assert_eq!(modes.iter().map(|m| m.name.as_str()).collect::<Vec<_>>(), vec!["Mon mode", "Mon mode 2"]);
}

// --- Remplacer, sur demande ---

#[test]
fn replacing_keeps_the_place_and_the_identifier_of_the_existing_mode() {
    let first = CustomMode::new("Avant", "x", Fidelity::Light);
    let target = sms();
    let last = CustomMode::new("Après", "y", Fidelity::Light);
    let mut modes = vec![first.clone(), target.clone(), last.clone()];
    let mut incoming = sms();
    incoming.style = "Nouvelle consigne.".into();
    incoming.fidelity = Fidelity::Free;
    incoming.examples = vec![ModeExample::new("n", "m")];
    let choice = Choice { include: true, replace: true };
    let outcome = import::apply(&mut modes, vec![incoming], &[choice]);
    assert_eq!(modes.len(), 3, "rien n'est ajouté");
    assert_eq!((&modes[0], &modes[2]), (&first, &last));
    assert_eq!(modes[1].id, target.id, "le mode choisi reste choisi");
    assert_eq!((modes[1].style.as_str(), modes[1].fidelity, modes[1].examples.len()), ("Nouvelle consigne.", Fidelity::Free, 1));
    assert_eq!(outcome.replaced, vec!["SMS".to_string()]);
    assert!(outcome.added.is_empty());
}

#[test]
fn replacing_a_mode_with_its_own_exported_file_keeps_its_example_identifiers() {
    let target = sms();
    let mut modes = vec![target.clone()];
    let read = import::parse_modes(&exported(&[&target])).unwrap();
    import::apply(&mut modes, read, &[Choice { include: true, replace: true }]);
    assert_eq!(modes, vec![target]);
}

#[test]
fn replace_without_a_same_name_mode_just_adds() {
    let mut modes = vec![CustomMode::new("Autre", "x", Fidelity::Light)];
    let outcome = import::apply(&mut modes, vec![sms()], &[Choice { include: true, replace: true }]);
    assert_eq!((modes.len(), outcome.added.len(), outcome.replaced.len()), (2, 1, 0));
}

#[test]
fn a_mode_left_out_is_not_imported() {
    let mut modes = Vec::new();
    let mut b = sms();
    b.name = "B".into();
    let outcome = import::apply(&mut modes, vec![sms(), b], &[Choice { include: false, replace: false }, Choice::default()]);
    assert_eq!(outcome.added, vec!["B".to_string()]);
    assert_eq!(modes.len(), 1);
}

// --- Aperçu ---

#[test]
fn the_preview_shows_conflicts_and_the_name_a_mode_would_get() {
    let existing = vec![sms()];
    let mut other = sms();
    other.name = "Neuf".into();
    other.examples.clear();
    let items = import::preview(&existing, &[sms(), other]);
    assert_eq!(items[0].conflict.as_deref(), Some("SMS"));
    assert_eq!((items[0].added_as.as_str(), items[0].examples, items[0].fidelity), ("SMS 2", 2, Fidelity::Reformulated));
    assert_eq!((items[1].conflict.clone(), items[1].added_as.as_str(), items[1].examples), (None, "Neuf", 0));
}

#[test]
fn a_long_instruction_like_a_real_dictionary_mode_goes_back_and_forth() {
    // Un vrai mode personnel de l'auteur avait une consigne de 5 064 caractères : l'essai réel a refusé sa propre exportation.
    let mut mode = sms();
    mode.style = "Mot ; ".repeat(1_500);
    assert!(mode.style.chars().count() > 5_064);
    assert_eq!(import::parse_modes(&exported(&[&mode])).unwrap(), vec![mode]);
}

#[test]
fn two_modes_of_the_same_name_both_set_to_replace_do_not_erase_each_other() {
    // Relecture : le second « remplacer » écrasait le premier mode du fichier, sans message.
    let existing = sms();
    let (mut a, mut b) = (sms(), sms());
    a.style = "Première.".into();
    b.id = scriibe_core::modes::new_id();
    b.style = "Seconde.".into();
    let mut modes = vec![existing.clone()];
    let replace = Choice { include: true, replace: true };
    let outcome = import::apply(&mut modes, vec![a, b], &[replace, replace]);
    assert_eq!(modes.len(), 2);
    assert_eq!((modes[0].name.as_str(), modes[0].style.as_str(), &modes[0].id), ("SMS", "Première.", &existing.id));
    assert_eq!((modes[1].name.as_str(), modes[1].style.as_str()), ("SMS 2", "Seconde."), "le second est ajouté, jamais perdu");
    assert_eq!((outcome.replaced, outcome.added), (vec!["SMS".to_string()], vec!["SMS 2".to_string()]));
}

#[test]
fn a_number_that_is_part_of_the_name_is_kept() {
    let taken = |names: &[&str]| names.iter().map(|n| n.to_string()).collect::<Vec<_>>();
    assert_eq!(import::free_name(&taken(&["Rapport 2026"]), "Rapport 2026"), "Rapport 2026 2");
    assert_eq!(import::free_name(&taken(&["Top 10"]), "Top 10"), "Top 10 2");
    assert_eq!(import::free_name(&taken(&["SMS", "SMS 2"]), "SMS 2"), "SMS 3", "un vrai numéro d'import, lui, ne s'accumule pas");
}

#[test]
fn invisible_format_characters_are_removed_from_names() {
    let text = file_with(json!([{"name": "SMS\u{202E}gnirts\u{200B} \u{FEFF}x", "fidelity": "light"}]));
    assert_eq!(import::parse_modes(&text).unwrap()[0].name, "SMSgnirts x");
}
