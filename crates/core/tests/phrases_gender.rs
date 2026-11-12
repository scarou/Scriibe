//! Phrases proposées à la lecture et genre (C3, tâche 2) : aucune ne s'accorde avec la personne qui lit, donc la forme
//! neutre vaut pour Masculin, Féminin et Non précisé ; ce garde-fou échoue si une phrase future en demande un.
//! Constat dans `docs/PETITS_PLUS.md` § Phrases.
use scriibe_core::bench::{phrases, phrases_en};

/// Mots qui peuvent suivre « je suis », « je me sens »… sans s'accorder au genre (prépositions, articles).
const NEUTRAL_NEXT: &[&str] =
    &["à", "au", "aux", "en", "de", "du", "dans", "chez", "sur", "avec", "pour", "le", "la", "les", "un", "une", "l'"];
const SUBJECT_FORMS: &[&str] =
    &["je suis ", "je me sens ", "je me suis ", "j'étais ", "je serai ", "je reste ", "je deviens ", "je m'étais "];

/// Les formes « sujet + mot » de la phrase dont le mot suivant pourrait s'accorder au genre de la personne.
fn agreeing_forms(text: &str) -> Vec<String> {
    let t = text.to_lowercase().replace('’', "'");
    let mut found = Vec::new();
    for form in SUBJECT_FORMS {
        for (at, _) in t.match_indices(form) {
            let next: String =
                t[at + form.len()..].split(|c: char| c.is_whitespace() || c == ',' || c == '.').next().unwrap_or("").to_string();
            if !NEUTRAL_NEXT.iter().any(|n| next == *n || (n.ends_with('\'') && next.starts_with(n))) {
                found.push(format!("{form}{next}"));
            }
        }
    }
    found
}

#[test]
fn the_guard_catches_an_agreeing_phrase() {
    assert_eq!(agreeing_forms("Je suis content de te voir."), vec!["je suis content"]);
    assert_eq!(agreeing_forms("Je me sens fatiguée."), vec!["je me sens fatiguée"]);
    assert!(agreeing_forms("Je serai à la maison vers dix-neuf heures.").is_empty());
}

#[test]
fn no_french_bench_phrase_agrees_with_the_speaker() {
    assert_eq!(phrases().len(), 17);
    for p in phrases() {
        assert!(agreeing_forms(&p.text).is_empty(), "{} : {:?}", p.text, agreeing_forms(&p.text));
    }
}

#[test]
fn no_english_bench_phrase_needs_a_gender_either() {
    // L'anglais n'accorde rien ; on vérifie seulement que le banc anglais est complet et ne contient pas de formes françaises accordées.
    assert_eq!(phrases_en().len(), 17);
    for p in phrases_en() {
        assert!(agreeing_forms(&p.text).is_empty(), "{}", p.text);
    }
}

#[test]
fn the_installation_example_sentence_is_neutral() {
    let docs = [
        include_str!("../../../docs/INSTALLATION_WINDOWS.md"),
        include_str!("../../../docs/INSTALLATION_LINUX.md"),
        include_str!("../../../docs/INSTALLATION_MACOS.md"),
    ];
    for d in docs {
        assert!(d.contains("« Bonjour, je teste Scriibe. »"));
    }
    assert!(agreeing_forms("Bonjour, je teste Scriibe.").is_empty());
}
