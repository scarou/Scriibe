//! Petites fonctions de texte partagées : comptage de mots, filtre des phrases inventées par Whisper.

use crate::re::{is_match, regex};

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
