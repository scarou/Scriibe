//! L'import d'un fichier `.scriibe.json` de modes (chantier 5) : lecture contrôlée, aperçu, fusion.
//!
//! Le fichier peut venir d'un tiers. Tout est contrôlé **avant** de toucher aux réglages : un fichier refusé donne un message clair et
//! ne change rien. Seuls les champs attendus sont lus (les autres sont ignorés, jamais copiés). Une consigne importée reste un simple
//! texte donné au modèle : elle n'est ni exécutée ni interprétée, et l'import ne touche à aucun autre réglage.
//! Un import **ajoute** ; il ne remplace un mode que si l'utilisateur le demande, pour ce mode.

use crate::export;
use crate::modes::{self, CustomMode, Fidelity, ModeExample};
use serde_json::Value;
use std::collections::HashSet;

/// Taille maximale d'un fichier d'import : 1 Mo (un mode avec ses exemples en pèse quelques Ko).
pub const MAX_BYTES: u64 = 1_048_576;
/// Nombre maximal de modes dans un fichier.
pub const MAX_MODES: usize = 50;
/// Nombre maximal d'exemples par mode.
pub const MAX_EXAMPLES: usize = 50;
/// Longueur maximale (en caractères) de la consigne et de chaque côté d'un exemple.
pub const MAX_TEXT: usize = 20_000;
/// Longueur maximale (en caractères) du nom d'un mode.
pub const MAX_NAME: usize = 100;

/// Le nom d'un mode importé qui n'en a pas.
pub const UNNAMED: &str = "Mode importé";

/// Lit un fichier d'export de modes. `Err` : un message clair pour la page (rien n'a été modifié).
pub fn parse_modes(text: &str) -> Result<Vec<CustomMode>, String> {
    let root = export::open_envelope(
        text,
        "modes",
        MAX_BYTES,
        "fichier trop gros : 1 Mo au maximum",
        "ce fichier ne contient pas de modes (autre type d'export)",
    )?;
    let list = root["content"]["customModes"].as_array().ok_or("ce fichier ne contient pas de modes")?;
    if list.is_empty() {
        return Err("aucun mode dans ce fichier".into());
    }
    if list.len() > MAX_MODES {
        return Err("trop de modes dans ce fichier : 50 au maximum".into());
    }
    list.iter().map(read_mode).collect()
}

fn too_long(text: &str, max: usize) -> bool {
    text.chars().count() > max
}

/// Un mode, avec les seuls champs attendus ; tout autre champ est ignoré.
fn read_mode(value: &Value) -> Result<CustomMode, String> {
    let o = value.as_object().ok_or("un mode du fichier est mal formé")?;
    let text_of = |key: &str| match o.get(key) {
        None | Some(Value::Null) => Ok(String::new()),
        Some(Value::String(s)) => Ok(s.clone()),
        Some(_) => Err("un mode du fichier est mal formé".to_string()),
    };
    let (name, style) = (text_of("name")?, text_of("style")?);
    if too_long(&name, MAX_NAME) {
        return Err("le nom d'un mode est trop long : 100 caractères au maximum".into());
    }
    if too_long(&style, MAX_TEXT) {
        return Err("un texte est trop long : 20 000 caractères au maximum".into());
    }
    // Le cran : celui du fichier, qui doit être valide ; à défaut l'ancienne case « reformule » (fichier de Scribe).
    let fidelity = match (o.get("fidelity"), o.get("rewrites")) {
        (Some(f), _) => f.as_str().and_then(Fidelity::from_raw).ok_or("un mode a un cran de fidélité inconnu")?,
        (None, Some(Value::Bool(true))) => Fidelity::Reformulated,
        (None, Some(Value::Bool(false))) => Fidelity::Light,
        (None, _) => return Err("un mode n'a pas de cran de fidélité".into()),
    };
    let vouvoie = match o.get("vouvoie") {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(_) => return Err("un mode du fichier est mal formé".into()),
    };
    let raw_examples = match o.get("examples") {
        None | Some(Value::Null) => &Vec::new(),
        Some(Value::Array(a)) => a,
        Some(_) => return Err("un mode du fichier est mal formé".into()),
    };
    if raw_examples.len() > MAX_EXAMPLES {
        return Err("un mode a trop d'exemples : 50 au maximum".into());
    }
    let mut examples = Vec::new();
    for e in raw_examples {
        let eo = e.as_object().ok_or("un exemple du fichier est mal formé")?;
        let side = |key: &str| match eo.get(key) {
            None | Some(Value::Null) => Ok(String::new()),
            Some(Value::String(s)) => Ok(s.clone()),
            Some(_) => Err("un exemple du fichier est mal formé".to_string()),
        };
        let (dictation, expected) = (side("dictation")?, side("expected")?);
        if too_long(&dictation, MAX_TEXT) || too_long(&expected, MAX_TEXT) {
            return Err("un texte est trop long : 20 000 caractères au maximum".into());
        }
        examples.push(ModeExample { id: modes::id_of(eo.get("id")), dictation: clean_text(&dictation), expected: clean_text(&expected) });
    }
    Ok(CustomMode { id: modes::id_of(o.get("id")), name: clean_name(&name), style: clean_text(&style), fidelity, vouvoie, examples })
}

/// Un texte libre sans caractères de contrôle (sauf le retour à la ligne et la tabulation).
fn clean_text(text: &str) -> String {
    text.chars().filter(|c| *c == '\n' || *c == '\t' || !c.is_control()).collect()
}

/// Les caractères de format invisibles (largeur nulle, inversion du sens d'écriture, trait d'union conditionnel) : dans un nom, ils
/// feraient lire le menu à l'envers ou en double.
fn is_invisible_format(c: char) -> bool {
    matches!(c, '\u{00AD}' | '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2060}'..='\u{2064}' | '\u{2066}'..='\u{2069}' | '\u{FEFF}')
}

/// Un texte sur une seule ligne, sans espaces en trop ni caractères de contrôle ou invisibles (un mot du dictionnaire, un nom de mode).
pub(crate) fn one_line(text: &str) -> String {
    let visible: String = clean_text(text).chars().filter(|c| !is_invisible_format(*c)).collect();
    visible.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Un nom sur une seule ligne, sans espaces en trop ni caractères invisibles ; « Mode importé » s'il est vide.
fn clean_name(name: &str) -> String {
    let one_line = one_line(name);
    if one_line.is_empty() {
        UNNAMED.to_string()
    } else {
        one_line
    }
}

fn same_name(a: &str, b: &str) -> bool {
    a.trim().to_lowercase() == b.trim().to_lowercase()
}

/// Le radical d'un nom : sans le numéro final qu'un import précédent a pu ajouter (« SMS 2 » → « SMS »), pour ne pas l'accumuler.
/// Seulement si le radical existe déjà : « Rapport 2026 » ou « Top 10 » gardent leur nombre (c'est une partie du nom).
fn stem<'a>(taken: &[String], name: &'a str) -> &'a str {
    match name.rsplit_once(' ') {
        Some((head, tail))
            if !head.trim().is_empty()
                && !tail.is_empty()
                && tail.chars().all(|c| c.is_ascii_digit())
                && taken.iter().any(|t| same_name(t, head)) =>
        {
            head.trim_end()
        }
        _ => name,
    }
}

/// Le nom sous lequel un mode est ajouté : le sien s'il est libre, sinon « radical 2 », « radical 3 »… (jamais un nom déjà pris).
pub fn free_name(taken: &[String], name: &str) -> String {
    let is_free = |candidate: &str| !taken.iter().any(|t| same_name(t, candidate));
    if is_free(name) {
        return name.to_string();
    }
    let stem = stem(taken, name);
    (2..).map(|n| format!("{stem} {n}")).find(|candidate| is_free(candidate)).expect("une suite infinie de numéros finit par un nom libre")
}

/// Ce que l'aperçu montre pour un mode du fichier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewItem {
    pub name: String,
    pub fidelity: Fidelity,
    pub examples: usize,
    /// Le nom d'un mode déjà présent qui porte le même nom (conflit) : l'utilisateur choisit d'ajouter ou de remplacer.
    pub conflict: Option<String>,
    /// Le nom sous lequel il serait ajouté (le sien s'il est libre).
    pub added_as: String,
}

/// L'aperçu avant import : pour chaque mode du fichier, son conflit éventuel et le nom qu'il aurait s'il était ajouté.
pub fn preview(existing: &[CustomMode], incoming: &[CustomMode]) -> Vec<PreviewItem> {
    let mut taken: Vec<String> = existing.iter().map(|m| m.name.clone()).collect();
    incoming
        .iter()
        .map(|m| {
            let conflict = existing.iter().find(|e| same_name(&e.name, &m.name)).map(|e| e.name.clone());
            let added_as = free_name(&taken, &m.name);
            taken.push(added_as.clone());
            PreviewItem { name: m.name.clone(), fidelity: m.fidelity, examples: m.examples.len(), conflict, added_as }
        })
        .collect()
}

/// Le choix de l'utilisateur pour un mode du fichier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Choice {
    /// Faux : ce mode n'est pas importé.
    pub include: bool,
    /// Vrai : il remplace le mode de même nom (sans effet s'il n'y en a pas : il est alors ajouté).
    pub replace: bool,
}

impl Default for Choice {
    fn default() -> Self {
        Self { include: true, replace: false }
    }
}

/// Ce qui a été fait.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// Les noms sous lesquels des modes ont été ajoutés.
    pub added: Vec<String>,
    /// Les noms des modes remplacés.
    pub replaced: Vec<String>,
}

/// Les identifiants déjà utilisés (modes et exemples), sans ceux du mode `except` (qu'un remplacement va retirer).
fn used_ids(modes: &[CustomMode], except: Option<usize>) -> HashSet<String> {
    modes
        .iter()
        .enumerate()
        .filter(|(i, _)| Some(*i) != except)
        .flat_map(|(_, m)| std::iter::once(m.id.clone()).chain(m.examples.iter().map(|e| e.id.clone())))
        .collect()
}

/// Fusionne les modes du fichier dans `modes`. Un `id` ou un `id` d'exemple déjà présent est remplacé par un nouveau. Un mode ajouté dont
/// le nom est pris reçoit un numéro ; un mode qui en remplace un garde l'`id` et la place de celui-ci (le mode choisi reste choisi).
/// `choices` : un choix par mode du fichier (un choix manquant vaut « ajouter »).
pub fn apply(modes: &mut Vec<CustomMode>, incoming: Vec<CustomMode>, choices: &[Choice]) -> Outcome {
    let mut outcome = Outcome::default();
    // Les places déjà remplacées ou ajoutées pendant ce passage : un second mode du fichier de même nom ne les écrase pas.
    let mut touched: HashSet<usize> = HashSet::new();
    for (i, mut mode) in incoming.into_iter().enumerate() {
        let choice = choices.get(i).copied().unwrap_or_default();
        if !choice.include {
            continue;
        }
        let target = if choice.replace {
            modes.iter().enumerate().position(|(index, e)| !touched.contains(&index) && same_name(&e.name, &mode.name))
        } else {
            None
        };
        let mut used = used_ids(modes, target);
        for example in &mut mode.examples {
            if !used.insert(example.id.clone()) {
                example.id = modes::new_id();
                used.insert(example.id.clone());
            }
        }
        match target {
            Some(index) => {
                mode.id = modes[index].id.clone();
                outcome.replaced.push(modes[index].name.clone());
                modes[index] = mode;
                touched.insert(index);
            }
            None => {
                if used.contains(&mode.id) {
                    mode.id = modes::new_id();
                }
                let taken: Vec<String> = modes.iter().map(|m| m.name.clone()).collect();
                mode.name = free_name(&taken, &mode.name);
                outcome.added.push(mode.name.clone());
                modes.push(mode);
                touched.insert(modes.len() - 1);
            }
        }
    }
    outcome
}
