//! Les utilisateurs vus de la page et du menu de l'icône (étape 4c de `docs/CADRAGE_PROFILS.md`, décision 3) : la liste, créer, copier,
//! renommer, supprimer, changer d'utilisateur, et le garde-fou « 409 » des requêtes qui écrivent chez l'utilisateur actif.
//!
//! **Le garde-fou.** La page (et le menu de l'icône) envoient l'identifiant de l'utilisateur qu'ils croient actif avec chaque requête qui
//! écrit chez lui (dictionnaire, réglages de l'utilisateur dont `customModes` et `customCommands`, choix d'un mode, suppression de
//! l'historique, imports). Si ce n'est plus l'utilisateur actif (changé depuis par le menu de l'icône ou un autre onglet), le serveur répond
//! 409 avec `"stale": true` et n'écrit rien : la page se recharge. Le contrôle et l'écriture se font sous le même verrou (`FileConfig::lock_for`),
//! si bien qu'un changement d'utilisateur ne peut pas s'intercaler entre les deux.

use scriibe_platform::profiles::{ProfileError, ProfileInfo, MAX_NAME};
use scriibe_platform::storage::{FileConfig, FileHistory};
use serde_json::{json, Value};
use std::sync::MutexGuard;

/// Pourquoi une requête qui écrit chez un utilisateur est refusée.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Failure {
    /// L'utilisateur actif n'est plus celui de l'appelant : 409, la page se recharge.
    Stale,
    /// Refusée avec un message (400).
    Refused(String),
    /// Échec d'écriture sur le disque (500).
    Failed(String),
}

impl From<String> for Failure {
    fn from(message: String) -> Self {
        Failure::Refused(message)
    }
}

impl From<ProfileError> for Failure {
    fn from(e: ProfileError) -> Self {
        match e {
            ProfileError::Stale => Failure::Stale,
            other => Failure::Refused(other.message()),
        }
    }
}

/// L'identifiant de l'utilisateur que l'appelant dit actif : `None` si la requête n'en dit rien (clé absente), `Some(None)` s'il dit
/// « aucun » (`null`), `Some(Some(id))` sinon.
pub(crate) fn given(input: &Value) -> Option<Option<&str>> {
    input.as_object()?.get("profile").map(Value::as_str)
}

/// Prend le verrou des modifications en contrôlant l'utilisateur actif quand la requête dit lequel elle croit actif.
pub(crate) fn guard<'a>(config: &'a FileConfig, given: Option<Option<&str>>) -> Result<MutexGuard<'a, ()>, Failure> {
    match given {
        Some(id) => Ok(config.lock_for(id)?),
        None => Ok(config.lock_edit()),
    }
}

/// Les utilisateurs et l'utilisateur actif, pour la liste des Réglages.
pub(crate) fn view(config: &FileConfig) -> Value {
    match config.list_profiles() {
        Ok(list) => json!({
            "ok": true,
            "active": config.active_profile_id(),
            "profiles": list.iter().map(|p| json!({"id": p.id, "name": p.name})).collect::<Vec<_>>(),
            "max_name": MAX_NAME,
        }),
        Err(e) => json!({"ok": false, "error": format!("Impossible de lire la liste des utilisateurs ({e}).")}),
    }
}

/// Crée un utilisateur, vide ou copié de l'utilisateur actif (`copy: true`). Il ne devient pas actif.
pub(crate) fn create(config: &FileConfig, input: &Value) -> Result<Value, Failure> {
    let name = input["name"].as_str().unwrap_or("");
    let made =
        if input["copy"].as_bool() == Some(true) { config.copy_profile(given(input), name)? } else { config.create_profile(name)? };
    Ok(json!({"ok": true, "created": {"id": made.id, "name": made.name}, "list": view(config)}))
}

pub(crate) fn rename(config: &FileConfig, input: &Value) -> Result<Value, Failure> {
    let renamed = config.rename_profile(input["id"].as_str().unwrap_or(""), input["name"].as_str().unwrap_or(""))?;
    Ok(json!({"ok": true, "renamed": {"id": renamed.id, "name": renamed.name}, "list": view(config)}))
}

/// Met un utilisateur de côté, sur confirmation (`confirmed: true`) : son dossier va dans `utilisateurs-supprimes/`, jamais effacé.
pub(crate) fn delete(config: &FileConfig, input: &Value) -> Result<Value, Failure> {
    if input["confirmed"].as_bool() != Some(true) {
        return Err(Failure::Refused("confirmation manquante".into()));
    }
    let moved = config.delete_profile(input["id"].as_str().unwrap_or(""))?;
    let kept = moved.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    Ok(json!({"ok": true, "kept_in": format!("{}/{kept}", scriibe_platform::profiles::DELETED_DIR), "list": view(config)}))
}

/// Ce qui attend un accord (un aperçu d'import ou de restauration) est lié à l'utilisateur qui l'a vu : un changement d'utilisateur l'abandonne.
pub(crate) fn cancel_pending() {
    super::dictionary_io::cancel();
    super::commands_io::cancel();
    super::import::cancel();
    super::backup::cancel();
}

/// Passe à un autre utilisateur : `activeProfile` est écrit, les réglages et le dictionnaire sont relus dans le dossier de l'utilisateur et
/// l'historique change de dossier (sous le même verrou). Refusé pendant une dictée (`busy`) : elle serait enregistrée chez l'autre.
/// Le prompt change (mots, mode, genre) : le préchargement du modèle se relance tout seul (`preload_signature`, avec son délai de stabilisation).
pub(crate) fn switch(config: &FileConfig, history: &FileHistory, busy: bool, id: &str) -> Result<ProfileInfo, Failure> {
    if busy {
        return Err(Failure::Refused("Une dictée est en cours : attends qu'elle soit finie pour changer d'utilisateur.".into()));
    }
    // La configuration passe au nouvel utilisateur depuis le verrou de l'historique : aucune lecture de l'historique ne tombe entre les deux.
    let (target, ()) =
        config.switch_to_profile_with(id, |folder, switch_config| history.switch_with(folder.to_path_buf(), switch_config))?;
    cancel_pending();
    Ok(target)
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use chrono::Utc;
    use scriibe_core::history::Dictation;
    use scriibe_core::settings::AppSettings;
    use scriibe_platform::storage::HistorySink;
    use std::path::PathBuf;
    use std::sync::Arc;

    pub(in crate::web) fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("scriibe-users-web-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Un dossier de données migré (un utilisateur « Principal »), sa configuration et son historique, comme au lancement.
    pub(in crate::web) fn world(name: &str) -> (PathBuf, Arc<FileConfig>, Arc<FileHistory>) {
        let dir = scratch(name);
        let report = scriibe_platform::migration::run(&dir);
        let folder = report.profile.expect("un utilisateur après la migration");
        let config = Arc::new(FileConfig::new(dir.clone()));
        config.switch_profile(Some(folder.clone()));
        let history = Arc::new(FileHistory::open(folder, AppSettings::default));
        (dir, config, history)
    }

    fn id_of(created: &Value) -> String {
        created["created"]["id"].as_str().unwrap().to_string()
    }

    #[test]
    fn the_user_a_request_has_in_mind_is_told_apart_from_none() {
        assert_eq!(given(&json!({"words": []})), None, "clé absente : la requête n'en dit rien");
        assert_eq!(given(&json!({"profile": null})), Some(None), "null : « aucun utilisateur »");
        assert_eq!(given(&json!({"profile": "AAAA"})), Some(Some("AAAA")));
        assert_eq!(given(&json!([1])), None);
    }

    #[test]
    fn the_guard_checks_the_user_only_when_the_request_names_one() {
        let (dir, config, _) = world("guard");
        let active = config.active_profile_id().unwrap();
        assert!(guard(&config, None).is_ok(), "sans utilisateur dans la requête : verrou pris, pas de contrôle");
        assert!(guard(&config, Some(Some(&active))).is_ok());
        assert_eq!(guard(&config, Some(Some("AUTRE"))).err(), Some(Failure::Stale));
        assert_eq!(guard(&config, Some(None)).err(), Some(Failure::Stale), "la page croyait n'avoir aucun utilisateur");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_list_has_the_users_and_the_active_one() {
        let (dir, config, _) = world("view");
        create(&config, &json!({"name": "Bruno"})).unwrap();
        let view = view(&config);
        assert_eq!(view["ok"], true);
        assert_eq!(view["active"], config.active_profile_id().unwrap().as_str());
        let names: Vec<&str> = view["profiles"].as_array().unwrap().iter().map(|p| p["name"].as_str().unwrap()).collect();
        assert_eq!(names, vec!["Bruno", "Principal"]);
        assert_eq!(view["max_name"], 40);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn creating_gives_an_empty_user_or_a_copy_and_a_stale_page_cannot_copy() {
        let (dir, config, _) = world("create");
        let active = config.active_profile_id().unwrap();
        let mut settings = config.settings();
        settings.first_name = "Alice".into();
        config.save_settings(&settings).unwrap();
        let empty = create(&config, &json!({"name": " Bruno "})).unwrap();
        assert_eq!(empty["created"]["name"], "Bruno");
        assert_eq!(empty["list"]["profiles"].as_array().unwrap().len(), 2, "la réponse porte la liste à jour");
        let copy = create(&config, &json!({"name": "Copie", "copy": true, "profile": active})).unwrap();
        let folder = dir.join("utilisateurs").join(id_of(&copy));
        assert_eq!(
            serde_json::from_str::<Value>(&std::fs::read_to_string(folder.join("profile.json")).unwrap()).unwrap()["firstName"],
            "Alice"
        );
        assert_eq!(create(&config, &json!({"name": "Autre", "copy": true, "profile": "AUTRE"})), Err(Failure::Stale));
        // Une requête sans clé `profile` n'a pas d'avis sur l'utilisateur actif : pas de contrôle (comme les autres routes), la copie se fait.
        assert!(create(&config, &json!({"name": "Sans clé", "copy": true})).is_ok());
        assert!(matches!(create(&config, &json!({"name": "copie"})), Err(Failure::Refused(m)) if m.contains("déjà ce nom")));
        assert!(matches!(create(&config, &json!({"name": "  "})), Err(Failure::Refused(m)) if m.contains("vide")));
        assert_eq!(config.active_profile_id().as_deref(), Some(active.as_str()), "créer ne change pas d'utilisateur");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn renaming_and_deleting_answer_with_the_new_list_and_deleting_needs_a_confirmation() {
        let (dir, config, _) = world("rename-delete");
        let active = config.active_profile_id().unwrap();
        let bruno = id_of(&create(&config, &json!({"name": "Bruno"})).unwrap());
        let renamed = rename(&config, &json!({"id": bruno, "name": "Bruno Z"})).unwrap();
        assert_eq!(renamed["renamed"]["name"], "Bruno Z");
        assert!(matches!(delete(&config, &json!({"id": bruno})), Err(Failure::Refused(m)) if m == "confirmation manquante"));
        assert!(matches!(delete(&config, &json!({"id": active, "confirmed": true})), Err(Failure::Refused(m)) if m.contains("actif")));
        let deleted = delete(&config, &json!({"id": bruno, "confirmed": true})).unwrap();
        assert!(deleted["kept_in"].as_str().unwrap().starts_with("utilisateurs-supprimes/"));
        assert_eq!(deleted["list"]["profiles"].as_array().unwrap().len(), 1);
        assert!(dir.join("utilisateurs-supprimes").is_dir());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn switching_moves_the_settings_the_dictionary_and_the_history_to_the_other_user() {
        let (dir, config, history) = world("switch");
        let alice = config.active_profile_id().unwrap();
        config.save_dictionary(&scriibe_core::dictionary::UserDictionary { words: vec!["alice".into()], corrections: vec![] }).unwrap();
        history.record(Dictation::new(Utc::now(), "Chrome", "un", "un", 12, 4.0, 0));
        assert_eq!(history.dictation_count(), 1);
        let bruno = id_of(&create(&config, &json!({"name": "Bruno"})).unwrap());
        let user = switch(&config, &history, false, &bruno).unwrap();
        assert_eq!(user.name, "Bruno");
        assert_eq!(config.active_profile_id().as_deref(), Some(bruno.as_str()));
        assert!(config.dictionary().words.is_empty(), "le dictionnaire de Bruno");
        assert_eq!(history.dictation_count(), 0, "l'historique de Bruno : vide, pas celui d'Alice");
        history.record(Dictation::new(Utc::now(), "Notes", "deux", "deux", 30, 9.0, 1));
        switch(&config, &history, false, &alice).unwrap();
        assert_eq!(history.dictation_count(), 1, "le retour trouve l'historique d'Alice, sans la dictée de Bruno");
        assert_eq!(config.dictionary().words, vec!["alice"]);
        assert_eq!(switch(&config, &history, false, "INCONNU").err(), Some(Failure::Refused("Utilisateur introuvable.".into())));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn switching_is_refused_during_a_dictation_and_changes_nothing() {
        let (dir, config, history) = world("busy");
        let alice = config.active_profile_id().unwrap();
        let bruno = id_of(&create(&config, &json!({"name": "Bruno"})).unwrap());
        let refused = switch(&config, &history, true, &bruno).err();
        assert!(matches!(refused, Some(Failure::Refused(m)) if m.contains("dictée est en cours")));
        assert_eq!(config.active_profile_id().as_deref(), Some(alice.as_str()));
        let _ = std::fs::remove_dir_all(dir);
    }
}
