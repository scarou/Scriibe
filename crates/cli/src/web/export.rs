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

/// Un chemin libre dans `dir` : `name`, puis `name` précédé d'un numéro (« 2-… »), sans jamais écraser un fichier.
#[cfg_attr(any(windows, target_os = "macos"), allow(dead_code))]
pub(crate) fn free_path(dir: &Path, name: &str, exists: impl Fn(&Path) -> bool) -> PathBuf {
    let first = dir.join(name);
    if !exists(&first) {
        return first;
    }
    (2..)
        .map(|n| dir.join(format!("{n}-{name}")))
        .find(|candidate| !exists(candidate))
        .expect("une suite infinie de numéros finit par un nom libre")
}

/// Le dossier d'enregistrement quand aucune boîte n'existe : Documents, ou le dossier personnel.
#[cfg_attr(any(windows, target_os = "macos"), allow(dead_code))]
fn fallback_folder() -> Option<PathBuf> {
    let home = PathBuf::from(std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?);
    let documents = home.join("Documents");
    Some(if documents.is_dir() { documents } else { home })
}

/// L'endroit choisi par l'utilisateur pour `name` : `None` s'il annule.
fn choose_path(name: &str) -> Result<Option<PathBuf>, String> {
    let _ = name;
    #[cfg(windows)]
    {
        let filter = [("Fichiers Scriibe (*.scriibe.json)", "*.json")];
        Ok(scriibe_platform::windows::file_dialog::pick_save("Enregistrer le mode", name, &filter).map(PathBuf::from))
    }
    #[cfg(target_os = "macos")]
    {
        Ok(scriibe_platform::macos::file_dialog::pick_save("Enregistrer le mode", name).map(PathBuf::from))
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let folder = fallback_folder().ok_or("aucun dossier d’enregistrement trouvé")?;
        Ok(Some(free_path(&folder, name, |p| p.exists())))
    }
}

/// `{"ok": true, "saved": true, "path": …}`, `{"ok": true, "saved": false}` si l'utilisateur annule, ou `{"ok": false, "error": …}`.
pub(crate) fn export_mode(settings: &AppSettings, input: &Value) -> Value {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
    let (name, text) = match prepare(settings, input["id"].as_str().unwrap_or(""), input["examples"].as_bool().unwrap_or(true), now) {
        Ok(v) => v,
        Err(e) => return json!({"ok": false, "error": e}),
    };
    let path = match choose_path(&name) {
        Ok(Some(path)) => path,
        Ok(None) => return json!({"ok": true, "saved": false}),
        Err(e) => return json!({"ok": false, "error": e}),
    };
    match std::fs::write(&path, text) {
        Ok(()) => json!({"ok": true, "saved": true, "path": path.to_string_lossy()}),
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
        let dir = Path::new("dossier");
        let taken = [dir.join("m.scriibe.json"), dir.join("2-m.scriibe.json")];
        assert_eq!(free_path(dir, "m.scriibe.json", |p| taken.iter().any(|t| t == p)), dir.join("3-m.scriibe.json"));
        assert_eq!(free_path(dir, "m.scriibe.json", |_| false), dir.join("m.scriibe.json"));
    }
}
