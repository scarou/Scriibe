//! Moteurs de Scriibe : les processus `whisper-server` et `llama-server`, leurs modèles et leurs options.
//!
//! Même principe que Scribe : les moteurs de modèles restent des processus séparés, appelés en HTTP. Ce crate contient pour l'instant
//! la partie qui se vérifie sans lancer de processus (choix des modèles, options, état annoncé) ; le lancement et les clients HTTP
//! viennent ensuite.

pub mod fs;
pub mod llama;
pub mod whisper;

/// Nom de l'appli, tel qu'il apparaît dans les messages.
pub const APP_NAME: &str = "Scriibe";

/// Dernier élément d'un chemin, avec `/` ou `\` (les chemins viennent de plusieurs systèmes).
pub fn file_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}
