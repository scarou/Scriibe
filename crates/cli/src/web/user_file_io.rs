//! Export et import d'un utilisateur entier (étape 5 de `docs/CADRAGE_PROFILS.md`, `kind: "profile"`), même démarche que le dictionnaire et les
//! commandes : le serveur construit ou lit le fichier, garde le résultat en attente avec un jeton, montre un aperçu ; rien n'est écrit avant
//! l'accord. Un import **crée toujours un nouvel utilisateur** et n'écrase jamais rien (`scriibe_core::profile_file`).
//!
//! L'export vise un utilisateur **par son identifiant** (l'actif ou un autre, pour pouvoir exporter avant de supprimer) : il ne dépend donc pas
//! de l'utilisateur que la page croit actif. Jamais de clé d'API, d'historique, de statistiques ni de champ de l'ordinateur.

use super::commands_io;
use super::export::{choose_target, write_without_overwriting, Target};
use super::import::{choose_file, read_file, typed_path, Picked};
use super::profiles_io::{self, Failure};
use scriibe_core::export as file_format;
use scriibe_core::modes::RewriteMode;
use scriibe_core::profile_file::{self as user_file, Categories, Contents, Incoming};
use scriibe_platform::storage::FileConfig;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Mutex;

static PENDING: Mutex<Option<(String, Incoming)>> = Mutex::new(None);

fn pending() -> std::sync::MutexGuard<'static, Option<(String, Incoming)>> {
    PENDING.lock().unwrap_or_else(|e| e.into_inner())
}

/// Le libellé d'un mode livré (la clé d'une consigne modifiée), pour l'aperçu.
fn built_in_label(id: &str) -> String {
    RewriteMode::from_raw(id).map_or_else(|| id.to_string(), |m| m.label().to_string())
}

fn contents_json(id: &str, c: &Contents) -> Value {
    json!({
        "ok": true,
        "id": id,
        "name": c.name,
        "first_name": c.first_name,
        "preferences": c.preferences,
        "words": c.words,
        "corrections": c.corrections.iter().map(|(heard, written)| json!({"heard": heard, "written": written})).collect::<Vec<_>>(),
        "modes": c.modes,
        "examples": c.examples,
        "styles": c.styles.iter().map(|s| built_in_label(s)).collect::<Vec<_>>(),
        "commands": c.commands,
    })
}

fn error_of(e: scriibe_platform::profiles::ProfileError) -> Value {
    json!({"ok": false, "error": e.message()})
}

/// `POST api/profile/export/preview` : `{"id"}`. Ce qui partirait, par catégorie, avant que la page ne demande où enregistrer.
pub(crate) fn export_preview(config: &FileConfig, input: &Value) -> Value {
    let id = input["id"].as_str().unwrap_or("");
    match config.profile_snapshot(id) {
        Ok((name, settings, dictionary)) => contents_json(id, &user_file::contents(&name, &settings, &dictionary)),
        Err(e) => error_of(e),
    }
}

/// Le nom proposé et le texte du fichier d'export. `Err` : un message pour la page (rien à exporter).
pub(crate) fn prepare(config: &FileConfig, id: &str, take: Categories, now_secs: i64) -> Result<(String, String), String> {
    let (name, settings, dictionary) = config.profile_snapshot(id).map_err(|e| e.message())?;
    let day = file_format::day_of(now_secs);
    let file = user_file::export_profile(&name, &settings, &dictionary, take, super::update::current_version(), &day);
    if file["content"].as_object().is_none_or(|c| c.is_empty()) {
        return Err("rien à exporter : coche une catégorie qui n'est pas vide".into());
    }
    let text = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())? + "\n";
    Ok((user_file::file_name(&day), text))
}

/// `POST api/profile/export` : `{"id", "identity"?, "preferences"?, …}` (cases ; le nom et le prénom sont décochés par défaut). Ouvre la boîte
/// « Enregistrer sous » du système (sous Linux : le dossier Documents, sans jamais écraser un fichier).
pub(crate) fn export(config: &FileConfig, input: &Value) -> Value {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
    let take = Categories::from_value(input, Categories::export_default());
    let (name, text) = match prepare(config, input["id"].as_str().unwrap_or(""), take, now) {
        Ok(v) => v,
        Err(e) => return json!({"ok": false, "error": e}),
    };
    let written = match choose_target("Enregistrer l'utilisateur", &name) {
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

fn preview_json(incoming: &Incoming) -> Value {
    let p = user_file::preview(incoming);
    json!({
        "name": p.name,
        "first_name": p.first_name,
        "preferences": p.preferences,
        "hidden_modes": p.hidden_modes,
        "words": p.words,
        "corrections": p.corrections,
        "modes": p.modes.iter().map(|m| json!({"name": m.name, "fidelity": m.fidelity.raw(), "examples": m.examples, "added_as": m.added_as})).collect::<Vec<_>>(),
        "styles": p.styles.iter().map(|s| built_in_label(s)).collect::<Vec<_>>(),
        "commands": commands_io::items_json(&p.commands),
        "suggested_name": p.name.clone().unwrap_or_else(|| user_file::UNNAMED.to_string()),
    })
}

/// Lit, contrôle et met en attente le fichier `path`, puis rend l'aperçu. Un fichier refusé ne change rien (et oublie l'attente d'avant).
pub(crate) fn read_path(path: &Path) -> Value {
    let incoming = match read_file(path).and_then(|text| user_file::parse_profile(&text)) {
        Ok(incoming) => incoming,
        Err(e) => {
            *pending() = None;
            return json!({"ok": false, "error": e});
        }
    };
    let mut out = preview_json(&incoming);
    let token = scriibe_core::modes::new_id();
    *pending() = Some((token.clone(), incoming));
    out["ok"] = json!(true);
    out["token"] = json!(token);
    out
}

/// `POST api/profile/import/read` : `{"path"?}` (chemin tapé honoré seulement sans boîte du système, c'est-à-dire sous Linux).
pub(crate) fn read(input: &Value) -> Value {
    let typed = input["path"].as_str().filter(|p| !p.trim().is_empty());
    let path = match (cfg!(not(any(windows, target_os = "macos"))), typed) {
        (true, Some(typed)) => typed_path(typed),
        _ => match choose_file("Importer un utilisateur") {
            Picked::File(path) => path,
            Picked::Cancelled => return json!({"ok": true, "cancelled": true}),
            Picked::Failed(e) => return json!({"ok": false, "error": e}),
            Picked::NoDialog => {
                return json!({"ok": false, "ask_path": true, "error": "le choix d'un fichier n'est pas encore pris en charge sur ce système"})
            }
        },
    };
    read_path(&path)
}

/// `POST api/profile/import/apply` : `{"token", "name"?, "include"?: {cases}}`. Crée un **nouvel** utilisateur (jamais actif d'office, jamais un
/// écrasement) avec les catégories cochées. Refuse si le jeton n'est pas celui de l'aperçu : l'attente est alors gardée.
pub(crate) fn apply_pending(config: &FileConfig, input: &Value) -> Result<Value, Failure> {
    let (incoming, name) = {
        let guard = pending();
        let (token, incoming) = guard.as_ref().ok_or_else(|| Failure::Refused("aucun fichier en attente : recommence l'import".into()))?;
        if input["token"].as_str() != Some(token.as_str()) {
            return Err(Failure::Refused("l'aperçu n'est plus à jour : recommence l'import".into()));
        }
        let name = input["name"]
            .as_str()
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .map(str::to_string)
            .or_else(|| incoming.name.clone())
            .unwrap_or_else(|| user_file::UNNAMED.to_string());
        (incoming.clone(), name)
    };
    let take = Categories::from_value(&input["include"], Categories::import_default());
    let built = user_file::build(&incoming, take);
    if built.profile.is_empty() && built.dictionary.words.is_empty() && built.dictionary.corrections.is_empty() {
        return Err(Failure::Refused("rien à importer : coche au moins une catégorie".into()));
    }
    let made = config.import_profile(&name, built.profile, &built.dictionary)?;
    consume();
    let o = &built.outcome;
    Ok(json!({
        "ok": true,
        "created": {"id": made.id, "name": made.name},
        "outcome": {
            "words": o.words, "corrections": o.corrections, "modes": o.modes, "examples": o.examples, "styles": o.styles,
            "commands": o.commands, "commands_skipped": o.commands_skipped, "preferences": o.preferences, "identity": o.identity,
        },
        "list": profiles_io::view(config),
    }))
}

pub(crate) fn consume() {
    *pending() = None;
}

pub(crate) fn cancel() -> Value {
    *pending() = None;
    json!({"ok": true})
}

#[cfg(test)]
mod tests {
    use super::super::profiles_io::tests::world;
    use super::*;
    use chrono::Utc;
    use scriibe_core::commands::CustomCommand;
    use scriibe_core::dictionary::{Correction, UserDictionary};
    use scriibe_core::history::Dictation;
    use scriibe_core::modes::{CustomMode, Fidelity, ModeExample};
    use scriibe_platform::storage::HistorySink;

    /// Les tests partagent l'attente de l'import : un à la fois.
    static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

    fn one_at_a_time() -> std::sync::MutexGuard<'static, ()> {
        ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Un utilisateur actif bien rempli, avec des champs d'ordinateur reconnaissables qui ne doivent jamais partir.
    fn filled(
        name: &str,
    ) -> (std::path::PathBuf, std::sync::Arc<FileConfig>, std::sync::Arc<scriibe_platform::storage::FileHistory>, String) {
        let (dir, config, history) = world(name);
        let mut settings = config.settings();
        settings.first_name = "Léa".into();
        settings.language = scriibe_core::settings::DictationLanguage::En;
        let mut sms = CustomMode::new("SMS", "Court.", Fidelity::Light);
        sms.examples = vec![ModeExample::new("dictée", "attendu")];
        settings.custom_modes = vec![sms];
        settings.custom_commands = vec![CustomCommand {
            id: format!("{}1", scriibe_core::commands::CUSTOM_PREFIX),
            name: "Barrer".into(),
            phrases: vec!["barre ça".into()],
            shortcut: "Mod+Shift+X".into(),
            needs_selection: false,
        }];
        settings.lm_studio_url = "http://hote-secret:1234".into();
        settings.microphone_name = "Micro secret".into();
        settings.typing_wpm = 99;
        config.save_settings(&settings).unwrap();
        config
            .save_dictionary(&UserDictionary {
                words: vec!["Scriibe".into()],
                corrections: vec![Correction { heard: "cri be".into(), written: "Scriibe".into() }],
            })
            .unwrap();
        history.record(Dictation::new(Utc::now(), "Chrome", "texte dicté très privé", "texte dicté très privé", 12, 4.0, 0));
        let id = config.active_profile_id().unwrap();
        (dir, config, history, id)
    }

    fn write_file(dir: &Path, text: &str) -> std::path::PathBuf {
        let path = dir.join("fichier.scriibe.json");
        std::fs::write(&path, text).unwrap();
        path
    }

    #[test]
    fn the_preview_of_an_export_lists_what_would_leave_for_any_user_active_or_not() {
        let (dir, config, _, id) = filled("preview");
        let other = config.create_profile("Bruno").unwrap();
        let preview = export_preview(&config, &json!({"id": id}));
        assert_eq!(
            (preview["ok"].clone(), preview["words"].clone(), preview["modes"].clone()),
            (json!(true), json!(["Scriibe"]), json!(["SMS"]))
        );
        assert_eq!(
            (preview["examples"].clone(), preview["commands"].clone(), preview["first_name"].clone()),
            (json!(1), json!(["Barrer"]), json!("Léa"))
        );
        let blank = export_preview(&config, &json!({"id": other.id}));
        assert_eq!(
            (blank["ok"].clone(), blank["words"].clone(), blank["modes"].clone()),
            (json!(true), json!([]), json!([])),
            "l'inventaire d'un utilisateur qui n'est pas actif"
        );
        assert_eq!(export_preview(&config, &json!({"id": "INCONNU"}))["ok"], false);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_exported_text_has_no_secret_no_history_and_no_name_unless_ticked() {
        let (dir, config, _, id) = filled("export-text");
        let (name, text) = prepare(&config, &id, Categories::export_default(), 0).unwrap();
        assert!(name.starts_with("scriibe-utilisateur-1970-01-01") && name.ends_with(".scriibe.json") && !name.contains("Principal"));
        for forbidden in ["hote-secret", "Micro secret", "typingWPM", "texte dicté très privé", "Léa", "Principal", "history"] {
            assert!(!text.contains(forbidden), "{forbidden} ne doit pas partir : {text}");
        }
        for wanted in ["Scriibe", "SMS", "Barrer", "attendu"] {
            assert!(text.contains(wanted), "{wanted} devrait partir");
        }
        let (_, with_identity) = prepare(&config, &id, Categories { identity: true, ..Categories::export_default() }, 0).unwrap();
        assert!(with_identity.contains("\"firstName\": \"Léa\"") && with_identity.contains("\"name\": \"Principal\""));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn nothing_to_export_and_an_unreadable_file_are_refused() {
        let (dir, config, _, id) = filled("export-refused");
        let none = Categories {
            identity: false,
            preferences: false,
            words: false,
            corrections: false,
            modes: false,
            examples: false,
            styles: false,
            commands: false,
        };
        assert!(prepare(&config, &id, none, 0).unwrap_err().contains("rien à exporter"));
        let folder = config.profile_folder().unwrap();
        std::fs::write(folder.join("dictionary.json"), "pas du json").unwrap();
        assert!(prepare(&config, &id, Categories::export_default(), 0).is_err(), "un dictionnaire illisible n'est pas exporté à moitié");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn an_import_creates_a_new_user_and_changes_nothing_else() {
        let _one = one_at_a_time();
        let (dir, config, _, id) = filled("import");
        let before = std::fs::read(config.profile_folder().unwrap().join("profile.json")).unwrap();
        let (_, text) = prepare(&config, &id, Categories { identity: true, ..Categories::export_default() }, 0).unwrap();
        let preview = read_path(&write_file(&dir, &text));
        assert_eq!(
            (preview["ok"].clone(), preview["words"].clone(), preview["suggested_name"].clone()),
            (json!(true), json!(1), json!("Principal"))
        );
        let applied = apply_pending(&config, &json!({"token": preview["token"], "name": "Copie de Léa"})).unwrap();
        let made = applied["created"]["id"].as_str().unwrap().to_string();
        assert_eq!(applied["created"]["name"], "Copie de Léa");
        assert_eq!(config.active_profile_id().as_deref(), Some(id.as_str()), "l'import ne change pas d'utilisateur");
        assert_eq!(
            std::fs::read(config.profile_folder().unwrap().join("profile.json")).unwrap(),
            before,
            "l'utilisateur en place est intact"
        );
        let (name, settings, dictionary) = config.profile_snapshot(&made).unwrap();
        assert_eq!(name, "Copie de Léa");
        assert_eq!((settings.first_name.as_str(), settings.custom_modes.len(), settings.custom_commands.len()), ("Léa", 1, 1));
        assert_eq!(dictionary.words, vec!["Scriibe"]);
        assert_eq!(
            settings.lm_studio_url, "http://hote-secret:1234",
            "les réglages de l'ordinateur sont ceux de cet ordinateur, communs à tous"
        );
        let folder = dir.join("utilisateurs").join(&made);
        assert!(!folder.join("history.json").exists() && !folder.join("history-totals.json").exists(), "pas d'historique");
        let profile = std::fs::read_to_string(folder.join("profile.json")).unwrap();
        assert!(!profile.contains("typingWPM") && !profile.contains("keepDictationText") && !profile.contains("hote-secret"));
        // L'attente est consommée : un second « Importer » avec le même jeton est refusé.
        assert!(
            matches!(apply_pending(&config, &json!({"token": preview["token"]})), Err(Failure::Refused(m)) if m.contains("aucun fichier en attente"))
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_name_already_taken_is_numbered_and_the_file_name_or_a_default_is_used_when_the_page_gives_none() {
        let _one = one_at_a_time();
        let (dir, config, _, id) = filled("import-names");
        let (_, text) = prepare(&config, &id, Categories { identity: true, ..Categories::export_default() }, 0).unwrap();
        let path = write_file(&dir, &text);
        let token = |p: &Value| p["token"].clone();
        let first = read_path(&path);
        // Le nom du fichier (« Principal ») est pris : l'import le numérote au lieu de refuser ou d'écraser.
        let one = apply_pending(&config, &json!({"token": token(&first)})).unwrap();
        assert_eq!(one["created"]["name"], "Principal 2");
        let second = read_path(&path);
        let two = apply_pending(&config, &json!({"token": token(&second), "name": "  Principal  "})).unwrap();
        assert_eq!(two["created"]["name"], "Principal 3", "numéro qui ne s'accumule pas");
        assert_eq!(two["list"]["profiles"].as_array().unwrap().len(), 3);
        // Sans nom dans le fichier (export par défaut) ni dans la requête : « Utilisateur importé ».
        let (_, anonymous) = prepare(&config, &id, Categories::export_default(), 0).unwrap();
        let third = read_path(&write_file(&dir, &anonymous));
        assert_eq!(third["suggested_name"], "Utilisateur importé");
        let three = apply_pending(&config, &json!({"token": token(&third)})).unwrap();
        assert_eq!(three["created"]["name"], "Utilisateur importé");
        assert!(matches!(apply_pending(&config, &json!({"token": "autre"})), Err(Failure::Refused(_))));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_boxes_of_the_preview_decide_what_is_created() {
        let _one = one_at_a_time();
        let (dir, config, _, id) = filled("import-boxes");
        let (_, text) = prepare(&config, &id, Categories { identity: true, ..Categories::export_default() }, 0).unwrap();
        let preview = read_path(&write_file(&dir, &text));
        let applied = apply_pending(
            &config,
            &json!({"token": preview["token"], "name": "Sans modes", "include": {"identity": false, "modes": false, "words": false, "commands": false}}),
        )
        .unwrap();
        let (_, settings, dictionary) = config.profile_snapshot(applied["created"]["id"].as_str().unwrap()).unwrap();
        assert_eq!(settings.first_name, "", "prénom décoché");
        assert!(settings.custom_modes.is_empty() && settings.custom_commands.is_empty() && dictionary.words.is_empty());
        assert_eq!(dictionary.corrections.len(), 1, "les corrections, cochées, sont importées");
        // Tout décoché : refusé, rien n'est créé.
        let again = read_path(&write_file(&dir, &text));
        let none = json!({"identity": false, "preferences": false, "words": false, "corrections": false, "modes": false, "examples": false, "styles": false, "commands": false});
        let before = config.list_profiles().unwrap().len();
        assert!(
            matches!(apply_pending(&config, &json!({"token": again["token"], "include": none})), Err(Failure::Refused(m)) if m.contains("rien à importer"))
        );
        assert_eq!(config.list_profiles().unwrap().len(), before);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_refused_file_has_no_effect_and_forgets_the_previous_preview() {
        let _one = one_at_a_time();
        let (dir, config, _, id) = filled("import-refused");
        let (_, text) = prepare(&config, &id, Categories::export_default(), 0).unwrap();
        let good = read_path(&write_file(&dir, &text));
        assert_eq!(good["ok"], true);
        let before = config.list_profiles().unwrap().len();
        let bad =
            read_path(&write_file(&dir, "{\"format\": \"scriibe-export\", \"version\": 1, \"kind\": \"dictionary\", \"content\": {}}"));
        assert_eq!(bad["ok"], false);
        assert!(bad["error"].as_str().unwrap().contains("autre type d'export"));
        assert!(
            matches!(apply_pending(&config, &json!({"token": good["token"]})), Err(Failure::Refused(_))),
            "l'attente d'avant est oubliée"
        );
        assert_eq!(config.list_profiles().unwrap().len(), before, "rien n'a été créé");
        assert_eq!(read_path(&dir.join("absent.scriibe.json"))["ok"], false);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn an_export_of_a_user_read_back_gives_the_same_user_in_a_folder_of_its_own() {
        let _one = one_at_a_time();
        let (dir, config, _, id) = filled("roundtrip");
        let (_, text) = prepare(&config, &id, Categories { identity: true, ..Categories::export_default() }, 0).unwrap();
        let made = apply_pending(&config, &json!({"token": read_path(&write_file(&dir, &text))["token"], "name": "Clone"})).unwrap();
        let clone = made["created"]["id"].as_str().unwrap();
        let (_, a, da) = config.profile_snapshot(&id).unwrap();
        let (_, b, db) = config.profile_snapshot(clone).unwrap();
        assert_eq!(da, db);
        let essence = |s: &scriibe_core::settings::AppSettings| {
            (
                s.first_name.clone(),
                s.language,
                s.custom_modes
                    .iter()
                    .map(|m| {
                        (
                            m.name.clone(),
                            m.style.clone(),
                            m.examples.iter().map(|e| (e.dictation.clone(), e.expected.clone())).collect::<Vec<_>>(),
                        )
                    })
                    .collect::<Vec<_>>(),
                s.custom_commands.iter().map(|c| (c.name.clone(), c.phrases.clone(), c.shortcut.clone())).collect::<Vec<_>>(),
            )
        };
        assert_eq!(essence(&a), essence(&b));
        assert_ne!(a.custom_modes[0].id, b.custom_modes[0].id, "identifiants neufs");
        // Le clone s'exporte à son tour, avec son propre nom.
        let (_, again) = prepare(&config, clone, Categories { identity: true, ..Categories::export_default() }, 0).unwrap();
        assert!(again.contains("\"firstName\": \"Léa\"") && again.contains("\"name\": \"Clone\""));
        let _ = std::fs::remove_dir_all(dir);
    }
}
