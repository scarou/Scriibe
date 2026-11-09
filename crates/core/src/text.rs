//! Petites fonctions de texte partagées : comptage de mots, filtre des phrases inventées par Whisper.

use crate::re::{is_match, regex};
use std::collections::HashSet;

/// Nombre de mots, séparés par des blancs.
pub fn word_count(text: &str) -> usize {
    text.split_whitespace().count()
}

regex!(TRIM_EDGES, r"^[\s\p{P}]+|[\s\p{P}]+$");

/// Phrases que Whisper invente quand l'audio est vide ou ne contient que du bruit (il les a apprises dans les sous-titres de vidéos).
/// À n'appliquer qu'à Whisper. Les fragments ne comptent que sur un texte court : « Active les sous-titres par défaut dans VLC »
/// reste une vraie dictée. Renvoie le motif reconnu, ou `None`.
pub fn known_hallucination(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    let t = TRIM_EDGES.replace_all(&lower, "").into_owned();
    const EXACT: [&str; 8] = [
        "www.sign.org",
        "sign.org",
        "merci d'avoir regardé",
        "merci d'avoir regardé cette vidéo",
        "thank you for watching",
        "thanks for watching",
        "sous-titrage st' 501",
        "musique",
    ];
    if EXACT.contains(&t.as_str()) {
        return Some(t);
    }
    if word_count(&t) > 8 {
        return None;
    }
    const FRAGMENTS: [&str; 3] = ["amara.org", "sous-titres réalisés par", "sous-titrage société radio-canada"];
    FRAGMENTS.iter().find(|f| t.contains(**f)).map(|f| f.to_string())
}

// Scriibe (S15) : un code épelé (« zéro neuf deux six F A 6 ») que Whisper lit comme un nombre décimal et écrit « 0,926FA6 ».
// Un chiffre, une virgule, puis au moins cinq caractères hexadécimaux mêlant chiffres et lettres, collés : ce n'est pas un nombre.
regex!(SPELLED_CODE, r"(?<![\p{L}\p{N},.])(\d),((?=[0-9A-Fa-f]*[A-Fa-f])(?=[0-9A-Fa-f]*\d)[0-9A-Fa-f]{5,})(?![\p{L}\p{N}])");

/// Recolle un code épelé que Whisper a coupé d'une virgule décimale : « 0,926FA6 » → « 0926FA6 ». Un nombre (« 3,14159 »,
/// « 1,5 Mo ») ou un code court (« 3,5db ») reste tel quel.
pub fn join_spelled_code(text: &str) -> String {
    SPELLED_CODE.replace_all(text, "$1$2").into_owned()
}

regex!(BADLY_SPLIT, r"([.?!][ \x{00A0}]+|\n[ \x{00A0}]*)[a-zàâçéèêëîïôûùüÿœ]");
regex!(
    ASKS_WITH_PERIOD,
    r"(^|[.!?…]\s+)(est-ce qu|qu'est-ce qu|pourquoi|combien|tu peux|peux-tu|pourrais-tu|tu pourrais|vous pouvez|pouvez-vous)[^.?!…]*\."
);
regex!(WHITESPACE_RUN, r"\s+");
regex!(HESITATION, r"^(e+u+h+|h+e+u+|h+m+|m+h+|hum+|bah|ben|hein|pff+)$");

/// Whisper a déjà ponctué et découpé, et rien ne demande le modèle : ni hésitation, ni reprise (« non pardon », « lundi, enfin mardi »,
/// « à 10 h, non, à 11 h »), ni répétition d'un ou deux mots, ni question finie par un point, ni texte assez long pour être découpé
/// en paragraphes. Dans le doute, le modèle est appelé : on perd du temps, pas du sens.
pub fn looks_clean(text: &str) -> bool {
    let trimmed = text.trim();
    let Some(first) = trimmed.chars().find(|c| !"«(\"“ ".contains(*c)) else { return false };
    let Some(last) = trimmed.chars().last() else { return false };
    if !(first.is_uppercase() || first.is_numeric()) || !".?!…»)\"”".contains(last) || word_count(trimmed) > 60 {
        return false;
    }
    let lower = trimmed.to_lowercase().replace('’', "'");
    // Phrase mal découpée : minuscule après une fin de phrase ou un retour à la ligne (« … » peut être suivi d'une minuscule).
    if is_match(&BADLY_SPLIT, trimmed) {
        return false;
    }
    // Question finie par un point.
    if is_match(&ASKS_WITH_PERIOD, &lower) {
        return false;
    }
    // Reprises : ces tournures s'entourent de virgules quand on se corrige.
    const REPRISES: [&str; 16] = [
        "non pardon",
        "pardon non",
        ", pardon",
        "pardon,",
        ", non,",
        "enfin non",
        ", enfin",
        "ou plutôt",
        "non plutôt",
        ", plutôt",
        "je me reprends",
        "je recommence",
        "bon ben",
        ", je veux dire",
        ", je voulais dire",
        ", tu vois",
    ];
    let spaced = WHITESPACE_RUN.replace_all(&lower, " ");
    if REPRISES.iter().any(|r| spaced.contains(r)) {
        return false;
    }
    // Hésitations, y compris allongées (« euuh », « hmm »), et mots répétés, phrase par phrase.
    const DOUBLED: [&str; 6] = ["oui", "non", "si", "très", "nous", "vous"];
    for sentence in lower.split(|c| ".?!…\n".contains(c)).filter(|s| !s.is_empty()) {
        let words: Vec<&str> = sentence.split(|c: char| !c.is_alphanumeric() && c != '\'' && c != '-').filter(|w| !w.is_empty()).collect();
        // « enfin » corrige souvent ce qui précède (« lundi enfin mardi »), même sans virgule : dans le doute, le modèle.
        if words.iter().any(|w| is_match(&HESITATION, w) || *w == "enfin") {
            return false;
        }
        if words.windows(2).any(|p| p[0] == p[1] && !DOUBLED.contains(&p[0])) {
            return false;
        }
        if words.len() >= 4 && (0..words.len() - 3).any(|i| words[i] == words[i + 2] && words[i + 1] == words[i + 3]) {
            return false;
        }
    }
    true
}

const FILLERS: [&str; 6] = ["euh", "heu", "hum", "bah", "ben", "bon"];

/// Nombres en lettres et heures : le modèle a le droit de les écrire en chiffres (« quinze heures trente » → « 15 h 30 »).
const NUMBER_WORDS: [&str; 31] = [
    "une",
    "deux",
    "trois",
    "quatre",
    "cinq",
    "six",
    "sept",
    "huit",
    "neuf",
    "dix",
    "onze",
    "douze",
    "treize",
    "quatorze",
    "quinze",
    "seize",
    "vingt",
    "vingts",
    "trente",
    "quarante",
    "cinquante",
    "soixante",
    "cent",
    "cents",
    "mille",
    "million",
    "millions",
    "milliard",
    "milliards",
    "heure",
    "heures",
];

fn significant_words(text: &str) -> HashSet<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= 3 && !w.chars().next().is_some_and(char::is_numeric))
        .filter(|w| !FILLERS.contains(w) && !NUMBER_WORDS.contains(w))
        .map(str::to_string)
        .collect()
}

/// Mots en commun entre la dictée et la sortie du modèle, pour repérer une réponse à la place d'une réécriture.
/// `kept` : part des mots de la dictée retrouvés dans la sortie ; `own` : part des mots de la sortie déjà présents dans la dictée.
/// On ne compte que les mots d'au moins trois lettres, hors hésitations et nombres (en chiffres ou en lettres).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Overlap {
    pub kept: f64,
    pub own: f64,
}

pub fn overlap(input: &str, output: &str) -> Overlap {
    overlap_without(input, output, &[]).0
}

/// Comme `overlap`, sans compter les mots de `ignored` (mots de commande dictés comme « parenthèse » ou « ligne », que
/// le modèle remplace par un signe). Renvoie aussi le nombre de mots significatifs de la dictée une fois ceux-là retirés.
pub fn overlap_without(input: &str, output: &str, ignored: &[&str]) -> (Overlap, usize) {
    let mut a = significant_words(input);
    let mut b = significant_words(output);
    a.retain(|w| !ignored.contains(&w.as_str()));
    b.retain(|w| !ignored.contains(&w.as_str()));
    let count = a.len();
    (overlap_of(&a, &b), count)
}

fn overlap_of(a: &HashSet<String>, b: &HashSet<String>) -> Overlap {
    if a.is_empty() || b.is_empty() {
        return Overlap { kept: if a.is_empty() { 1.0 } else { 0.0 }, own: if b.is_empty() { 1.0 } else { 0.0 } };
    }
    let common = a.intersection(b).count() as f64;
    Overlap { kept: common / a.len() as f64, own: common / b.len() as f64 }
}
