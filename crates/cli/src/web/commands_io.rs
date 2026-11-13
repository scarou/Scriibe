//! Export et import des commandes perso (chantier 4, étape 3), même démarche que le dictionnaire (`dictionary_io.rs`) : le serveur construit
//! ou lit le fichier, garde le résultat en attente avec un jeton, montre un aperçu ; rien n'est écrit avant l'accord, et l'import ajoute
//! sans jamais remplacer (`scriibe_core::import_commands`).

use super::export::{choose_target, write_without_overwriting, Target};
use super::import::{choose_file, read_file, typed_path, Picked};
use scriibe_core::commands::CustomCommand;
use scriibe_core::export as file_format;
use scriibe_core::import_commands::{self as importing, Candidate, Outcome, Problem};
use scriibe_core::settings::AppSettings;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Mutex;

static PENDING: Mutex<Option<(String, Vec<CustomCommand>)>> = Mutex::new(None);

fn pending() -> std::sync::MutexGuard<'static, Option<(String, Vec<CustomCommand>)>> {
    PENDING.lock().unwrap_or_else(|e| e.into_inner())
}

/// Le nom proposé et le texte du fichier d'export. `Err` : un message pour la page.
pub(crate) fn prepare(settings: &AppSettings, now_secs: i64) -> Result<(String, String), String> {
    if settings.custom_commands.is_empty() {
        return Err("rien à exporter : tu n'as pas encore de commande à toi".into());
    }
    let day = file_format::day_of(now_secs);
    let file = file_format::commands_export(&settings.custom_commands, super::update::current_version(), &day);
    let text = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())? + "\n";
    Ok((file_format::commands_file_name(&day), text))
}

pub(crate) fn export(settings: &AppSettings) -> Value {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
    let (name, text) = match prepare(settings, now) {
        Ok(v) => v,
        Err(e) => return json!({"ok": false, "error": e}),
    };
    let written = match choose_target("Enregistrer les commandes", &name) {
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

fn items_json(items: &[Candidate]) -> Value {
    items
        .iter()
        .map(|c| {
            let problem = c.problem.map(|p| match p {
                Problem::NoName => "no_name",
                Problem::BadShortcut => "bad_shortcut",
                Problem::NoPhrase => "no_phrase",
            });
            json!({"name": c.command.name, "phrases": c.command.phrases, "dropped": c.dropped, "shortcut": c.command.shortcut,
                   "needs_selection": c.command.needs_selection, "problem": problem})
        })
        .collect()
}

/// Lit, contrôle et met en attente le fichier `path`, puis rend l'aperçu. Un fichier refusé ne change rien (et oublie l'attente d'avant).
pub(crate) fn read_path(settings: &AppSettings, path: &Path) -> Value {
    let commands = match read_file(path).and_then(|text| importing::parse_commands(&text)) {
        Ok(commands) => commands,
        Err(e) => {
            *pending() = None;
            return json!({"ok": false, "error": e});
        }
    };
    let items = items_json(&importing::preview(settings, &commands));
    let token = scriibe_core::modes::new_id();
    *pending() = Some((token.clone(), commands));
    json!({"ok": true, "token": token, "items": items})
}

/// `POST api/commands/import/read` : `{"path"?}` (chemin tapé honoré seulement sans boîte du système, c'est-à-dire sous Linux).
pub(crate) fn read(settings: &AppSettings, input: &Value) -> Value {
    let typed = input["path"].as_str().filter(|p| !p.trim().is_empty());
    let path = match (cfg!(not(any(windows, target_os = "macos"))), typed) {
        (true, Some(typed)) => typed_path(typed),
        _ => match choose_file("Importer des commandes") {
            Picked::File(path) => path,
            Picked::Cancelled => return json!({"ok": true, "cancelled": true}),
            Picked::Failed(e) => return json!({"ok": false, "error": e}),
            Picked::NoDialog => {
                return json!({"ok": false, "ask_path": true, "error": "le choix d'un fichier n'est pas encore pris en charge sur ce système"})
            }
        },
    };
    read_path(settings, &path)
}

/// Fusionne dans `settings` (à enregistrer par l'appelant, qui appelle ensuite `consume`) les commandes cochées du fichier en attente.
/// Refuse si le jeton n'est pas celui de l'aperçu, ou si le nombre de cases ne correspond pas : l'attente est alors gardée.
pub(crate) fn apply_pending(settings: &mut AppSettings, input: &Value) -> Result<Outcome, String> {
    let guard = pending();
    let (token, commands) = guard.as_ref().ok_or("aucun fichier en attente : recommence l'import")?;
    let stale = "l'aperçu n'est plus à jour : recommence l'import";
    if input["token"].as_str() != Some(token.as_str()) {
        return Err(stale.into());
    }
    let include: Option<Vec<bool>> = input["include"].as_array().map(|a| a.iter().map(|v| v.as_bool().unwrap_or(false)).collect());
    if include.as_ref().is_some_and(|i| i.len() != commands.len()) {
        return Err(stale.into());
    }
    Ok(importing::apply(settings, commands, &include.unwrap_or_default()))
}

pub(crate) fn consume() {
    *pending() = None;
}

pub(crate) fn cancel() -> Value {
    *pending() = None;
    json!({"ok": true})
}

pub(crate) fn outcome_json(o: &Outcome) -> Value {
    json!({"ok": true, "added": o.added, "skipped": o.skipped})
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

    struct TestDir(PathBuf);

    impl TestDir {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("scriibe-commands-io-test-{}-{name}", std::process::id()));
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn file(&self, name: &str, text: &str) -> PathBuf {
            let path = self.0.join(name);
            std::fs::write(&path, text).unwrap();
            path
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn command(id: &str, name: &str, phrase: &str) -> CustomCommand {
        CustomCommand {
            id: id.into(),
            name: name.into(),
            phrases: vec![phrase.into()],
            shortcut: "Mod+Shift+X".into(),
            needs_selection: true,
        }
    }

    fn mine() -> AppSettings {
        AppSettings { custom_commands: vec![command("perso.1", "Barré", "mettre en barré")], ..AppSettings::default() }
    }

    #[test]
    fn the_export_has_the_agreed_name_and_the_whitelisted_fields() {
        let (name, text) = prepare(&mine(), 1_700_000_000).unwrap();
        assert_eq!(name, "scriibe-commandes-2023-11-14.scriibe.json");
        let value: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["kind"], "commands");
        let mut keys: Vec<&String> = value["content"]["customCommands"][0].as_object().unwrap().keys().collect();
        keys.sort();
        assert_eq!(keys, ["name", "needsSelection", "phrases", "shortcut"], "pas d'id : il contient l'heure exacte de création");
        assert!(prepare(&AppSettings::default(), 0).is_err(), "rien à exporter sans commande");
    }

    #[test]
    fn a_refused_file_changes_nothing_and_forgets_the_previous_wait() {
        let _one = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let dir = TestDir::new("refused");
        let good = dir.file("bon.scriibe.json", &prepare(&mine(), 0).unwrap().1);
        assert_eq!(read_path(&AppSettings::default(), &good)["ok"], true);
        let out = read_path(&AppSettings::default(), &dir.file("mauvais.scriibe.json", "pas du json"));
        assert_eq!(out["ok"], false);
        assert!(pending().is_none());
        let mut settings = mine();
        assert!(apply_pending(&mut settings, &json!({"token": "x"})).is_err());
        assert_eq!(settings.custom_commands, mine().custom_commands);
    }

    #[test]
    fn the_agreement_adds_with_a_new_id_and_a_numbered_name_and_keeps_what_is_there() {
        let _one = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let dir = TestDir::new("merge");
        // Même id, même nom, une phrase déjà prise (« mettre en barré »), une nouvelle, et un raccourci illisible.
        let file = dir.file(
            "c.scriibe.json",
            r#"{"format":"scriibe-export","version":1,"kind":"commands","content":{"customCommands":[
                {"id":"perso.1","name":"Barré","phrases":["mettre en barré","passer en barré"],"shortcut":"Mod+Shift+X","needsSelection":false},
                {"id":"perso.9","name":"Cassé","phrases":["faire quelque chose"],"shortcut":"nimporte quoi"}]}}"#,
        );
        let mut settings = mine();
        let out = read_path(&settings, &file);
        assert_eq!(out["items"][0]["name"], "Barré 2");
        assert_eq!(out["items"][0]["phrases"], json!(["passer en barré"]));
        assert_eq!(out["items"][0]["dropped"], json!(["mettre en barré"]));
        assert_eq!(out["items"][1]["problem"], "bad_shortcut");
        let token = out["token"].as_str().unwrap().to_string();
        let done = apply_pending(&mut settings, &json!({"token": token, "include": [true, true]})).unwrap();
        assert_eq!((done.added.clone(), done.skipped), (vec!["Barré 2".to_string()], 1));
        assert_eq!(settings.custom_commands[0], mine().custom_commands[0], "la commande en place n'a pas bougé");
        assert_eq!(settings.custom_commands.len(), 2);
        assert_ne!(settings.custom_commands[1].id, "perso.1", "id en collision regénéré");
        assert!(settings.custom_commands[1].id.starts_with("perso."));
        assert!(!settings.custom_commands[1].needs_selection);
        consume();
    }

    #[test]
    fn an_unticked_command_is_not_imported_and_a_stale_agreement_is_refused() {
        let _one = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let dir = TestDir::new("stale");
        let file = dir.file("c.scriibe.json", &prepare(&mine(), 0).unwrap().1);
        let mut empty = AppSettings::default();
        let token = read_path(&empty, &file)["token"].as_str().unwrap().to_string();
        assert!(apply_pending(&mut empty, &json!({"token": "autre"})).is_err());
        assert!(apply_pending(&mut empty, &json!({"token": token, "include": [true, true]})).is_err(), "trop de cases");
        assert_eq!(apply_pending(&mut empty, &json!({"token": token, "include": [false]})).unwrap().added.len(), 0);
        assert!(empty.custom_commands.is_empty());
        consume();
    }
}
