//! Restauration d'une sauvegarde complète (`docs/SYSTEME.md` § Cadrage, question 3).
//!
//! Le processus vivant garde l'historique en mémoire et le réécrit à la première lecture : on ne peut donc pas remplacer les fichiers
//! pendant qu'il tourne. La restauration passe par un redémarrage : les fichiers sont d'abord écrits dans un dossier de travail
//! (`.restauration/`), avec une marque `ok` posée en dernier ; au lancement suivant, **avant toute lecture**, [`apply_pending`] copie
//! les fichiers actuels dans `sauvegarde-avant-import-<date>/` puis échange chaque fichier par renommage. Une interruption à n'importe
//! quel moment laisse soit l'ancien état, soit un état à finir au lancement suivant (l'opération se rejoue sans rien perdre).

use crate::migration::{PROFILE_FILE, USERS_DIR};
use scriibe_core::settings::is_profile_id;
use std::path::{Path, PathBuf};

/// Les fichiers que la sauvegarde remplace. `settings.json` est à la racine du dossier de données ; les autres sont ceux de
/// l'utilisateur : à la racine dans l'ancien rangement, dans `utilisateurs/<id>/` après la migration (étape 4b). `profile.json`
/// n'existe qu'après la migration.
pub const FILES: [&str; 5] = ["settings.json", PROFILE_FILE, "dictionary.json", "history.json", "history-totals.json"];
/// Le dossier de travail, dans le dossier de données.
pub const STAGING: &str = ".restauration";
const MARK: &str = "ok";
/// Deuxième ligne de la marque : l'utilisateur dont les fichiers sont remplacés (`utilisateur=<id>`).
const PROFILE_LINE: &str = "utilisateur=";

/// Prépare la restauration : écrit les fichiers (`(nom, texte)`, noms pris dans [`FILES`] seulement) dans le dossier de travail et pose la
/// marque avec le nom du dossier de copie de l'état actuel (`stamp` : « 20261113-142501 »). `profile` : l'identifiant de l'utilisateur
/// actif si le dossier est migré (ses fichiers vont alors dans `utilisateurs/<id>/`). Un ancien dossier de travail est écarté.
pub fn stage(folder: &Path, files: &[(&str, String)], stamp: &str, profile: Option<&str>) -> Result<(), String> {
    if profile.is_some_and(|id| !is_profile_id(id)) {
        return Err("utilisateur invalide".into());
    }
    let staging = folder.join(STAGING);
    if staging.exists() {
        std::fs::remove_dir_all(&staging).map_err(|e| format!("ancien dossier de travail : {e}"))?;
    }
    std::fs::create_dir_all(&staging).map_err(|e| format!("dossier de travail : {e}"))?;
    for (name, text) in files {
        if !FILES.contains(name) || (*name == PROFILE_FILE && profile.is_none()) {
            return Err(format!("fichier inattendu : {name}"));
        }
        std::fs::write(staging.join(name), text).map_err(|e| format!("écriture de {name} : {e}"))?;
    }
    let mut mark = format!("avant-import-{stamp}");
    if let Some(id) = profile {
        mark.push_str(&format!("\n{PROFILE_LINE}{id}"));
    }
    // La marque est posée en dernier : sans elle, le dossier de travail est ignoré (écriture interrompue).
    std::fs::write(staging.join(MARK), mark).map_err(|e| format!("marque : {e}"))
}

/// Applique une restauration préparée, s'il y en a une. Rend un message pour le journal. À appeler avant toute lecture du dossier
/// (et avant la migration : une restauration préparée dans un dossier migré vise l'utilisateur noté dans la marque).
pub fn apply_pending(folder: &Path) -> Option<String> {
    let staging = folder.join(STAGING);
    let mark = std::fs::read_to_string(staging.join(MARK)).ok()?;
    let mut lines = mark.lines().map(str::trim);
    let copy_name = lines.next().unwrap_or_default();
    let profile = lines.next().map(|l| l.strip_prefix(PROFILE_LINE).filter(|id| is_profile_id(id)));
    // Le nom du dossier de copie et l'utilisateur viennent d'un fichier : ils doivent avoir exactement la forme écrite par `stage`.
    if !copy_name.starts_with("avant-import-") || !copy_name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') || profile == Some(None)
    {
        let _ = std::fs::remove_dir_all(&staging);
        return Some("restauration: marque invalide, ignorée".to_string());
    }
    let user_folder = match profile.flatten() {
        Some(id) => {
            let dir = folder.join(USERS_DIR).join(id);
            let real_dir = std::fs::symlink_metadata(&dir).is_ok_and(|m| m.is_dir());
            if !real_dir || !dir.join(PROFILE_FILE).is_file() {
                let _ = std::fs::remove_dir_all(&staging);
                return Some(format!("restauration: l'utilisateur {id} n'existe plus, rien n'a changé ; recommence la restauration"));
            }
            dir
        }
        None => folder.to_path_buf(),
    };
    let target = |name: &str| if name == "settings.json" { folder.join(name) } else { user_folder.join(name) };
    let before = folder.join(copy_name);
    if let Err(e) = std::fs::create_dir_all(&before) {
        return Some(format!("restauration: dossier de copie impossible ({e}), rien n'a changé"));
    }
    // Copie de l'état actuel, une seule fois : si une reprise passe ici, le dossier existe déjà avec l'état d'origine.
    for name in FILES {
        let (current, copy) = (target(name), before.join(name));
        if current.is_file() && !copy.exists() {
            // Copie sous un nom temporaire puis renommage : une copie visible est toujours complète (un arrêt en pleine copie ne laisse
            // pas un fichier tronqué que la reprise prendrait pour l'état d'origine).
            let temp = before.join(format!(".{name}.tmp"));
            if let Err(e) = std::fs::copy(&current, &temp).and_then(|_| std::fs::rename(&temp, &copy)) {
                // Sans copie de l'état actuel, rien n'est remplacé (disque plein, antivirus…) : la restauration est abandonnée, à refaire.
                let _ = std::fs::remove_file(&temp);
                let _ = std::fs::remove_dir_all(&staging);
                return Some(format!("restauration: copie de {name} impossible ({e}), rien n'a changé ; recommence la restauration"));
            }
        }
    }
    let mut replaced = Vec::new();
    for name in FILES {
        let staged: PathBuf = staging.join(name);
        if staged.is_file() {
            match std::fs::rename(&staged, target(name)) {
                Ok(()) => replaced.push(name),
                Err(e) => {
                    return Some(format!("restauration: {name} n'a pas pu être remplacé ({e}) ; elle sera reprise au prochain lancement"))
                }
            }
        }
    }
    let _ = std::fs::remove_dir_all(&staging);
    Some(format!(
        "restauration: {} remplacé(s) ; état précédent copié dans {}",
        replaced.join(", "),
        before.file_name().and_then(|n| n.to_str()).unwrap_or("?")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("scriibe-restore-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn nothing_happens_without_the_mark() {
        let dir = scratch("sans-marque");
        std::fs::write(dir.join("settings.json"), "actuel").unwrap();
        std::fs::create_dir_all(dir.join(STAGING)).unwrap();
        std::fs::write(dir.join(STAGING).join("settings.json"), "neuf").unwrap();
        assert_eq!(apply_pending(&dir), None);
        assert_eq!(std::fs::read_to_string(dir.join("settings.json")).unwrap(), "actuel");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_current_state_is_copied_then_replaced_and_untouched_files_stay() {
        let dir = scratch("echange");
        std::fs::write(dir.join("settings.json"), "actuel").unwrap();
        std::fs::write(dir.join("history.json"), "[1]").unwrap();
        std::fs::write(dir.join("scriibe.log"), "journal").unwrap();
        stage(&dir, &[("settings.json", "neuf".into()), ("dictionary.json", "{}".into())], "20261113-142501", None).unwrap();
        let message = apply_pending(&dir).unwrap();
        assert!(message.contains("avant-import-20261113-142501"), "{message}");
        assert_eq!(std::fs::read_to_string(dir.join("settings.json")).unwrap(), "neuf");
        assert_eq!(std::fs::read_to_string(dir.join("dictionary.json")).unwrap(), "{}");
        assert_eq!(std::fs::read_to_string(dir.join("history.json")).unwrap(), "[1]", "un fichier absent de la sauvegarde reste");
        assert_eq!(std::fs::read_to_string(dir.join("scriibe.log")).unwrap(), "journal");
        let before = dir.join("avant-import-20261113-142501");
        assert_eq!(std::fs::read_to_string(before.join("settings.json")).unwrap(), "actuel");
        assert_eq!(std::fs::read_to_string(before.join("history.json")).unwrap(), "[1]");
        assert!(!dir.join(STAGING).exists(), "le dossier de travail est retiré");
        assert_eq!(apply_pending(&dir), None, "rien à refaire");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn an_interrupted_swap_is_resumed_without_losing_the_original_copy() {
        let dir = scratch("reprise");
        std::fs::write(dir.join("settings.json"), "origine").unwrap();
        stage(&dir, &[("settings.json", "neuf".into())], "20261113-000000", None).unwrap();
        apply_pending(&dir).unwrap();
        // Une reprise après interruption : l'état d'origine est déjà copié, un nouveau passage ne l'écrase pas.
        std::fs::write(dir.join("settings.json"), "milieu").unwrap();
        stage(&dir, &[("settings.json", "neuf2".into())], "20261113-000000", None).unwrap();
        apply_pending(&dir).unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("settings.json")).unwrap(), "neuf2");
        assert_eq!(std::fs::read_to_string(dir.join("avant-import-20261113-000000").join("settings.json")).unwrap(), "origine");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Si la copie de l'état actuel échoue, rien n'est remplacé et la restauration est abandonnée.
    #[cfg(unix)]
    #[test]
    fn a_failed_copy_of_the_current_state_stops_the_swap() {
        use std::os::unix::fs::PermissionsExt;
        // En tant que root, une copie n'échoue pas : le test n'a pas de sens.
        // SAFETY : geteuid n'a pas d'effet de bord.
        if unsafe { libc::geteuid() } == 0 {
            return;
        }
        let dir = scratch("copie-impossible");
        std::fs::write(dir.join("settings.json"), "actuel").unwrap();
        stage(&dir, &[("settings.json", "neuf".into())], "20261113-120000", None).unwrap();
        // Le dossier de copie existe mais n'est pas modifiable : la copie échoue.
        let before = dir.join("avant-import-20261113-120000");
        std::fs::create_dir_all(&before).unwrap();
        std::fs::set_permissions(&before, std::fs::Permissions::from_mode(0o555)).unwrap();
        let message = apply_pending(&dir).unwrap();
        std::fs::set_permissions(&before, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(message.contains("rien n'a changé"), "{message}");
        assert_eq!(std::fs::read_to_string(dir.join("settings.json")).unwrap(), "actuel", "l'état actuel est intact");
        assert!(!dir.join(STAGING).exists(), "la restauration est abandonnée, pas rejouée à chaque lancement");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn only_the_known_files_can_be_staged() {
        let dir = scratch("noms");
        assert!(stage(&dir, &[("../evil.json", "x".into())], "s", None).is_err());
        assert!(stage(&dir, &[("models", "x".into())], "s", None).is_err());
        assert!(stage(&dir, &[("profile.json", "{}".into())], "s", None).is_err(), "pas de profile.json dans l'ancien rangement");
        assert!(stage(&dir, &[("settings.json", "{}".into())], "s", Some("../x")).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Après la migration (étape 4b), les fichiers de l'utilisateur sont remplacés dans son dossier, et leur état actuel est copié avant.
    #[test]
    fn in_a_migrated_folder_the_user_files_are_replaced_in_the_user_folder() {
        let dir = scratch("utilisateur");
        let user = dir.join(USERS_DIR).join("AAAA");
        std::fs::create_dir_all(&user).unwrap();
        std::fs::write(dir.join("settings.json"), "racine").unwrap();
        std::fs::write(user.join(PROFILE_FILE), "profil").unwrap();
        std::fs::write(user.join("history.json"), "[1]").unwrap();
        let files = [("settings.json", "neuf".into()), (PROFILE_FILE, "profil neuf".into()), ("dictionary.json", "{}".into())];
        stage(&dir, &files, "20261114-100000", Some("AAAA")).unwrap();
        let message = apply_pending(&dir).unwrap();
        assert!(message.contains("profile.json"), "{message}");
        assert_eq!(std::fs::read_to_string(dir.join("settings.json")).unwrap(), "neuf");
        assert_eq!(std::fs::read_to_string(user.join(PROFILE_FILE)).unwrap(), "profil neuf");
        assert_eq!(std::fs::read_to_string(user.join("dictionary.json")).unwrap(), "{}");
        assert_eq!(std::fs::read_to_string(user.join("history.json")).unwrap(), "[1]", "un fichier absent de la sauvegarde reste");
        assert!(!dir.join("dictionary.json").exists(), "rien à la racine");
        let before = dir.join("avant-import-20261114-100000");
        assert_eq!(std::fs::read_to_string(before.join("settings.json")).unwrap(), "racine");
        assert_eq!(std::fs::read_to_string(before.join(PROFILE_FILE)).unwrap(), "profil");
        assert_eq!(std::fs::read_to_string(before.join("history.json")).unwrap(), "[1]");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Une marque qui vise un utilisateur disparu, ou un identifiant forgé, ne remplace rien.
    #[test]
    fn a_mark_for_a_missing_or_forged_user_changes_nothing() {
        let dir = scratch("utilisateur-absent");
        std::fs::write(dir.join("settings.json"), "racine").unwrap();
        stage(&dir, &[("settings.json", "neuf".into())], "20261114-100000", Some("BBBB")).unwrap();
        assert!(apply_pending(&dir).unwrap().contains("n'existe plus"));
        assert_eq!(std::fs::read_to_string(dir.join("settings.json")).unwrap(), "racine");
        stage(&dir, &[("settings.json", "neuf".into())], "20261114-100000", None).unwrap();
        std::fs::write(dir.join(STAGING).join(MARK), "avant-import-20261114-100000\nutilisateur=../..").unwrap();
        assert!(apply_pending(&dir).unwrap().contains("marque invalide"));
        assert_eq!(std::fs::read_to_string(dir.join("settings.json")).unwrap(), "racine");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Relecture de la migration, I3 : une copie de l'état actuel interrompue (temporaire laissé) ne compte pas comme faite.
    #[test]
    fn an_interrupted_copy_of_the_current_state_is_redone_in_full() {
        let dir = scratch("copie-tronquee");
        std::fs::write(dir.join("settings.json"), "origine complète").unwrap();
        stage(&dir, &[("settings.json", "neuf".into())], "20261114-120000", None).unwrap();
        let before = dir.join("avant-import-20261114-120000");
        std::fs::create_dir_all(&before).unwrap();
        std::fs::write(before.join(".settings.json.tmp"), "orig").unwrap();
        apply_pending(&dir).unwrap();
        assert_eq!(std::fs::read_to_string(before.join("settings.json")).unwrap(), "origine complète");
        assert!(!before.join(".settings.json.tmp").exists());
        assert_eq!(std::fs::read_to_string(dir.join("settings.json")).unwrap(), "neuf");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
