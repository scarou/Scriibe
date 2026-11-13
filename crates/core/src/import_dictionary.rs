//! L'import d'un fichier `.scriibe.json` de dictionnaire (chantier 4, étape 2) : lecture contrôlée, aperçu, fusion.
//!
//! Le fichier peut venir d'un tiers. Tout est contrôlé **avant** de toucher au dictionnaire : un fichier refusé donne un message clair et
//! ne change rien. Seuls les champs attendus sont lus (les autres sont ignorés, jamais copiés). Un import **ajoute, il ne remplace jamais** :
//! un mot déjà présent n'est pas touché, et une correction dont l'« entendu » existe déjà avec un autre « écrit » laisse la correction en
//! place (le conflit est montré dans l'aperçu).

use crate::dictionary::{dedup_corrections, heard_key, word_key, Correction, UserDictionary};
use crate::export;
use crate::import::{one_line, MAX_BYTES};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

/// Nombre maximal de mots dans un fichier. Les mots vont tous dans le prompt du modèle à chaque dictée : un fichier d'un tiers ne doit pas
/// pouvoir le gonfler au point de dépasser le contexte.
pub const MAX_WORDS: usize = 500;
/// Nombre total maximal de caractères des mots d'un fichier (≈ 2 500 jetons).
pub const MAX_WORDS_CHARS: usize = 10_000;
/// Nombre de mots au-delà duquel un import n'en ajoute plus (le dictionnaire de la personne, mots déjà là compris).
pub const MAX_TOTAL_WORDS: usize = 1_000;
/// Nombre maximal de corrections dans un fichier (chacune est une recherche faite à chaque dictée).
pub const MAX_CORRECTIONS: usize = 500;
/// Longueur maximale (en caractères) d'un mot ou d'une expression.
pub const MAX_WORD: usize = 100;
/// Longueur maximale (en caractères) d'un « entendu ».
pub const MAX_HEARD: usize = 200;
/// Longueur maximale (en caractères) d'un « écrit ».
pub const MAX_WRITTEN: usize = 500;

/// Ce que contient un fichier de dictionnaire, nettoyé : une seule ligne par élément, sans doublon.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Incoming {
    pub words: Vec<String>,
    pub corrections: Vec<Correction>,
}

/// Lit un fichier d'export de dictionnaire. `Err` : un message clair pour la page (rien n'a été modifié).
pub fn parse_dictionary(text: &str) -> Result<Incoming, String> {
    let root = export::open_envelope(
        text,
        "dictionary",
        MAX_BYTES,
        "fichier trop gros : 1 Mo au maximum",
        "ce fichier ne contient pas de dictionnaire (autre type d'export)",
    )?;
    let content = root["content"].as_object().ok_or("ce fichier ne contient pas de dictionnaire")?;
    let words = list_of(content.get("words"), MAX_WORDS, "trop de mots dans ce fichier : 500 au maximum")?
        .iter()
        .map(|w| {
            let word = w.as_str().ok_or("un mot du fichier est mal formé")?;
            if too_long(word, MAX_WORD) {
                return Err("un mot est trop long : 100 caractères au maximum".to_string());
            }
            Ok(one_line(word))
        })
        .collect::<Result<Vec<_>, String>>()?;
    if words.iter().map(|w| w.chars().count()).sum::<usize>() > MAX_WORDS_CHARS {
        return Err("les mots de ce fichier sont trop longs en tout : 10 000 caractères au maximum".into());
    }
    let corrections = list_of(content.get("corrections"), MAX_CORRECTIONS, "trop de corrections dans ce fichier : 500 au maximum")?
        .iter()
        .map(|c| {
            let o = c.as_object().ok_or("une correction du fichier est mal formée")?;
            let side = |key: &str, max: usize| -> Result<String, String> {
                let text = o.get(key).and_then(Value::as_str).ok_or("une correction du fichier est mal formée")?;
                if too_long(text, max) {
                    return Err(format!("un texte de correction est trop long : {max} caractères au maximum"));
                }
                Ok(one_line(text))
            };
            Ok(Correction { heard: side("heard", MAX_HEARD)?, written: side("written", MAX_WRITTEN)? })
        })
        .collect::<Result<Vec<_>, String>>()?;

    // Les éléments vides sont écartés, puis les doublons du fichier lui-même (le premier reste, comme dans le dictionnaire).
    let mut seen = HashSet::new();
    let words: Vec<String> = words.into_iter().filter(|w| !w.is_empty() && seen.insert(word_key(w))).collect();
    let corrections = dedup_corrections(corrections.into_iter().filter(|c| !c.heard.is_empty() && !c.written.is_empty()).collect());
    if words.is_empty() && corrections.is_empty() {
        return Err("aucun mot ni correction dans ce fichier".into());
    }
    Ok(Incoming { words, corrections })
}

fn too_long(text: &str, max: usize) -> bool {
    text.chars().count() > max
}

/// La liste d'une catégorie : absente ou `null`, elle est vide ; autre chose qu'une liste, le fichier est refusé.
fn list_of<'a>(value: Option<&'a Value>, max: usize, too_many: &str) -> Result<&'a [Value], String> {
    match value {
        None | Some(Value::Null) => Ok(&[]),
        Some(Value::Array(a)) if a.len() > max => Err(too_many.into()),
        Some(Value::Array(a)) => Ok(a),
        Some(_) => Err("le dictionnaire du fichier est mal formé".into()),
    }
}

/// Où en est un élément du fichier par rapport au dictionnaire en place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// Pas encore dans le dictionnaire : il sera ajouté s'il est coché.
    New,
    /// Déjà dans le dictionnaire (même mot, ou même correction) : rien à faire.
    Same,
    /// Une correction du même « entendu » existe avec un autre « écrit » (celui-ci, en place) : elle est gardée, celle du fichier non.
    Conflict { existing: String },
}

/// L'aperçu avant import : chaque élément du fichier avec son statut, dans l'ordre du fichier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preview {
    pub words: Vec<(String, Status)>,
    pub corrections: Vec<(Correction, Status)>,
}

/// Les clés du dictionnaire en place, calculées une fois : l'aperçu et la fusion comparent des milliers d'éléments, ce ne doit pas être
/// un parcours complet (avec allocations) par élément.
struct Index {
    words: HashSet<String>,
    heard: HashMap<String, String>,
}

impl Index {
    fn of(dictionary: &UserDictionary) -> Self {
        Self {
            words: dictionary.words.iter().map(|w| word_key(w)).collect(),
            heard: dictionary.corrections.iter().map(|c| (heard_key(&c.heard), c.written.clone())).collect(),
        }
    }

    fn word(&self, word: &str) -> Status {
        if self.words.contains(&word_key(word)) {
            Status::Same
        } else {
            Status::New
        }
    }

    fn correction(&self, correction: &Correction) -> Status {
        match self.heard.get(&heard_key(&correction.heard)) {
            None => Status::New,
            Some(written) if *written == correction.written => Status::Same,
            Some(written) => Status::Conflict { existing: written.clone() },
        }
    }
}

/// L'aperçu de `incoming` face au dictionnaire en place.
pub fn preview(existing: &UserDictionary, incoming: &Incoming) -> Preview {
    let index = Index::of(existing);
    Preview {
        words: incoming.words.iter().map(|w| (w.clone(), index.word(w))).collect(),
        corrections: incoming.corrections.iter().map(|c| (c.clone(), index.correction(c))).collect(),
    }
}

/// Ce qui a été fait.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub words_added: usize,
    pub corrections_added: usize,
    /// Les éléments cochés qui étaient déjà dans le dictionnaire (rien changé).
    pub already: usize,
    /// Les corrections cochées dont l'« entendu » existait avec un autre « écrit » (celle en place est gardée).
    pub conflicts: usize,
    /// Les mots cochés qui n'ont pas été ajoutés : le dictionnaire avait atteint [`MAX_TOTAL_WORDS`].
    pub over_limit: usize,
}

/// Fusionne dans `existing` les éléments cochés de `incoming`. **Ajoute seulement** : rien de ce qui est en place n'est modifié,
/// déplacé ni retiré. Les statuts sont recalculés ici, face au dictionnaire d'à présent (il a pu changer depuis l'aperçu).
/// `words` et `corrections` : un choix par élément du fichier (un choix manquant vaut « coché »).
pub fn apply(existing: &mut UserDictionary, incoming: &Incoming, words: &[bool], corrections: &[bool]) -> Outcome {
    let mut outcome = Outcome::default();
    let mut index = Index::of(existing);
    for (i, word) in incoming.words.iter().enumerate() {
        if !words.get(i).copied().unwrap_or(true) {
            continue;
        }
        match index.word(word) {
            Status::New if existing.words.len() >= MAX_TOTAL_WORDS => outcome.over_limit += 1,
            Status::New => {
                index.words.insert(word_key(word));
                existing.words.push(word.clone());
                outcome.words_added += 1;
            }
            _ => outcome.already += 1,
        }
    }
    for (i, correction) in incoming.corrections.iter().enumerate() {
        if !corrections.get(i).copied().unwrap_or(true) {
            continue;
        }
        match index.correction(correction) {
            Status::New => {
                index.heard.insert(heard_key(&correction.heard), correction.written.clone());
                existing.corrections.push(correction.clone());
                outcome.corrections_added += 1;
            }
            Status::Same => outcome.already += 1,
            Status::Conflict { .. } => outcome.conflicts += 1,
        }
    }
    outcome
}
