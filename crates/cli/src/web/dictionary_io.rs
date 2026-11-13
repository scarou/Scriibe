//! Export et import du dictionnaire (chantier 4, étape 2), même démarche que pour les modes (`export.rs`, `import.rs`) : la page demande,
//! le serveur construit ou lit le fichier, l'utilisateur choisit l'emplacement (boîte du système ; chemin tapé ou dossier Documents sous
//! Linux). L'import lit le fichier **par son chemin** (jamais envoyé par la page), le contrôle, garde le résultat en attente et montre un
//! aperçu ; rien n'est écrit dans le dictionnaire avant l'accord, et la fusion ne fait qu'ajouter (`scriibe_core::import_dictionary`).

use super::export::{choose_target, write_without_overwriting, Target};
use super::import::{choose_file, read_file, typed_path, Picked};
use scriibe_core::dictionary::UserDictionary;
use scriibe_core::export as file_format;
use scriibe_core::import_dictionary::{self as importing, Incoming, Outcome, Preview, Status};
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Mutex;

/// Le contenu lu et contrôlé, en attente de l'accord de l'utilisateur, avec le jeton de l'aperçu montré : l'accord doit le rappeler,
/// pour qu'une autre page (ou un autre clic) ne puisse pas faire importer un fichier que l'utilisateur n'a pas vu.
static PENDING: Mutex<Option<(String, Incoming)>> = Mutex::new(None);

fn pending() -> std::sync::MutexGuard<'static, Option<(String, Incoming)>> {
    PENDING.lock().unwrap_or_else(|e| e.into_inner())
}

/// Le nom proposé et le texte du fichier d'export. `words` et `corrections` : les catégories qui partent. `Err` : un message pour la page.
pub(crate) fn prepare(dictionary: &UserDictionary, words: bool, corrections: bool, now_secs: i64) -> Result<(String, String), String> {
    let nothing = (!words || dictionary.words.is_empty()) && (!corrections || dictionary.corrections.is_empty());
    if nothing {
        return Err("rien à exporter : coche une catégorie qui n'est pas vide".into());
    }
    let day = file_format::day_of(now_secs);
    let file = file_format::dictionary_export(dictionary, words, corrections, super::update::current_version(), &day);
    let text = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())? + "\n";
    Ok((file_format::dictionary_file_name(&day), text))
}

/// `{"ok": true, "saved": true, "path": …}`, `{"ok": true, "saved": false}` si l'utilisateur annule, ou `{"ok": false, "error": …}`.
pub(crate) fn export(dictionary: &UserDictionary, input: &Value) -> Value {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
    let (name, text) =
        match prepare(dictionary, input["words"].as_bool().unwrap_or(true), input["corrections"].as_bool().unwrap_or(true), now) {
            Ok(v) => v,
            Err(e) => return json!({"ok": false, "error": e}),
        };
    let written = match choose_target("Enregistrer le dictionnaire", &name) {
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

fn status_json(status: &Status) -> Value {
    match status {
        Status::New => json!({"status": "new"}),
        Status::Same => json!({"status": "same"}),
        Status::Conflict { existing } => json!({"status": "conflict", "existing": existing}),
    }
}

fn preview_json(preview: &Preview) -> Value {
    let words: Vec<Value> = preview
        .words
        .iter()
        .map(|(word, status)| {
            let mut item = status_json(status);
            item["word"] = json!(word);
            item
        })
        .collect();
    let corrections: Vec<Value> = preview
        .corrections
        .iter()
        .map(|(c, status)| {
            let mut item = status_json(status);
            item["heard"] = json!(c.heard);
            item["written"] = json!(c.written);
            item
        })
        .collect();
    json!({"words": words, "corrections": corrections})
}

/// Lit, contrôle et met en attente le fichier `path`, puis rend l'aperçu. Un fichier refusé ne change rien (et oublie l'attente d'avant).
pub(crate) fn read_path(dictionary: &UserDictionary, path: &Path) -> Value {
    let incoming = match read_file(path).and_then(|text| importing::parse_dictionary(&text)) {
        Ok(incoming) => incoming,
        Err(e) => {
            *pending() = None;
            return json!({"ok": false, "error": e});
        }
    };
    let mut out = preview_json(&importing::preview(dictionary, &incoming));
    let token = scriibe_core::modes::new_id();
    *pending() = Some((token.clone(), incoming));
    out["ok"] = json!(true);
    out["token"] = json!(token);
    out
}

/// `POST api/dictionary/import/read` : `{"path"?}`. Un chemin tapé n'est honoré que là où il n'y a pas de boîte (Linux) ; ailleurs, la boîte
/// du système s'ouvre. Sans boîte et sans chemin, `ask_path: true`.
pub(crate) fn read(dictionary: &UserDictionary, input: &Value) -> Value {
    let typed = input["path"].as_str().filter(|p| !p.trim().is_empty());
    let path = match (cfg!(not(any(windows, target_os = "macos"))), typed) {
        (true, Some(typed)) => typed_path(typed),
        _ => match choose_file("Importer un dictionnaire") {
            Picked::File(path) => path,
            Picked::Cancelled => return json!({"ok": true, "cancelled": true}),
            Picked::Failed(e) => return json!({"ok": false, "error": e}),
            Picked::NoDialog => {
                return json!({"ok": false, "ask_path": true, "error": "le choix d'un fichier n'est pas encore pris en charge sur ce système"})
            }
        },
    };
    read_path(dictionary, &path)
}

/// Les cases cochées : une liste de booléens, un par élément du fichier ; `None` si la clé est absente.
fn flags(input: &Value, key: &str) -> Option<Vec<bool>> {
    input[key].as_array().map(|a| a.iter().map(|v| v.as_bool().unwrap_or(false)).collect())
}

/// Fusionne dans `dictionary` (à enregistrer par l'appelant, qui appelle ensuite `consume`) les éléments cochés du fichier en attente.
/// Refuse si le jeton n'est pas celui de l'aperçu en attente, ou si les cases ne correspondent pas à ses éléments : l'attente est
/// alors gardée telle quelle.
pub(crate) fn apply_pending(dictionary: &mut UserDictionary, input: &Value) -> Result<Outcome, String> {
    let guard = pending();
    let (token, incoming) = guard.as_ref().ok_or("aucun fichier en attente : recommence l'import")?;
    let stale = "l'aperçu n'est plus à jour : recommence l'import";
    if input["token"].as_str() != Some(token.as_str()) {
        return Err(stale.into());
    }
    let (words, corrections) = (flags(input, "words"), flags(input, "corrections"));
    if words.as_ref().is_some_and(|w| w.len() != incoming.words.len())
        || corrections.as_ref().is_some_and(|c| c.len() != incoming.corrections.len())
    {
        return Err(stale.into());
    }
    Ok(importing::apply(dictionary, incoming, &words.unwrap_or_default(), &corrections.unwrap_or_default()))
}

/// Oublie l'attente : après un enregistrement réussi du dictionnaire (jamais avant, pour pouvoir recommencer si l'écriture échoue).
pub(crate) fn consume() {
    *pending() = None;
}

/// `POST api/dictionary/import/cancel` : oublie le fichier en attente.
pub(crate) fn cancel() -> Value {
    *pending() = None;
    json!({"ok": true})
}

pub(crate) fn outcome_json(o: &Outcome) -> Value {
    json!({"ok": true, "words_added": o.words_added, "corrections_added": o.corrections_added, "already": o.already, "conflicts": o.conflicts, "over_limit": o.over_limit})
}

#[cfg(test)]
mod tests {
    use super::*;
    use scriibe_core::dictionary::Correction;
    use std::path::PathBuf;

    /// Les tests partagent l'attente de l'import : un à la fois.
    static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

    struct TestDir(PathBuf);

    impl TestDir {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("scriibe-dictionary-io-test-{}-{name}", std::process::id()));
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

    fn correction(heard: &str, written: &str) -> Correction {
        Correction { heard: heard.into(), written: written.into() }
    }

    fn dictionary() -> UserDictionary {
        UserDictionary { words: vec!["Scriibe".into(), "Claude Code".into()], corrections: vec![correction("cri be", "Scriibe")] }
    }

    #[test]
    fn the_export_has_the_agreed_name_and_only_the_chosen_categories() {
        let (name, text) = prepare(&dictionary(), true, true, 1_700_000_000).unwrap();
        assert_eq!(name, "scriibe-dictionnaire-2023-11-14.scriibe.json");
        let value: Value = serde_json::from_str(&text).unwrap();
        assert_eq!((value["kind"].as_str(), value["exportedAt"].as_str()), (Some("dictionary"), Some("2023-11-14")));
        assert_eq!(value["content"]["words"].as_array().unwrap().len(), 2);
        assert_eq!(value["content"]["corrections"][0]["heard"], "cri be");

        let (_, only_words) = prepare(&dictionary(), true, false, 0).unwrap();
        let value: Value = serde_json::from_str(&only_words).unwrap();
        assert!(value["content"].get("corrections").is_none() && value["content"]["words"].is_array());
    }

    #[test]
    fn nothing_to_export_is_refused() {
        assert!(prepare(&UserDictionary::default(), true, true, 0).is_err());
        assert!(prepare(&dictionary(), false, false, 0).is_err());
        let only_words = UserDictionary { words: vec!["a".into()], corrections: vec![] };
        assert!(prepare(&only_words, false, true, 0).is_err(), "la seule catégorie cochée est vide");
    }

    #[test]
    fn a_refused_file_changes_nothing_and_forgets_the_previous_wait() {
        let _one = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let dir = TestDir::new("refused");
        let good = dir.file("bon.scriibe.json", &prepare(&dictionary(), true, true, 0).unwrap().1);
        assert_eq!(read_path(&UserDictionary::default(), &good)["ok"], true);
        assert!(pending().is_some());

        let bad = dir.file("mauvais.scriibe.json", "pas du json");
        let out = read_path(&UserDictionary::default(), &bad);
        assert_eq!(out["ok"], false);
        assert!(out["error"].as_str().unwrap().contains("JSON"));
        assert!(pending().is_none(), "un fichier refusé ne laisse rien en attente");
        let mut untouched = dictionary();
        assert!(apply_pending(&mut untouched, &json!({"token": "x"})).is_err());
        assert_eq!(untouched, dictionary());
    }

    #[test]
    fn the_preview_then_the_agreement_adds_without_replacing() {
        let _one = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let dir = TestDir::new("merge");
        let file = dir.file(
            "d.scriibe.json",
            r#"{"format":"scriibe-export","version":1,"kind":"dictionary","content":{
                "words":["scriibe","Tauri"],
                "corrections":[{"heard":"cri be","written":"AUTRE"},{"heard":"tauri","written":"Tauri"}]}}"#,
        );
        let mut mine = dictionary();
        let out = read_path(&mine, &file);
        assert_eq!(out["ok"], true);
        assert_eq!((out["words"][0]["status"].as_str(), out["words"][1]["status"].as_str()), (Some("same"), Some("new")));
        assert_eq!(
            (out["corrections"][0]["status"].as_str(), out["corrections"][0]["existing"].as_str()),
            (Some("conflict"), Some("Scriibe"))
        );
        assert_eq!(mine, dictionary(), "l'aperçu ne change rien");

        let token = out["token"].as_str().unwrap().to_string();
        let done = apply_pending(&mut mine, &json!({"token": token, "words": [true, true], "corrections": [true, true]})).unwrap();
        assert_eq!((done.words_added, done.corrections_added, done.already, done.conflicts), (1, 1, 1, 1));
        assert_eq!(mine.words, vec!["Scriibe", "Claude Code", "Tauri"]);
        assert_eq!(
            mine.corrections,
            vec![correction("cri be", "Scriibe"), correction("tauri", "Tauri")],
            "la correction en place est gardée"
        );
        consume();
    }

    #[test]
    fn an_unticked_item_is_not_imported() {
        let _one = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let dir = TestDir::new("unticked");
        let file = dir.file("d.scriibe.json", &prepare(&dictionary(), true, true, 0).unwrap().1);
        let mut empty = UserDictionary::default();
        let token = read_path(&empty, &file)["token"].as_str().unwrap().to_string();
        let done = apply_pending(&mut empty, &json!({"token": token, "words": [true, false], "corrections": [false]})).unwrap();
        assert_eq!((done.words_added, done.corrections_added), (1, 0));
        assert_eq!(empty.words, vec!["Scriibe"]);
        consume();
    }

    #[test]
    fn a_wrong_token_or_wrong_number_of_boxes_is_stale_and_keeps_the_wait() {
        let _one = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let dir = TestDir::new("stale");
        let file = dir.file("d.scriibe.json", &prepare(&dictionary(), true, true, 0).unwrap().1);
        let mut empty = UserDictionary::default();
        let token = read_path(&empty, &file)["token"].as_str().unwrap().to_string();
        assert!(apply_pending(&mut empty, &json!({"token": "autre"})).is_err());
        assert!(apply_pending(&mut empty, &json!({"token": token, "words": [true]})).is_err(), "trop peu de cases");
        assert_eq!(empty, UserDictionary::default());
        assert!(apply_pending(&mut empty, &json!({"token": token})).is_ok(), "l'attente est gardée : on peut recommencer");
        consume();
    }
}
