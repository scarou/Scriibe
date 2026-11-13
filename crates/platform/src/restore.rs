//! Restauration d'une sauvegarde complète (`docs/SYSTEME.md` § Cadrage, question 3).
//!
//! Le processus vivant garde l'historique en mémoire et le réécrit à la première lecture : on ne peut donc pas remplacer les fichiers
//! pendant qu'il tourne. La restauration passe par un redémarrage : les fichiers sont d'abord écrits dans un dossier de travail
//! (`.restauration/`), avec une marque `ok` posée en dernier ; au lancement suivant, **avant toute lecture**, [`apply_pending`] copie
//! les fichiers actuels dans `sauvegarde-avant-import-<date>/` puis échange chaque fichier par renommage. Une interruption à n'importe
//! quel moment laisse soit l'ancien état, soit un état à finir au lancement suivant (l'opération se rejoue sans rien perdre).

use std::path::{Path, PathBuf};

/// Les fichiers que la sauvegarde remplace, dans le dossier de données.
pub const FILES: [&str; 4] = ["settings.json", "dictionary.json", "history.json", "history-totals.json"];
/// Le dossier de travail, dans le dossier de données.
pub const STAGING: &str = ".restauration";
const MARK: &str = "ok";

/// Prépare la restauration : écrit les fichiers (`(nom, texte)`, noms pris dans [`FILES`] seulement) dans le dossier de travail et pose la
/// marque avec le nom du dossier de copie de l'état actuel (`stamp` : « 20261113-142501 »). Un ancien dossier de travail est écarté.
pub fn stage(folder: &Path, files: &[(&str, String)], stamp: &str) -> Result<(), String> {
    let staging = folder.join(STAGING);
    if staging.exists() {
        std::fs::remove_dir_all(&staging).map_err(|e| format!("ancien dossier de travail : {e}"))?;
    }
    std::fs::create_dir_all(&staging).map_err(|e| format!("dossier de travail : {e}"))?;
    for (name, text) in files {
        if !FILES.contains(name) {
            return Err(format!("fichier inattendu : {name}"));
        }
        std::fs::write(staging.join(name), text).map_err(|e| format!("écriture de {name} : {e}"))?;
    }
    // La marque est posée en dernier : sans elle, le dossier de travail est ignoré (écriture interrompue).
    std::fs::write(staging.join(MARK), format!("avant-import-{stamp}")).map_err(|e| format!("marque : {e}"))
}

/// Applique une restauration préparée, s'il y en a une. Rend un message pour le journal. À appeler avant toute lecture du dossier.
pub fn apply_pending(folder: &Path) -> Option<String> {
    let staging = folder.join(STAGING);
    let mark = std::fs::read_to_string(staging.join(MARK)).ok()?;
    let mark = mark.trim();
    // Le nom du dossier de copie vient d'un fichier : il doit avoir exactement la forme écrite par `stage`.
    if !mark.starts_with("avant-import-") || !mark.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        let _ = std::fs::remove_dir_all(&staging);
        return Some("restauration: marque invalide, ignorée".to_string());
    }
    let before = folder.join(mark);
    if let Err(e) = std::fs::create_dir_all(&before) {
        return Some(format!("restauration: dossier de copie impossible ({e}), rien n'a changé"));
    }
    // Copie de l'état actuel, une seule fois : si une reprise passe ici, le dossier existe déjà avec l'état d'origine.
    for name in FILES {
        let (current, copy) = (folder.join(name), before.join(name));
        if current.is_file() && !copy.exists() {
            if let Err(e) = std::fs::copy(&current, &copy) {
                // Sans copie de l'état actuel, rien n'est remplacé (disque plein, antivirus…) : la restauration est abandonnée, à refaire.
                let _ = std::fs::remove_file(&copy);
                let _ = std::fs::remove_dir_all(&staging);
                return Some(format!("restauration: copie de {name} impossible ({e}), rien n'a changé ; recommence la restauration"));
            }
        }
    }
    let mut replaced = Vec::new();
    for name in FILES {
        let staged: PathBuf = staging.join(name);
        if staged.is_file() {
            match std::fs::rename(&staged, folder.join(name)) {
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
        stage(&dir, &[("settings.json", "neuf".into()), ("dictionary.json", "{}".into())], "20261113-142501").unwrap();
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
        stage(&dir, &[("settings.json", "neuf".into())], "20261113-000000").unwrap();
        apply_pending(&dir).unwrap();
        // Une reprise après interruption : l'état d'origine est déjà copié, un nouveau passage ne l'écrase pas.
        std::fs::write(dir.join("settings.json"), "milieu").unwrap();
        stage(&dir, &[("settings.json", "neuf2".into())], "20261113-000000").unwrap();
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
        stage(&dir, &[("settings.json", "neuf".into())], "20261113-120000").unwrap();
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
    fn only_the_four_known_files_can_be_staged() {
        let dir = scratch("noms");
        assert!(stage(&dir, &[("../evil.json", "x".into())], "s").is_err());
        assert!(stage(&dir, &[("models", "x".into())], "s").is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
