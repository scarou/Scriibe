//! Les utilisateurs rangés dans `utilisateurs/<id>/` (étape 4c de `docs/CADRAGE_PROFILS.md`, décision 3) : les lister, en créer un (vide ou
//! copié de l'utilisateur actif), le renommer, le supprimer. Un utilisateur est un dossier à identifiant opaque ; son nom affiché est la
//! clé `name` de son `profile.json` (jamais le nom du dossier : pas de caractère interdit sous Windows, pas de collision de majuscules sous macOS).
//!
//! Ce module ne connaît pas l'utilisateur actif ni le verrou des modifications : `FileConfig` (storage.rs) les ajoute (`list_profiles`,
//! `create_profile`, `rename_profile`, `delete_profile`, `switch_to_profile`).
//!
//! - **Création** (M8 de la relecture de la migration) : l'utilisateur est construit dans `utilisateurs/.creation-<id>/`, puis le dossier est renommé
//!   en `utilisateurs/<id>/`. Un arrêt brutal ne laisse donc jamais un dossier d'identifiant valide à moitié rempli, que la migration
//!   prendrait pour un utilisateur « réparable ».
//! - **Copie** : `profile.json` (réglages de l'utilisateur) et `dictionary.json`. **Ni l'historique ni les statistiques** (décision 3).
//! - **Suppression** : le dossier est déplacé dans `utilisateurs-supprimes/<id>-<date>/`, à la racine du dossier de données. Scriibe
//!   ne l'efface jamais.

use crate::migration::{self, DEFAULT_NAME, NAME_KEY, PROFILE_FILE, USERS_DIR};
use scriibe_core::settings::is_profile_id;
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

/// Où vont les utilisateurs supprimés (à la racine du dossier de données).
pub const DELETED_DIR: &str = "utilisateurs-supprimes";
/// Début du nom du dossier de travail d'une création (caché, jamais un identifiant valide : `is_profile_id` refuse le point).
const WORK_PREFIX: &str = ".creation-";
/// Longueur maximale d'un nom affiché, en caractères (comme le prénom de la salutation).
pub const MAX_NAME: usize = 40;

/// Un utilisateur.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileInfo {
    pub id: String,
    pub name: String,
    pub folder: PathBuf,
}

/// Pourquoi une opération sur les utilisateurs échoue (le message est rendu par le programme, traduit par la page).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileError {
    /// Le nom est vide, trop long ou contient des caractères de commande.
    BadName(String),
    /// Un autre utilisateur porte déjà ce nom (sans tenir compte des majuscules).
    NameTaken,
    /// Aucun utilisateur de cet identifiant.
    Unknown,
    /// L'utilisateur actif ne se supprime pas : on passe d'abord à un autre.
    Active,
    /// `profile.json` est illisible : rien n'est écrit pour ne pas l'abîmer.
    Unreadable,
    /// L'utilisateur actif n'est plus celui que l'appelant avait en tête (changé depuis, par la page, le menu de l'icône ou un autre onglet).
    Stale,
    /// Une erreur du disque.
    Io(String),
}

impl ProfileError {
    pub fn message(&self) -> String {
        match self {
            ProfileError::BadName(why) => why.clone(),
            ProfileError::NameTaken => "Un autre utilisateur porte déjà ce nom.".into(),
            ProfileError::Unknown => "Utilisateur introuvable.".into(),
            ProfileError::Active => "L'utilisateur actif ne se supprime pas : passe d'abord à un autre.".into(),
            ProfileError::Unreadable => "Le fichier profile.json de cet utilisateur est illisible : Scriibe n'y écrit rien.".into(),
            ProfileError::Stale => "L'utilisateur actif a changé : la page se recharge.".into(),
            ProfileError::Io(e) => format!("Impossible d'écrire sur le disque ({e})."),
        }
    }
}

impl From<std::io::Error> for ProfileError {
    fn from(e: std::io::Error) -> Self {
        ProfileError::Io(e.to_string())
    }
}

/// Un nom affiché propre : espaces de début et de fin retirés, pas vide, au plus [`MAX_NAME`] caractères, sans caractère de commande.
pub fn clean_name(raw: &str) -> Result<String, ProfileError> {
    let name = raw.trim();
    if name.is_empty() {
        return Err(ProfileError::BadName("Le nom est vide.".into()));
    }
    if name.chars().count() > MAX_NAME {
        return Err(ProfileError::BadName(format!("Le nom est trop long ({MAX_NAME} caractères au plus).")));
    }
    if name.chars().any(char::is_control) {
        return Err(ProfileError::BadName("Le nom contient un caractère interdit.".into()));
    }
    Ok(name.to_string())
}

fn same_name(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

/// Le nom sous lequel un utilisateur importé est créé : le sien s'il est libre, sinon numéroté (« Anne 2 »). Le nom numéroté tient toujours dans
/// [`MAX_NAME`] : un nom de 40 caractères déjà pris perd la fin de son radical pour faire place au numéro, au lieu d'être refusé.
pub fn free_user_name(taken: &[String], name: &str) -> String {
    let numbered = scriibe_core::import::free_name(taken, name);
    if numbered.chars().count() <= MAX_NAME {
        return numbered;
    }
    (2..)
        .find_map(|n| {
            let suffix = format!(" {n}");
            let room = MAX_NAME.saturating_sub(suffix.chars().count());
            let stem: String = name.chars().take(room).collect();
            let candidate = format!("{}{suffix}", stem.trim_end());
            (!taken.iter().any(|t| same_name(t, &candidate))).then_some(candidate)
        })
        .unwrap_or(numbered)
}

/// Le nom affiché d'un `profile.json` lu (« Principal » s'il n'en a pas).
fn name_in(profile: &Map<String, Value>) -> String {
    profile.get(NAME_KEY).and_then(Value::as_str).map(str::trim).filter(|n| !n.is_empty()).unwrap_or(DEFAULT_NAME).to_string()
}

/// Le `profile.json` d'un dossier d'utilisateur, comme objet. Illisible (ou pas un objet) : `None`.
fn read_profile(folder: &Path) -> Option<Map<String, Value>> {
    let text = std::fs::read_to_string(folder.join(PROFILE_FILE)).ok()?;
    match serde_json::from_str::<Value>(&text).ok()? {
        Value::Object(o) => Some(o),
        _ => None,
    }
}

/// Les utilisateurs de `utilisateurs/` (vrais dossiers à identifiant valide, avec un `profile.json`), triés par nom puis par identifiant.
/// Un `profile.json` illisible garde son utilisateur dans la liste (nom « Principal ») : il ne disparaît pas de l'écran.
pub fn list(data: &Path) -> std::io::Result<Vec<ProfileInfo>> {
    let dir = data.join(USERS_DIR);
    let entries = match migration::retry(|| std::fs::read_dir(&dir)) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut found = Vec::new();
    for entry in entries {
        let entry = entry?;
        let Some(id) = entry.file_name().to_str().map(str::to_string) else { continue };
        if !is_profile_id(&id) || !entry.file_type()?.is_dir() {
            continue;
        }
        let folder = entry.path();
        if !folder.join(PROFILE_FILE).is_file() {
            continue;
        }
        let name = read_profile(&folder).map_or_else(|| DEFAULT_NAME.to_string(), |p| name_in(&p));
        found.push(ProfileInfo { id, name, folder });
    }
    found.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()).then_with(|| a.id.cmp(&b.id)));
    Ok(found)
}

/// L'utilisateur de cet identifiant, s'il existe.
pub fn find(data: &Path, id: &str) -> std::io::Result<Option<ProfileInfo>> {
    Ok(list(data)?.into_iter().find(|p| p.id == id))
}

/// Efface ce qu'une création interrompue a laissé (dossiers de travail cachés de `utilisateurs/`).
fn remove_unfinished(data: &Path) {
    let Ok(entries) = std::fs::read_dir(data.join(USERS_DIR)) else { return };
    for entry in entries.flatten() {
        if entry.file_name().to_string_lossy().starts_with(WORK_PREFIX) {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

/// Un utilisateur de plus. `name` : le nom affiché (déjà propre ou non : il est contrôlé ici). `copy_from` : le dossier d'un utilisateur
/// dont on reprend les réglages et le dictionnaire (jamais l'historique ni les statistiques) ; `None` : un utilisateur vide.
pub fn create(data: &Path, name: &str, copy_from: Option<&Path>) -> Result<ProfileInfo, ProfileError> {
    // Le profil d'origine se lit avant tout : s'il est illisible, rien n'est créé (la copie serait un utilisateur vide).
    let profile = match copy_from {
        Some(source) => read_profile(source).ok_or(ProfileError::Unreadable)?,
        None => Map::new(),
    };
    // Un dictionnaire absent reste absent ; un dictionnaire qu'on n'arrive pas à lire arrête la copie (sinon : une copie sans dictionnaire, en silence).
    let dictionary = match copy_from {
        Some(source) => match std::fs::read_to_string(source.join("dictionary.json")) {
            Ok(text) => Some(text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.into()),
        },
        None => None,
    };
    create_with(data, name, profile, dictionary)
}

/// Un utilisateur de plus, avec ce contenu : les clés de son `profile.json` (le nom affiché y est mis ici) et, s'il y en a un, le texte de
/// son `dictionary.json`. Sert à la copie ([`create`]) et à l'import d'un utilisateur (étape 5). Il est construit dans un dossier de travail
/// puis renommé : un arrêt brutal ne laisse jamais un utilisateur à moitié rempli.
pub fn create_with(
    data: &Path,
    name: &str,
    mut profile: Map<String, Value>,
    dictionary: Option<String>,
) -> Result<ProfileInfo, ProfileError> {
    let name = clean_name(name)?;
    if list(data)?.iter().any(|p| same_name(&p.name, &name)) {
        return Err(ProfileError::NameTaken);
    }
    profile.insert(NAME_KEY.into(), Value::String(name.clone()));
    let users = data.join(USERS_DIR);
    std::fs::create_dir_all(&users)?;
    remove_unfinished(data);
    let id = scriibe_core::modes::new_id();
    let work = users.join(format!("{WORK_PREFIX}{id}"));
    let built = (|| -> std::io::Result<()> {
        std::fs::create_dir(&work)?;
        let text = serde_json::to_string_pretty(&Value::Object(profile)).map_err(std::io::Error::other)?;
        crate::storage::write_atomic(&work.join(PROFILE_FILE), &text)?;
        if let Some(text) = &dictionary {
            crate::storage::write_atomic(&work.join("dictionary.json"), text)?;
        }
        Ok(())
    })();
    let folder = users.join(&id);
    let done = built.and_then(|()| migration::retry(|| std::fs::rename(&work, &folder)));
    if let Err(e) = done {
        let _ = std::fs::remove_dir_all(&work);
        return Err(e.into());
    }
    Ok(ProfileInfo { id, name, folder })
}

/// Change le nom affiché d'un utilisateur : seule la clé `name` de son `profile.json` change, le reste est gardé tel quel. Un
/// `profile.json` illisible n'est pas réécrit.
pub fn rename(data: &Path, id: &str, name: &str) -> Result<ProfileInfo, ProfileError> {
    let name = clean_name(name)?;
    let all = list(data)?;
    let Some(target) = all.iter().find(|p| p.id == id) else { return Err(ProfileError::Unknown) };
    if all.iter().any(|p| p.id != id && same_name(&p.name, &name)) {
        return Err(ProfileError::NameTaken);
    }
    let mut profile = read_profile(&target.folder).ok_or(ProfileError::Unreadable)?;
    profile.insert(NAME_KEY.into(), Value::String(name.clone()));
    let text = serde_json::to_string_pretty(&Value::Object(profile)).map_err(std::io::Error::other)?;
    crate::storage::write_atomic(&target.folder.join(PROFILE_FILE), &text)?;
    Ok(ProfileInfo { id: id.to_string(), name, folder: target.folder.clone() })
}

/// Un nom de dossier libre dans `parent` : `<base>`, sinon `<base>-2`, `<base>-3`…
fn free_name(parent: &Path, base: &str) -> PathBuf {
    let first = parent.join(base);
    if !first.exists() {
        return first;
    }
    (2..).map(|n| parent.join(format!("{base}-{n}"))).find(|p| !p.exists()).unwrap_or(first)
}

/// Met un utilisateur de côté : son dossier est déplacé dans `utilisateurs-supprimes/<id>-<date>/`, jamais effacé. `active` : l'identifiant
/// de l'utilisateur actif, qu'on ne supprime pas. Rend le dossier où il se trouve maintenant.
pub fn delete(data: &Path, id: &str, active: Option<&str>, today: &str) -> Result<PathBuf, ProfileError> {
    let Some(target) = find(data, id)? else { return Err(ProfileError::Unknown) };
    if active.is_some_and(|a| a.eq_ignore_ascii_case(id)) {
        return Err(ProfileError::Active);
    }
    let deleted = data.join(DELETED_DIR);
    std::fs::create_dir_all(&deleted)?;
    let to = free_name(&deleted, &format!("{id}-{today}"));
    migration::retry(|| std::fs::rename(&target.folder, &to))?;
    Ok(to)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> PathBuf {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("scriibe-profiles-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(path: &Path, text: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    /// Un utilisateur de départ, comme la migration le laisse.
    fn first_user(data: &Path, name: &str) -> ProfileInfo {
        let id = "AAAA-1111".to_string();
        let folder = data.join(USERS_DIR).join(&id);
        write(&folder.join(PROFILE_FILE), &format!(r#"{{"name": "{name}", "firstName": "Anne", "language": "en"}}"#));
        write(&folder.join("dictionary.json"), r#"{"words": ["Scriibe"], "corrections": []}"#);
        write(&folder.join("history.json"), r#"{"history": [{"id": "x"}]}"#);
        write(&folder.join("history-totals.json"), r#"{"days": []}"#);
        ProfileInfo { id, name: name.into(), folder }
    }

    #[test]
    fn names_are_trimmed_and_refused_when_empty_long_or_with_control_characters() {
        assert_eq!(clean_name("  Anne  ").unwrap(), "Anne");
        assert!(matches!(clean_name("   "), Err(ProfileError::BadName(_))));
        assert!(matches!(clean_name(&"a".repeat(41)), Err(ProfileError::BadName(_))));
        assert_eq!(clean_name(&"é".repeat(40)).unwrap().chars().count(), 40, "40 caractères, pas 40 octets");
        assert!(matches!(clean_name("a\nb"), Err(ProfileError::BadName(_))));
        assert!(matches!(clean_name("a\u{7}b"), Err(ProfileError::BadName(_))));
    }

    #[test]
    fn the_list_has_real_user_folders_only_sorted_by_name() {
        let data = data();
        let users = data.join(USERS_DIR);
        write(&users.join("B-2").join(PROFILE_FILE), r#"{"name": "bruno"}"#);
        write(&users.join("A-1").join(PROFILE_FILE), r#"{"name": "Zoé"}"#);
        write(&users.join("C-3").join(PROFILE_FILE), r#"{"firstName": "sans nom"}"#);
        write(&users.join(".migration-X").join(PROFILE_FILE), r#"{"name": "caché"}"#);
        write(&users.join(".creation-Y").join(PROFILE_FILE), r#"{"name": "caché aussi"}"#);
        write(&users.join("sans profile").join("dictionary.json"), "{}");
        write(&users.join("D 4").join(PROFILE_FILE), r#"{"name": "identifiant refusé"}"#);
        write(&users.join("E-5").join(PROFILE_FILE), "pas du json");
        write(&users.join("fichier.txt"), "x");
        let names: Vec<(String, String)> = list(&data).unwrap().into_iter().map(|p| (p.id, p.name)).collect();
        assert_eq!(
            names,
            vec![
                ("B-2".into(), "bruno".into()),
                ("C-3".into(), "Principal".into()),
                ("E-5".into(), "Principal".into()),
                ("A-1".into(), "Zoé".into()),
            ],
            "ni dossier caché, ni identifiant refusé, ni dossier sans profil ; un nom absent ou un profil illisible donne « Principal »"
        );
        assert!(list(&data.join("rien")).unwrap().is_empty(), "pas de dossier utilisateurs : liste vide, pas d'erreur");
    }

    #[test]
    fn an_empty_user_has_only_a_name() {
        let data = data();
        first_user(&data, "Anne");
        let made = create(&data, "  Bruno ", None).unwrap();
        assert_eq!(made.name, "Bruno");
        assert!(is_profile_id(&made.id));
        assert_eq!(
            std::fs::read_to_string(made.folder.join(PROFILE_FILE)).unwrap().replace(char::is_whitespace, ""),
            r#"{"name":"Bruno"}"#
        );
        assert!(!made.folder.join("dictionary.json").exists() && !made.folder.join("history.json").exists());
        assert_eq!(list(&data).unwrap().len(), 2);
    }

    #[test]
    fn a_copy_takes_the_settings_and_the_dictionary_but_never_the_history_or_the_statistics() {
        let data = data();
        let anne = first_user(&data, "Anne");
        let copy = create(&data, "Copie d'Anne", Some(&anne.folder)).unwrap();
        let profile: Value = serde_json::from_str(&std::fs::read_to_string(copy.folder.join(PROFILE_FILE)).unwrap()).unwrap();
        assert_eq!(profile["name"], "Copie d'Anne", "le nom est celui qu'on a donné, pas celui de la source");
        assert_eq!((profile["firstName"].as_str(), profile["language"].as_str()), (Some("Anne"), Some("en")));
        assert_eq!(
            std::fs::read_to_string(copy.folder.join("dictionary.json")).unwrap(),
            std::fs::read_to_string(anne.folder.join("dictionary.json")).unwrap()
        );
        assert!(!copy.folder.join("history.json").exists(), "pas d'historique");
        assert!(!copy.folder.join("history-totals.json").exists(), "pas de statistiques");
        assert!(anne.folder.join("history.json").is_file(), "la source est intacte");
    }

    #[test]
    fn a_name_already_taken_is_refused_without_regard_to_case() {
        let data = data();
        first_user(&data, "Anne");
        assert_eq!(create(&data, "anne", None), Err(ProfileError::NameTaken));
        assert_eq!(create(&data, "ANNE ", None), Err(ProfileError::NameTaken));
        assert!(matches!(create(&data, "", None), Err(ProfileError::BadName(_))));
        assert_eq!(list(&data).unwrap().len(), 1, "rien n'a été créé");
    }

    #[test]
    fn a_copy_of_an_unreadable_profile_creates_nothing() {
        let data = data();
        let anne = first_user(&data, "Anne");
        write(&anne.folder.join(PROFILE_FILE), "pas du json");
        assert_eq!(create(&data, "Copie", Some(&anne.folder)), Err(ProfileError::Unreadable));
        assert_eq!(list(&data).unwrap().len(), 1);
    }

    #[test]
    fn creation_works_in_a_hidden_folder_and_leftovers_are_swept() {
        let data = data();
        first_user(&data, "Anne");
        let leftover = data.join(USERS_DIR).join(".creation-OLD");
        write(&leftover.join(PROFILE_FILE), r#"{"name":"à moitié"}"#);
        let made = create(&data, "Bruno", None).unwrap();
        assert!(!leftover.exists(), "le dossier de travail d'une création interrompue est effacé");
        let folders: Vec<String> =
            std::fs::read_dir(data.join(USERS_DIR)).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
        assert!(folders.contains(&made.id) && folders.iter().all(|f| !f.starts_with(WORK_PREFIX)), "{folders:?}");
    }

    #[test]
    fn renaming_changes_the_name_only_and_keeps_every_other_key() {
        let data = data();
        let anne = first_user(&data, "Anne");
        let renamed = rename(&data, &anne.id, " Annette ").unwrap();
        assert_eq!(renamed.name, "Annette");
        let profile: Value = serde_json::from_str(&std::fs::read_to_string(anne.folder.join(PROFILE_FILE)).unwrap()).unwrap();
        assert_eq!(
            (profile["name"].as_str(), profile["firstName"].as_str(), profile["language"].as_str()),
            (Some("Annette"), Some("Anne"), Some("en"))
        );
        assert!(anne.folder.join("history.json").is_file());
        // Le même nom pour soi-même (changement de majuscules) est permis ; celui d'un autre non.
        let bruno = create(&data, "Bruno", None).unwrap();
        assert!(rename(&data, &anne.id, "ANNETTE").is_ok());
        assert_eq!(rename(&data, &bruno.id, "annette"), Err(ProfileError::NameTaken));
        assert_eq!(rename(&data, "inconnu", "X"), Err(ProfileError::Unknown));
    }

    #[test]
    fn an_unreadable_profile_is_not_rewritten_by_a_rename() {
        let data = data();
        let anne = first_user(&data, "Anne");
        write(&anne.folder.join(PROFILE_FILE), "pas du json");
        assert_eq!(rename(&data, &anne.id, "Annette"), Err(ProfileError::Unreadable));
        assert_eq!(std::fs::read_to_string(anne.folder.join(PROFILE_FILE)).unwrap(), "pas du json");
    }

    #[test]
    fn a_deleted_user_is_moved_aside_never_erased_and_the_active_one_stays() {
        let data = data();
        let anne = first_user(&data, "Anne");
        let bruno = create(&data, "Bruno", Some(&anne.folder)).unwrap();
        write(&bruno.folder.join("history.json"), r#"{"history": [{"id": "b"}]}"#);
        assert_eq!(delete(&data, &anne.id, Some(&anne.id), "2026-11-14"), Err(ProfileError::Active));
        assert_eq!(
            delete(&data, &anne.id, Some(&anne.id.to_lowercase()), "2026-11-14"),
            Err(ProfileError::Active),
            "sans tenir compte des majuscules"
        );
        assert!(anne.folder.is_dir());
        let moved = delete(&data, &bruno.id, Some(&anne.id), "2026-11-14").unwrap();
        assert_eq!(moved, data.join(DELETED_DIR).join(format!("{}-2026-11-14", bruno.id)));
        assert!(moved.join("history.json").is_file() && moved.join(PROFILE_FILE).is_file(), "tout y est, historique compris");
        assert!(!bruno.folder.exists());
        assert_eq!(list(&data).unwrap().len(), 1);
        assert_eq!(delete(&data, &bruno.id, Some(&anne.id), "2026-11-14"), Err(ProfileError::Unknown));
    }

    #[test]
    fn a_second_deletion_the_same_day_never_overwrites_the_first() {
        let data = data();
        let anne = first_user(&data, "Anne");
        let bruno = create(&data, "Bruno", None).unwrap();
        let first = delete(&data, &bruno.id, Some(&anne.id), "2026-11-14").unwrap();
        // Même identifiant remis à la main puis supprimé de nouveau le même jour.
        std::fs::create_dir_all(data.join(USERS_DIR).join(&bruno.id)).unwrap();
        write(&data.join(USERS_DIR).join(&bruno.id).join(PROFILE_FILE), r#"{"name":"Bruno bis"}"#);
        let second = delete(&data, &bruno.id, Some(&anne.id), "2026-11-14").unwrap();
        assert_ne!(first, second);
        assert!(first.is_dir() && second.is_dir());
    }
}

#[cfg(test)]
#[path = "profiles_config_tests.rs"]
mod config_tests;
