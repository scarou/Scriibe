//! Le dictionnaire dans un tableur : export en CSV (une ligne par mot ou par correction), relecture du fichier modifié, comparaison avec
//! le dictionnaire en place. Pour filtrer, trier et regrouper dans Excel, Numbers ou LibreOffice, pas pour calculer.
//!
//! Colonnes : `Catégorie;Entendu;Écrit`. Catégorie : `mot` (le mot est dans « Écrit ») ou `correction`. Séparateur `;` (Excel en français),
//! UTF-8 avec marque d'ordre (accents bien lus) ; à la relecture, `;`, `,` et tabulation sont reconnus d'après la ligne d'en-tête.

use crate::dictionary::{heard_key, word_key, Correction, UserDictionary};
use crate::import_dictionary::{read_content, Incoming};
use serde_json::json;
use std::collections::HashMap;

const HEADER: [&str; 3] = ["Catégorie", "Entendu", "Écrit"];
const BOM: char = '\u{feff}';

fn cell(text: &str) -> String {
    if text.contains([';', '"', '\n', '\r']) || text.starts_with(' ') || text.ends_with(' ') {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_string()
    }
}

/// Le fichier CSV du dictionnaire : mots d'abord, puis corrections, dans l'ordre du dictionnaire.
pub fn to_csv(dictionary: &UserDictionary) -> String {
    let mut out = String::new();
    out.push(BOM);
    out.push_str(&HEADER.join(";"));
    out.push_str("\r\n");
    for word in &dictionary.words {
        out.push_str(&format!("mot;;{}\r\n", cell(word)));
    }
    for c in &dictionary.corrections {
        out.push_str(&format!("correction;{};{}\r\n", cell(&c.heard), cell(&c.written)));
    }
    out
}

/// Découpe un CSV en lignes de cellules (guillemets doublés, retours à la ligne permis dans une cellule entre guillemets).
fn rows(text: &str, delimiter: char) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if quoted {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    current.push('"');
                } else {
                    quoted = false;
                }
            } else {
                current.push(c);
            }
        } else if c == '"' && current.is_empty() {
            quoted = true;
        } else if c == delimiter {
            row.push(std::mem::take(&mut current));
        } else if c == '\n' || c == '\r' {
            if c == '\r' && chars.peek() == Some(&'\n') {
                chars.next();
            }
            row.push(std::mem::take(&mut current));
            rows.push(std::mem::take(&mut row));
        } else {
            current.push(c);
        }
    }
    if !current.is_empty() || !row.is_empty() {
        row.push(current);
        rows.push(row);
    }
    rows
}

/// Minuscules sans accents, pour reconnaître « Catégorie », « categorie », « ÉCRIT »…
fn plain(text: &str) -> String {
    text.trim()
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'à' | 'â' | 'ä' => 'a',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            c => c,
        })
        .collect()
}

/// Lit un CSV (celui de `to_csv`, rouvert et enregistré par un tableur). `Err` : un message pour la page, rien n'est modifié.
pub fn from_csv(text: &str) -> Result<Incoming, String> {
    let text = text.trim_start_matches(BOM);
    let first_line = text.lines().next().unwrap_or("");
    let delimiter = [';', '\t', ','].into_iter().max_by_key(|d| first_line.matches(*d).count()).unwrap_or(';');
    if !first_line.contains(delimiter) {
        return Err("ce fichier n'est pas un CSV du dictionnaire (colonnes : Catégorie, Entendu, Écrit)".into());
    }
    let mut lines = rows(text, delimiter).into_iter().enumerate();
    let (_, header) = lines.next().ok_or("le fichier est vide")?;
    let names: Vec<String> = header.iter().map(|h| plain(h)).collect();
    let column = |name: &str| names.iter().position(|n| n == name);
    let (Some(category), Some(heard), Some(written)) = (column("categorie"), column("entendu"), column("ecrit")) else {
        return Err("l'en-tête doit contenir les colonnes Catégorie, Entendu et Écrit".into());
    };
    let (mut words, mut corrections) = (Vec::new(), Vec::new());
    for (index, row) in lines {
        let get = |i: usize| row.get(i).map(|s| s.trim()).unwrap_or("");
        let (kind, h, w) = (plain(get(category)), get(heard), get(written));
        if kind.is_empty() && h.is_empty() && w.is_empty() {
            continue;
        }
        match kind.as_str() {
            "mot" => {
                let word = if w.is_empty() { h } else { w };
                if word.is_empty() {
                    return Err(format!("ligne {} : un mot sans texte", index + 1));
                }
                words.push(json!(word));
            }
            "correction" => {
                if h.is_empty() || w.is_empty() {
                    return Err(format!("ligne {} : une correction a besoin d'un « Entendu » et d'un « Écrit »", index + 1));
                }
                corrections.push(json!({"heard": h, "written": w}));
            }
            _ => return Err(format!("ligne {} : la catégorie doit être « mot » ou « correction »", index + 1)),
        }
    }
    read_content(&json!({"words": words, "corrections": corrections}))
}

/// Ce que le fichier change par rapport au dictionnaire en place, pour l'aperçu avant remplacement.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Diff {
    pub words_added: Vec<String>,
    pub words_removed: Vec<String>,
    pub corrections_added: Vec<Correction>,
    pub corrections_removed: Vec<Correction>,
    /// (entendu, écrit avant, écrit après)
    pub corrections_changed: Vec<(String, String, String)>,
}

impl Diff {
    pub fn is_empty(&self) -> bool {
        self.words_added.is_empty()
            && self.words_removed.is_empty()
            && self.corrections_added.is_empty()
            && self.corrections_removed.is_empty()
            && self.corrections_changed.is_empty()
    }
}

pub fn diff(existing: &UserDictionary, incoming: &Incoming) -> Diff {
    let mut d = Diff::default();
    let had: HashMap<String, &String> = existing.words.iter().map(|w| (word_key(w), w)).collect();
    let has: HashMap<String, &String> = incoming.words.iter().map(|w| (word_key(w), w)).collect();
    d.words_added = incoming.words.iter().filter(|w| !had.contains_key(&word_key(w))).cloned().collect();
    d.words_removed = existing.words.iter().filter(|w| !has.contains_key(&word_key(w))).cloned().collect();
    let before: HashMap<String, &Correction> = existing.corrections.iter().map(|c| (heard_key(&c.heard), c)).collect();
    let after: HashMap<String, &Correction> = incoming.corrections.iter().map(|c| (heard_key(&c.heard), c)).collect();
    for c in &incoming.corrections {
        match before.get(&heard_key(&c.heard)) {
            None => d.corrections_added.push(c.clone()),
            Some(old) if old.written != c.written => d.corrections_changed.push((c.heard.clone(), old.written.clone(), c.written.clone())),
            Some(_) => {}
        }
    }
    d.corrections_removed = existing.corrections.iter().filter(|c| !after.contains_key(&heard_key(&c.heard))).cloned().collect();
    d
}

/// Le dictionnaire que le fichier décrit, tel qu'il remplacera l'actuel.
pub fn into_dictionary(incoming: Incoming) -> UserDictionary {
    UserDictionary { words: incoming.words, corrections: incoming.corrections }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(h: &str, w: &str) -> Correction {
        Correction { heard: h.into(), written: w.into() }
    }

    fn dictionary() -> UserDictionary {
        UserDictionary {
            words: vec!["Scriibe".into(), "Claude Code".into(), "a;b \"c\"".into()],
            corrections: vec![c("cri be", "Scriibe"), c("Est-Aïku", "Haïku")],
        }
    }

    #[test]
    fn csv_round_trip_keeps_words_corrections_and_awkward_cells() {
        let csv = to_csv(&dictionary());
        assert!(csv.starts_with('\u{feff}'), "marque UTF-8 pour Excel");
        let back = from_csv(&csv).unwrap();
        assert_eq!(back.words, dictionary().words);
        assert_eq!(back.corrections, dictionary().corrections);
    }

    #[test]
    fn a_spreadsheet_may_use_commas_or_tabs_and_reorder_rows() {
        let text = "categorie,entendu,ecrit\ncorrection,cri be,Scriibe\nmot,,Haïku\n,,\n";
        let back = from_csv(text).unwrap();
        assert_eq!(back.words, vec!["Haïku"]);
        assert_eq!(back.corrections, vec![c("cri be", "Scriibe")]);
        assert_eq!(from_csv("Catégorie\tEntendu\tÉcrit\nmot\t\tAïku\n").unwrap().words, vec!["Aïku"]);
    }

    #[test]
    fn bad_files_are_refused_with_the_line() {
        assert!(from_csv("a;b\n1;2\n").unwrap_err().contains("en-tête"));
        assert!(from_csv("Catégorie;Entendu;Écrit\nchose;a;b\n").unwrap_err().contains("ligne 2"));
        assert!(from_csv("Catégorie;Entendu;Écrit\ncorrection;a;\n").unwrap_err().contains("ligne 2"));
        assert!(from_csv("pas un csv").is_err());
    }

    #[test]
    fn diff_lists_what_the_file_adds_removes_and_changes() {
        let incoming =
            from_csv("Catégorie;Entendu;Écrit\nmot;;Scriibe\nmot;;Nouveau\ncorrection;cri be;Scribe\ncorrection;autre;Autre\n").unwrap();
        let d = diff(&dictionary(), &incoming);
        assert_eq!(d.words_added, vec!["Nouveau"]);
        assert_eq!(d.words_removed.len(), 2);
        assert_eq!(d.corrections_added, vec![c("autre", "Autre")]);
        assert_eq!(d.corrections_removed, vec![c("Est-Aïku", "Haïku")]);
        assert_eq!(d.corrections_changed, vec![("cri be".to_string(), "Scriibe".to_string(), "Scribe".to_string())]);
        assert!(diff(&dictionary(), &from_csv(&to_csv(&dictionary())).unwrap()).is_empty());
    }
}
