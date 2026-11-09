//! Garde-fou du mode Simple sur les dictées longues (S19) : cas tirés de l'historique de Christophe (09/11).
use scriibe_core::cleaner::accepts;
use scriibe_core::modes::Fidelity;

#[test]
fn long_dictation_changed_in_meaning_is_rejected() {
    let raw = "Par exemple, le projet pilote PC avait été réalisé sous la supervision, il me semble, sous la commande slash supervisé et j'ai donc rajouté les fonctionnalités depuis une session";
    let out = "Par exemple, le projet pilote-pc avait été réalisé sous la supervision. Je l'ai ensuite ajouté aux fonctionnalités depuis une session.";
    assert!(!accepts(raw, out, Fidelity::Light));
}

#[test]
fn dictated_signs_do_not_count_as_lost_words() {
    let raw = "Voici ma liste de courses, deux points, retour à la ligne, puce du pain, retour à la ligne, puce des légumes, retour à la ligne, puce du lait";
    let out = "Voici ma liste de courses :\n• du pain\n• des légumes\n• du lait";
    assert!(accepts(raw, out, Fidelity::Light));
}

#[test]
fn short_dictation_keeps_the_old_threshold() {
    assert!(accepts("Oui, écrit la revue et puis range l'inbox", "Oui, écris la revue et ensuite range l'inbox.", Fidelity::Light));
}

#[test]
fn long_faithful_dictation_passes() {
    let raw = "alors je pense qu'on devrait vérifier le dossier de projet avant de lancer la compilation sur le poste windows";
    let out = "Alors je pense qu'on devrait vérifier le dossier de projet avant de lancer la compilation sur le poste Windows.";
    assert!(accepts(raw, out, Fidelity::Light));
}

// Crans de fidélité (S19, n°18).
use scriibe_core::modes::CustomMode;
use scriibe_core::settings::AppSettings;

const RAW: &str = "alors je pense qu'on devrait vérifier le dossier de projet avant de lancer la compilation sur le poste windows";

#[test]
fn literal_asks_for_ninety_percent_of_the_words() {
    // « vérifier » et « lancer » changés : 2 mots sur 12 perdus, donc environ 83 % gardés.
    let out = "Alors je pense qu'on devrait contrôler le dossier de projet avant de démarrer la compilation sur le poste Windows.";
    assert!(accepts(RAW, out, Fidelity::Light));
    assert!(!accepts(RAW, out, Fidelity::Literal));
}

#[test]
fn free_ignores_the_share_of_kept_words_but_not_the_other_guards() {
    let rewritten = "Je souhaite que nous contrôlions ensemble l'ensemble du répertoire avant de démarrer.";
    assert!(!accepts(RAW, rewritten, Fidelity::Light));
    assert!(!accepts(RAW, rewritten, Fidelity::Reformulated));
    assert!(accepts(RAW, rewritten, Fidelity::Free));
    // Une question reste une question, et la sortie ne peut pas être démesurée.
    assert!(!accepts("tu viens ce soir ?", "Je ne sais pas.", Fidelity::Free));
    assert!(!accepts("bonjour", &"x ".repeat(300), Fidelity::Free));
    // Pas de réponse en tête.
    assert!(!accepts("range mes fichiers dans le dossier", "Oui, je range les fichiers.", Fidelity::Free));
}

#[test]
fn old_custom_modes_read_their_rewrites_box_as_a_fidelity() {
    let old: serde_json::Value =
        serde_json::json!({"id": "00000000-0000-0000-0000-000000000001", "name": "A", "style": "x", "rewrites": true});
    assert_eq!(CustomMode::from_value(&old).unwrap().fidelity, Fidelity::Reformulated);
    let old: serde_json::Value =
        serde_json::json!({"id": "00000000-0000-0000-0000-000000000002", "name": "B", "style": "x", "rewrites": false});
    assert_eq!(CustomMode::from_value(&old).unwrap().fidelity, Fidelity::Light);
    let free = CustomMode::new("C", "x", Fidelity::Free);
    assert!(free.profile().rewrites);
    assert!(!CustomMode::new("D", "x", Fidelity::Literal).profile().rewrites);
}

#[test]
fn simple_takes_its_fidelity_from_the_settings() {
    let mut s = AppSettings::default();
    assert_eq!(s.active_profile().fidelity, Fidelity::Light);
    s.simple_fidelity = Fidelity::Free;
    let p = s.active_profile();
    assert_eq!(p.fidelity, Fidelity::Free);
    assert!(!p.rewrites, "Simple garde sa consigne fidèle, même au cran Libre");
    let back = AppSettings::from_json(&serde_json::to_string(&s).unwrap()).unwrap();
    assert_eq!(back.simple_fidelity, Fidelity::Free);
    // Reformulé aussi : il desserre le contrôle, la consigne de Simple reste « garde tes mots ».
    let mut r = AppSettings::from_json(r#"{"simpleFidelity":"reformulated"}"#).unwrap();
    assert_eq!(r.simple_fidelity, Fidelity::Reformulated);
    r.rewrite_mode = RewriteMode::Simple;
    assert!(!r.active_profile().rewrites);
    assert_eq!(r.active_profile().fidelity, Fidelity::Reformulated);
}

// Apaisé (CNV), mode livré (S19, n°16).
use scriibe_core::modes::{Examples, RewriteMode};

#[test]
fn calm_is_a_built_in_mode_at_the_free_level_with_its_own_examples() {
    let p = AppSettings { rewrite_mode: RewriteMode::Calm, ..AppSettings::default() }.active_profile();
    assert_eq!(p.fidelity, Fidelity::Free);
    assert!(p.rewrites);
    assert!(p.style.as_deref().is_some_and(|s| s.contains("communication non violente")));
    assert!(matches!(&p.examples, Examples::Own(list) if list.len() == 3));
}

#[test]
fn calm_accepts_a_real_nvc_rewrite_that_reformulated_would_refuse() {
    let raw = "t'es jamais là quand j'ai besoin de toi franchement t'en as rien à faire de moi";
    let nvc = "Plusieurs fois, quand j'avais besoin de toi, tu n'étais pas là. Ça me rend triste, parce que j'ai besoin de pouvoir compter sur toi. Est-ce qu'on peut en parler ?";
    assert!(!accepts(raw, nvc, Fidelity::Reformulated));
    assert!(accepts(raw, nvc, Fidelity::Free));
    // Une question dictée doit rester une question, même en réécriture libre.
    assert!(!accepts("tu viens ce soir ?", "Je préférerais que tu viennes ce soir.", Fidelity::Free));
}
