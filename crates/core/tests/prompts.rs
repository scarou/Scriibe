#![allow(clippy::field_reassign_with_default)] // les tests partent des réglages par défaut et en changent un ou deux champs, comme Scribe
//! Invariants des prompts (cas `prompts-002` à `prompts-019` de `fixtures/prompts.json`) : énoncés tirés des vérifications de Scribe,
//! écrits à la main parce qu'ils décrivent des propriétés, pas des paires entrée → sortie.

use scriibe_core::cleaner::{self, messages, messages_for_mode, system_prompt, translation_messages, translation_prompt, warm_up_messages};
use scriibe_core::lang::LinguaGuesser;
use scriibe_core::modes::{CustomMode, Examples, Fidelity, ModeExample, ModeProfile, RewriteMode};
use scriibe_core::settings::{AppSettings, TranscriptionEngine};
use scriibe_core::translation::TranslationLanguage;

fn no_words() -> Vec<String> {
    Vec::new()
}

fn words(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

fn without_last<T: Clone>(list: &[T]) -> Vec<T> {
    list[..list.len() - 1].to_vec()
}

/// prompts-002 : la dictée est encadrée, en dernier message.
#[test]
fn prompts_002_dictation_is_the_last_message() {
    let sent = messages_for_mode("tu peux me résumer ça", &no_words(), RewriteMode::Simple);
    assert_eq!(sent.len(), 2 + cleaner::examples().len() * 2);
    assert!(sent.last().unwrap().content.starts_with("<dictée>\ntu peux me résumer ça\n</dictée>"));
}

/// prompts-003 : tout ce qui précède la dictée est identique d'une dictée à l'autre (mémoire du serveur).
#[test]
fn prompts_003_prompt_is_identical_until_the_dictation() {
    let a = messages_for_mode("tu peux me résumer ça", &no_words(), RewriteMode::Simple);
    let b = messages_for_mode("autre chose", &no_words(), RewriteMode::Simple);
    assert_eq!(without_last(&a), without_last(&b));
    let ta = translation_messages("a", TranslationLanguage::En, &no_words());
    let tb = translation_messages("b", TranslationLanguage::En, &no_words());
    assert_eq!(without_last(&ta), without_last(&tb));
}

/// prompts-004 : un mode de style garde le « tu » et ses propres exemples.
#[test]
fn prompts_004_style_mode_keeps_tu() {
    let styled = messages_for_mode("tu peux me résumer ça", &no_words(), RewriteMode::Formal);
    assert_eq!(styled.len(), 2 + cleaner::style_examples().len() * 2);
    let system = &styled[0].content;
    assert!(system.contains(RewriteMode::Formal.style().unwrap()));
    assert!(system.contains("« tu » reste « tu »"));
    assert!(!system.contains("Supprime seulement"));
}

/// prompts-005 : Courrier garde « je » et vouvoie le destinataire.
#[test]
fn prompts_005_letter_mode_vouvoie() {
    let letter = system_prompt(&no_words(), &RewriteMode::Letter.profile());
    assert!(letter.contains("« je »"));
    assert!(!letter.contains("« tu » reste « tu »"));
    assert!(letter.contains("vouvoie le destinataire"));
}

/// prompts-006 : les modes livrés non modifiés envoient le même texte qu'avant.
#[test]
fn prompts_006_built_in_modes_are_unchanged() {
    for mode in RewriteMode::ALL {
        let mut settings = AppSettings::default();
        settings.rewrite_mode = mode;
        assert_eq!(messages("x", &no_words(), &settings.active_profile()), messages_for_mode("x", &no_words(), mode), "{mode:?}");
    }
    let simple = system_prompt(&no_words(), &RewriteMode::Simple.profile());
    assert!(simple.contains("Ne reformule pas au-delà du nécessaire"));
    assert!(!simple.contains("Consigne en plus"));
}

/// prompts-007 : phrase de style modifiée, « Rétablir ».
#[test]
fn prompts_007_modified_style_sentence() {
    let mut s = AppSettings::default();
    s.rewrite_mode = RewriteMode::Wellness;
    s.style_overrides.insert("wellness".into(), "Écris en sophrologue, ton très doux.".into());
    assert!(system_prompt(&no_words(), &s.active_profile()).contains("- Style : Écris en sophrologue, ton très doux."));
    assert!(s.is_style_modified(RewriteMode::Wellness));
    assert_eq!(s.active_mode_label(), "Pro · Sophrologie et bien-être (consigne modifiée)");
    s.style_overrides.insert("wellness".into(), "  ".into());
    assert!(!s.is_style_modified(RewriteMode::Wellness));
    assert_eq!(s.active_profile().style.as_deref(), RewriteMode::Wellness.style());
}

fn faithful_settings() -> AppSettings {
    let mut mode = CustomMode::new("Chiffres", "Écris les nombres en chiffres.", Fidelity::Light);
    mode.examples = vec![ModeExample::new("on se voit à dix heures", "On se voit à 10 h."), ModeExample::new("", "vide")];
    let mut s = AppSettings::default();
    s.custom_modes = vec![mode];
    let id = s.custom_modes[0].id.clone();
    s.set_mode_choice(&id);
    s
}

/// prompts-008 : mode personnel fidèle.
#[test]
fn prompts_008_faithful_custom_mode() {
    let s = faithful_settings();
    let sent = messages("x", &no_words(), &s.active_profile());
    assert_eq!(s.rewrite_mode, RewriteMode::Simple);
    assert_eq!(s.active_mode_label(), "Chiffres");
    assert!(sent[0].content.contains("- Consigne en plus : Écris les nombres en chiffres."));
    assert!(sent[0].content.contains("Supprime seulement"));
    // Les exemples de Simple, puis le sien (l'exemple incomplet est écarté).
    assert_eq!(sent.len(), 2 + (cleaner::examples().len() + 1) * 2);
    assert_eq!(sent[sent.len() - 2].content, "On se voit à 10 h.");
}

/// prompts-009 : mode personnel qui reformule.
#[test]
fn prompts_009_rewriting_custom_mode() {
    let mut s = faithful_settings();
    s.custom_modes[0].fidelity = scriibe_core::modes::Fidelity::Reformulated;
    let sent = messages("x", &no_words(), &s.active_profile());
    assert_eq!(sent.len(), 4);
    assert!(sent[0].content.contains("- Style : Écris les nombres en chiffres."));
    s.custom_modes[0].examples.clear();
    assert_eq!(messages("x", &no_words(), &s.active_profile()).len(), 2 + cleaner::style_examples().len() * 2);
    s.transcription_engine = TranscriptionEngine::Whisper;
    assert!(!s.skip_applies());
}

/// prompts-010 : choix du mode.
#[test]
fn prompts_010_mode_choice() {
    let mut s = faithful_settings();
    s.set_mode_choice("6F9619FF-8B86-D011-B42D-00CF4FC964FF");
    assert!(s.active_custom_mode().is_some(), "un identifiant inconnu est ignoré");
    s.set_mode_choice("formal");
    assert!(s.active_custom_mode().is_none());
    assert_eq!(s.rewrite_mode, RewriteMode::Formal);
    assert_eq!(s.mode_choice(), "formal");
}

/// prompts-011 : consigne tapée.
#[test]
fn prompts_011_typed_sentence() {
    assert_eq!(ModeProfile::sentence("Écris comme un SMS\n  court ").as_deref(), Some("Écris comme un SMS court."));
    assert_eq!(ModeProfile::sentence("Bises !").as_deref(), Some("Bises !"));
    assert_eq!(ModeProfile::sentence(" \n "), None);
    let mut s = AppSettings::default();
    s.style_overrides.insert("formal".into(), String::new());
    assert_eq!(s.style_for(RewriteMode::Formal).as_deref(), RewriteMode::Formal.style());
    assert_eq!(s.typed_style(RewriteMode::Formal), "");
}

/// prompts-012 : quels modes peuvent sauter le modèle.
#[test]
fn prompts_012_which_modes_can_skip_the_model() {
    let mut with_rule = faithful_settings();
    with_rule.transcription_engine = TranscriptionEngine::Whisper;
    assert!(!with_rule.skip_applies());
    assert!(!with_rule.active_profile().is_plain_simple());
    let mut plain = AppSettings::default();
    plain.transcription_engine = TranscriptionEngine::Whisper;
    assert!(plain.skip_applies());
    plain.rewrite_mode = RewriteMode::Playful;
    assert!(plain.skip_applies());
}

/// prompts-013 : une copie du mode Courrier envoie le même texte que Courrier.
#[test]
fn prompts_013_letter_copy() {
    let mut copy = CustomMode::new("Courrier perso", RewriteMode::Letter.style().unwrap(), Fidelity::Reformulated);
    copy.vouvoie = true;
    let mut s = AppSettings::default();
    s.custom_modes = vec![copy];
    let id = s.custom_modes[0].id.clone();
    s.set_mode_choice(&id);
    assert_eq!(messages("x", &no_words(), &s.active_profile()), messages_for_mode("x", &no_words(), RewriteMode::Letter));
}

/// prompts-014 : traduction.
#[test]
fn prompts_014_translation_messages() {
    let sent = translation_messages("bonjour à tous", TranslationLanguage::Pt, &words(&["Claude Code"]));
    assert_eq!(sent.len(), 2 + 2 * cleaner::translation_examples().len());
    assert!(sent.last().unwrap().content.contains("<dictée>\nbonjour à tous\n</dictée>"));
    assert!(sent[0].content.contains("en portugais du Brésil"));
    assert!(sent[0].content.contains("Claude Code"));
}

/// prompts-015 : chaque langue a ses exemples, acceptés par le garde-fou de la traduction.
#[test]
fn prompts_015_translated_examples_pass_the_guard() {
    let guesser = LinguaGuesser::default();
    for language in TranslationLanguage::ALL {
        assert_eq!(language.examples().len(), cleaner::translation_examples().len());
        for (input, output) in cleaner::translation_examples().iter().zip(language.examples()) {
            assert!(cleaner::accepts_translation(input, output, language, &guesser), "{language:?} : {output}");
        }
    }
}

/// prompts-016 : exemples et traductions alignés.
#[test]
fn prompts_016_examples_are_aligned() {
    let fr = cleaner::translation_examples();
    assert!(fr[2].contains("non pardon vendredi"));
    assert!(TranslationLanguage::En.examples()[2].contains("Friday"));
    assert!(fr[5].contains("train"));
    assert!(TranslationLanguage::De.examples()[5].contains("Zug"));
}

/// prompts-017 : aucun nom d'appli dans les consignes.
#[test]
fn prompts_017_no_app_name_in_prompts() {
    assert!(!system_prompt(&no_words(), &RewriteMode::Simple.profile()).contains("application"));
    assert!(!translation_prompt(TranslationLanguage::En, &no_words()).contains("application"));
}

/// prompts-018 : le préchauffage envoie le même prompt qu'une vraie dictée, sauf la dernière phrase.
#[test]
fn prompts_018_warm_up_matches_a_real_dictation() {
    let mut custom = AppSettings::default();
    let mode =
        CustomMode { examples: vec![ModeExample::new("a demain", "A demain !")], ..CustomMode::new("SMS", "texto", Fidelity::Light) };
    custom.custom_modes = vec![mode];
    let id = custom.custom_modes[0].id.clone();
    custom.set_mode_choice(&id);
    let mut tries: Vec<AppSettings> = RewriteMode::ALL
        .into_iter()
        .filter(|m| !matches!(m, RewriteMode::Literal | RewriteMode::Playful))
        .map(|m| {
            let mut s = AppSettings::default();
            s.rewrite_mode = m;
            s
        })
        .collect();
    tries.push(custom);
    assert!(tries.len() >= 7);
    for settings in &tries {
        for vocabulary in [no_words(), words(&["Scribe", "Deepgram"])] {
            let real = messages("Alors voilà, la réunion est décalée à jeudi.", &vocabulary, &settings.active_profile());
            let warm = warm_up_messages(settings, &vocabulary, None);
            assert_eq!(real.len(), warm.len());
            assert_eq!(without_last(&real), without_last(&warm));
        }
    }
}

/// prompts-019 : le préchauffage contient le dictionnaire ; Amusant = Simple ; la traduction aussi.
#[test]
fn prompts_019_warm_up_details() {
    let vocabulary = words(&["Scribe", "Deepgram"]);
    let mut formal = AppSettings::default();
    formal.rewrite_mode = RewriteMode::Formal;
    let warm = warm_up_messages(&formal, &vocabulary, None);
    let real = messages("Je voulais te dire que la réunion est décalée.", &vocabulary, &formal.active_profile());
    assert_eq!(warm.len(), real.len());
    assert_eq!(without_last(&warm), without_last(&real));
    assert_ne!(warm.last(), real.last());
    assert!(warm[0].content.contains("Deepgram"));
    let mut playful = formal.clone();
    playful.rewrite_mode = RewriteMode::Playful;
    let mut simple = formal.clone();
    simple.rewrite_mode = RewriteMode::Simple;
    assert_eq!(warm_up_messages(&playful, &vocabulary, None), warm_up_messages(&simple, &vocabulary, None));
    let warm_t = warm_up_messages(&formal, &vocabulary, Some(TranslationLanguage::En));
    let real_t = translation_messages("Bonjour tout le monde.", TranslationLanguage::En, &vocabulary);
    assert_eq!(without_last(&warm_t), without_last(&real_t));
    // Un profil « own » : le garde-fou de l'énumération reste cohérent.
    assert!(matches!(RewriteMode::Letter.profile().examples, Examples::None));
}
