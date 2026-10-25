//! Whisper sur cet ordinateur : choix du modèle et libération au repos. Port de la partie pure de `WhisperServer.swift` (Scribe).

use crate::file_name;
use std::collections::HashSet;

/// Port de `whisper-server`.
pub const PORT: u16 = 8178;

/// Un fichier `ggml-*.bin` que `whisper-server` sait charger. Les modèles d'autres moteurs portent le même préfixe mais pas le même
/// réseau (Parakeet : « unknown tensor in model file », le serveur s'arrête) ; les modèles de test et de détection de voix ne transcrivent pas.
pub fn is_whisper_model(path: &str) -> bool {
    let name = file_name(path).to_lowercase();
    const OTHERS: [&str; 7] = ["for-tests", "silero", "vad", "parakeet", "tdt", "canary", "conformer"];
    name.starts_with("ggml-") && name.ends_with(".bin") && !OTHERS.iter().any(|o| name.contains(o))
}

/// Les deux chemins sont-ils la même copie d'un modèle (même nom, même taille) ?
pub fn same_model(a: &str, b: &str, size: impl Fn(&str) -> Option<u64>) -> bool {
    file_name(a).to_lowercase() == file_name(b).to_lowercase() && size(a) == size(b)
}

/// Un seul chemin par modèle : le même nom et la même taille veulent dire la même copie (un fichier copié dans plusieurs projets,
/// ou lié en dur). Le premier de la liste est gardé : la détection met la copie propre à l'appli devant.
pub fn distinct(paths: &[String], size: impl Fn(&str) -> Option<u64>) -> Vec<String> {
    let mut seen = HashSet::new();
    paths
        .iter()
        .filter(|path| {
            let key = format!("{}#{}", file_name(path).to_lowercase(), size(path).map_or(-1, |s| s as i64));
            seen.insert(key)
        })
        .cloned()
        .collect()
}

/// Les modèles « turbo » et « large » d'abord : meilleur compromis précision/vitesse en français.
/// `own_models_folder` : le dossier `models/` de l'appli (sa copie passe avant les autres).
pub fn score(path: &str, own_models_folder: Option<&str>) -> i32 {
    let n = path.to_lowercase();
    if own_models_folder.is_some_and(|own| path.starts_with(own)) {
        return 10;
    }
    if n.contains("large-v3-turbo") {
        3
    } else if n.contains("large") {
        2
    } else if n.contains("medium") {
        1
    } else {
        0
    }
}

/// Faut-il libérer Whisper ? Pur, pour pouvoir le vérifier : réglage actif, aucune transcription en cours, assez de repos.
pub fn should_release(idle_seconds: f64, limit_minutes: i64, busy: bool) -> bool {
    limit_minutes > 0 && !busy && idle_seconds > limit_minutes as f64 * 60.0
}
