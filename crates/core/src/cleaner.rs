//! Mise au propre : ce que reçoit le modèle (consigne, exemples, dictée), nettoyage de sa sortie, garde-fous.
//! Port de l'énumération `Cleaner` de `Services.swift` (Scribe), sans les appels réseau.

use crate::lang::LanguageGuesser;
use crate::modes::{Examples, ModeProfile, RewriteMode};
use crate::prompt_text::*;
use crate::re::regex;
use crate::settings::AppSettings;
use crate::text::{looks_clean, overlap, overlap_without, word_count};
use crate::translation::TranslationLanguage;
use std::collections::HashSet;

/// Un message de la conversation avec le modèle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub role: &'static str,
    pub content: String,
}

fn message(role: &'static str, content: impl Into<String>) -> Message {
    Message { role, content: content.into() }
}

// MARK: Garde-fous

/// Message d'une reformulation écartée par le garde-fou (le texte brut est alors collé).
pub const REWRITE_REJECTED: &str = "Reformulation écartée";
/// Message de la traduction écartée par le garde-fou (le repli remet alors la dictée au propre en français).
pub const TRANSLATION_REJECTED: &str = "Traduction écartée";

/// Au moins ce nombre de mots significatifs (hors mots de commande) pour exiger `SIMPLE_KEPT_LONG` en mode Simple.
const SIMPLE_MIN_WORDS: usize = 8;
/// Part minimale des mots de la dictée gardés par le mode Simple sur une dictée longue.
const SIMPLE_KEPT_LONG: f64 = 0.75;
/// Mots de commande dictés (le modèle les remplace par un signe) : ignorés dans la part de mots gardés du mode Simple.
const DICTATED_SIGN_WORDS: &[&str] = &[
    "ouvrez",
    "ouvre",
    "ouvrir",
    "fermez",
    "ferme",
    "fermer",
    "parenthèse",
    "parenthèses",
    "guillemet",
    "guillemets",
    "deux",
    "points",
    "point",
    "virgule",
    "ligne",
    "retour",
    "numéro",
    "suivant",
    "puce",
    "tirer",
    "crochet",
    "crochets",
    "accolades",
    "slash",
    "arobase",
    "open",
    "close",
    "parenthesis",
    "bracket",
    "quote",
    "colon",
    "comma",
    "newline",
    "line",
    "new",
    "dash",
    "bullet",
];

/// Garde-fou : une réécriture reprend les mots de la dictée. Une sortie vide, démesurée, ou faite surtout de mots nouveaux
/// veut dire que le modèle a répondu (ou résumé) au lieu de réécrire : on colle alors le texte brut.
/// Les modes de style (soutenu, pro) reformulent : la sortie peut être plus longue et partager moins de mots.
pub fn accepts_for_mode(input: &str, output: &str, mode: RewriteMode) -> bool {
    accepts(input, output, mode.rewrites_freely())
}

/// `rewrites` : le mode reformule (modes métiers, modes personnels qui reformulent), le garde-fou est plus souple.
pub fn accepts(input: &str, output: &str, rewrites: bool) -> bool {
    let input_len = input.chars().count();
    let max_length = if rewrites { input_len * 3 + 120 } else { input_len * 2 + 80 };
    if output.is_empty() || output.chars().count() > max_length {
        return false;
    }
    // Une question dictée reste une question : sans point d'interrogation, la sortie est sans doute la réponse.
    if input.contains('?') && !output.contains('?') {
        return false;
    }
    if !keeps_signs(input, output) {
        return false;
    }
    // « Oui, … », « Voici … » en tête alors que la dictée ne commence pas ainsi : le modèle a répondu.
    let opener = leading_words(output, 1).into_iter().next().unwrap_or_default();
    if ["oui", "non", "voici", "bien", "certainement", "certes", "d'accord"].contains(&opener.as_str())
        && !leading_words(input, 3).contains(&opener)
    {
        return false;
    }
    let o = overlap(input, output);
    // En style, une question reformulée peut partager peu de mots (le « ? » la protège déjà) ; une demande exécutée,
    // elle, est faite surtout de mots nouveaux : sans « ? », la sortie doit venir en bonne partie de la dictée.
    if !rewrites {
        if !(o.kept >= 0.4 && o.own >= 0.5) {
            return false;
        }
        // Dictée longue : le modèle doit garder l'essentiel des mots (S19, rejeu de l'historique : un 0,75 pris tel quel
        // écartait de bonnes dictées à signes dictés, d'où les mots de commande comptés à part et le plancher de longueur,
        // sous lequel une seule correction de Whisper ferait chuter la part).
        let (strict, words) = overlap_without(input, output, DICTATED_SIGN_WORDS);
        return words < SIMPLE_MIN_WORDS || strict.kept >= SIMPLE_KEPT_LONG;
    }
    o.kept >= 0.15 && o.own >= if input.contains('?') { 0.15 } else { 0.3 }
}

/// Les signes dictés (paires de parenthèses et de crochets, guillemets, retours à la ligne) ne doivent pas se perdre.
/// Le modèle peut en ajouter (« "c'est réglé" » → « « c'est réglé » ») ; un « :) » isolé ne compte pas.
/// Guillemets de toutes les langues : « » devient " " en anglais, „ “ en allemand.
fn keeps_signs(input: &str, output: &str) -> bool {
    fn count(text: &str, c: char) -> usize {
        text.chars().filter(|x| *x == c).count()
    }
    fn pairs(text: &str, open: char, close: char) -> usize {
        count(text, open).min(count(text, close))
    }
    fn quotes(text: &str) -> usize {
        text.chars().filter(|c| "«»\"“”„".contains(*c)).count()
    }
    pairs(output, '(', ')') >= pairs(input, '(', ')')
        && pairs(output, '[', ']') >= pairs(input, '[', ']')
        && quotes(output) >= quotes(input)
        && count(output, '\n') >= count(input, '\n')
}

/// Garde-fou de la traduction : les mots en commun ne disent rien ici, mais une question reste une question, les signes restent,
/// la longueur reste proche de la dictée (une traduction fait 0,8 à 1,3 fois le français ; une réponse ou une demande exécutée
/// s'allonge), aucun paragraphe ni formule de mail n'apparaît, le texte n'est pas resté en français, et il ne commence pas par
/// « Yes, », « Sure, » ou « Here is » si la dictée ne le fait pas.
pub fn accepts_translation(input: &str, output: &str, language: TranslationLanguage, guesser: &dyn LanguageGuesser) -> bool {
    let input_len = input.chars().count();
    if output.is_empty() || output.chars().count() > input_len * 3 / 2 + 30 {
        return false;
    }
    if input.contains('?') && !output.contains('?') {
        return false;
    }
    if !keeps_signs(input, output) {
        return false;
    }
    // Retours à la ligne en plus sur une dictée courte : le modèle a écrit le mail au lieu de traduire la demande.
    let newlines = |t: &str| t.chars().filter(|c| *c == '\n').count();
    if input_len < 400 && newlines(output) > newlines(input) {
        return false;
    }
    let opener = leading_words(output, 2);
    let dictated = leading_words(input, 3);
    // Pas « here », « no »… seuls : ils traduisent aussi « ici », « non » dictés en tête de phrase.
    const REPLIES: [&str; 12] = ["yes", "sure", "certainly", "here's", "sí", "claro", "ja", "natürlich", "sì", "ecco", "certo", "sim"];
    const AGREEING: [&str; 26] = [
        "oui",
        "voici",
        "voilà",
        "bien",
        "certainement",
        "certes",
        "d'accord",
        "ok",
        "évidemment",
        "sûrement",
        "absolument",
        "exactement",
        "carrément",
        "volontiers",
        "clairement",
        "effectivement",
        "si",
        "naturellement",
        "entendu",
        "tout",
        "pas",
        "avec",
        "parfait",
        "super",
        "génial",
        "ça",
    ];
    let first = opener.first().map(String::as_str).unwrap_or("");
    let replies = REPLIES.contains(&first) || opener == ["here", "is"];
    if replies && dictated.iter().all(|w| !AGREEING.contains(&w.as_str())) {
        return false;
    }
    const MAIL: [&str; 25] = [
        "dear", "hi", "hello", "hey", "subject", "hola", "estimado", "estimada", "querido", "querida", "asunto", "hallo", "liebe",
        "lieber", "betreff", "ciao", "gentile", "caro", "cara", "oggetto", "olá", "oi", "prezado", "prezada", "assunto",
    ];
    const GREETING: [&str; 13] =
        ["salut", "bonjour", "bonsoir", "coucou", "cher", "chère", "chers", "madame", "monsieur", "objet", "hello", "hi", "hey"];
    let is_mail = MAIL.contains(&first);
    let greets = |w: &String| GREETING.contains(&w.as_str());
    if is_mail && dictated.iter().all(|w| !greets(w)) {
        return false;
    }
    // Resté en français. Recopie ou simple mise au propre : aucun mot nouveau. Sinon, on n'écoute que les mots absents
    // de la dictée (un nom propre comme « Société Générale » ferait prendre une vraie traduction pour du français).
    fn words(text: &str) -> Vec<String> {
        text.to_lowercase().split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).map(str::to_string).collect()
    }
    let heard: HashSet<String> = words(input).into_iter().collect();
    let output_words = words(output);
    let novel: Vec<&String> = output_words.iter().filter(|w| !heard.contains(*w)).collect();
    if output_words.len() >= 3 && novel.is_empty() {
        return false;
    }
    if novel.len() >= 3 {
        let sample = novel.iter().map(|w| w.as_str()).collect::<Vec<_>>().join(" ");
        let odds = guesser.odds(&sample, language);
        if odds.french > 0.5 && odds.french > odds.target {
            return false;
        }
    }
    true
}

/// Premiers mots hors hésitations, en minuscules, apostrophe droite, « ouais » lu « oui » (« Euh, ah oui bon » → « oui », « bon »).
fn leading_words(text: &str, count: usize) -> Vec<String> {
    const SKIPPED: [&str; 11] = ["euh", "heu", "hum", "bah", "ben", "bon", "alors", "ah", "eh", "oh", "donc"];
    text.to_lowercase()
        .replace('’', "'")
        .split(|c: char| c.is_whitespace() || ",.;:!?…".contains(c))
        .filter(|w| !w.is_empty())
        .map(|w| if w == "ouais" { "oui".to_string() } else { w.to_string() })
        .filter(|w| !SKIPPED.contains(&w.as_str()))
        .take(count)
        .collect()
}

// MARK: Nettoyage de la sortie du modèle

/// Paragraphe ajouté par le modèle : en italique (« *…* »), ou « (Note… », « (Remarque… ». Un « (PS …) » dicté reste.
fn is_note(paragraph: &str) -> bool {
    let p = paragraph.to_lowercase();
    (p.starts_with('*') && p.ends_with('*') && p.chars().count() > 2)
        || p.starts_with("(note")
        || p.starts_with("(remarque")
        || p.starts_with("note :")
        || p.starts_with("note:")
        || p.starts_with("remarque :")
}

regex!(DICTATION_TAGS, r"(?i)</?dict[ée]e>");
regex!(BOLD, r"\*\*([^*\n]+?)\*\*");

fn trim_blanks(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() && c != '\n' && c != '\r')
}

/// Retire ce que les modèles ajoutent autour du texte : balises, bloc de code, guillemets, gras et italique Markdown,
/// et note finale (après « --- », ou dernier paragraphe en italique ou entre parenthèses).
pub fn strip(s: &str) -> String {
    let mut t = s.replace("*«", "«").replace("»*", "»");
    t = DICTATION_TAGS.replace_all(&t, "").into_owned();
    t = BOLD.replace_all(&t, "$1").into_owned();
    t = t.trim().to_string();
    if t.starts_with("```") {
        t = t.split('\n').skip(1).collect::<Vec<_>>().join("\n");
        if t.ends_with("```") {
            t.truncate(t.len() - 3);
        }
        t = t.trim().to_string();
    }
    // Note du modèle après « --- » : coupée seulement si ce qui suit ressemble à une note, pas à du texte dicté.
    if let Some(at) = t.rfind("\n---") {
        let after = t[at + 4..].trim_matches(|c: char| c == '-' || c.is_whitespace());
        if is_note(after) {
            t.truncate(at);
        }
    }
    let mut paragraphs: Vec<String> = t.split("\n\n").map(|p| p.trim().to_string()).collect();
    while paragraphs.len() > 1 && paragraphs.last().is_some_and(|last| last.is_empty() || is_note(last)) {
        paragraphs.pop();
    }
    t = paragraphs.join("\n\n");
    // Guillemets autour de tout le texte, sauf s'il en contient d'autres (« "Viens", dit-il, "maintenant" »).
    for (open, close) in [("\"", "\""), ("«", "»"), ("“", "”")] {
        if t.starts_with(open) && t.ends_with(close) && t.chars().count() > 2 {
            let mut chars = t.chars();
            chars.next();
            chars.next_back();
            let inner = chars.as_str();
            if !inner.contains(open) && !inner.contains(close) {
                t = trim_blanks(inner).to_string();
            }
        }
    }
    t
}

// MARK: Prompts

fn fill(template: &str, replacements: &[(&str, &str)]) -> String {
    replacements.iter().fold(template.to_string(), |acc, (key, value)| acc.replace(key, value))
}

/// Les modes livrés envoient exactement leur consigne d'avant ; un mode personnel fidèle ajoute sa consigne à celle de Simple.
pub fn system_prompt(vocabulary: &[String], profile: &ModeProfile) -> String {
    let rewrites = profile.rewrites;
    let style_line = match &profile.style {
        None => FAITHFUL_LINE.to_string(),
        Some(style) if rewrites => format!("- Style : {style} Tu peux reformuler les phrases pour respecter ce style, en gardant le sens."),
        Some(style) => format!("- Consigne en plus : {style}\n{FAITHFUL_LINE}"),
    };
    let lines = [
        SYSTEM_HEAD.to_string(),
        SYSTEM_QUESTION.to_string(),
        format!("{SYSTEM_REMOVE_PREFIX}{}{SYSTEM_REMOVE_SUFFIX}", if rewrites { "" } else { "seulement " }),
        format!("{SYSTEM_PERSONS_PREFIX}{}", if profile.vouvoie { SYSTEM_PERSONS_VOUVOIE } else { SYSTEM_PERSONS_TUTOIE }),
        (if rewrites { SYSTEM_KEEP_REWRITES } else { SYSTEM_KEEP_FAITHFUL }).to_string(),
        SYSTEM_REPRISE.to_string(),
        SYSTEM_PUNCTUATION.to_string(),
        SYSTEM_SIGNS.to_string(),
        SYSTEM_TYPOGRAPHY.to_string(),
        style_line,
        SYSTEM_LANGUAGE.to_string(),
    ];
    let mut prompt = lines.join("\n");
    let terms: Vec<&str> = vocabulary.iter().map(String::as_str).filter(|t| !t.is_empty()).collect();
    if !terms.is_empty() {
        prompt.push_str(&format!("{VOCABULARY_PREFIX}{}{VOCABULARY_SUFFIX}", terms.join(", ")));
    }
    prompt.push_str(SYSTEM_TAIL);
    prompt
}

fn wrap(dictation: &str, rewrites: bool) -> String {
    let how = if rewrites { WRAP_HOW_REWRITES } else { WRAP_HOW_FAITHFUL };
    fill(WRAP_TEMPLATE, &[("\\(dictation)", dictation), ("\\(how)", how)])
}

/// Tout ce que reçoit le modèle : consigne, exemples (dictée puis réponse attendue), puis la dictée encadrée de balises avec un rappel.
/// Sans les balises ni les exemples, les petits modèles prennent une question dictée pour une question qu'on leur pose.
/// Pas de nom d'appli : tout ce qui précède la dictée est identique d'une dictée à l'autre, et le serveur le reprend de sa mémoire.
pub fn messages(text: &str, vocabulary: &[String], profile: &ModeProfile) -> Vec<Message> {
    let mut list = vec![message("system", system_prompt(vocabulary, profile))];
    let shots: Vec<(String, String)> = match &profile.examples {
        Examples::Faithful => FAITHFUL_EXAMPLES.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect(),
        Examples::Style => STYLE_EXAMPLES.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect(),
        Examples::None => Vec::new(),
        // Un mode fidèle garde les exemples de Simple (reprises, questions) et ajoute les siens après ; un mode qui
        // reformule ne prend que les siens (ceux de Soutenu tireraient vers un autre style).
        Examples::Own(own) => {
            let base: Vec<(String, String)> =
                if profile.rewrites { Vec::new() } else { FAITHFUL_EXAMPLES.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect() };
            base.into_iter().chain(own.iter().map(|e| (e.dictation.clone(), e.expected.clone()))).collect()
        }
    };
    for (dictation, rewritten) in shots {
        list.push(message("user", wrap(&dictation, profile.rewrites)));
        list.push(message("assistant", rewritten));
    }
    list.push(message("user", wrap(text, profile.rewrites)));
    list
}

/// Les messages pour un mode livré, avec sa phrase de style d'origine.
pub fn messages_for_mode(text: &str, vocabulary: &[String], mode: RewriteMode) -> Vec<Message> {
    messages(text, vocabulary, &mode.profile())
}

/// Les exemples de la mise au propre en mode Simple (dictée, texte attendu).
pub fn examples() -> &'static [(&'static str, &'static str)] {
    &FAITHFUL_EXAMPLES
}

/// Les exemples des modes de style (dictée, texte attendu).
pub fn style_examples() -> &'static [(&'static str, &'static str)] {
    &STYLE_EXAMPLES
}

/// Les dictées d'exemple de la traduction (leurs traductions sont dans [`TranslationLanguage::examples`]).
pub fn translation_examples() -> &'static [&'static str] {
    &TRANSLATION_EXAMPLES
}

fn wrap_for_translation(dictation: &str, language: TranslationLanguage) -> String {
    fill(WRAP_TRANSLATION_TEMPLATE, &[("\\(dictation)", dictation), ("\\(language.promptName)", language.prompt_name())])
}

/// Même méthode que la mise au propre : consigne, exemples traduits dans la langue cible, dictée entre balises.
pub fn translation_messages(text: &str, language: TranslationLanguage, vocabulary: &[String]) -> Vec<Message> {
    let mut list = vec![message("system", translation_prompt(language, vocabulary))];
    for (dictation, translated) in TRANSLATION_EXAMPLES.iter().zip(language.examples()) {
        list.push(message("user", wrap_for_translation(dictation, language)));
        list.push(message("assistant", translated));
    }
    list.push(message("user", wrap_for_translation(text, language)));
    list
}

pub fn translation_prompt(language: TranslationLanguage, vocabulary: &[String]) -> String {
    let lines = [
        fill(TRANSLATION_HEAD, &[("\\(language.promptName)", language.prompt_name())]),
        TRANSLATION_LINE_1.to_string(),
        TRANSLATION_LINE_2.to_string(),
        TRANSLATION_LINE_3.to_string(),
        TRANSLATION_LINE_4.to_string(),
        TRANSLATION_LINE_5.to_string(),
        TRANSLATION_LINE_6.to_string(),
        TRANSLATION_LINE_7.to_string(),
    ];
    let mut prompt = lines.join("\n");
    let terms: Vec<&str> = vocabulary.iter().map(String::as_str).filter(|t| !t.is_empty()).collect();
    if !terms.is_empty() {
        prompt.push_str(&format!("{TRANSLATION_VOCABULARY_PREFIX}{}{VOCABULARY_SUFFIX}", terms.join(", ")));
    }
    prompt.push_str(TRANSLATION_TAIL);
    prompt
}

/// Le prompt d'une vraie dictée, avec une phrase factice : tout ce qui précède la dictée est identique d'une dictée à l'autre.
pub fn warm_up_messages(settings: &AppSettings, vocabulary: &[String], translating: Option<TranslationLanguage>) -> Vec<Message> {
    let text = "Bonjour.";
    if let Some(language) = translating {
        return translation_messages(text, language, vocabulary);
    }
    let mut plain = settings.clone();
    // Amusant : la mise au propre se fait en Simple (puis le moteur schtroumpf, sans modèle).
    if plain.rewrite_mode == RewriteMode::Playful {
        plain.rewrite_mode = RewriteMode::Simple;
    }
    messages(text, vocabulary, &plain.active_profile())
}

/// Empreinte (FNV-1a 64 bits) de tout ce qu'envoient les 8 modes livrés, sans phrase de style modifiée, pour une dictée et un
/// vocabulaire fixes : elle ne doit changer que si on modifie volontairement un prompt.
pub fn built_in_prompts_fingerprint() -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    let vocabulary = vec!["Claude Code".to_string()];
    for mode in RewriteMode::ALL {
        for m in messages_for_mode("dictée témoin", &vocabulary, mode) {
            for byte in format!("{}|{}\n", m.role, m.content).bytes() {
                hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3);
            }
        }
    }
    format!("{hash:x}")
}

/// Option « sauter si déjà propre » : le modèle n'est pas appelé (Simple ou Amusant, avec Whisper, hors traduction,
/// plus de trois mots — en dessous, il est déjà sauté — et un texte que `looks_clean` juge propre).
pub fn skips_cleanup(text: &str, settings: &AppSettings, translating: bool) -> bool {
    !translating && settings.skip_when_clean && settings.skip_applies() && word_count(text) > 3 && looks_clean(text)
}
