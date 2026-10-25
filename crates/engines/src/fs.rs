//! L'accès aux fichiers dont les moteurs ont besoin, derrière un trait : les tests le remplacent par une liste de fichiers en mémoire.

use std::path::{Path, PathBuf};

pub trait FileSystem {
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
