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

/// Les modes lus et contrôlés, en attente de l'accord de l'utilisateur, avec le jeton de l'aperçu montré : l'accord doit le rappeler,
/// pour qu'une autre page (ou un autre clic) ne puisse pas faire importer un fichier que l'utilisateur n'a pas vu.
static PENDING: Mutex<Option<(String, Vec<CustomMode>)>> = Mutex::new(None);

fn pending() -> std::sync::MutexGuard<'static, Option<(String, Vec<CustomMode>)>> {
    PENDING.lock().unwrap_or_else(|e| e.into_inner())
}

/// Le contenu du fichier, après les contrôles de taille. `Err` : un message pour la page.
pub(crate) fn read_file(path: &Path) -> Result<String, String> {
    read_file_max(path, import::MAX_BYTES, "fichier trop gros : 1 Mo au maximum")
}

/// Comme [`read_file`], avec une autre limite de taille (la sauvegarde complète : 64 Mio).
pub(crate) fn read_file_max(path: &Path, max: u64, too_big: &str) -> Result<String, String> {
    let unreadable = || "le fichier n'a pas pu être lu".to_string();
    let metadata = std::fs::metadata(path).map_err(|_| unreadable())?;
    if !metadata.is_file() {
        return Err(unreadable());
    }
    if metadata.len() > max {
        return Err(too_big.into());
    }
    // Lecture bornée aussi : le fichier a pu grossir depuis le contrôle.
    let mut bytes = Vec::new();
    std::fs::File::open(path).map_err(|_| unreadable())?.take(max + 1).read_to_end(&mut bytes).map_err(|_| unreadable())?;
    if bytes.len() as u64 > max {
        return Err(too_big.into());
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
    let token = scriibe_core::modes::new_id();
    *pending() = Some((token.clone(), modes));
    json!({"ok": true, "token": token, "items": preview_json(&items)})
}

/// Ce que l'utilisateur a tapé comme chemin (Linux) : sans guillemets ni espaces autour, `~` développé.
pub(crate) fn typed_path(text: &str) -> PathBuf {
    let text = text.trim().trim_matches(|c| c == '"' || c == '\'').trim();
    match (text.strip_prefix("~/"), std::env::var_os("HOME")) {
        (Some(rest), Some(home)) => PathBuf::from(home).join(rest),
        _ => PathBuf::from(text),
    }
}

/// Ce que donne la boîte « Ouvrir » du système (selon le système, certaines variantes ne servent pas).
#[allow(dead_code)]
pub(crate) enum Picked {
    File(PathBuf),
    Cancelled,
    /// Aucune boîte sur ce système (Linux) : la page demande le chemin.
    NoDialog,
    Failed(String),
}

pub(crate) fn choose_file(title: &str) -> Picked {
    let _ = title;
    #[cfg(windows)]
    {
        let filter = [("Fichiers Scriibe (*.scriibe.json)", "*.json")];
        match scriibe_platform::windows::file_dialog::pick_file(title, &filter) {
            Some(path) => Picked::File(PathBuf::from(path)),
            None => Picked::Cancelled,
        }
    }
    #[cfg(target_os = "macos")]
    {
        let filter = [("Fichiers Scriibe (*.scriibe.json)", "*.json")];
        match scriibe_platform::macos::file_dialog::try_pick_file(title, &filter) {
            Ok(Some(path)) => Picked::File(PathBuf::from(path)),
            Ok(None) => Picked::Cancelled,
            Err(e) => Picked::Failed(e),
        }
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        Picked::NoDialog
    }
}

/// `POST api/modes/import/read` : `{"path"?}`. Un chemin tapé n'est honoré que là où il n'y a pas de boîte (Linux) ; ailleurs, la boîte
/// du système s'ouvre (la page n'a pas à désigner de fichier). Sans boîte et sans chemin, `ask_path: true`.
pub(crate) fn read(settings: &AppSettings, input: &Value) -> Value {
    let typed = input["path"].as_str().filter(|p| !p.trim().is_empty());
    let path = match (cfg!(not(any(windows, target_os = "macos"))), typed) {
        (true, Some(typed)) => typed_path(typed),
        _ => match choose_file("Importer un mode") {
            Picked::File(path) => path,
            Picked::Cancelled => return json!({"ok": true, "cancelled": true}),
            Picked::Failed(e) => return json!({"ok": false, "error": e}),
            Picked::NoDialog => {
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

/// Fusionne les modes en attente dans `settings` (à enregistrer par l'appelant, qui appelle ensuite `consume`). Refuse si le jeton n'est
/// pas celui de l'aperçu en attente, ou si les choix ne correspondent pas à ses modes : l'attente est alors gardée telle quelle.
pub(crate) fn apply_pending(settings: &mut AppSettings, input: &Value) -> Result<import::Outcome, String> {
    let guard = pending();
    let (token, modes) = guard.as_ref().ok_or("aucun fichier en attente : recommence l'import")?;
    let stale = "l'aperçu n'est plus à jour : recommence l'import";
    if input["token"].as_str() != Some(token.as_str()) {
        return Err(stale.into());
    }
    let choices = choices_of(input);
    if input["choices"].as_array().is_some_and(|a| a.len() != modes.len()) {
        return Err(stale.into());
    }
    Ok(import::apply(&mut settings.custom_modes, modes.clone(), &choices))
}

/// Oublie l'attente : après un enregistrement réussi des réglages (jamais avant, pour pouvoir recommencer si l'écriture échoue).
pub(crate) fn consume() {
    *pending() = None;
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

    /// Un dossier d'essai à soi, supprimé en fin de test.
    struct TestDir(PathBuf);

    impl TestDir {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("scriibe-import-test-{}-{name}", std::process::id()));
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

    fn sms() -> CustomMode {
        let mut m = CustomMode::new("SMS", "Court.", Fidelity::Light);
        m.examples = vec![ModeExample::new("a", "b")];
        m
    }

    fn file_text(mode: &CustomMode) -> String {
        export::modes_export(&[mode], true, "0.3.0", "2026-11-12").to_string()
    }

    #[test]
    fn a_refused_file_leaves_nothing_pending() {
        let _guard = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let dir = TestDir::new("refuse");
        let settings = AppSettings { custom_modes: vec![sms()], ..AppSettings::default() };
        // D'abord un bon fichier en attente, puis un fichier abîmé : l'attente est vidée, rien ne peut plus être appliqué.
        let ok = read_modes(&settings, &dir.file("ok.scriibe.json", &file_text(&sms())));
        let token = ok["token"].as_str().unwrap().to_string();
        let bad = read_modes(&settings, &dir.file("abime.scriibe.json", "{ pas du json"));
        assert_eq!((bad["ok"].clone(), bad["error"].clone()), (json!(false), json!("ce fichier n'est pas un fichier JSON lisible")));
        assert!(pending().is_none());
        let mut after = settings.clone();
        assert!(apply_pending(&mut after, &json!({"token": token})).is_err(), "rien à appliquer");
        assert_eq!(after.custom_modes, settings.custom_modes);
    }

    #[test]
    fn a_file_that_is_too_big_or_missing_is_refused_before_being_read() {
        let dir = TestDir::new("gros");
        let big = dir.file("gros.scriibe.json", &"x".repeat(import::MAX_BYTES as usize + 1));
        assert_eq!(read_file(&big), Err("fichier trop gros : 1 Mo au maximum".to_string()));
        assert_eq!(read_file(Path::new("n-existe-pas.json")), Err("le fichier n'a pas pu être lu".to_string()));
        assert!(read_file(&dir.0).is_err(), "un dossier n'est pas un fichier");
        let not_utf8 = dir.0.join("binaire.scriibe.json");
        std::fs::write(&not_utf8, [0xff, 0xfe, 0x00]).unwrap();
        assert_eq!(read_file(&not_utf8), Err("ce fichier n'est pas un fichier JSON lisible".to_string()));
    }

    #[test]
    fn the_preview_then_the_agreement_adds_or_replaces() {
        let _guard = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let dir = TestDir::new("accord");
        let existing = sms();
        let settings = AppSettings { custom_modes: vec![existing.clone()], ..AppSettings::default() };
        let mut incoming = sms();
        incoming.style = "Autre.".into();
        let path = dir.file("ok.scriibe.json", &file_text(&incoming));
        let out = read_modes(&settings, &path);
        assert_eq!(out["ok"], true);
        assert_eq!((out["items"][0]["conflict"].clone(), out["items"][0]["added_as"].clone()), (json!("SMS"), json!("SMS 2")));
        let token = out["token"].as_str().unwrap().to_string();

        // Accord : remplacer.
        let mut replaced = settings.clone();
        let agreement = json!({"token": token, "choices": [{"include": true, "replace": true}]});
        let done = apply_pending(&mut replaced, &agreement).unwrap();
        assert_eq!(done.replaced, vec!["SMS".to_string()]);
        assert_eq!(
            (replaced.custom_modes.len(), replaced.custom_modes[0].style.as_str(), &replaced.custom_modes[0].id),
            (1, "Autre.", &existing.id)
        );
        assert!(pending().is_some(), "l'attente reste tant que les réglages ne sont pas enregistrés");
        consume();
        assert!(apply_pending(&mut replaced, &agreement).is_err(), "attente consommée : un second accord ne ferait rien");

        // Même fichier, accord par défaut : ajouter avec un numéro.
        let token = read_modes(&settings, &path)["token"].as_str().unwrap().to_string();
        let mut added = settings.clone();
        apply_pending(&mut added, &json!({"token": token})).unwrap();
        assert_eq!(added.custom_modes.iter().map(|m| m.name.as_str()).collect::<Vec<_>>(), vec!["SMS", "SMS 2"]);
        assert_eq!(added.custom_modes[0], existing);
        consume();
    }

    #[test]
    fn an_agreement_for_another_preview_is_refused_and_changes_nothing() {
        // Relecture : deux pages ouvertes (ou deux clics) ; la seconde lecture ne doit pas se faire appliquer avec l'accord de la première.
        let _guard = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let dir = TestDir::new("jeton");
        let settings = AppSettings { custom_modes: vec![sms()], ..AppSettings::default() };
        let mut other = sms();
        other.name = "Autre".into();
        let first = read_modes(&settings, &dir.file("a.scriibe.json", &file_text(&sms())));
        let second = read_modes(&settings, &dir.file("b.scriibe.json", &file_text(&other)));
        let (first, second) = (first["token"].as_str().unwrap().to_string(), second["token"].as_str().unwrap().to_string());
        assert_ne!(first, second);
        let mut target = settings.clone();
        let stale = apply_pending(&mut target, &json!({"token": first}));
        assert_eq!(stale, Err("l'aperçu n'est plus à jour : recommence l'import".to_string()));
        assert_eq!(target.custom_modes, settings.custom_modes, "rien n'a changé");
        assert!(apply_pending(&mut target, &Value::Null).is_err(), "sans jeton non plus");
        assert!(pending().is_some(), "l'aperçu de la seconde page reste valable");
        // Un nombre de choix qui ne correspond pas à l'aperçu est refusé aussi.
        let wrong = json!({"token": second, "choices": [{"include": true}, {"include": true}]});
        assert!(apply_pending(&mut target, &wrong).is_err());
        assert!(apply_pending(&mut target, &json!({"token": second})).is_ok());
        consume();
    }

    #[test]
    fn a_typed_path_loses_its_quotes() {
        assert_eq!(typed_path("  \"/a/b.json\" "), PathBuf::from("/a/b.json"));
        assert_eq!(typed_path("'x.json'"), PathBuf::from("x.json"));
    }
}
