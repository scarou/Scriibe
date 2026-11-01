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
fn whisper_writing_puce_as_pus_starts_a_bullet_after_a_line_break() {
    // Dictées réelles du 01/11/2026 (« Pus », puis « PUS »).
    assert_eq!(
        listed("Voici ma liste de courses, retour à la ligne, Pus du pain, retour à la ligne, pus des légumes, retour à la ligne, pus du lait."),
        "Voici ma liste de courses\n- du pain\n- des légumes\n- du lait."
    );
    assert_eq!(listed("Courses, retour à la ligne, PUS du pain."), "Courses\n- du pain.");
    for text in ["Pus du pain.", "Le pus est jaune.", "Retour à la ligne, il pus partir."] {
        assert!(!listed(text).contains("- "), "{text} ne doit pas devenir une puce : {}", listed(text));
    }
}

#[test]
fn a_plus_sign_elsewhere_stays_text() {
    for text in
        ["+ du pain", "Un plus un font 1 + 1 = 2.", "Appelle le +33 6 12 34 56 78.", "Retour à la ligne +33 6 12", "Il a écrit\n+ \n"]
    {
        assert!(!listed(text).contains("- "), "{text} ne doit pas devenir une puce : {}", listed(text));
    }
}
