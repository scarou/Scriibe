//! Export et import du dictionnaire (chantier 4, étape 2) : fichiers refusés, doublons, aller-retour identique, fusion qui n'ajoute que.

use scriibe_core::dictionary::{Correction, UserDictionary};
use scriibe_core::export::{dictionary_export, dictionary_file_name, envelope};
use scriibe_core::import_dictionary::{
    apply, parse_dictionary, preview, Incoming, Status, MAX_CORRECTIONS, MAX_HEARD, MAX_TOTAL_WORDS, MAX_WORD, MAX_WORDS, MAX_WORDS_CHARS,
    MAX_WRITTEN,
};
use serde_json::{json, Value};

fn correction(heard: &str, written: &str) -> Correction {
    Correction { heard: heard.into(), written: written.into() }
}

fn dictionary() -> UserDictionary {
    UserDictionary {
        words: vec!["Scriibe".into(), "Claude Code".into(), "to-do".into()],
        corrections: vec![
            correction("cri be", "Scriibe"),
            correction("cloud code", "Claude Code"),
            correction("trois points d'exclamation", "!!!"),
        ],
    }
}

fn file_of(content: Value) -> String {
    envelope("dictionary", "0.4.0", "2026-11-13", content).to_string()
}

fn refused(text: &str) -> String {
    parse_dictionary(text).expect_err("le fichier doit être refusé")
}

#[test]
fn an_export_then_an_import_gives_back_the_same_dictionary() {
    let original = dictionary();
    let text = dictionary_export(&original, true, true, "0.4.0", "2026-11-13").to_string();
    let incoming = parse_dictionary(&text).unwrap();
    assert_eq!(incoming, Incoming { words: original.words.clone(), corrections: original.corrections.clone() });
    let mut rebuilt = UserDictionary::default();
    apply(&mut rebuilt, &incoming, &[], &[]);
    assert_eq!(rebuilt, original, "mêmes mots, mêmes corrections, même ordre");
}

#[test]
fn the_export_has_the_envelope_the_chosen_categories_and_nothing_else() {
    let all = dictionary_export(&dictionary(), true, true, "0.4.0", "2026-11-13");
    assert_eq!(
        (all["format"].as_str(), all["version"].as_u64(), all["kind"].as_str(), all["app"].as_str(), all["exportedAt"].as_str()),
        (Some("scriibe-export"), Some(1), Some("dictionary"), Some("Scriibe 0.4.0"), Some("2026-11-13"))
    );
    let mut keys: Vec<&String> = all["content"].as_object().unwrap().keys().collect();
    keys.sort();
    assert_eq!(keys, ["corrections", "words"]);
    let words_only = dictionary_export(&dictionary(), true, false, "0.4.0", "2026-11-13");
    assert!(words_only["content"].get("corrections").is_none());
    let corrections_only = dictionary_export(&dictionary(), false, true, "0.4.0", "2026-11-13");
    assert!(corrections_only["content"].get("words").is_none());
    assert_eq!(dictionary_file_name("2026-11-13"), "scriibe-dictionnaire-2026-11-13.scriibe.json");
}

#[test]
fn files_that_are_not_a_dictionary_export_are_refused_with_a_clear_message() {
    assert!(refused("pas du json").contains("JSON"));
    assert!(refused("[1, 2]").contains("pas un fichier d'export"));
    assert!(refused(r#"{"format":"autre","version":1,"kind":"dictionary","content":{}}"#).contains("pas un fichier d'export"));
    assert!(refused(r#"{"format":"scriibe-export","kind":"dictionary","content":{}}"#).contains("pas un fichier d'export"), "sans version");
    assert!(refused(r#"{"format":"scriibe-export","version":2,"kind":"dictionary","content":{"words":["a"]}}"#).contains("plus récente"));
    assert!(refused(r#"{"format":"scriibe-export","version":0,"kind":"dictionary","content":{"words":["a"]}}"#)
        .contains("pas un fichier d'export"));
    // Un autre type d'export (modes, sauvegarde) n'est pas pris pour un dictionnaire.
    for kind in ["modes", "backup", "commands", "profile"] {
        let text = envelope(kind, "0.4.0", "2026-11-13", json!({"words": ["a"]})).to_string();
        assert!(refused(&text).contains("autre type d'export"), "{kind}");
    }
    assert!(
        refused(r#"{"format":"scriibe-export","version":1,"kind":"dictionary","content":[1]}"#).contains("ne contient pas de dictionnaire")
    );
}

#[test]
fn an_empty_or_malformed_content_is_refused() {
    assert!(refused(&file_of(json!({}))).contains("aucun mot ni correction"));
    assert!(refused(&file_of(json!({"words": [], "corrections": []}))).contains("aucun mot ni correction"));
    assert!(refused(&file_of(json!({"words": ["  ", "\n"], "corrections": [{"heard": "", "written": "x"}]})))
        .contains("aucun mot ni correction"));
    assert!(refused(&file_of(json!({"words": "Scriibe"}))).contains("mal formé"));
    assert!(refused(&file_of(json!({"words": ["a", 3]}))).contains("un mot du fichier est mal formé"));
    assert!(refused(&file_of(json!({"corrections": ["cri be"]}))).contains("une correction du fichier est mal formée"));
    assert!(refused(&file_of(json!({"corrections": [{"heard": "a"}]}))).contains("une correction du fichier est mal formée"));
    assert!(refused(&file_of(json!({"corrections": [{"heard": "a", "written": 3}]}))).contains("une correction du fichier est mal formée"));
}

#[test]
fn the_limits_on_size_count_and_length_refuse_the_whole_file() {
    let exactly = |n: usize| (0..n).map(|i| format!("mot{i}")).collect::<Vec<_>>();
    assert_eq!(parse_dictionary(&file_of(json!({"words": exactly(MAX_WORDS)}))).unwrap().words.len(), MAX_WORDS);
    assert!(refused(&file_of(json!({"words": exactly(MAX_WORDS + 1)}))).contains("trop de mots"));
    let many = |n: usize| (0..n).map(|i| json!({"heard": format!("a{i}"), "written": "b"})).collect::<Vec<_>>();
    assert_eq!(parse_dictionary(&file_of(json!({"corrections": many(MAX_CORRECTIONS)}))).unwrap().corrections.len(), MAX_CORRECTIONS);
    assert!(refused(&file_of(json!({"corrections": many(MAX_CORRECTIONS + 1)}))).contains("trop de corrections"));

    assert!(parse_dictionary(&file_of(json!({"words": ["a".repeat(MAX_WORD)]}))).is_ok());
    assert!(refused(&file_of(json!({"words": ["a".repeat(MAX_WORD + 1)]}))).contains("un mot est trop long"));
    assert!(
        parse_dictionary(&file_of(json!({"corrections": [{"heard": "a".repeat(MAX_HEARD), "written": "b".repeat(MAX_WRITTEN)}]}))).is_ok()
    );
    assert!(refused(&file_of(json!({"corrections": [{"heard": "a".repeat(MAX_HEARD + 1), "written": "b"}]}))).contains("trop long"));
    assert!(refused(&file_of(json!({"corrections": [{"heard": "a", "written": "b".repeat(MAX_WRITTEN + 1)}]}))).contains("trop long"));

    let huge = file_of(json!({"words": ["a"], "padding": "x".repeat(1_048_577)}));
    assert!(refused(&huge).contains("trop gros"));
}

#[test]
fn repeats_inside_the_file_are_collapsed_to_the_first_and_texts_are_cleaned() {
    let incoming = parse_dictionary(&file_of(json!({
        "words": ["Claude Code", "claude   code", "École", "école", "  Tauri \n", "\u{200B}Whisper\u{0007}"],
        "corrections": [
            {"heard": "cloud code", "written": "Claude Code"},
            {"heard": "Cloud  CODE", "written": "autre"},
            {"heard": "l’avion", "written": "l'avion"},
            {"heard": "L'avion", "written": "L'Avion"},
            {"heard": "ligne\nun", "written": "ligne\nun"},
        ],
        "champFutur": {"a": 1},
    })))
    .unwrap();
    assert_eq!(incoming.words, vec!["Claude Code", "École", "Tauri", "Whisper"]);
    assert_eq!(
        incoming.corrections,
        vec![correction("cloud code", "Claude Code"), correction("l’avion", "l'avion"), correction("ligne un", "ligne un")]
    );
}

#[test]
fn the_preview_says_what_is_new_what_is_already_there_and_what_conflicts() {
    let incoming = Incoming {
        words: vec!["scriibe".into(), "Tauri".into()],
        corrections: vec![correction("CRI BE", "Scriibe"), correction("cri be", "x"), correction("tauri", "Tauri")],
    };
    let seen = preview(&dictionary(), &incoming);
    assert_eq!(seen.words, vec![("scriibe".to_string(), Status::Same), ("Tauri".to_string(), Status::New)]);
    assert_eq!(seen.corrections[0].1, Status::Same, "même « entendu » (casse ignorée) et même « écrit »");
    assert_eq!(seen.corrections[1].1, Status::Conflict { existing: "Scriibe".into() });
    assert_eq!(seen.corrections[2].1, Status::New);
}

#[test]
fn an_import_only_adds_it_never_replaces_moves_or_removes_what_is_there() {
    let mut mine = dictionary();
    let before = mine.clone();
    let incoming = Incoming {
        words: vec!["scriibe".into(), "Tauri".into()],
        corrections: vec![correction("cri be", "AUTRE"), correction("tauri", "Tauri")],
    };
    let out = apply(&mut mine, &incoming, &[], &[]);
    assert_eq!((out.words_added, out.corrections_added, out.already, out.conflicts), (1, 1, 1, 1));
    assert_eq!(&mine.words[..before.words.len()], &before.words[..], "les mots en place n'ont pas bougé");
    assert_eq!(&mine.corrections[..before.corrections.len()], &before.corrections[..], "la correction en conflit est restée telle quelle");
    assert_eq!(mine.words.last().map(String::as_str), Some("Tauri"));
    assert_eq!(mine.corrections.last(), Some(&correction("tauri", "Tauri")));

    // Importer deux fois le même fichier ne change plus rien.
    let again = mine.clone();
    let second = apply(&mut mine, &incoming, &[], &[]);
    assert_eq!((second.words_added, second.corrections_added), (0, 0));
    assert_eq!(mine, again);
}

#[test]
fn only_the_ticked_items_are_imported() {
    let incoming =
        Incoming { words: vec!["Un".into(), "Deux".into(), "Trois".into()], corrections: vec![correction("a", "A"), correction("b", "B")] };
    let mut empty = UserDictionary::default();
    let out = apply(&mut empty, &incoming, &[true, false, true], &[false, true]);
    assert_eq!((out.words_added, out.corrections_added), (2, 1));
    assert_eq!(empty.words, vec!["Un", "Trois"]);
    assert_eq!(empty.corrections, vec![correction("b", "B")]);
}

#[test]
fn the_words_of_a_file_cannot_swell_the_prompt() {
    // 500 mots de 100 caractères = 50 000 caractères : refusé, alors que chaque mot et le nombre de mots sont dans les limites.
    let long = (0..MAX_WORDS).map(|i| format!("{i:0>100}")).collect::<Vec<_>>();
    assert!(refused(&file_of(json!({"words": long}))).contains("trop longs en tout"));
    // Pile à la limite de caractères : accepté.
    let per_word = MAX_WORDS_CHARS / 100;
    let ok = (0..100).map(|i| format!("{i:0>per_word$}")).collect::<Vec<_>>();
    assert_eq!(parse_dictionary(&file_of(json!({"words": ok}))).unwrap().words.len(), 100);
}

#[test]
fn an_import_stops_adding_words_once_the_dictionary_is_full_and_says_so() {
    let mut mine = UserDictionary { words: (0..MAX_TOTAL_WORDS - 2).map(|i| format!("mot {i}")).collect(), corrections: vec![] };
    let incoming = Incoming { words: vec!["Un".into(), "Deux".into(), "Trois".into()], corrections: vec![correction("a", "A")] };
    let out = apply(&mut mine, &incoming, &[], &[]);
    assert_eq!((out.words_added, out.over_limit, out.corrections_added), (2, 1, 1));
    assert_eq!(mine.words.len(), MAX_TOTAL_WORDS);
    assert!(!mine.words.contains(&"Trois".to_string()));
}

#[test]
fn preview_and_merge_stay_fast_with_thousands_of_words() {
    let mine = UserDictionary {
        words: (0..5_000).map(|i| format!("mot existant {i}")).collect(),
        corrections: (0..2_000).map(|i| correction(&format!("entendu {i}"), "x")).collect(),
    };
    let incoming = Incoming {
        words: (0..MAX_WORDS).map(|i| format!("mot existant {}", i * 10)).chain((0..MAX_WORDS).map(|i| format!("nouveau {i}"))).collect(),
        corrections: (0..MAX_CORRECTIONS).map(|i| correction(&format!("entendu {}", i * 4), "x")).collect(),
    };
    let started = std::time::Instant::now();
    let seen = preview(&mine, &incoming);
    assert_eq!(seen.words.iter().filter(|(_, s)| *s == Status::Same).count(), MAX_WORDS);
    let mut merged = mine.clone();
    apply(&mut merged, &incoming, &[], &[]);
    assert!(started.elapsed() < std::time::Duration::from_secs(2), "aperçu et fusion ont pris {:?}", started.elapsed());
}
