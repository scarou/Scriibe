//! La sauvegarde complète (`kind: "backup"`, chantier 7 ; `docs/SYSTEME.md` § Cadrage question 3 et exception de `docs/CADRAGE_PROFILS.md`).
//!
//! Proposée avant une désinstallation, elle reprend les fichiers du dossier de données **tels quels** : réglages (modes personnels et champs
//! propres à l'ordinateur compris), dictionnaire, historique et totaux. Jamais les clés d'API, les modèles, les bancs ni les journaux.
//! C'est le seul export qui contient l'historique, et le seul import qui **remplace** au lieu d'ajouter.
//!
//! Le fichier peut être relu, déplacé ou retouché : tout est contrôlé par les lectures tolérantes existantes avant de toucher à quoi que
//! ce soit.

use crate::dictionary::UserDictionary;
use crate::export;
use crate::history::{lossy_list, DayTotal, Dictation};
use crate::settings::AppSettings;
use serde_json::{json, Map, Value};

/// Taille maximale d'une sauvegarde : 64 Mio (un historique dépasse vite la limite de 1 Mio des imports de modes).
pub const MAX_BYTES: u64 = 64 * 1024 * 1024;
/// La version des réglages que cette version de Scriibe sait lire (un fichier plus récent est refusé).
pub const SETTINGS_SCHEMA: u64 = 1;

/// Les réglages propres à l'ordinateur : touches, micro, sons, chemins de modèles, carte graphique, adresses de serveurs locaux.
/// Ils ne sont restaurés que si la sauvegarde vient du même système (`docs/CADRAGE_PROFILS.md`, décision 1).
pub const COMPUTER_FIELDS: [&str; 13] = [
    "hotkey",
    "dictationKeys",
    "translateKeys",
    "formatShortcuts",
    "microphoneUID",
    "microphoneName",
    "startSoundName",
    "endSoundName",
    "whisperModelPath",
    "llamaCppModel",
    "llamaDevice",
    "lmStudioURL",
    "ollamaURL",
];

/// Les fichiers lus tels quels dans le dossier de données (`None` : fichier absent).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BackupFiles {
    pub settings: Option<Value>,
    pub dictionary: Option<Value>,
    pub history: Option<Value>,
    pub history_totals: Option<Value>,
}

/// Le fichier de sauvegarde. `os` est « macos », « windows » ou « linux » (`std::env::consts::OS`).
pub fn backup_export(app_version: &str, day: &str, os: &str, files: &BackupFiles) -> Value {
    let mut content = Map::new();
    content.insert("os".into(), json!(os));
    for (key, value) in [
        ("settings", &files.settings),
        ("dictionary", &files.dictionary),
        ("history", &files.history),
        ("historyTotals", &files.history_totals),
    ] {
        if let Some(value) = value {
            content.insert(key.into(), value.clone());
        }
    }
    export::envelope("backup", app_version, day, Value::Object(content))
}

/// Le nom proposé : `scriibe-sauvegarde-<jour>.scriibe.json`.
pub fn file_name(day: &str) -> String {
    format!("scriibe-sauvegarde-{day}{}", export::EXTENSION)
}

/// Une sauvegarde lue et contrôlée.
#[derive(Debug, Clone, PartialEq)]
pub struct Backup {
    pub os: String,
    pub app: String,
    pub exported_at: String,
    pub files: BackupFiles,
}

/// Ce que contient une sauvegarde, pour l'aperçu (jamais le contenu des dictées).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    pub dictations: usize,
    pub days: usize,
    pub words: usize,
    pub corrections: usize,
    pub modes: usize,
}

/// Lit et contrôle une sauvegarde. `Err` : un message clair pour la page, rien n'a été modifié.
pub fn parse_backup(text: &str) -> Result<Backup, String> {
    if text.len() as u64 > MAX_BYTES {
        return Err("sauvegarde trop grosse : 64 Mo au maximum".into());
    }
    let root: Value = serde_json::from_str(text).map_err(|_| "ce fichier n'est pas un fichier JSON lisible".to_string())?;
    if root["format"] != export::FORMAT {
        return Err("ce n'est pas un fichier d'export de Scriibe".into());
    }
    match root["version"].as_u64() {
        Some(v) if (1..=u64::from(export::VERSION)).contains(&v) => {}
        Some(_) => return Err("ce fichier vient d'une version plus récente de Scriibe : mets Scriibe à jour pour le lire".into()),
        None => return Err("ce n'est pas un fichier d'export de Scriibe".into()),
    }
    if root["kind"] != "backup" {
        return Err("ce fichier n'est pas une sauvegarde complète (autre type d'export)".into());
    }
    let content = root["content"].as_object().ok_or("cette sauvegarde est vide")?;
    let settings = content.get("settings").filter(|v| v.is_object()).ok_or("cette sauvegarde ne contient pas de réglages")?;
    if settings["schemaVersion"].as_u64().is_some_and(|v| v > SETTINGS_SCHEMA) {
        return Err("ces réglages viennent d'une version plus récente de Scriibe : mets Scriibe à jour pour les lire".into());
    }
    AppSettings::from_value(settings).ok_or("les réglages de cette sauvegarde sont illisibles")?;
    let dictionary = content.get("dictionary").cloned();
    if let Some(d) = &dictionary {
        UserDictionary::from_json(&d.to_string()).map_err(|_| "le dictionnaire de cette sauvegarde est illisible".to_string())?;
    }
    let history = content.get("history").cloned();
    if let Some(h) = &history {
        lossy_list(&h.to_string(), Dictation::from_value).map_err(|_| "l'historique de cette sauvegarde est illisible".to_string())?;
    }
    let history_totals = content.get("historyTotals").cloned();
    if let Some(h) = &history_totals {
        lossy_list(&h.to_string(), DayTotal::from_value).map_err(|_| "les totaux de cette sauvegarde sont illisibles".to_string())?;
    }
    Ok(Backup {
        os: content.get("os").and_then(Value::as_str).unwrap_or("").to_string(),
        app: root["app"].as_str().unwrap_or("").chars().take(60).collect(),
        exported_at: root["exportedAt"].as_str().unwrap_or("").chars().take(20).collect(),
        files: BackupFiles { settings: Some(settings.clone()), dictionary, history, history_totals },
    })
}

impl Backup {
    /// Les nombres de l'aperçu.
    pub fn summary(&self) -> Summary {
        let dictionary = self.files.dictionary.as_ref().and_then(|d| UserDictionary::from_json(&d.to_string()).ok()).unwrap_or_default();
        let settings = self.files.settings.as_ref().and_then(AppSettings::from_value).unwrap_or_default();
        Summary {
            dictations: self
                .files
                .history
                .as_ref()
                .and_then(|h| lossy_list(&h.to_string(), Dictation::from_value).ok())
                .map_or(0, |l| l.len()),
            days: self
                .files
                .history_totals
                .as_ref()
                .and_then(|h| lossy_list(&h.to_string(), DayTotal::from_value).ok())
                .map_or(0, |l| l.len()),
            words: dictionary.words.len(),
            corrections: dictionary.corrections.len(),
            modes: settings.custom_modes.len(),
        }
    }

    /// Cette sauvegarde vient-elle du système sur lequel on la restaure ?
    pub fn same_os(&self, current_os: &str) -> bool {
        self.os == current_os
    }
}

/// Les réglages à écrire à la restauration : ceux de la sauvegarde ; si elle vient d'un autre système, les champs propres à
/// l'ordinateur gardent leur valeur actuelle (ou sont retirés s'ils n'existent pas ici : la valeur par défaut s'applique).
pub fn restored_settings(from_backup: &Value, current: Option<&Value>, same_os: bool) -> Value {
    let mut out = from_backup.clone();
    if same_os {
        return out;
    }
    if let Some(o) = out.as_object_mut() {
        for key in COMPUTER_FIELDS {
            match current.and_then(|c| c.get(key)) {
                Some(value) => {
                    o.insert(key.to_string(), value.clone());
                }
                None => {
                    o.remove(key);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files() -> BackupFiles {
        BackupFiles {
            settings: Some(json!({"hotkey": "right_option", "whisperModelPath": "/Users/a/m.bin", "language": "fr", "customModes": []})),
            dictionary: Some(json!({"words": ["Scriibe", "Whisper"], "corrections": [{"heard": "cloud code", "written": "Claude Code"}]})),
            history: Some(json!([])),
            history_totals: Some(json!([])),
        }
    }

    #[test]
    fn the_backup_has_the_agreed_envelope_and_files_without_keys() {
        let v = backup_export("0.3.0", "2026-11-13", "macos", &files());
        assert_eq!((v["format"].as_str(), v["kind"].as_str(), v["version"].as_u64()), (Some("scriibe-export"), Some("backup"), Some(1)));
        assert_eq!(v["content"]["os"], "macos");
        assert_eq!(v["content"]["dictionary"]["words"][1], "Whisper");
        for forbidden in ["deepgram", "openrouter", "apiKey", "bancs", "journal"] {
            assert!(!v.to_string().to_lowercase().contains(&forbidden.to_lowercase()), "{forbidden}");
        }
        assert_eq!(file_name("2026-11-13"), "scriibe-sauvegarde-2026-11-13.scriibe.json");
    }

    #[test]
    fn a_backup_is_read_back_and_counted() {
        let text = backup_export("0.3.0", "2026-11-13", "macos", &files()).to_string();
        let b = parse_backup(&text).unwrap();
        assert_eq!(b.os, "macos");
        assert!(b.same_os("macos") && !b.same_os("windows"));
        let s = b.summary();
        assert_eq!((s.words, s.corrections, s.dictations, s.days, s.modes), (2, 1, 0, 0, 0));
    }

    #[test]
    fn a_wrong_or_damaged_file_is_refused_with_a_clear_message() {
        let ok = backup_export("0.3.0", "2026-11-13", "macos", &files());
        assert!(parse_backup("pas du json").unwrap_err().contains("JSON"));
        assert!(parse_backup("{}").unwrap_err().contains("export de Scriibe"));
        let mut other = ok.clone();
        other["kind"] = json!("modes");
        assert!(parse_backup(&other.to_string()).unwrap_err().contains("pas une sauvegarde"));
        let mut newer = ok.clone();
        newer["version"] = json!(2);
        assert!(parse_backup(&newer.to_string()).unwrap_err().contains("plus récente"));
        let mut no_settings = ok.clone();
        no_settings["content"].as_object_mut().unwrap().remove("settings");
        assert!(parse_backup(&no_settings.to_string()).unwrap_err().contains("réglages"));
        let mut schema = ok.clone();
        schema["content"]["settings"]["schemaVersion"] = json!(99);
        assert!(parse_backup(&schema.to_string()).unwrap_err().contains("plus récente"));
        let mut history = ok;
        history["content"]["history"] = json!({"pas": "une liste"});
        assert!(parse_backup(&history.to_string()).unwrap_err().contains("historique"));
    }

    #[test]
    fn the_limit_is_64_mib_not_the_1_mib_of_mode_imports() {
        assert_eq!(MAX_BYTES, 64 * 1024 * 1024);
        let big = format!("{{\"x\":\"{}\"}}", "a".repeat(2_000_000));
        // Trop gros pour un import de modes, mais lu (et refusé pour une autre raison) ici.
        assert!(parse_backup(&big).unwrap_err().contains("export de Scriibe"));
    }

    #[test]
    fn computer_fields_follow_the_backup_only_on_the_same_system() {
        let backup = json!({"hotkey": "right_option", "whisperModelPath": "/Users/a/m.bin", "language": "fr", "microphoneUID": "mac-mic"});
        let current = json!({"hotkey": "right_control", "language": "en"});
        let same = restored_settings(&backup, Some(&current), true);
        assert_eq!((same["hotkey"].as_str(), same["whisperModelPath"].as_str()), (Some("right_option"), Some("/Users/a/m.bin")));
        let other = restored_settings(&backup, Some(&current), false);
        assert_eq!(other["hotkey"], "right_control", "la touche de cet ordinateur est gardée");
        assert!(other.get("whisperModelPath").is_none(), "un chemin de l'autre ordinateur n'est pas repris");
        assert!(other.get("microphoneUID").is_none());
        assert_eq!(other["language"], "fr", "le reste vient de la sauvegarde");
        assert_eq!(restored_settings(&backup, None, false).get("hotkey"), None);
    }
}
