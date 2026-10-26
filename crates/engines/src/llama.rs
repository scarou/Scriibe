//! Mise au propre sur cet ordinateur sans autre appli : l'appli lance `llama-server` (llama.cpp) avec le modèle GGUF choisi et son
//! assistant MTP (décodage spéculatif), comme pour `whisper-server`. Port de la partie pure de `LlamaServer.swift` (Scribe).

use crate::fs::{find_binary, FileSystem};
use crate::http::ServiceError;
use crate::server::{EngineServer, Launcher, ServerConfig};
use crate::{file_name, APP_NAME};
use fancy_regex::Regex;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, LazyLock};
use std::time::Duration;

/// Port de l'appli. Les essais en ligne de commande prennent le suivant, pour ne pas arrêter le serveur d'une appli restée ouverte.
pub const PORT: u16 = 8179;
pub const CLI_PORT: u16 = 8180;

static QUANTIZATION: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)-(i?q[0-9]+(_[a-z0-9]+)*|bf16|f16|f32)$").expect("motif valide"));

/// Nom sans extension ni quantification : « gemma-4-E4B-it-Q4_0.gguf » → « gemma-4-E4B-it ».
pub fn base_name(file: &str) -> String {
    let name = file.rsplit_once('.').map_or(file, |(stem, _)| stem);
    match QUANTIZATION.find(name) {
        Ok(Some(m)) => name[..m.start()].to_string(),
        _ => name.to_string(),
    }
}

/// Les options de `llama-server` : deux emplacements (le préchauffage de la touche n'efface pas le prompt gardé de la dictée ; mesuré
/// 0,34 s contre 0,40 s avec un seul) qui partagent un contexte de 8 192 jetons (`-kvu` : sans lui, 4 096 chacun, trop peu pour une
/// longue dictée), tout sur la carte graphique, et l'assistant MTP (5 jetons proposés) s'il existe.
pub fn arguments(model: &str, assistant: Option<&str>, port: u16) -> Vec<String> {
    let mut args: Vec<String> = [
        "-m",
        model,
        "--host",
        "127.0.0.1",
        "--port",
        &port.to_string(),
        "-c",
        "8192",
        "-ngl",
        "99",
        "-np",
        "2",
        "-kvu",
        "-fa",
        "on",
        "--jinja",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    if let Some(assistant) = assistant {
        args.extend(["--spec-type", "draft-mtp", "--spec-draft-model", assistant, "--spec-draft-n-max", "5"].map(String::from));
    }
    args
}

/// Options que `llama-server --help` doit connaître : une mise à jour de llama.cpp peut en retirer ou renommer une.
pub const REQUIRED_OPTIONS: [&str; 10] = [
    "--spec-type",
    "draft-mtp",
    "--spec-draft-model",
    "--spec-draft-n-max",
    "--kv-unified",
    "--flash-attn",
    "--parallel",
    "--jinja",
    "--gpu-layers",
    "--ctx-size",
];

/// Le modèle choisi pour l'appli (gemma-4-e4b) d'abord, puis la copie propre à l'appli.
fn score(path: &str, own_models_folder: Option<&Path>) -> i32 {
    let mut score = 0;
    if own_models_folder.is_some_and(|own| Path::new(path).starts_with(own)) {
        score += 1;
    }
    if path.to_lowercase().contains("gemma-4-e4b") {
        score += 2;
    }
    score
}

/// Modèles GGUF repérés dans ces dossiers. Les fichiers d'assistant (`mtp-`), de vision (`mmproj-`) et d'« embedding » ne sont pas
/// des modèles de mise au propre. Le meilleur score d'abord, puis l'ordre alphabétique (le « premier » ne dépend pas du hasard).
pub fn detect_models(fs: &dyn FileSystem, folders: &[PathBuf], own_models_folder: Option<&Path>) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for folder in folders {
        for path in fs.walk(folder) {
            let name = path.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
            if !name.ends_with(".gguf") || name.starts_with("mmproj-") || name.starts_with("mtp-") || name.contains("embed") {
                continue;
            }
            found.push(path.to_string_lossy().into_owned());
        }
    }
    found.sort_by(|a, b| score(b, own_models_folder).cmp(&score(a, own_models_folder)).then_with(|| a.cmp(b)));
    found
}

/// L'assistant MTP de ce modèle : un fichier `mtp-<modèle>-<quantification>.gguf` à côté de lui
/// (`gemma-4-E4B-it-Q4_0.gguf` → `mtp-gemma-4-E4B-it-Q8_0.gguf`). Sans lui, le modèle tourne sans décodage spéculatif.
pub fn assistant(fs: &dyn FileSystem, model_path: &str) -> Option<String> {
    let path = Path::new(model_path);
    let base = base_name(path.file_name()?.to_str()?);
    if base.is_empty() {
        return None;
    }
    let dir = path.parent()?;
    let mut siblings = fs.list_dir(dir);
    siblings.sort();
    let wanted = base.to_lowercase();
    siblings
        .into_iter()
        .find(|s| {
            let l = s.to_lowercase();
            l.starts_with("mtp-") && l.ends_with(".gguf") && l.contains(&wanted)
        })
        .map(|s| dir.join(s).to_string_lossy().into_owned())
}

/// Ce que dit Réglages sous le modèle : binaire absent, aucun fichier, ou le modèle choisi avec son assistant MTP.
/// `install_hint` : la commande qui installe `llama-server` sur ce système (« brew install llama.cpp »…).
pub fn status(fs: &dyn FileSystem, binary: Option<&str>, models: &[String], chosen: &str, install_hint: &str) -> String {
    if binary.is_none() {
        return format!("llama-server introuvable : installe-le avec « {install_hint} »");
    }
    if models.is_empty() && chosen.is_empty() {
        return "Aucun fichier .gguf trouvé : choisis-en un avec le bouton dossier".to_string();
    }
    let mut chosen = chosen.to_string();
    let mut replaced = String::new();
    if !chosen.is_empty() && !fs.exists(Path::new(&chosen)) {
        let lost = file_name(&chosen).to_string();
        let Some(other) = models.first() else {
            return format!("Fichier introuvable : {lost} — choisis un autre modèle (bouton dossier)");
        };
        // Le choix est ignoré, et on le dit : le modèle repéré sert à la place.
        replaced = format!("Fichier introuvable : {lost} — {} est utilisé à la place · ", file_name(other));
        chosen = other.clone();
    }
    let assist = if assistant(fs, &chosen).is_none() { "sans assistant MTP (plus lent : 0,6 à 1 s)" } else { "avec son assistant MTP" };
    let plural = if models.len() > 1 { "s" } else { "" };
    format!(
        "{replaced}llama.cpp lancé par {APP_NAME} · {} modèle{plural} · {assist} · le premier appel charge le modèle (quelques secondes)",
        models.len()
    )
}

/// Sans dictée depuis ce temps, le serveur s'arrête et rend ses ≈ 5 Go ; la touche de dictée le relance (préchauffage).
pub const IDLE_LIMIT: Duration = Duration::from_secs(30 * 60);

fn binary_name() -> &'static str {
    if cfg!(windows) {
        "llama-server.exe"
    } else {
        "llama-server"
    }
}

/// Emplacements habituels de `llama-server`.
pub fn binary_candidates() -> Vec<PathBuf> {
    let mut list: Vec<PathBuf> =
        ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin"].iter().map(|d| Path::new(d).join(binary_name())).collect();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            list.push(dir.join(binary_name()));
            list.push(dir.join("llama").join(binary_name()));
        }
    }
    // Un dossier par moteur : whisper.cpp et llama.cpp livrent des DLL `ggml*.dll` de même nom mais de versions différentes.
    if let Some(dir) = crate::engines_folder() {
        list.push(dir.join("llama").join(binary_name()));
        list.push(dir.join(binary_name()));
    }
    list
}

/// La commande qui installe `llama-server` sur ce système.
pub fn install_hint() -> &'static str {
    if cfg!(target_os = "macos") {
        "brew install llama.cpp"
    } else if cfg!(windows) {
        "winget install llama.cpp"
    } else {
        "installe llama.cpp (llama-server) avec ton gestionnaire de paquets"
    }
}

/// Le modèle llama.cpp choisi ; à défaut (aucun choix, ou fichier déplacé ou supprimé) le premier repéré.
pub fn resolved_model(fs: &dyn FileSystem, chosen: &str, detected: &[String]) -> String {
    if !chosen.is_empty() && fs.exists(Path::new(chosen)) {
        return chosen.to_string();
    }
    detected.first().cloned().unwrap_or_else(|| chosen.to_string())
}

pub struct LlamaLauncher {
    config: ServerConfig,
    fs: Arc<dyn FileSystem>,
    binary: Option<PathBuf>,
}

impl LlamaLauncher {
    /// `log_file` : où garder la sortie d'erreur du serveur (`llama-server.log` dans le dossier de données).
    pub fn new(fs: Arc<dyn FileSystem>, port: u16, log_file: Option<PathBuf>, path_var: Option<&std::ffi::OsStr>) -> Self {
        let binary = find_binary(fs.as_ref(), "llama-server", &binary_candidates(), path_var);
        Self {
            config: ServerConfig {
                name: "llama.cpp",
                port,
                ping_path: "/health",
                ping_needs_ok: true,
                // Jusqu'à 2 min : le premier chargement prépare la carte graphique.
                max_polls: 480,
                poll: Duration::from_millis(250),
                orphan_needles: vec!["llama-server".into(), "-m ".into(), format!("--port {port} ")],
                log_file,
            },
            fs,
            binary,
        }
    }
}

impl Launcher for LlamaLauncher {
    fn config(&self) -> &ServerConfig {
        &self.config
    }

    fn binary(&self) -> Option<PathBuf> {
        self.binary.clone()
    }

    fn model_exists(&self, model: &str) -> bool {
        self.fs.exists(Path::new(model))
    }

    fn args(&self, model: &str, use_assistant: bool) -> (Vec<String>, bool) {
        let assistant = if use_assistant { assistant(self.fs.as_ref(), model) } else { None };
        let mut args = arguments(model, assistant.as_deref(), self.config.port);
        // Écart avec Scribe (docs/ECARTS.md) : sans carte graphique, la génération se fait sur le processeur ; on ne lance pas plus de
        // fils que de cœurs physiques.
        args.extend(["-t".to_string(), crate::compute_threads().to_string()]);
        (args, assistant.is_some())
    }

    fn describe(&self, assistant: bool) -> String {
        format!("{}, {} fils", if assistant { ", avec assistant MTP" } else { ", sans assistant MTP" }, crate::compute_threads())
    }

    fn missing_binary(&self) -> String {
        format!("llama-server introuvable ({})", install_hint())
    }

    fn missing_model(&self) -> String {
        "Modèle llama.cpp introuvable — voir Réglages".into()
    }

    fn start_failed(&self) -> String {
        "llama.cpp n'a pas pu démarrer (modèle illisible ? voir llama-server.log)".into()
    }
}

/// Crée le serveur llama.cpp : arrêté après 30 min sans dictée.
pub fn new_server(launcher: LlamaLauncher, rt: crate::server::Runtime) -> EngineServer<LlamaLauncher> {
    let server = EngineServer::new(launcher, rt);
    server.set_idle_limit(Some(IDLE_LIMIT));
    server
}

/// Démarre le serveur, mais n'attend pas plus de `within` : une dictée n'attend pas plus longtemps un serveur qui ne démarre pas
/// (le texte brut est alors collé). Le démarrage continue en arrière-plan.
pub fn ensure_running_within(server: &Arc<EngineServer<LlamaLauncher>>, model: &str, within: Duration) -> Result<(), ServiceError> {
    let (sender, receiver) = mpsc::channel();
    let worker = server.clone();
    let model = model.to_string();
    std::thread::spawn(move || {
        let _ = sender.send(worker.ensure_running(&model));
    });
    match receiver.recv_timeout(within) {
        Ok(result) => result,
        Err(_) => Err(ServiceError::new("llama.cpp met trop de temps à démarrer")),
    }
}
