//! Export et import des commandes perso (chantier 4, étape 3) : fichiers refusés, id et noms, phrases, raccourci, aller-retour.

use scriibe_core::commands::CustomCommand;
use scriibe_core::export::{commands_export, commands_file_name, envelope};
use scriibe_core::import_commands::{apply, evaluate, parse_commands, preview, Problem, MAX_COMMANDS, MAX_NAME, MAX_PHRASE, MAX_PHRASES};
use scriibe_core::settings::AppSettings;
use serde_json::{json, Value};

fn command(id: &str, name: &str, phrases: &[&str], shortcut: &str) -> CustomCommand {
    CustomCommand {
        id: id.into(),
        name: name.into(),
        phrases: phrases.iter().map(|p| p.to_string()).collect(),
        shortcut: shortcut.into(),
        needs_selection: true,
    }
}

fn file_of(list: Value) -> String {
    envelope("commands", "0.4.0", "2026-11-13", json!({"customCommands": list})).to_string()
}

fn refused(text: &str) -> String {
    parse_commands(text).expect_err("le fichier doit être refusé")
}

fn settings_with(commands: Vec<CustomCommand>) -> AppSettings {
    AppSettings { custom_commands: commands, ..AppSettings::default() }
}

#[test]
fn an_export_then_an_import_gives_back_the_same_commands() {
    let original = vec![
        command("perso.1", "Barré", &["mettre en barré", "passer en barré"], "Mod+Shift+X"),
        CustomCommand { needs_selection: false, ..command("perso.2", "Copier tout", &["copier tout"], "Mod+Shift+C") },
    ];
    let file = commands_export(&original, "0.4.0", "2026-11-13");
    assert_eq!((file["kind"].as_str(), file["exportedAt"].as_str()), (Some("commands"), Some("2026-11-13")));
    assert_eq!(commands_file_name("2026-11-13"), "scriibe-commandes-2026-11-13.scriibe.json");
    assert!(
        file["content"]["customCommands"].as_array().unwrap().iter().all(|c| c.get("id").is_none()),
        "l'id (heure de création) n'est pas exporté"
    );
    assert!(!file.to_string().contains("perso."), "aucune trace des identifiants dans le fichier");
    let incoming = parse_commands(&file.to_string()).unwrap();
    let mut empty = AppSettings::default();
    let out = apply(&mut empty, &incoming, &[]);
    assert_eq!(out.added, vec!["Barré", "Copier tout"]);
    let strip = |list: &[CustomCommand]| list.iter().map(|c| CustomCommand { id: String::new(), ..c.clone() }).collect::<Vec<_>>();
    assert_eq!(strip(&empty.custom_commands), strip(&original), "mêmes titres, phrases, raccourcis, réglage de sélection, même ordre");
    assert!(
        empty.custom_commands.iter().all(|c| c.id.starts_with("perso.") && c.id[6..].chars().all(|d| d.is_ascii_digit())),
        "identifiants neufs et numériques"
    );
}

#[test]
fn files_that_are_not_a_commands_export_are_refused() {
    assert!(refused("pas du json").contains("JSON"));
    assert!(refused(r#"{"format":"autre","version":1,"kind":"commands","content":{}}"#).contains("pas un fichier d'export"));
    assert!(
        refused(r#"{"format":"scriibe-export","version":2,"kind":"commands","content":{"customCommands":[]}}"#).contains("plus récente")
    );
    for kind in ["dictionary", "modes", "backup"] {
        let text = envelope(kind, "0.4.0", "2026-11-13", json!({"customCommands": [{"name": "a"}]})).to_string();
        assert!(refused(&text).contains("autre type d'export"), "{kind}");
    }
    assert!(refused(r#"{"format":"scriibe-export","version":1,"kind":"commands","content":{}}"#).contains("ne contient pas de commandes"));
    assert!(refused(&file_of(json!([]))).contains("aucune commande"));
    assert!(refused(&file_of(json!(["x"]))).contains("mal formée"));
    assert!(refused(&file_of(json!([{"name": 3}]))).contains("mal formée"));
    assert!(refused(&file_of(json!([{"phrases": "a b"}]))).contains("mal formée"));
    assert!(refused(&file_of(json!([{"phrases": [4]}]))).contains("mal formée"));
    assert!(refused(&file_of(json!([{"needsSelection": "oui"}]))).contains("mal formée"));
}

#[test]
fn the_limits_refuse_the_whole_file() {
    let many = |n: usize| Value::Array((0..n).map(|i| json!({"id": format!("perso.{i}"), "name": format!("c{i}")})).collect());
    assert_eq!(parse_commands(&file_of(many(MAX_COMMANDS))).unwrap().len(), MAX_COMMANDS);
    assert!(refused(&file_of(many(MAX_COMMANDS + 1))).contains("trop de commandes"));
    let phrases = |n: usize| (0..n).map(|i| format!("phrase numéro {i}")).collect::<Vec<_>>();
    assert!(parse_commands(&file_of(json!([{"name": "a", "phrases": phrases(MAX_PHRASES)}]))).is_ok());
    assert!(refused(&file_of(json!([{"name": "a", "phrases": phrases(MAX_PHRASES + 1)}]))).contains("trop de phrases"));
    assert!(parse_commands(&file_of(json!([{"name": "a".repeat(MAX_NAME), "phrases": ["b".repeat(MAX_PHRASE)]}]))).is_ok());
    assert!(refused(&file_of(json!([{"name": "a".repeat(MAX_NAME + 1)}]))).contains("trop long"));
    assert!(refused(&file_of(json!([{"name": "a", "phrases": ["b".repeat(MAX_PHRASE + 1)]}]))).contains("trop long"));
    assert!(refused(&file_of(json!([{"name": "a", "padding": "x".repeat(1_048_577)}]))).contains("trop gros"));
}

#[test]
fn a_colliding_id_is_regenerated_and_a_taken_name_is_numbered_without_piling_up() {
    let mut settings = settings_with(vec![command("perso.1", "Barré", &["mettre en barré"], "Mod+Shift+X")]);
    let incoming = vec![
        command("perso.1", "Barré", &["passer en barré"], "Mod+Shift+B"),
        command("perso.1", "Barré 2", &["rayer le texte"], "Mod+Shift+R"),
    ];
    let out = apply(&mut settings, &incoming, &[]);
    assert_eq!(out.added, vec!["Barré 2", "Barré 3"], "le numéro final d'un export précédent ne s'accumule pas");
    let ids: Vec<&str> = settings.custom_commands.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids[0], "perso.1");
    assert!(ids.iter().all(|id| id.starts_with("perso.")));
    assert_eq!(ids.iter().collect::<std::collections::HashSet<_>>().len(), 3, "trois identifiants distincts : {ids:?}");
    // L'id du fichier n'est jamais repris, même libre : toujours un nouveau, numérique (jamais « perso.nouveau », réservé au contrôle des phrases).
    for wanted in ["perso.77", "n'importe quoi", "perso.nouveau"] {
        let got = evaluate(&AppSettings::default(), &command(wanted, "A", &["dire a b"], "Mod+B"));
        assert!(
            got.command.id.starts_with("perso.") && got.command.id[6..].chars().all(|d| d.is_ascii_digit()) && got.command.id != wanted,
            "{wanted} → {}",
            got.command.id
        );
    }
    // Un fichier qui porte l'id « perso.nouveau » ne garde pas cet id : ses phrases restent contrôlées par la suite.
    let mut s = AppSettings::default();
    apply(&mut s, &[command("perso.nouveau", "A", &["dire un truc"], "Mod+B")], &[]);
    let again = evaluate(&s, &command("perso.x", "B", &["dire un truc"], "Mod+I"));
    assert_eq!(again.problem, Some(Problem::NoPhrase), "la phrase est déjà prise par la première commande");
}

#[test]
fn the_shortcut_is_checked_with_shortcut_parse_and_a_bad_one_is_flagged_not_imported() {
    let settings = AppSettings::default();
    let bad = command("perso.1", "A", &["dire a b"], "nimporte quoi");
    assert_eq!(evaluate(&settings, &bad).problem, Some(Problem::BadShortcut));
    assert_eq!(evaluate(&settings, &command("perso.1", "A", &["dire a b"], "")).problem, Some(Problem::BadShortcut));
    assert_eq!(
        evaluate(&settings, &command("perso.1", "A", &["dire a b"], "Mod+Q")).problem,
        Some(Problem::BadShortcut),
        "Mod+Q est refusé par la page aussi"
    );
    assert_eq!(evaluate(&settings, &command("perso.1", "", &["dire a b"], "Mod+B")).problem, Some(Problem::NoName));
    let mut s = AppSettings::default();
    let out = apply(&mut s, &[bad], &[]);
    assert_eq!((out.added.len(), out.skipped), (0, 1));
    assert!(s.custom_commands.is_empty());
}

#[test]
fn phrases_already_used_or_too_short_are_dropped_and_a_command_left_without_phrase_is_flagged() {
    let settings = settings_with(vec![command("perso.1", "Barré", &["mettre en barré"], "Mod+Shift+X")]);
    // « ajouter au dictionnaire » est une phrase du programme ; « mettre en barré » est à une commande en place ; « gras » est trop court.
    let c = command("perso.2", "Mix", &["Ajouter au dictionnaire", "METTRE EN BARRÉ", "gras", "dire bonjour", "dire  bonjour"], "Mod+B");
    let got = evaluate(&settings, &c);
    assert_eq!(got.command.phrases, vec!["dire bonjour"]);
    assert_eq!(got.dropped.len(), 4);
    assert_eq!(got.problem, None);
    let none = evaluate(&settings, &command("perso.3", "Rien", &["mettre en barré"], "Mod+B"));
    assert_eq!(none.problem, Some(Problem::NoPhrase));
}

#[test]
fn the_preview_does_not_let_two_commands_of_the_file_share_a_phrase_or_a_name() {
    let settings = AppSettings::default();
    let incoming =
        vec![command("perso.1", "A", &["dire un truc"], "Mod+B"), command("perso.2", "A", &["dire un truc", "dire autre chose"], "Mod+I")];
    let items = preview(&settings, &incoming);
    assert_eq!(items[0].command.name, "A");
    assert_eq!(items[1].command.name, "A 2");
    assert_eq!(items[1].command.phrases, vec!["dire autre chose"]);
}

#[test]
fn an_import_only_adds_and_unticked_commands_stay_out() {
    let mut settings = settings_with(vec![command("perso.1", "Barré", &["mettre en barré"], "Mod+Shift+X")]);
    let before = settings.clone();
    let incoming = vec![command("perso.5", "Un", &["dire un"], "Mod+B"), command("perso.6", "Deux", &["dire deux"], "Mod+I")];
    let out = apply(&mut settings, &incoming, &[false, true]);
    assert_eq!(out.added, vec!["Deux"]);
    assert_eq!(settings.custom_commands[0], before.custom_commands[0]);
    assert_eq!(settings.custom_commands.len(), 2);
    assert_eq!(settings.voice_commands, before.voice_commands, "aucun autre réglage ne change");
    // Importer deux fois : la seconde fois, la phrase est déjà prise, rien ne s'ajoute.
    let again = apply(&mut settings, &incoming, &[false, true]);
    assert_eq!((again.added.len(), again.skipped), (0, 1));
}
