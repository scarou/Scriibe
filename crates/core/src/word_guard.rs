//! Contrôle mot à mot après le modèle (banc du 25/11/2026, `docs/BANC_MISE_AU_PROPRE.md`).
//!
//! Aligne la dictée et la sortie du modèle, mot à mot. Ne passent que les changements sans risque pour le sens : ponctuation, casse,
//! accents, traits d'union, nombres (chiffres ↔ lettres), accords de genre et de nombre, mots du dictionnaire, signes dictés et
//! hésitations. Tout autre remplacement, suppression ou ajout est défait : le mot dicté revient.

use std::collections::HashSet;

/// Un mot d'un texte : sa place (octets) et sa forme comparable (minuscules, sans accents).
struct Word {
    start: usize,
    end: usize,
    norm: String,
}

fn fold(c: char) -> Option<char> {
    let c = c.to_lowercase().next()?;
    Some(match c {
        'à' | 'â' | 'ä' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'î' | 'ï' => 'i',
        'ô' | 'ö' | 'œ' => 'o',
        'ù' | 'û' | 'ü' => 'u',
        'ç' => 'c',
        other => other,
    })
}

fn norm(word: &str) -> String {
    word.chars().filter_map(fold).collect()
}

/// Les mots (suites de lettres et de chiffres) ; l'apostrophe et le trait d'union séparent, comme dans la mesure du banc.
fn split(text: &str) -> Vec<Word> {
    let mut words = Vec::new();
    let mut start = None;
    for (i, c) in text.char_indices() {
        if c.is_alphanumeric() {
            start.get_or_insert(i);
        } else if let Some(s) = start.take() {
            words.push(Word { start: s, end: i, norm: norm(&text[s..i]) });
        }
    }
    if let Some(s) = start {
        words.push(Word { start: s, end: text.len(), norm: norm(&text[s..]) });
    }
    words
}

const HESITATIONS: &[&str] = &["euh", "heu", "hum", "hmm", "mmh"];

/// Mots de commande des signes dictés (« ouvrez les guillemets », « point d'interrogation »…) : la mise en forme les change légitimement.
const SIGNS: &[&str] = &[
    "ouvrez",
    "ouvre",
    "ouvrir",
    "fermez",
    "ferme",
    "fermer",
    "guillemets",
    "guillemet",
    "parenthese",
    "parentheses",
    "crochet",
    "crochets",
    "virgule",
    "point",
    "interrogation",
    "exclamation",
    "ligne",
    "la",
    "les",
    "deux",
    "points",
    "tiret",
    "barre",
    "slash",
    "a",
    "retour",
    "paragraphe",
];

/// Une zone qui change est un signe dicté si tous ses mots sont des mots de commande et qu'un au moins n'est pas un mot courant
/// (« la », « les », « a », « deux » seuls ne suffisent pas : les perdre serait perdre un mot dicté).
fn is_sign_zone(words: &[&str]) -> bool {
    words.iter().all(|w| SIGNS.contains(w) || HESITATIONS.contains(w))
        && words.iter().any(|w| !["la", "les", "deux", "a"].contains(w) && !HESITATIONS.contains(w))
}

/// Formes d'un même verbe ou déterminant qui s'accordent (sujet, genre, nombre) : irrégulières, hors de portée de la règle des terminaisons.
const FAMILIES: &[&[&str]] = &[
    &["suis", "es", "est", "sommes", "etes", "sont"],
    &["ai", "as", "a", "avons", "avez", "ont"],
    &["vais", "vas", "va", "allons", "allez", "vont"],
    &["etait", "etais", "etions", "etiez", "etaient"],
    &["avait", "avais", "avions", "aviez", "avaient"],
    &["peut", "peux", "peuvent"],
    &["veut", "veux", "veulent"],
    &["doit", "dois", "doivent"],
    &["fait", "fais", "font"],
    &["bon", "bons", "bonne", "bonnes"],
    &["le", "la", "les", "l"],
    &["un", "une"],
    &["ce", "cet", "cette", "ces"],
    &["mon", "ma", "mes"],
    &["ton", "ta", "tes"],
    &["son", "sa", "ses"],
    &["leur", "leurs"],
    &["au", "aux"],
    &["tout", "toute", "tous", "toutes"],
    &["celui", "celle", "ceux", "celles"],
    &["quel", "quelle", "quels", "quelles"],
    &["nouveau", "nouvel", "nouvelle", "nouveaux", "nouvelles"],
    &["beau", "bel", "belle", "beaux", "belles"],
];

const NUMBER_WORDS: &[&str] = &[
    "zero",
    "un",
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
    "trente",
    "quarante",
    "cinquante",
    "soixante",
    "cent",
    "cents",
    "mille",
    "million",
    "millions",
    "et",
    "heure",
    "heures",
    "euro",
    "euros",
    "pour",
    "demi",
    "demie",
    "quart",
    "moins",
    "h",
];

/// Le mot et ce qu'il devient sans une terminaison d'accord (genre, nombre, troisième personne du pluriel).
fn stems(word: &str) -> Vec<&str> {
    let mut forms = vec![word];
    forms.extend(["ees", "ee", "es", "ent", "e", "s", "x"].iter().filter_map(|tail| word.strip_suffix(tail)));
    forms
}

/// Même mot au genre, au nombre ou à la personne près.
fn agrees(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    if FAMILIES.iter().any(|f| f.contains(&a) && f.contains(&b)) {
        return true;
    }
    stems(a).iter().any(|x| x.len() >= 4 && stems(b).contains(x))
}

fn has_digit(words: &[&str]) -> bool {
    words.iter().any(|w| w.chars().any(|c| c.is_ascii_digit()))
}

fn number_words(words: &[&str]) -> bool {
    words.iter().all(|w| NUMBER_WORDS.contains(w) || w.chars().any(|c| c.is_ascii_digit()))
}

/// Un remplacement `old` (dicté) → `new` (sortie du modèle) qu'on laisse passer ?
fn accept_replacement(old: &[&str], new: &[&str], kept: &HashSet<String>) -> bool {
    if new.iter().all(|w| kept.contains(*w)) {
        return true; // mot du dictionnaire (« scrib » → « scriibe »)
    }
    if is_sign_zone(&old.iter().chain(new).copied().collect::<Vec<_>>()) {
        return true;
    }
    if old.len() == new.len() && old.iter().zip(new).all(|(a, b)| agrees(a, b)) {
        return true;
    }
    // Chiffres ↔ lettres, et chiffres regroupés (« 1 000 » → « 1000 »).
    if (has_digit(old) || has_digit(new)) && number_words(old) && number_words(new) {
        return true;
    }
    // Mot coupé ou recollé (« peut être » ↔ « peutetre ») : mêmes lettres.
    old.concat() == new.concat()
}

fn accept_deletion(old: &[&str]) -> bool {
    old.iter().all(|w| HESITATIONS.contains(w)) || is_sign_zone(old)
}

fn accept_insertion(new: &[&str], kept: &HashSet<String>) -> bool {
    new.iter().all(|w| kept.contains(*w)) || is_sign_zone(new)
}

/// Alignement par plus longue sous-suite commune : `(début dictée, fin dictée, début sortie, fin sortie)` des zones qui diffèrent.
fn differences(a: &[Word], b: &[Word]) -> Vec<(usize, usize, usize, usize)> {
    let (n, m) = (a.len(), b.len());
    let mut lcs = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            lcs[i][j] = if a[i].norm == b[j].norm { lcs[i + 1][j + 1] + 1 } else { lcs[i + 1][j].max(lcs[i][j + 1]) };
        }
    }
    let (mut i, mut j) = (0, 0);
    let mut zones = Vec::new();
    let mut open: Option<(usize, usize)> = None;
    while i < n || j < m {
        if i < n && j < m && a[i].norm == b[j].norm {
            if let Some((si, sj)) = open.take() {
                zones.push((si, i, sj, j));
            }
            i += 1;
            j += 1;
        } else {
            open.get_or_insert((i, j));
            if j < m && (i == n || lcs[i][j + 1] >= lcs[i + 1][j]) {
                j += 1;
            } else {
                i += 1;
            }
        }
    }
    if let Some((si, sj)) = open {
        zones.push((si, n, sj, m));
    }
    zones
}

fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map(|c| c.to_uppercase().chain(chars).collect()).unwrap_or_default()
}

fn lowercase_first(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map(|c| c.to_lowercase().chain(chars).collect()).unwrap_or_default()
}

/// La sortie du modèle, où tout changement de mots non permis est défait. `vocabulary` : les termes du dictionnaire.
pub fn restore_dictated_words(dictation: &str, output: &str, vocabulary: &[String]) -> String {
    let kept: HashSet<String> = vocabulary.iter().flat_map(|t| split(t)).map(|w| w.norm).collect();
    let (a, b) = (split(dictation), split(output));
    let mut result = String::new();
    let mut cursor = 0; // position dans `output` déjà recopiée
    for (ai, aj, bi, bj) in differences(&a, &b) {
        let old: Vec<&str> = a[ai..aj].iter().map(|w| w.norm.as_str()).collect();
        let new: Vec<&str> = b[bi..bj].iter().map(|w| w.norm.as_str()).collect();
        let ok = match (old.is_empty(), new.is_empty()) {
            (false, false) => accept_replacement(&old, &new, &kept),
            (false, true) => accept_deletion(&old),
            (true, false) => accept_insertion(&new, &kept),
            (true, true) => true,
        };
        if ok {
            continue;
        }
        let zone_start = if bi < b.len() { b[bi].start } else { output.len() };
        let zone_end = if bj > bi { b[bj - 1].end } else { zone_start };
        result.push_str(&output[cursor..zone_start]);
        let dictated = if aj > ai { &dictation[a[ai].start..a[aj - 1].end] } else { "" };
        let starts_sentence = result.trim_end().is_empty() || result.trim_end().ends_with(['.', '!', '?', '…', '\n']);
        let model_capital = bj > bi && output[b[bi].start..].chars().next().is_some_and(char::is_uppercase);
        let mut restored = dictated.to_string();
        if (starts_sentence || model_capital) && !restored.is_empty() {
            restored = capitalize(&restored);
        }
        if bj > bi {
            // Remplacement refusé : les mots dictés reprennent la place de ceux du modèle (ou disparaît, si rien n'était dicté).
            result.push_str(&restored);
            cursor = zone_end;
            if restored.is_empty() {
                // Mot ajouté par le modèle : retiré avec l'espace qui le suivait.
                let rest = &output[cursor..];
                cursor += rest.len() - rest.trim_start_matches(' ').len();
                if result.ends_with(' ') && output[cursor..].starts_with([',', '.', ';', ':', '!', '?']) {
                    result.pop();
                }
            }
            continue;
        }
        if !restored.is_empty() {
            // Mot supprimé par le modèle : il revient, suivi d'une espace ; le mot suivant reprend la casse de la dictée.
            if !result.is_empty() && !result.ends_with(char::is_whitespace) {
                result.push(' ');
            }
            result.push_str(&restored);
            result.push(' ');
            cursor = zone_start;
            if starts_sentence && bi < b.len() {
                let upper = a.get(aj).is_some_and(|w| dictation[w.start..].chars().next().is_some_and(char::is_uppercase));
                let rest = &output[cursor..];
                let word_end = rest.find(|c: char| !c.is_alphanumeric()).unwrap_or(rest.len());
                let word = &rest[..word_end];
                result.push_str(&if upper { capitalize(word) } else { lowercase_first(word) });
                cursor += word_end;
            }
        }
    }
    result.push_str(&output[cursor..]);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fix(dictation: &str, output: &str) -> String {
        restore_dictated_words(dictation, output, &["Scriibe".to_string()])
    }

    #[test]
    fn punctuation_case_and_accents_pass() {
        assert_eq!(fix("on part a midi", "On part à midi."), "On part à midi.");
    }

    #[test]
    fn agreement_corrections_pass() {
        assert_eq!(fix("les fiches est prêt", "Les fiches sont prêtes."), "Les fiches sont prêtes.");
        assert_eq!(
            fix("j'ai reçu les document que tu m'a envoyé", "J'ai reçu les documents que tu m'as envoyés."),
            "J'ai reçu les documents que tu m'as envoyés."
        );
    }

    #[test]
    fn numbers_and_dictionary_pass() {
        assert_eq!(fix("rendez-vous à cinq heures", "Rendez-vous à 5 h."), "Rendez-vous à 5 h.");
        assert_eq!(fix("ouvre scrib", "Ouvre Scriibe."), "Ouvre Scriibe.");
    }

    #[test]
    fn hesitations_are_dropped() {
        assert_eq!(fix("euh je viens", "Je viens."), "Je viens.");
    }

    #[test]
    fn replaced_word_comes_back() {
        assert_eq!(fix("ça marche bien", "Cela marche bien."), "Ça marche bien.");
        assert_eq!(fix("on voit des fiches", "On voit ces fiches."), "On voit des fiches.");
    }

    #[test]
    fn deleted_word_comes_back() {
        assert_eq!(fix("je pense que mais bon c'est fait", "Je pense que bon c'est fait."), "Je pense que mais bon c'est fait.");
        assert_eq!(fix("et le modèle part", "Le modèle part."), "Et le modèle part.");
    }

    #[test]
    fn added_word_is_removed() {
        assert_eq!(fix("on part demain", "On part donc demain."), "On part demain.");
    }

    #[test]
    fn dictated_signs_pass_but_plain_words_do_not() {
        assert_eq!(fix("il dit ouvrez les guillemets oui fermez les guillemets", "Il dit « oui »."), "Il dit « oui ».");
        assert_eq!(fix("il va à la gare", "Il va à gare."), "Il va à la gare.");
    }

    #[test]
    fn identical_text_is_unchanged() {
        assert_eq!(fix("bonjour à tous", "Bonjour à tous."), "Bonjour à tous.");
        assert_eq!(fix("", ""), "");
    }
}
