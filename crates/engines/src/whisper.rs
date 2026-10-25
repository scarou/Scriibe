//! Whisper sur cet ordinateur : choix du modèle et libération au repos. Port de la partie pure de `WhisperServer.swift` (Scribe).

use crate::file_name;
use crate::fs::{find_binary, FileSystem};
use crate::http::{send, HttpRequest, ServiceError};
use crate::server::{EngineServer, Launcher, Runtime, ServerConfig};
use scriibe_core::settings::DictationLanguage;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

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

/// Emplacements habituels de `whisper-server`.
pub fn binary_candidates() -> Vec<PathBuf> {
    let mut list: Vec<PathBuf> =
        ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin"].iter().map(|d| Path::new(d).join(binary_name())).collect();
    // À côté de l'appli, et dans le dossier des moteurs de l'appli (Windows : %LOCALAPPDATA%\Scriibe\engines).
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            list.push(dir.join(binary_name()));
        }
    }
    if let Some(dir) = crate::engines_folder() {
        list.push(dir.join(binary_name()));
    }
    list
}

fn binary_name() -> &'static str {
    if cfg!(windows) {
        "whisper-server.exe"
    } else {
        "whisper-server"
    }
}

/// La commande qui installe `whisper-server` sur ce système.
pub fn install_hint() -> &'static str {
    if cfg!(target_os = "macos") {
        "brew install whisper-cpp"
    } else {
        "télécharge whisper-server (whisper.cpp) et place-le à côté de l'appli ou dans le PATH"
    }
}

/// Le modèle Whisper choisi, ou à défaut le meilleur repéré.
pub fn resolved_model(chosen: &str, detected: &[String]) -> String {
    if !chosen.is_empty() {
        return chosen.to_string();
    }
    detected.first().cloned().unwrap_or_default()
}

pub struct WhisperLauncher {
    config: ServerConfig,
    fs: Arc<dyn FileSystem>,
    binary: Option<PathBuf>,
    threads: usize,
}

impl WhisperLauncher {
    pub fn new(fs: Arc<dyn FileSystem>, port: u16, path_var: Option<&std::ffi::OsStr>) -> Self {
        let binary = find_binary(fs.as_ref(), "whisper-server", &binary_candidates(), path_var);
        let threads = std::thread::available_parallelism().map_or(4, |n| n.get()).min(8);
        Self {
            config: ServerConfig {
                name: "whisper",
                port,
                ping_path: "/",
                ping_needs_ok: false,
                // Jusqu'à ≈ 30 s : le tout premier chargement prépare la carte graphique.
                max_polls: 120,
                poll: Duration::from_millis(250),
                orphan_needles: vec!["whisper-server".into(), format!("--port {port} ")],
                log_file: None,
            },
            fs,
            binary,
            threads,
        }
    }
}

impl Launcher for WhisperLauncher {
    fn config(&self) -> &ServerConfig {
        &self.config
    }

    fn binary(&self) -> Option<PathBuf> {
        self.binary.clone()
    }

    fn model_exists(&self, model: &str) -> bool {
        self.fs.exists(Path::new(model))
    }

    fn args(&self, model: &str, _use_assistant: bool) -> (Vec<String>, bool) {
        let args =
            ["-m", model, "--host", "127.0.0.1", "--port", &self.config.port.to_string(), "-l", "auto", "-t", &self.threads.to_string()];
        (args.iter().map(|s| s.to_string()).collect(), false)
    }

    fn missing_binary(&self) -> String {
        format!("whisper-server introuvable ({})", install_hint())
    }

    fn missing_model(&self) -> String {
        "Modèle Whisper introuvable — voir Réglages".into()
    }

    fn start_failed(&self) -> String {
        "Whisper n'a pas pu démarrer (modèle illisible ?)".into()
    }
}

/// Whisper sur cet ordinateur : le serveur et la transcription.
pub struct Whisper {
    pub server: EngineServer<WhisperLauncher>,
    rt: Runtime,
}

impl Whisper {
    pub fn new(launcher: WhisperLauncher, rt: Runtime) -> Self {
        Self { server: EngineServer::new(launcher, rt.clone()), rt }
    }

    /// Transcrit un enregistrement WAV. `idle_minutes` : le réglage « Libérer Whisper au repos » (0 : jamais), lu à chaque appel pour
    /// suivre un changement.
    pub fn transcribe(
        &self,
        wav: &[u8],
        model_path: &str,
        language: DictationLanguage,
        vocabulary: &[String],
        idle_minutes: i64,
    ) -> Result<String, ServiceError> {
        self.server.set_idle_limit((idle_minutes > 0).then(|| Duration::from_secs(idle_minutes as u64 * 60)));
        self.server.ensure_running(model_path)?;
        // Une transcription en cours empêche la libération ; le repos se compte à partir de sa fin.
        let _busy = self.server.begin_use();
        let boundary = format!("scriibe-{}", scriibe_core::modes::new_id());
        let body = multipart_body(&boundary, wav, language, vocabulary);
        let port = self.server.launcher().config().port;
        let request = HttpRequest::post(format!("http://127.0.0.1:{port}/inference"), body, Duration::from_secs(60))
            .header("Content-Type", format!("multipart/form-data; boundary={boundary}"));
        let data = send(self.rt.http.as_ref(), request, "Whisper", true)?;
        let json: serde_json::Value = serde_json::from_slice(&data).map_err(|_| ServiceError::new("Réponse Whisper illisible"))?;
        let text = json["text"].as_str().ok_or_else(|| ServiceError::new("Réponse Whisper illisible"))?;
        Ok(clean(text))
    }
}

/// Le corps de la requête `/inference` : le fichier, puis les champs.
pub fn multipart_body(boundary: &str, wav: &[u8], language: DictationLanguage, vocabulary: &[String]) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(
        format!("--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"audio.wav\"\r\nContent-Type: audio/wav\r\n\r\n")
            .as_bytes(),
    );
    body.extend_from_slice(wav);
    body.extend_from_slice(b"\r\n");
    let mut field = |name: &str, value: &str| {
        body.extend_from_slice(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n").as_bytes());
    };
    field("response_format", "json");
    field("temperature", "0.0");
    field("language", if language == DictationLanguage::Multi { "auto" } else { language.raw() });
    // Les mots du dictionnaire servent d'amorce : Whisper les orthographie mieux.
    let prompt = vocabulary.iter().filter(|w| !w.is_empty()).cloned().collect::<Vec<_>>().join(", ");
    if !prompt.is_empty() {
        field("prompt", &prompt);
    }
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    body
}

/// Retire les annotations de Whisper ([Musique], [BLANK_AUDIO]…) et les espaces superflus.
pub fn clean(text: &str) -> String {
    static TAGS: std::sync::LazyLock<fancy_regex::Regex> =
        std::sync::LazyLock::new(|| fancy_regex::Regex::new(r"\[[^\]]*\]").expect("motif valide"));
    let without_tags = TAGS.replace_all(text, " ");
    without_tags.split_whitespace().collect::<Vec<_>>().join(" ")
}
