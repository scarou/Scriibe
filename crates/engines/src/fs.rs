//! L'accès aux fichiers dont les moteurs ont besoin, derrière un trait : les tests le remplacent par une liste de fichiers en mémoire.

use std::path::{Path, PathBuf};

pub trait FileSystem: Send + Sync {
    /// Le fichier (ou dossier) existe.
    fn exists(&self, path: &Path) -> bool;
    /// Noms des fichiers directement dans ce dossier.
    fn list_dir(&self, dir: &Path) -> Vec<String>;
    /// Tous les fichiers sous ce dossier, en descendant dans les sous-dossiers, sans les fichiers et dossiers cachés.
    fn walk(&self, root: &Path) -> Vec<PathBuf>;
}

/// Le vrai disque.
pub struct RealFs;

impl FileSystem for RealFs {
    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn list_dir(&self, dir: &Path) -> Vec<String> {
        std::fs::read_dir(dir)
            .map(|entries| entries.filter_map(Result::ok).filter_map(|e| e.file_name().into_string().ok()).collect())
            .unwrap_or_default()
    }

    fn walk(&self, root: &Path) -> Vec<PathBuf> {
        fn visit(dir: &Path, out: &mut Vec<PathBuf>) {
            let Ok(entries) = std::fs::read_dir(dir) else { return };
            for entry in entries.filter_map(Result::ok) {
                if entry.file_name().to_string_lossy().starts_with('.') {
                    continue;
                }
                let path = entry.path();
                if path.is_dir() {
                    visit(&path, out);
                } else {
                    out.push(path);
                }
            }
        }
        let mut out = Vec::new();
        visit(root, &mut out);
        out
    }
}

/// Cherche un programme : d'abord aux emplacements habituels, puis dans les dossiers du `PATH` (avec `.exe` sous Windows).
pub fn find_binary(fs: &dyn FileSystem, name: &str, candidates: &[PathBuf], path_var: Option<&std::ffi::OsStr>) -> Option<PathBuf> {
    if let Some(found) = candidates.iter().find(|c| fs.exists(c)) {
        return Some(found.clone());
    }
    let file = if cfg!(windows) { format!("{name}.exe") } else { name.to_string() };
    path_var.and_then(|p| std::env::split_paths(p).map(|dir| dir.join(&file)).find(|f| fs.exists(f)))
}
