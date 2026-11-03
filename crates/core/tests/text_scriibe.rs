//! Le texte de Whisper retouché propre à Scriibe (absent de Scribe) : voir `docs/ECARTS.md`.

use scriibe_core::text::join_spelled_code;

#[test]
fn a_spelled_code_cut_by_a_decimal_comma_is_joined() {
    // Dictée réelle du 03/11/2026 : « zéro neuf deux six F A 6 » entendu et écrit « 0,926FA6 » par Whisper.
    assert_eq!(
        join_spelled_code("Le numéro de version est 0.2.1, ouvrez la parenthèse, 0,926FA6, fermez la parenthèse."),
        "Le numéro de version est 0.2.1, ouvrez la parenthèse, 0926FA6, fermez la parenthèse."
    );
    assert_eq!(join_spelled_code("0,926fa6"), "0926fa6");
}

#[test]
fn numbers_and_short_codes_keep_their_comma() {
    for kept in [
        "Il fait 3,14159.",
        "Il pèse 1,5 Mo.",
        "Le son monte à 3,5db.",
        "Le prix est de 12,50 euros.",
        "Un code 0,926 FA6.",
        "La vitesse est de 1,5e10.",
    ] {
        assert_eq!(join_spelled_code(kept), kept);
    }
}
