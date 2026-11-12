//! Export d'un mode personnel (chantier 5) : la page demande, le serveur construit le fichier (liste blanche de `scriibe_core::export`)
//! et l'utilisateur choisit où l'enregistrer (boîte « Enregistrer sous » sous Windows et macOS ; sous Linux, aucune boîte n'existe encore :
//! le fichier va dans le dossier Documents, ou à défaut le dossier personnel, sous un nom qui ne s'écrase pas).

use scriibe_core::export;
use scriibe_core::settings::AppSettings;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Le nom proposé et le texte du fichier pour le mode `id`. `Err` : un message pour la page.
pub(crate) fn prepare(settings: &AppSettings, id: &str, include_examples: bool, now_secs: i64) -> Result<(String, String), String> {
    let mode = settings.custom_modes.iter().find(|m| m.id == id).ok_or("mode inconnu")?;
    let day = export::day_of(now_secs);
    let file = export::modes_export(&[mode], include_examples, super::update::current_version(), &day);
    let text = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())? + "\n";
    Ok((export::mode_file_name(&mode.name, &day), text))
}

/// Écrit `text` dans `dir` sous `name`, puis « 2-name », « 3-name »… : le fichier est **créé** (jamais ouvert pour être tronqué), donc un
/// fichier existant, un lien symbolique ou une course avec un autre processus ne peuvent pas être écrasés. Rend le chemin écrit.
#[cfg_attr(any(windows, target_os = "macos"), allow(dead_code))]
pub(crate) fn write_without_overwriting(dir: &Path, name: &str, text: &str) -> std::io::Result<PathBuf> {
    use std::io::Write;
    for n in 1.. {
        let path = if n == 1 { dir.join(name) } else { dir.join(format!("{n}-{name}")) };
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                file.write_all(text.as_bytes())?;
                return Ok(path);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
    }
    unreachable!("une suite infinie de numéros finit par un nom libre")
}

/// Le dossier d'enregistrement quand aucune boîte n'existe : Documents, ou le dossier personnel.
#[cfg_attr(any(windows, target_os = "macos"), allow(dead_code))]
fn fallback_folder() -> Option<PathBuf> {
    let home = PathBuf::from(std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?);
    let documents = home.join("Documents");
    Some(if documents.is_dir() { documents } else { home })
}

/// Où enregistrer `name` (selon le système, une des deux variantes ne sert pas).
#[allow(dead_code)]
enum Target {
    /// Un fichier choisi dans la boîte du système (qui a déjà demandé confirmation avant d'écraser).
    File(PathBuf),
    /// Aucune boîte (Linux) : ce dossier, sans rien écraser.
    Folder(PathBuf),
}

/// L'endroit choisi par l'utilisateur pour `name` : `None` s'il annule.
fn choose_target(name: &str) -> Result<Option<Target>, String> {
    let _ = name;
    #[cfg(windows)]
    {
        let filter = [("Fichiers Scriibe (*.scriibe.json)", "*.json")];
        Ok(scriibe_platform::windows::file_dialog::pick_save("Enregistrer le mode", name, &filter).map(|p| Target::File(PathBuf::from(p))))
    }
    #[cfg(target_os = "macos")]
    {
        Ok(scriibe_platform::macos::file_dialog::pick_save("Enregistrer le mode", name)?.map(|p| Target::File(PathBuf::from(p))))
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        Ok(Some(Target::Folder(fallback_folder().ok_or("aucun dossier d’enregistrement trouvé")?)))
    }
}

/// `{"ok": true, "saved": true, "path": …}`, `{"ok": true, "saved": false}` si l'utilisateur annule, ou `{"ok": false, "error": …}`.
pub(crate) fn export_mode(settings: &AppSettings, input: &Value) -> Value {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
    let (name, text) = match prepare(settings, input["id"].as_str().unwrap_or(""), input["examples"].as_bool().unwrap_or(true), now) {
        Ok(v) => v,
        Err(e) => return json!({"ok": false, "error": e}),
    };
    let written = match choose_target(&name) {
        Ok(Some(Target::File(path))) => std::fs::write(&path, text).map(|()| path),
        Ok(Some(Target::Folder(dir))) => write_without_overwriting(&dir, &name, &text),
        Ok(None) => return json!({"ok": true, "saved": false}),
        Err(e) => return json!({"ok": false, "error": e}),
    };
    match written {
        Ok(path) => json!({"ok": true, "saved": true, "path": path.to_string_lossy()}),
        Err(_) => json!({"ok": false, "error": "enregistrement impossible : l’emplacement choisi n’est pas accessible en écriture"}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scriibe_core::modes::{CustomMode, Fidelity, ModeExample};

    fn settings_with(mode: &CustomMode) -> AppSettings {
        AppSettings { custom_modes: vec![mode.clone()], ..AppSettings::default() }
    }

    #[test]
    fn the_file_for_a_mode_has_the_agreed_name_and_content() {
        let mut mode = CustomMode::new("SMS", "Court.", Fidelity::Light);
        mode.examples = vec![ModeExample::new("a", "b")];
        let (name, text) = prepare(&settings_with(&mode), &mode.id, true, 1_700_000_000).unwrap();
        assert_eq!(name, "scriibe-mode-sms-2023-11-14.scriibe.json");
        let value: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["kind"], "modes");
        assert_eq!(value["content"]["customModes"][0]["examples"].as_array().unwrap().len(), 1);
        let (_, without) = prepare(&settings_with(&mode), &mode.id, false, 1_700_000_000).unwrap();
        assert!(serde_json::from_str::<Value>(&without).unwrap()["content"]["customModes"][0]["examples"].as_array().unwrap().is_empty());
    }

    #[test]
    fn an_unknown_mode_is_refused() {
        assert_eq!(prepare(&AppSettings::default(), "inconnu", true, 0), Err("mode inconnu".to_string()));
    }

    #[test]
    fn an_existing_file_is_never_overwritten_by_the_fallback() {
        let dir = std::env::temp_dir().join(format!("scriibe-export-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let first = write_without_overwriting(&dir, "m.scriibe.json", "un").unwrap();
        let second = write_without_overwriting(&dir, "m.scriibe.json", "deux").unwrap();
        let third = write_without_overwriting(&dir, "m.scriibe.json", "trois").unwrap();
        assert_eq!(
            (first.file_name().unwrap(), second.file_name().unwrap(), third.file_name().unwrap()),
            ("m.scriibe.json".as_ref(), "2-m.scriibe.json".as_ref(), "3-m.scriibe.json".as_ref())
        );
        assert_eq!(std::fs::read_to_string(&first).unwrap(), "un", "le premier fichier n'a pas bougé");
        assert_eq!(std::fs::read_to_string(&third).unwrap(), "trois");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
