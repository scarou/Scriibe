//! Les fichiers d'export de Scriibe (`.scriibe.json`) : l'enveloppe commune fixée par `docs/CADRAGE_PROFILS.md` (décision 4),
//! et l'export des modes personnels (chantier 5).
//!
//! L'export est construit **champ par champ, d'après une liste blanche**, jamais en sérialisant les réglages : un futur champ
//! sensible ne peut pas partir par défaut. Aucune clé d'API, aucun historique, aucun champ propre à l'ordinateur.

use crate::commands::CustomCommand;
use crate::dictionary::UserDictionary;
use crate::modes::CustomMode;
use serde_json::{json, Value};

/// Valeur de `format` de tout fichier d'export.
pub const FORMAT: &str = "scriibe-export";
/// Version de l'enveloppe. Un fichier d'une version plus grande est refusé à l'import.
pub const VERSION: u32 = 1;
/// Extension des fichiers d'export.
pub const EXTENSION: &str = ".scriibe.json";

/// L'enveloppe commune : `kind` dit ce que contient `content`. `day` est le jour seul (« 2026-11-12 »), jamais l'heure.
pub fn envelope(kind: &str, app_version: &str, day: &str, content: Value) -> Value {
    json!({
        "format": FORMAT,
        "version": VERSION,
        "kind": kind,
        "app": format!("Scriibe {app_version}"),
        "exportedAt": day,
        "content": content,
    })
}

/// Ouvre un fichier d'export : taille, JSON, `format`, `version`, `kind`. Rend la racine du fichier ; `Err` : un message clair pour la page
/// (rien n'a été modifié). Le contrôle est le même pour tous les types de fichier (modes, dictionnaire, sauvegarde…) ; chacun garde ses
/// propres messages de taille (`too_big`) et de mauvais type (`wrong_kind`).
pub fn open_envelope(text: &str, kind: &str, max_bytes: u64, too_big: &str, wrong_kind: &str) -> Result<Value, String> {
    if text.len() as u64 > max_bytes {
        return Err(too_big.into());
    }
    let root: Value = serde_json::from_str(text).map_err(|_| "ce fichier n'est pas un fichier JSON lisible".to_string())?;
    if root["format"] != FORMAT {
        return Err("ce n'est pas un fichier d'export de Scriibe".into());
    }
    match root["version"].as_u64() {
        Some(v) if (1..=u64::from(VERSION)).contains(&v) => {}
        Some(v) if v > u64::from(VERSION) => {
            return Err("ce fichier vient d'une version plus récente de Scriibe : mets Scriibe à jour pour le lire".into())
        }
        // Pas de numéro, ou 0 : ce n'est pas un fichier d'export (il n'y a pas de version « plus ancienne » que la 1).
        _ => return Err("ce n'est pas un fichier d'export de Scriibe".into()),
    }
    if root["kind"] != kind {
        return Err(wrong_kind.into());
    }
    Ok(root)
}

/// Le jour (UTC) d'un instant donné en secondes depuis 1970, au format « 2026-11-12 ».
pub fn day_of(unix_seconds: i64) -> String {
    chrono::DateTime::from_timestamp(unix_seconds, 0).map_or_else(|| "1970-01-01".into(), |d| d.format("%Y-%m-%d").to_string())
}

/// Un mode, avec les seuls champs de la liste blanche : `id`, `name`, `style`, `fidelity`, `rewrites` (pour Scribe), `vouvoie`,
/// `examples` (vide si l'auteur les retire).
pub fn mode_entry(mode: &CustomMode, include_examples: bool) -> Value {
    let examples: Vec<Value> = if include_examples {
        mode.examples.iter().map(|e| json!({"id": e.id, "dictation": e.dictation, "expected": e.expected})).collect()
    } else {
        Vec::new()
    };
    json!({
        "id": mode.id,
        "name": mode.name,
        "style": mode.style,
        "fidelity": mode.fidelity.raw(),
        "rewrites": mode.fidelity.rewrites(),
        "vouvoie": mode.vouvoie,
        "examples": examples,
    })
}

/// Le fichier d'export de modes (`kind: "modes"`, `content: {customModes: […]}`).
pub fn modes_export(modes: &[&CustomMode], include_examples: bool, app_version: &str, day: &str) -> Value {
    let entries: Vec<Value> = modes.iter().map(|m| mode_entry(m, include_examples)).collect();
    envelope("modes", app_version, day, json!({"customModes": entries}))
}

/// Le nom de fichier proposé : `scriibe-mode-<nom du mode>-<jour>.scriibe.json`, en minuscules sans accent ni signe.
/// Il ne contient que le nom du mode, jamais celui de l'utilisateur.
pub fn mode_file_name(mode_name: &str, day: &str) -> String {
    let mut slug = String::new();
    for c in mode_name.chars().flat_map(char::to_lowercase) {
        let plain = match c {
            'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            c if c.is_ascii_alphanumeric() => c,
            _ => '-',
        };
        if plain != '-' || !slug.ends_with('-') {
            slug.push(plain);
        }
    }
    let slug: String = slug.trim_matches('-').chars().take(40).collect();
    let slug = slug.trim_end_matches('-');
    let slug = if slug.is_empty() { "sans-nom" } else { slug };
    format!("scriibe-mode-{slug}-{day}{EXTENSION}")
}

/// Le fichier d'export du dictionnaire (`kind: "dictionary"`, `content: {words, corrections}`). Seules les catégories demandées partent
/// (une catégorie retirée n'a pas de clé) ; mots et corrections sont recopiés champ par champ.
pub fn dictionary_export(dictionary: &UserDictionary, words: bool, corrections: bool, app_version: &str, day: &str) -> Value {
    let mut content = serde_json::Map::new();
    if words {
        content.insert("words".into(), json!(dictionary.words));
    }
    if corrections {
        let list: Vec<Value> = dictionary.corrections.iter().map(|c| json!({"heard": c.heard, "written": c.written})).collect();
        content.insert("corrections".into(), Value::Array(list));
    }
    envelope("dictionary", app_version, day, Value::Object(content))
}

/// Le nom de fichier proposé : `scriibe-dictionnaire-<jour>.scriibe.json` (jamais le nom de l'utilisateur).
pub fn dictionary_file_name(day: &str) -> String {
    format!("scriibe-dictionnaire-{day}{EXTENSION}")
}

/// Le fichier d'export des commandes perso (`kind: "commands"`, `content: {customCommands: […]}`), champ par champ. L'`id` n'est pas exporté :
/// il contient l'heure exacte de création de la commande, et l'import en donne toujours un nouveau.
pub fn commands_export(commands: &[CustomCommand], app_version: &str, day: &str) -> Value {
    let list: Vec<Value> = commands
        .iter()
        .map(|c| json!({"name": c.name, "phrases": c.phrases, "shortcut": c.shortcut, "needsSelection": c.needs_selection}))
        .collect();
    envelope("commands", app_version, day, json!({"customCommands": list}))
}

/// Le nom de fichier proposé : `scriibe-commandes-<jour>.scriibe.json`.
pub fn commands_file_name(day: &str) -> String {
    format!("scriibe-commandes-{day}{EXTENSION}")
}
