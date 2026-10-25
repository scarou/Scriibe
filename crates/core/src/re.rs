//! Expressions régulières : une seule compilation par motif, et une seule façon de les écrire.

use fancy_regex::Regex;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

static CACHE: LazyLock<Mutex<HashMap<String, Arc<Regex>>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// Compile un motif construit à l'exécution (dictionnaire, signes), une seule fois par motif.
/// Renvoie `None` si le motif est invalide (par exemple un mot du dictionnaire qui casserait l'expression).
pub(crate) fn cached(pattern: &str) -> Option<Arc<Regex>> {
    let mut cache = CACHE.lock().ok()?;
    if let Some(re) = cache.get(pattern) {
        return Some(re.clone());
    }
    let re = Arc::new(Regex::new(pattern).ok()?);
    cache.insert(pattern.to_string(), re.clone());
    Some(re)
}

/// Compile un motif écrit en dur ; une erreur ici est une faute de programmation, pas de données.
pub(crate) fn compile(pattern: &str) -> Regex {
    Regex::new(pattern).unwrap_or_else(|e| panic!("motif invalide {pattern:?} : {e}"))
}

/// Déclare un motif compilé une seule fois : `regex!(NOM, r"...")` ou `regex!(NOM, format!(...))`.
macro_rules! regex {
    ($name:ident, $pattern:expr) => {
        static $name: std::sync::LazyLock<fancy_regex::Regex> = std::sync::LazyLock::new(|| $crate::re::compile(&$pattern));
    };
}
pub(crate) use regex;

/// Positions (octets) de chaque correspondance et de son groupe `group`, dans l'ordre du texte.
/// Une erreur d'exécution du moteur (trop de retours en arrière) ne produit aucune correspondance, comme un échec de recherche.
pub(crate) fn matches_with_group(re: &Regex, text: &str, group: usize) -> Vec<((usize, usize), (usize, usize))> {
    re.captures_iter(text)
        .filter_map(Result::ok)
        .filter_map(|c| {
            let whole = c.get(0)?;
            let g = c.get(group).map(|m| (m.start(), m.end())).unwrap_or((whole.start(), whole.start()));
            Some(((whole.start(), whole.end()), g))
        })
        .collect()
}

/// Vrai si le motif trouve quelque chose dans le texte.
pub(crate) fn is_match(re: &Regex, text: &str) -> bool {
    re.is_match(text).unwrap_or(false)
}
