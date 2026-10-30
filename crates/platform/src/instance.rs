//! Une seule Scriibe par dossier de données : deux instances se disputeraient la touche, l'icône et le serveur Whisper (S09 : Whisper
//! ne répondait plus avec deux Scriibe lancées).
//!
//! Le verrou est un fichier ouvert sans partage (Windows) : le système le libère à la fin du processus, même après un plantage. La
//! première instance écrit son adresse locale dans `instance.url` ; la suivante la lit pour lui demander de montrer sa fenêtre.
//! Pas encore de verrou sur les autres systèmes : toute instance y est « première ».

use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const LOCK_FILE: &str = "instance.lock";
const URL_FILE: &str = "instance.url";

/// Ce que `acquire` a trouvé.
pub enum Instance {
    /// Personne d'autre : cette instance peut démarrer. À garder vivante jusqu'à la fin du programme.
    First(InstanceLock),
    /// Une autre Scriibe tourne déjà sur ce dossier de données.
    Running,
}

pub struct InstanceLock {
    _file: Option<File>,
    url_path: PathBuf,
}

impl InstanceLock {
    /// Annonce l'adresse de l'interface aux instances suivantes.
    pub fn publish(&self, url: &str) {
        let _ = std::fs::write(&self.url_path, url);
    }
}

impl Drop for InstanceLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.url_path);
    }
}

#[cfg(windows)]
fn lock(folder: &Path) -> Option<Result<File, ()>> {
    use std::os::windows::fs::OpenOptionsExt;
    // `share_mode(0)` : personne d'autre ne peut ouvrir ce fichier tant que celui-ci est ouvert (ERROR_SHARING_VIOLATION).
    match File::options().write(true).create(true).truncate(false).share_mode(0).open(folder.join(LOCK_FILE)) {
        Ok(file) => Some(Ok(file)),
        Err(e) if e.raw_os_error() == Some(32) => Some(Err(())),
        // Dossier illisible ou autre erreur : mieux vaut démarrer sans verrou que de ne pas démarrer.
        Err(_) => None,
    }
}

#[cfg(not(windows))]
fn lock(_: &Path) -> Option<Result<File, ()>> {
    None
}

/// Prend le verrou de ce dossier de données, ou dit qu'une autre instance l'a déjà.
pub fn acquire(folder: &Path) -> Instance {
    let first = |file| {
        // L'adresse d'une instance arrêtée sans nettoyer (`process::exit`) ne doit pas être prise pour celle-ci.
        let url_path = folder.join(URL_FILE);
        let _ = std::fs::remove_file(&url_path);
        Instance::First(InstanceLock { _file: file, url_path })
    };
    match lock(folder) {
        Some(Err(())) => Instance::Running,
        Some(Ok(file)) => first(Some(file)),
        None => first(None),
    }
}

/// L'adresse annoncée par l'instance en cours ; attend jusqu'à `wait` si elle démarre encore.
pub fn running_url(folder: &Path, wait: Duration) -> Option<String> {
    let start = Instant::now();
    loop {
        if let Ok(url) = std::fs::read_to_string(folder.join(URL_FILE)) {
            let url = url.trim();
            if !url.is_empty() {
                return Some(url.to_string());
            }
        }
        if start.elapsed() >= wait {
            return None;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Autorise l'instance en cours à prendre le premier plan : Windows ne l'accorde qu'au processus que l'utilisateur vient de lancer.
pub fn allow_foreground() {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{AllowSetForegroundWindow, ASFW_ANY};
        // SAFETY : appel sans pointeur ; échoue sans effet si le processus n'a pas le droit.
        unsafe {
            AllowSetForegroundWindow(ASFW_ANY);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("scriibe-instance-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[cfg(windows)]
    #[test]
    fn a_second_instance_is_refused_until_the_first_ends() {
        let dir = folder("second");
        let Instance::First(first) = acquire(&dir) else { panic!("la première instance doit démarrer") };
        assert!(matches!(acquire(&dir), Instance::Running));
        drop(first);
        assert!(matches!(acquire(&dir), Instance::First(_)));
    }

    #[test]
    fn the_first_instance_announces_its_address_and_removes_it_at_the_end() {
        let dir = folder("url");
        assert_eq!(running_url(&dir, Duration::ZERO), None);
        let Instance::First(first) = acquire(&dir) else { panic!("la première instance doit démarrer") };
        first.publish("http://127.0.0.1:1234/secret/");
        assert_eq!(running_url(&dir, Duration::ZERO).as_deref(), Some("http://127.0.0.1:1234/secret/"));
        drop(first);
        assert_eq!(running_url(&dir, Duration::ZERO), None);
    }

    #[test]
    fn another_data_folder_is_another_instance() {
        let (a, b) = (folder("a"), folder("b"));
        let first = acquire(&a);
        assert!(matches!(first, Instance::First(_)));
        assert!(matches!(acquire(&b), Instance::First(_)));
    }
}
