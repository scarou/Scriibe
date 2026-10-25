//! Moteurs de Scriibe : les processus `whisper-server` et `llama-server`, leurs modèles et leurs options.
//!
//! Même principe que Scribe : les moteurs de modèles restent des processus séparés, appelés en HTTP. Ce crate contient pour l'instant
//! la partie qui se vérifie sans lancer de processus (choix des modèles, options, état annoncé) ; le lancement et les clients HTTP
//! viennent ensuite.

pub mod fs;
pub mod http;
pub mod llama;
pub mod runtime;
pub mod server;
pub mod services;
pub mod testing;
pub mod whisper;

/// Nom de l'appli, tel qu'il apparaît dans les messages.
pub const APP_NAME: &str = "Scriibe";

/// Dernier élément d'un chemin, avec `/` ou `\` (les chemins viennent de plusieurs systèmes).
pub fn file_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// Dossier où l'appli range les moteurs téléchargés (`whisper-server`, `llama-server`), s'il peut être déterminé.
pub fn engines_folder() -> Option<std::path::PathBuf> {
    let base = if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA").map(std::path::PathBuf::from)?
    } else if cfg!(target_os = "macos") {
        std::path::PathBuf::from(std::env::var_os("HOME")?).join("Library/Application Support")
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(std::path::PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".local/share")))?
    };
    Some(base.join(APP_NAME).join("engines"))
}
