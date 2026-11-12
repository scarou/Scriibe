//! Import d'un fichier de modes (chantier 5) : la page demande, le serveur lit le fichier **par son chemin** (jamais envoyé par la page),
//! le contrôle (`scriibe_core::import`), garde le résultat en attente et montre un aperçu ; rien n'est écrit dans les réglages avant
//! l'accord de l'utilisateur (`apply`). Sous Windows et macOS une boîte « Ouvrir » choisit le fichier ; sous Linux, où elle n'existe pas
//! encore, la page demande le chemin du fichier.

use scriibe_core::import::{self, Choice, PreviewItem};
use scriibe_core::modes::CustomMode;
use scriibe_core::settings::AppSettings;
use serde_json::{json, Value};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Les modes lus et contrôlés, en attente de l'accord de l'utilisateur.
static PENDING: Mutex<Option<Vec<CustomMode>>> = Mutex::new(None);

fn pending() -> std::sync::MutexGuard<'static, Option<Vec<CustomMode>>> {
    PENDING.lock().unwrap_or_else(|e| e.into_inner())
}

/// Le contenu du fichier, après les contrôles de taille. `Err` : un message pour la page.
pub(crate) fn read_file(path: &Path) -> Result<String, String> {
    let unreadable = || "le fichier n'a pas pu être lu".to_string();
    let metadata = std::fs::metadata(path).map_err(|_| unreadable())?;
    if !metadata.is_file() {
        return Err(unreadable());
    }
    if metadata.len() > import::MAX_BYTES {
        return Err("fichier trop gros : 1 Mo au maximum".into());
    }
    // Lecture bornée aussi : le fichier a pu grossir depuis le contrôle.
    let mut bytes = Vec::new();
    std::fs::File::open(path).map_err(|_| unreadable())?.take(import::MAX_BYTES + 1).read_to_end(&mut bytes).map_err(|_| unreadable())?;
    if bytes.len() as u64 > import::MAX_BYTES {
        return Err("fichier trop gros : 1 Mo au maximum".into());
    }
    String::from_utf8(bytes).map_err(|_| "ce fichier n'est pas un fichier JSON lisible".to_string())
}

fn preview_json(items: &[PreviewItem]) -> Value {
    items
        .iter()
        .map(|i| {
            json!({"name": i.name, "fidelity": i.fidelity.raw(), "examples": i.examples, "conflict": i.conflict, "added_as": i.added_as})
        })
        .collect()
}

/// Lit, contrôle et met en attente le fichier `path`, puis rend l'aperçu. Un fichier refusé ne change rien.
pub(crate) fn read_modes(settings: &AppSettings, path: &Path) -> Value {
    let modes = match read_file(path).and_then(|text| import::parse_modes(&text)) {
        Ok(modes) => modes,
        Err(e) => {
            *pending() = None;
            return json!({"ok": false, "error": e});
        }
    };
    let items = import::preview(&settings.custom_modes, &modes);
    *pending() = Some(modes);
    json!({"ok": true, "items": preview_json(&items)})
}

/// Ce que l'utilisateur a tapé comme chemin (Linux) : sans guillemets ni espaces autour, `~` développé.
fn typed_path(text: &str) -> PathBuf {
    let text = text.trim().trim_matches(|c| c == '"' || c == '\'').trim();
    match (text.strip_prefix("~/"), std::env::var_os("HOME")) {
        (Some(rest), Some(home)) => PathBuf::from(home).join(rest),
        _ => PathBuf::from(text),
    }
}

/// Le fichier choisi dans la boîte du système : `None` si l'utilisateur annule ; `Err` si aucune boîte n'existe (Linux).
fn choose_file() -> Result<Option<PathBuf>, ()> {
    #[cfg(windows)]
    {
        let filter = [("Fichiers Scriibe (*.scriibe.json)", "*.json")];
        Ok(scriibe_platform::windows::file_dialog::pick_file("Importer un mode", &filter).map(PathBuf::from))
    }
    #[cfg(target_os = "macos")]
    {
        let filter = [("Fichiers Scriibe (*.scriibe.json)", "*.json")];
        Ok(scriibe_platform::macos::file_dialog::pick_file("Importer un mode", &filter).map(PathBuf::from))
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        Err(())
    }
}

/// `POST api/modes/import/read` : `{"path"?}`. Sans chemin, la boîte du système s'ouvre ; sans boîte (Linux), `ask_path: true`.
pub(crate) fn read(settings: &AppSettings, input: &Value) -> Value {
    let path = match input["path"].as_str().filter(|p| !p.trim().is_empty()) {
        Some(typed) => typed_path(typed),
        None => match choose_file() {
            Ok(Some(path)) => path,
            Ok(None) => return json!({"ok": true, "cancelled": true}),
            Err(()) => {
                return json!({"ok": false, "ask_path": true, "error": "le choix d'un fichier n'est pas encore pris en charge sur ce système"})
            }
        },
    };
    read_modes(settings, &path)
}

/// Ce que l'utilisateur a décidé pour chaque mode : `[{"include": bool, "replace": bool}]`.
fn choices_of(input: &Value) -> Vec<Choice> {
    input["choices"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|c| Choice { include: c["include"].as_bool().unwrap_or(true), replace: c["replace"].as_bool().unwrap_or(false) })
                .collect()
        })
        .unwrap_or_default()
}

/// Fusionne les modes en attente dans `settings` (à enregistrer par l'appelant) ; `Err` s'il n'y en a pas.
pub(crate) fn apply_pending(settings: &mut AppSettings, input: &Value) -> Result<import::Outcome, String> {
    let modes = pending().take().ok_or("aucun fichier en attente : recommence l'import")?;
    Ok(import::apply(&mut settings.custom_modes, modes, &choices_of(input)))
}

/// `POST api/modes/import/cancel` : oublie le fichier en attente.
pub(crate) fn cancel() -> Value {
    *pending() = None;
    json!({"ok": true})
}

pub(crate) fn outcome_json(o: &import::Outcome) -> Value {
    json!({"ok": true, "added": o.added, "replaced": o.replaced})
}

#[cfg(test)]
mod tests {
    use super::*;
    use scriibe_core::export;
    use scriibe_core::modes::{Fidelity, ModeExample};

    /// Les tests partagent l'attente de l'import : un à la fois.
    static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

    fn file(name: &str, text: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("scriibe-import-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, text).unwrap();
        path
    }

    fn sms() -> CustomMode {
        let mut m = CustomMode::new("SMS", "Court.", Fidelity::Light);
        m.examples = vec![ModeExample::new("a", "b")];
        m
    }

    #[test]
    fn a_refused_file_changes_nothing_and_leaves_nothing_pending() {
        let _guard = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let settings = AppSettings { custom_modes: vec![sms()], ..AppSettings::default() };
        let before = serde_json::to_value(&settings).unwrap();
        let bad = file("abime.scriibe.json", "{ pas du json");
        let out = read_modes(&settings, &bad);
        assert_eq!(out["ok"], false);
        assert_eq!(out["error"], "ce fichier n'est pas un fichier JSON lisible");
        assert!(pending().is_none());
        let mut after = settings.clone();
        assert!(apply_pending(&mut after, &Value::Null).is_err(), "rien à appliquer");
        assert_eq!(serde_json::to_value(&after).unwrap(), before);
    }

    #[test]
    fn a_file_that_is_too_big_or_missing_is_refused_before_being_read() {
        let big = file("gros.scriibe.json", &"x".repeat(import::MAX_BYTES as usize + 1));
        assert_eq!(read_file(&big), Err("fichier trop gros : 1 Mo au maximum".to_string()));
        assert_eq!(read_file(Path::new("n-existe-pas.json")), Err("le fichier n'a pas pu être lu".to_string()));
        assert!(read_file(big.parent().unwrap()).is_err(), "un dossier n'est pas un fichier");
    }

    #[test]
    fn the_preview_then_the_agreement_adds_or_replaces() {
        let _guard = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let existing = sms();
        let settings = AppSettings { custom_modes: vec![existing.clone()], ..AppSettings::default() };
        let mut incoming = sms();
        incoming.style = "Autre.".into();
        let text = export::modes_export(&[&incoming], true, "0.3.0", "2026-11-12").to_string();
        let out = read_modes(&settings, &file("ok.scriibe.json", &text));
        assert_eq!(out["ok"], true);
        assert_eq!(out["items"][0]["conflict"], "SMS");
        assert_eq!(out["items"][0]["added_as"], "SMS 2");

        // Accord : remplacer.
        let mut replaced = settings.clone();
        let done = apply_pending(&mut replaced, &json!({"choices": [{"include": true, "replace": true}]})).unwrap();
        assert_eq!(done.replaced, vec!["SMS".to_string()]);
        assert_eq!(
            (replaced.custom_modes.len(), replaced.custom_modes[0].style.as_str(), &replaced.custom_modes[0].id),
            (1, "Autre.", &existing.id)
        );
        assert!(pending().is_none(), "l'attente est consommée : un second accord ne ferait rien");
        assert!(apply_pending(&mut replaced, &Value::Null).is_err());

        // Même fichier, accord par défaut : ajouter avec un numéro.
        read_modes(&settings, &file("ok.scriibe.json", &text));
        let mut added = settings.clone();
        apply_pending(&mut added, &Value::Null).unwrap();
        assert_eq!(added.custom_modes.iter().map(|m| m.name.as_str()).collect::<Vec<_>>(), vec!["SMS", "SMS 2"]);
        assert_eq!(added.custom_modes[0], existing);
    }

    #[test]
    fn a_typed_path_loses_its_quotes() {
        assert_eq!(typed_path("  \"/a/b.json\" "), PathBuf::from("/a/b.json"));
        assert_eq!(typed_path("'x.json'"), PathBuf::from("x.json"));
    }
}
