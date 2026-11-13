//! L'import d'un fichier `.scriibe.json` de commandes perso (chantier 4, étape 3) : lecture contrôlée, aperçu, fusion.
//!
//! Même règles que pour le dictionnaire : le fichier peut venir d'un tiers, tout est contrôlé avant de toucher aux réglages, un fichier
//! refusé ne change rien, un import **ajoute** (jamais de remplacement). L'`id` est toujours regénéré, un nom déjà pris est numéroté, le
//! raccourci doit être lisible par [`Shortcut::parse`], une phrase déjà utilisée par une autre commande (ou trop courte) est écartée.

use crate::commands::{all_defs, normalize, validate_phrase, CustomCommand, Lang, CUSTOM_PREFIX};
use crate::export;
use crate::import::{free_name, one_line, MAX_BYTES};
use crate::settings::AppSettings;
use crate::shortcut::Shortcut;
use serde_json::Value;

/// Nombre maximal de commandes dans un fichier.
pub const MAX_COMMANDS: usize = 100;
/// Nombre maximal de phrases par commande.
pub const MAX_PHRASES: usize = 20;
/// Longueur maximale (en caractères) du titre, d'une phrase et du raccourci.
pub const MAX_NAME: usize = 100;
pub const MAX_PHRASE: usize = 200;
pub const MAX_SHORTCUT: usize = 100;

fn too_long(text: &str, max: usize) -> bool {
    text.chars().count() > max
}

/// Lit un fichier d'export de commandes. `Err` : un message clair pour la page (rien n'a été modifié). Un raccourci illisible n'est pas
/// une erreur du fichier : il est signalé dans l'aperçu et la commande n'est pas importée.
pub fn parse_commands(text: &str) -> Result<Vec<CustomCommand>, String> {
    let root = export::open_envelope(
        text,
        "commands",
        MAX_BYTES,
        "fichier trop gros : 1 Mo au maximum",
        "ce fichier ne contient pas de commandes (autre type d'export)",
    )?;
    let list = root["content"]["customCommands"].as_array().ok_or("ce fichier ne contient pas de commandes")?;
    if list.is_empty() {
        return Err("aucune commande dans ce fichier".into());
    }
    if list.len() > MAX_COMMANDS {
        return Err("trop de commandes dans ce fichier : 100 au maximum".into());
    }
    list.iter().map(read_command).collect()
}

fn read_command(value: &Value) -> Result<CustomCommand, String> {
    let bad = || "une commande du fichier est mal formée".to_string();
    let o = value.as_object().ok_or_else(bad)?;
    let text_of = |key: &str, max: usize| -> Result<String, String> {
        match o.get(key) {
            None | Some(Value::Null) => Ok(String::new()),
            Some(Value::String(s)) if too_long(s, max) => Err(format!("un texte de commande est trop long : {max} caractères au maximum")),
            Some(Value::String(s)) => Ok(one_line(s)),
            Some(_) => Err(bad()),
        }
    };
    let raw_phrases = match o.get("phrases") {
        None | Some(Value::Null) => &Vec::new(),
        Some(Value::Array(a)) => a,
        Some(_) => return Err(bad()),
    };
    if raw_phrases.len() > MAX_PHRASES {
        return Err("une commande a trop de phrases : 20 au maximum".into());
    }
    let mut phrases = Vec::new();
    for p in raw_phrases {
        let p = p.as_str().ok_or_else(bad)?;
        if too_long(p, MAX_PHRASE) {
            return Err(format!("un texte de commande est trop long : {MAX_PHRASE} caractères au maximum"));
        }
        phrases.push(one_line(p));
    }
    let needs_selection = match o.get("needsSelection") {
        None | Some(Value::Null) => true,
        Some(Value::Bool(b)) => *b,
        Some(_) => return Err(bad()),
    };
    Ok(CustomCommand {
        // L'identifiant du fichier est ignoré : l'import en donne toujours un nouveau (`free_id`).
        id: String::new(),
        name: text_of("name", MAX_NAME)?,
        phrases,
        shortcut: text_of("shortcut", MAX_SHORTCUT)?,
        needs_selection,
    })
}

/// Pourquoi une commande du fichier ne peut pas être importée (`None` : elle peut l'être).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Problem {
    /// Pas de titre.
    NoName,
    /// Raccourci absent ou illisible.
    BadShortcut,
    /// Plus aucune phrase utilisable (trop courtes, déjà prises par une autre commande, ou doublons).
    NoPhrase,
}

/// Ce que donnerait une commande du fichier ajoutée à `settings` : la commande finie, ou ce qui l'en empêche.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub command: CustomCommand,
    /// Les phrases du fichier écartées (déjà prises, trop courtes, répétées).
    pub dropped: Vec<String>,
    pub problem: Option<Problem>,
}

fn id_is_free(settings: &AppSettings, id: &str) -> bool {
    id.starts_with(CUSTOM_PREFIX) && !settings.custom_commands.iter().any(|c| c.id == id)
}

/// Un identifiant `perso.<n>` (toujours numérique) libre.
fn free_id(settings: &AppSettings) -> String {
    let start = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis());
    (0u128..)
        .map(|n| format!("{CUSTOM_PREFIX}{}", start + n))
        .find(|id| id_is_free(settings, id))
        .expect("une suite infinie finit par un identifiant libre")
}

/// Évalue `incoming` face aux commandes en place de `settings` (sans rien modifier).
pub fn evaluate(settings: &AppSettings, incoming: &CustomCommand) -> Candidate {
    let defs = all_defs(settings);
    let usable = |phrase: &str| {
        [Lang::Fr, Lang::En].iter().all(|lang| validate_phrase(phrase, "perso.nouveau", *lang, &defs, &settings.voice_commands).is_ok())
    };
    let mut seen = Vec::new();
    let (mut kept, mut dropped) = (Vec::new(), Vec::new());
    for phrase in incoming.phrases.iter().filter(|p| !p.is_empty()) {
        let key = normalize(phrase);
        if usable(phrase) && !seen.contains(&key) {
            seen.push(key);
            kept.push(phrase.clone());
        } else {
            dropped.push(phrase.clone());
        }
    }
    let taken: Vec<String> = settings.custom_commands.iter().map(|c| c.name.clone()).collect();
    let name = if incoming.name.is_empty() { String::new() } else { free_name(&taken, &incoming.name) };
    let problem = if incoming.name.is_empty() {
        Some(Problem::NoName)
    } else if Shortcut::parse(&incoming.shortcut).is_none() {
        Some(Problem::BadShortcut)
    } else if kept.is_empty() {
        Some(Problem::NoPhrase)
    } else {
        None
    };
    let command = CustomCommand {
        id: free_id(settings),
        name,
        phrases: kept,
        shortcut: incoming.shortcut.clone(),
        needs_selection: incoming.needs_selection,
    };
    Candidate { command, dropped, problem }
}

/// L'aperçu avant import : chaque commande du fichier, évaluée face aux commandes en place.
pub fn preview(settings: &AppSettings, incoming: &[CustomCommand]) -> Vec<Candidate> {
    let mut working = settings.clone();
    incoming
        .iter()
        .map(|c| {
            let candidate = evaluate(&working, c);
            if candidate.problem.is_none() {
                working.custom_commands.push(candidate.command.clone());
            }
            candidate
        })
        .collect()
}

/// Ce qui a été fait.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// Les titres sous lesquels des commandes ont été ajoutées.
    pub added: Vec<String>,
    /// Les commandes cochées qui ne pouvaient plus l'être (le réglage a changé depuis l'aperçu).
    pub skipped: usize,
}

/// Ajoute à `settings` les commandes cochées de `incoming` (un choix par commande du fichier, un choix manquant vaut « coché »).
/// Ne remplace ni ne retire rien ; les phrases, titres et identifiants sont recalculés face aux réglages d'à présent.
pub fn apply(settings: &mut AppSettings, incoming: &[CustomCommand], include: &[bool]) -> Outcome {
    let mut outcome = Outcome::default();
    for (i, command) in incoming.iter().enumerate() {
        if !include.get(i).copied().unwrap_or(true) {
            continue;
        }
        let candidate = evaluate(settings, command);
        if candidate.problem.is_some() {
            outcome.skipped += 1;
        } else {
            outcome.added.push(candidate.command.name.clone());
            settings.custom_commands.push(candidate.command);
        }
    }
    outcome
}
