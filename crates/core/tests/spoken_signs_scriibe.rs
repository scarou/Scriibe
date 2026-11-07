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

// ---- S18 (tâche 6) : signes dictés ajoutés (docs/COMMANDES.md § 4, docs/ECARTS.md) ----
// Un test par signe : un cas positif, des phrases ordinaires qui ne doivent pas bouger, et l'idempotence (une deuxième application ne change rien).

use scriibe_core::spoken_signs::{apply_for, Active};

fn said(text: &str, active: Active) -> String {
    apply_for(text, ListBullet::Dash, true, active)
}

macro_rules! sign_test {
    ($name:ident, $active:expr, $input:expr => $expected:expr, keeps: [$($keep:expr),* $(,)?]) => {
        #[test]
        fn $name() {
            let out = said($input, $active);
            assert_eq!(out, $expected, "entrée : {}", $input);
            assert_eq!(said(&out, $active), out, "pas idempotent : {out}");
            for text in [$($keep),*] {
                assert_eq!(said(text, $active), text, "{text} ne doit pas changer");
            }
        }
    };
}

// A. Ponctuation
sign_test!(point_d_exclamation_fr, Active::FR, "C'est génial point d'exclamation Ensuite on part." => "C'est génial! Ensuite on part.",
    keeps: ["Ce point d'exclamation est de trop.", "Un point d'exclamation suffit.", "Un point de vue."]);
sign_test!(exclamation_mark_en, Active::EN, "That is great exclamation mark Next we go." => "That is great! Next we go.",
    keeps: ["The exclamation mark is too much.", "An exclamation point helps."]);
sign_test!(point_d_interrogation_fr, Active::FR, "Tu viens point d'interrogation" => "Tu viens?",
    keeps: ["Le point d'interrogation manque.", "Il n'y a pas de point d'attention."]);
sign_test!(question_mark_en, Active::EN, "Are you coming question mark" => "Are you coming?",
    keeps: ["A question mark is missing.", "That is the question."]);
sign_test!(point_virgule_fr, Active::FR, "Du pain point-virgule du lait" => "Du pain; du lait",
    keeps: ["Le point-virgule est mal placé.", "Un point virgule de trop."]);
sign_test!(semicolon_en, Active::EN, "Bread semicolon milk" => "Bread; milk",
    keeps: ["The semicolon is wrong.", "A semi-colon helps."]);
sign_test!(deux_points_fr, Active::FR, "Voici la liste deux-points du pain" => "Voici la liste: du pain",
    keeps: ["J'ai marqué deux points.", "Il a gagné deux points de plus.", "Les deux-points sont de trop."]);
sign_test!(deux_points_imperatif_fr, Active::FR, "Voici la liste écris deux points du pain" => "Voici la liste: du pain",
    keeps: ["J'ai marqué deux points."]);
sign_test!(colon_en, Active::EN, "Here is the list colon bread" => "Here is the list: bread",
    keeps: ["The colon is part of the body.", "He has colon cancer.", "Colon cancer is common."]);

// B. Symboles
sign_test!(arobase_fr, Active::FR, "jean arobase gmail point com" => "jean@gmail.com",
    keeps: ["L'arobase est un signe.", "Mon arobase est cassée."]);
sign_test!(at_sign_en, Active::EN, "jean at sign gmail dot com" => "jean@gmail.com",
    keeps: ["Meet me at the cafe.", "The at sign is small."]);
sign_test!(point_com_fr, Active::FR, "Va sur scriibe point com maintenant" => "Va sur scriibe.com maintenant",
    keeps: ["Ce point commun est clair.", "Un point de vue.", "C'est un point fort."]);
sign_test!(dot_com_en, Active::EN, "Go to scriibe dot com now" => "Go to scriibe.com now",
    keeps: ["Connect the dots.", "The dot is red."]);
sign_test!(barre_oblique_fr, Active::FR, "dossier barre oblique fichier" => "dossier/fichier",
    keeps: ["La barre oblique est penchée.", "Une barre de fer."]);
sign_test!(forward_slash_en, Active::EN, "folder forward slash file" => "folder/file", keeps: ["The forward slash leans."]);
sign_test!(antislash_both, Active::FR, "dossier antislash fichier" => "dossier\\fichier", keeps: ["Mon antislash est cassé."]);
sign_test!(backslash_en, Active::EN, "folder backslash file" => "folder\\file", keeps: ["The backslash leans."]);
sign_test!(barre_oblique_inverse_fr, Active::FR, "dossier barre oblique inverse fichier" => "dossier\\fichier", keeps: []);
sign_test!(underscore_both, Active::FR, "nom underscore fichier" => "nom_fichier", keeps: ["Le underscore est un signe.", "Un tiret bas du mur."]);
sign_test!(tiret_bas_fr, Active::FR, "nom tiret bas fichier" => "nom_fichier", keeps: ["Il fait bas."]);
sign_test!(tilde_both, Active::EN, "dossier tilde utilisateur" => "dossier~utilisateur", keeps: ["The tilde is above the n.", "Un tilde est un signe."]);
sign_test!(esperluette_fr, Active::FR, "Tom esperluette Jerry" => "Tom & Jerry", keeps: ["L'esperluette est un signe.", "Une esperluette."]);
sign_test!(ampersand_en, Active::EN, "Tom ampersand Jerry" => "Tom & Jerry", keeps: ["The ampersand is a sign.", "An ampersand."]);
sign_test!(asterisque_fr, Active::FR, "note astérisque" => "note*", keeps: ["L'astérisque est une étoile.", "Un astérisque de plus.", "Une étoile filante."]);
sign_test!(asterisk_en, Active::EN, "note asterisk" => "note*", keeps: ["The asterisk is a star.", "A shooting star."]);
sign_test!(hashtag_fr, Active::FR, "Poste hashtag scriibe" => "Poste #scriibe",
    keeps: ["Mon hashtag est long.", "Un hashtag de plus.", "Un do dièse."]);
sign_test!(croisillon_fr, Active::FR, "touche croisillon" => "touche #", keeps: ["Le croisillon est un signe."]);
sign_test!(pound_sign_en, Active::EN, "pound sign one" => "#one", keeps: ["I paid a pound."]);
sign_test!(accolade_fr, Active::FR, "fonction ouvrez l'accolade corps fermez l'accolade" => "fonction {corps}",
    keeps: ["Je ferme l'accolade.", "L'accolade est jolie."]);
sign_test!(curly_bracket_en, Active::EN, "function open curly bracket body close curly bracket" => "function {body}",
    keeps: ["The curly bracket is red.", "I close a brace."]);

// C. Anglais : équivalents des signes français
sign_test!(parenthesis_en, Active::EN, "Hello open parenthesis world close parenthesis" => "Hello (world)",
    keeps: ["The open parenthesis is red.", "I close the door."]);
sign_test!(bracket_en, Active::EN, "Hello open bracket world close bracket" => "Hello [world]", keeps: ["The open bracket is red."]);
sign_test!(quote_en, Active::EN, "He said open quote hi close quote" => "He said \"hi\"", keeps: ["The open quote is red."]);
sign_test!(ellipsis_en, Active::EN, "Well ellipsis" => "Well…", keeps: ["The ellipsis is three dots.", "A dot on the map."]);
sign_test!(new_line_en, Active::EN, "Dear Tom new line Thanks" => "Dear Tom\nThanks", keeps: ["I wrote a new line of code.", "The new line is blank."]);
sign_test!(new_paragraph_en, Active::EN, "Dear Tom new paragraph Thanks" => "Dear Tom\n\nThanks", keeps: ["I wrote a new paragraph."]);

#[test]
fn english_signs_stay_off_in_french_dictation_and_french_ones_in_english() {
    for text in ["Le colon est là.", "He asked a question mark at all.", "Please write exclamation mark here.", "dot com"] {
        assert_eq!(said(text, Active::FR), text, "{text} : pas de signe anglais en dictée française");
    }
    assert_eq!(said("Tu viens point d'interrogation", Active::EN), "Tu viens point d'interrogation");
    assert_eq!(said("jean arobase gmail point com", Active::EN), "jean arobase gmail point com");
    // Les deux langues ensemble (« plusieurs langues ») et les symboles neutres (texte traduit).
    assert_eq!(said("Tu viens point d'interrogation", Active::BOTH), "Tu viens?");
    assert_eq!(said("Are you coming question mark", Active::BOTH), "Are you coming?");
    assert_eq!(said("nom underscore fichier", Active::NEUTRAL), "nom_fichier");
    assert_eq!(said("Are you coming question mark", Active::NEUTRAL), "Are you coming question mark");
}

#[test]
fn ordinary_words_never_become_signs() {
    for text in [
        "Il est plus grand, au moins deux fois.",
        "Une virgule de trop.",
        "Il est à égalité.",
        "Un euro et dix dollars.",
        "Un point de vue.",
    ] {
        for active in [Active::FR, Active::EN, Active::BOTH] {
            assert_eq!(said(text, active), text);
        }
    }
}
