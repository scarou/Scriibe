//! Les états d'affichage de la page, gardés par Scriibe : cartes repliées, case « Comparer avec le modèle » du banc, hauteur des cartes
//! du Dictionnaire, copie de la langue de l'interface. Le serveur local prend un port libre à chaque lancement : le stockage du
//! navigateur (rangé par adresse et port) repartait vide, ce fichier non.
//!
//! `ui-state.json`, à la racine du dossier de données, à côté de `settings.json` : propre à l'ordinateur (pas à l'utilisateur), jamais
//! dans la sauvegarde. Fichier absent, vide, abîmé ou d'une version inconnue : valeurs par défaut, sans erreur ; la prochaine
//! écriture le remplace (sauf une version plus récente, laissée telle quelle). Seules les entrées connues, aux valeurs permises,
//! sont gardées.

use crate::storage::write_atomic;
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub const FILE: &str = "ui-state.json";
const VERSION: u64 = 1;
/// Nombre de cartes retenues au plus (il y en a une soixantaine) et longueur d'un identifiant : un fichier qui ne grossit pas.
const MAX_FOLDS: usize = 300;
const MAX_ID_LEN: usize = 80;
/// Hauteur permise des cartes du Dictionnaire (px), comme la poignée de la page.
const HEIGHT: std::ops::RangeInclusive<i64> = 180..=1400;

pub struct UiStateStore {
    path: PathBuf,
    /// Une écriture à la fois : lire, fusionner, écrire ne se chevauchent pas.
    lock: Mutex<()>,
}

/// Ce que dit le fichier : l'état propre, ou `Newer` pour un fichier d'une version plus récente de Scriibe.
enum Disk {
    State(Map<String, Value>),
    Newer,
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= MAX_ID_LEN && id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':'))
}

fn height(value: &Value) -> Option<i64> {
    let n = value.as_i64().or_else(|| value.as_f64().filter(|f| f.is_finite()).map(|f| f.round() as i64))?;
    HEIGHT.contains(&n).then_some(n)
}

/// Fusionne `fragment` dans `state` (déjà propre) en ne prenant que les entrées connues et permises. `null` retire une entrée
/// (`folds` : une carte ; `dictionaryHeight`). Les autres clés, comme `profile` que la page ajoute à ses envois, sont ignorées.
fn merge(state: &mut Map<String, Value>, fragment: &Value) {
    let Some(fragment) = fragment.as_object() else { return };
    if let Some(folds) = fragment.get("folds").and_then(Value::as_object) {
        let mut kept = state.get("folds").and_then(Value::as_object).cloned().unwrap_or_default();
        for (id, value) in folds {
            if !valid_id(id) {
                continue;
            }
            match value {
                Value::Null => {
                    kept.remove(id);
                }
                Value::Bool(_) if kept.contains_key(id) || kept.len() < MAX_FOLDS => {
                    kept.insert(id.clone(), value.clone());
                }
                _ => {}
            }
        }
        if kept.is_empty() {
            state.remove("folds");
        } else {
            state.insert("folds".into(), Value::Object(kept));
        }
    }
    if let Some(on) = fragment.get("benchCompare").and_then(Value::as_bool) {
        state.insert("benchCompare".into(), json!(on));
    }
    match fragment.get("dictionaryHeight") {
        Some(Value::Null) => {
            state.remove("dictionaryHeight");
        }
        Some(value) => {
            if let Some(n) = height(value) {
                state.insert("dictionaryHeight".into(), json!(n));
            }
        }
        None => {}
    }
    if let Some(lang) = fragment.get("uiLanguage").and_then(Value::as_str).filter(|l| matches!(*l, "fr" | "en" | "auto")) {
        state.insert("uiLanguage".into(), json!(lang));
    }
}

impl UiStateStore {
    pub fn new(folder: &Path) -> Self {
        Self { path: folder.join(FILE), lock: Mutex::new(()) }
    }

    fn read_disk(&self) -> Disk {
        let mut state = Map::new();
        let Ok(text) = std::fs::read_to_string(&self.path) else { return Disk::State(state) };
        let Ok(Value::Object(file)) = serde_json::from_str::<Value>(&text) else { return Disk::State(state) };
        match file.get("version").and_then(Value::as_u64) {
            Some(v) if v > VERSION => return Disk::Newer,
            Some(VERSION) => {}
            _ => return Disk::State(state),
        }
        merge(&mut state, &Value::Object(file));
        Disk::State(state)
    }

    /// L'état gardé, avec les seules entrées connues et valides (objet vide si rien n'est gardé).
    pub fn read(&self) -> Value {
        let _guard = self.lock.lock().unwrap_or_else(|p| p.into_inner());
        match self.read_disk() {
            Disk::State(state) => Value::Object(state),
            Disk::Newer => json!({}),
        }
    }

    /// Fusionne un fragment, écrit le fichier (écriture sûre) et rend l'état obtenu. Une erreur d'écriture est rendue en texte : la
    /// page garde alors l'état en mémoire, sans rien afficher.
    pub fn patch(&self, fragment: &Value) -> Result<Value, String> {
        let _guard = self.lock.lock().unwrap_or_else(|p| p.into_inner());
        let Disk::State(mut state) = self.read_disk() else {
            return Err("fichier d'une version plus récente de Scriibe, laissé tel quel".into());
        };
        merge(&mut state, fragment);
        let mut file = Map::new();
        file.insert("version".into(), json!(VERSION));
        file.extend(state.clone());
        let text = serde_json::to_string_pretty(&Value::Object(file)).map_err(|e| e.to_string())?;
        write_atomic(&self.path, &text).map_err(|e| format!("états de la fenêtre non enregistrés : {e}"))?;
        Ok(Value::Object(state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("scriibe-ui-state-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_three_states_survive_a_restart() {
        let dir = temp("restart");
        let store = UiStateStore::new(&dir);
        store.patch(&json!({"folds": {"modes-mes-modes": true, "utilisateurs": false}})).unwrap();
        store.patch(&json!({"benchCompare": false})).unwrap();
        store.patch(&json!({"dictionaryHeight": 420})).unwrap();
        store.patch(&json!({"uiLanguage": "en"})).unwrap();
        // Une nouvelle instance, comme après avoir quitté puis relancé Scriibe.
        let state = UiStateStore::new(&dir).read();
        assert_eq!(state["folds"], json!({"modes-mes-modes": true, "utilisateurs": false}));
        assert_eq!(state["benchCompare"], json!(false));
        assert_eq!(state["dictionaryHeight"], json!(420));
        assert_eq!(state["uiLanguage"], json!("en"));
    }

    #[test]
    fn nothing_kept_gives_an_empty_state() {
        let dir = temp("empty");
        assert_eq!(UiStateStore::new(&dir).read(), json!({}));
    }

    #[test]
    fn two_fragments_do_not_overwrite_each_other() {
        let dir = temp("fragments");
        let store = UiStateStore::new(&dir);
        store.patch(&json!({"folds": {"a": true}})).unwrap();
        store.patch(&json!({"folds": {"b": false}, "benchCompare": true})).unwrap();
        store.patch(&json!({"folds": {"a": null}})).unwrap();
        assert_eq!(store.read(), json!({"folds": {"b": false}, "benchCompare": true}));
        store.patch(&json!({"dictionaryHeight": 300})).unwrap();
        store.patch(&json!({"dictionaryHeight": null})).unwrap();
        assert!(store.read().get("dictionaryHeight").is_none(), "null retire la hauteur");
    }

    #[test]
    fn concurrent_writes_all_land() {
        let dir = temp("threads");
        let store = std::sync::Arc::new(UiStateStore::new(&dir));
        let handles: Vec<_> = (0..8)
            .map(|i| {
                let store = store.clone();
                std::thread::spawn(move || store.patch(&json!({"folds": {format!("carte-{i}"): i % 2 == 0}})).unwrap())
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(UiStateStore::new(&dir).read()["folds"].as_object().unwrap().len(), 8);
    }

    #[test]
    fn a_missing_empty_or_damaged_file_gives_defaults_without_error_and_is_replaced_by_the_next_write() {
        for (name, content) in [
            ("damaged", "pas du json {"),
            ("empty-file", ""),
            ("array", "[1,2]"),
            ("no-version", r#"{"benchCompare": false}"#),
            ("old-version", r#"{"version": 0, "benchCompare": false}"#),
            ("truncated", r#"{"version": 1, "folds": {"a": tr"#),
        ] {
            let dir = temp(name);
            std::fs::write(dir.join(FILE), content).unwrap();
            let store = UiStateStore::new(&dir);
            assert_eq!(store.read(), json!({}), "{name}");
            store.patch(&json!({"benchCompare": true})).unwrap();
            assert_eq!(UiStateStore::new(&dir).read(), json!({"benchCompare": true}), "{name}");
        }
    }

    #[test]
    fn a_file_from_a_newer_scriibe_is_ignored_and_never_overwritten() {
        let dir = temp("newer");
        let text = r#"{"version": 9, "benchCompare": false, "avenir": 1}"#;
        std::fs::write(dir.join(FILE), text).unwrap();
        let store = UiStateStore::new(&dir);
        assert_eq!(store.read(), json!({}));
        assert!(store.patch(&json!({"benchCompare": true})).is_err());
        assert_eq!(std::fs::read_to_string(dir.join(FILE)).unwrap(), text);
    }

    #[test]
    fn out_of_range_or_unknown_values_are_ignored() {
        let dir = temp("bounds");
        let store = UiStateStore::new(&dir);
        store.patch(&json!({"dictionaryHeight": 90, "benchCompare": "oui", "uiLanguage": "de", "profile": "bruno", "inconnu": 1})).unwrap();
        assert_eq!(store.read(), json!({}));
        store.patch(&json!({"dictionaryHeight": 5000})).unwrap();
        store.patch(&json!({"dictionaryHeight": 180})).unwrap();
        assert_eq!(store.read(), json!({"dictionaryHeight": 180}));
        store.patch(&json!({"dictionaryHeight": 419.6})).unwrap();
        assert_eq!(store.read()["dictionaryHeight"], json!(420));
        store.patch(&json!({"dictionaryHeight": "haut"})).unwrap();
        assert_eq!(store.read()["dictionaryHeight"], json!(420), "une valeur inutilisable ne change rien");
    }

    #[test]
    fn card_ids_are_checked_and_their_number_is_capped() {
        let dir = temp("ids");
        let store = UiStateStore::new(&dir);
        let long = "x".repeat(MAX_ID_LEN + 1);
        store.patch(&json!({"folds": {"": true, long.clone(): true, "a b": true, "ok-1.2:x": true, "texte": "oui"}})).unwrap();
        assert_eq!(store.read(), json!({"folds": {"ok-1.2:x": true}}));
        let many: Map<String, Value> = (0..MAX_FOLDS + 50).map(|i| (format!("c{i}"), json!(true))).collect();
        store.patch(&json!({"folds": many})).unwrap();
        assert_eq!(store.read()["folds"].as_object().unwrap().len(), MAX_FOLDS);
        // Une carte déjà connue se change encore quand la liste est pleine, et se retire.
        store.patch(&json!({"folds": {"ok-1.2:x": false}})).unwrap();
        assert_eq!(store.read()["folds"]["ok-1.2:x"], json!(false));
    }

    #[test]
    fn unknown_keys_and_bad_entries_in_the_file_are_dropped_on_read() {
        let dir = temp("dirty");
        std::fs::write(
            dir.join(FILE),
            r#"{"version": 1, "folds": {"a": true, "b": 3, "c d": false}, "benchCompare": 1, "dictionaryHeight": 99999, "uiLanguage": "fr", "autre": []}"#,
        )
        .unwrap();
        assert_eq!(UiStateStore::new(&dir).read(), json!({"folds": {"a": true}, "uiLanguage": "fr"}));
    }

    #[test]
    fn no_temporary_file_is_left_behind() {
        let dir = temp("temp-files");
        let store = UiStateStore::new(&dir);
        for i in 0..5 {
            store.patch(&json!({"dictionaryHeight": 200 + i})).unwrap();
        }
        let names: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
        assert_eq!(names, vec![FILE.to_string()]);
    }
}
