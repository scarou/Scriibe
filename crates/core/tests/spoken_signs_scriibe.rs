//! Les signes dictés propres à Scriibe (absents de Scribe) : voir `docs/ECARTS.md`.

use scriibe_core::spoken_signs::{apply, ListBullet};

fn listed(text: &str) -> String {
    apply(text, ListBullet::Dash, true)
}

#[test]
fn whisper_writing_puce_as_plus_starts_a_bullet_after_a_line_break() {
    // Dictée réelle du 01/11/2026 : « puce » entendu et écrit « + » par Whisper.
    assert_eq!(
        listed("Voici ma liste de courses de points retour à la ligne + du pain, retour à la ligne + des légumes, retour à la ligne + du lait."),
        "Voici ma liste de courses de points\n- du pain\n- des légumes\n- du lait."
    );
    assert_eq!(apply("Courses\n+ du lait", ListBullet::Dot, true), "Courses\n• du lait");
}

#[test]
fn a_plus_sign_elsewhere_stays_text() {
    for text in
        ["+ du pain", "Un plus un font 1 + 1 = 2.", "Appelle le +33 6 12 34 56 78.", "Retour à la ligne +33 6 12", "Il a écrit\n+ \n"]
    {
        assert!(!listed(text).contains("- "), "{text} ne doit pas devenir une puce : {}", listed(text));
    }
}
