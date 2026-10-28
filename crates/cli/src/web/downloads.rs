//! Téléchargements proposés par la page d'essais. Rien ne part sans que la personne ait vu le nom, la taille et la source, puis confirmé.
//! Les noms de fichiers sont résolus à l'instant (API de GitHub et de Hugging Face) : on affiche la vraie taille avant de télécharger.

use super::hub;
use serde_json::Value;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

/// Ce que la page propose d'office : les programmes des moteurs, et quelques modèles recommandés (les autres se choisissent sur
/// Hugging Face, voir `hub`).
pub struct Item {
    pub id: &'static str,
    pub label: &'static str,
    pub detail: &'static str,
    /// « program », « whisper » (modèle de transcription) ou « llama » (modèle de mise au propre).
    pub kind: &'static str,
}

pub const ITEMS: [Item; 8] = [
    Item {
        id: "whisper-server",
        label: "whisper-server (Windows)",
        detail: "Le programme de transcription (whisper.cpp), avec ses DLL.",
        kind: "program",
    },
    Item {
        id: "whisper-base",
        label: "Modèle Whisper « base »",
        detail: "Le plus rapide ; quelques erreurs sur les mots courts.",
        kind: "whisper",
    },
    Item { id: "whisper-small", label: "Modèle Whisper « small »", detail: "Plus précis, plus lent.", kind: "whisper" },
    Item {
        id: "llama-server",
        label: "llama-server (Windows, processeur)",
        detail: "Le programme de mise au propre (llama.cpp), version processeur.",
        kind: "program",
    },
    Item {
        id: "llama-vulkan",
        label: "llama-server (Windows, carte graphique Vulkan)",
        detail: "Remplace la version processeur : la mise au propre tourne sur la carte graphique (à comparer aux temps du processeur). Pour revenir en arrière, retélécharger la version processeur.",
        kind: "program",
    },
    Item {
        id: "qwen-0.5b",
        label: "Modèle de mise au propre « Qwen 2.5 0,5 B »",
        detail: "Très petit (≈ 400 Mo) : le plus rapide sur processeur, à essayer avant de juger sa qualité (Apache 2.0).",
        kind: "llama",
    },
    Item {
        id: "qwen-1.5b",
        label: "Modèle de mise au propre « Qwen 2.5 1,5 B »",
        detail: "Petit modèle d'instructions (licence Apache 2.0), ≈ 1 Go.",
        kind: "llama",
    },
    Item {
        id: "qwen-3b",
        label: "Modèle de mise au propre « Qwen 2.5 3 B »",
        detail: "Deux fois plus gros, meilleur, plus lent.",
        kind: "llama",
    },
];

#[derive(Debug, Clone)]
pub struct Resolved {
    pub name: String,
    pub size: u64,
    pub url: String,
    /// Dossier où le fichier (ou le contenu du zip) est rangé.
    pub dest: PathBuf,
    pub unzip: bool,
    /// Vide d'abord les programmes et DLL du dossier : deux versions d'un moteur (processeur, Vulkan) ne doivent pas se mélanger.
    pub clean: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Progress {
    Running { done: u64, total: u64 },
    Done,
    Failed(String),
}

#[derive(Default)]
pub struct Downloads {
    progress: Mutex<HashMap<String, Progress>>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(15)).timeout_read(Duration::from_secs(60)).build()
}

fn get_json(url: &str) -> Result<Value, String> {
    let response = agent()
        .get(url)
        .set("User-Agent", "scriibe")
        .set("Accept", "application/json")
        .timeout(Duration::from_secs(30))
        .call()
        .map_err(|e| format!("{url} : {e}"))?;
    serde_json::from_reader(response.into_reader()).map_err(|e| format!("réponse illisible de {url} ({e})"))
}

/// Comme `get_json`, avec des paramètres d'adresse (encodés par `ureq`).
fn get_json_query(url: &str, query: &[(&str, &str)]) -> Result<Value, String> {
    let mut request = agent().get(url).set("User-Agent", "scriibe").set("Accept", "application/json").timeout(Duration::from_secs(30));
    for (key, value) in query {
        request = request.query(key, value);
    }
    let response = request.call().map_err(|e| format!("{url} : {e}"))?;
    serde_json::from_reader(response.into_reader()).map_err(|e| format!("réponse illisible de {url} ({e})"))
}

/// Les fichiers d'un dépôt Hugging Face qu'on peut prendre comme modèle de ce genre, avec leur taille.
pub fn hub_list(repo: &str, kind: hub::Kind) -> Result<Vec<hub::HubFile>, String> {
    if !hub::valid_repo(repo) {
        return Err(format!("« {repo} » n'est pas un nom de dépôt (attendu : propriétaire/nom)"));
    }
    let tree = get_json(&format!("https://huggingface.co/api/models/{repo}/tree/main"))?;
    Ok(hub::files_from_tree(&tree, kind))
}

/// Des dépôts Hugging Face qui correspondent à ces mots (les modèles GGUF pour la mise au propre).
pub fn hub_search(words: &str, kind: hub::Kind, sort: &str) -> Result<Vec<hub::HubRepo>, String> {
    let mut query = vec![("search", words), ("sort", hub::sort_key(sort)), ("direction", "-1"), ("limit", "15")];
    if kind == hub::Kind::Llama {
        query.push(("filter", "gguf"));
    }
    Ok(hub::repos_from_search(&get_json_query("https://huggingface.co/api/models", &query)?))
}

/// Premier fichier de ces versions (la plus récente d'abord) dont le nom satisfait `wanted`.
fn release_asset(repo: &str, wanted: impl Fn(&str) -> bool) -> Result<(String, u64, String), String> {
    let releases = get_json(&format!("https://api.github.com/repos/{repo}/releases?per_page=30"))?;
    for release in releases.as_array().into_iter().flatten() {
        for asset in release["assets"].as_array().into_iter().flatten() {
            let name = asset["name"].as_str().unwrap_or("");
            if wanted(name) {
                let url = asset["browser_download_url"].as_str().ok_or("adresse de téléchargement absente")?;
                return Ok((name.to_string(), asset["size"].as_u64().unwrap_or(0), url.to_string()));
            }
        }
    }
    Err(format!("aucun fichier correspondant dans les dernières versions de {repo}"))
}

/// Taille d'un fichier d'un dépôt Hugging Face.
fn hub_file(repo: &str, file: &str) -> Result<(String, u64, String), String> {
    let tree = get_json(&format!("https://huggingface.co/api/models/{repo}/tree/main"))?;
    let entries = tree.as_array().ok_or("liste de fichiers illisible")?;
    match entries.iter().find(|e| e["path"].as_str() == Some(file)) {
        Some(entry) => {
            let size = entry["lfs"]["size"].as_u64().or_else(|| entry["size"].as_u64()).unwrap_or(0);
            Ok((file.to_string(), size, format!("https://huggingface.co/{repo}/resolve/main/{file}")))
        }
        None => {
            let names: Vec<&str> =
                entries.iter().filter_map(|e| e["path"].as_str()).filter(|n| n.ends_with(".gguf") || n.ends_with(".bin")).collect();
            Err(format!("« {file} » n'est pas dans {repo} (fichiers : {})", names.join(", ")))
        }
    }
}

/// Le nom, la taille et la source de ce qu'on téléchargerait.
pub fn resolve(id: &str, data: &Path) -> Result<Resolved, String> {
    let models = data.join("models");
    let engines = scriibe_engines::engines_folder().ok_or("dossier des moteurs introuvable")?;
    let windows_only = |what: &str| -> Result<(), String> {
        if cfg!(windows) {
            Ok(())
        } else {
            Err(format!("{what} : proposé seulement sous Windows pour l'instant"))
        }
    };
    if let Some((repo, file, _)) = hub::parse_download_id(id) {
        return hub_resolved(&repo, &file, models);
    }
    let (found, dest, unzip) = match id {
        "whisper-server" => {
            windows_only("whisper-server")?;
            (release_asset("ggml-org/whisper.cpp", |n| n == "whisper-bin-x64.zip")?, engines.join("whisper"), true)
        }
        "llama-server" => {
            windows_only("llama-server")?;
            (
                release_asset("ggml-org/llama.cpp", |n| n.starts_with("llama-") && n.ends_with("-bin-win-cpu-x64.zip"))?,
                engines.join("llama"),
                true,
            )
        }
        "llama-vulkan" => {
            windows_only("llama-server Vulkan")?;
            (
                release_asset("ggml-org/llama.cpp", |n| n.starts_with("llama-") && n.ends_with("-bin-win-vulkan-x64.zip"))?,
                engines.join("llama"),
                true,
            )
        }
        "whisper-base" => (hub_file("ggerganov/whisper.cpp", "ggml-base.bin")?, models, false),
        "whisper-small" => (hub_file("ggerganov/whisper.cpp", "ggml-small.bin")?, models, false),
        "qwen-0.5b" => (hub_file("Qwen/Qwen2.5-0.5B-Instruct-GGUF", "qwen2.5-0.5b-instruct-q4_k_m.gguf")?, models, false),
        "qwen-1.5b" => (hub_file("Qwen/Qwen2.5-1.5B-Instruct-GGUF", "qwen2.5-1.5b-instruct-q4_k_m.gguf")?, models, false),
        "qwen-3b" => (hub_file("Qwen/Qwen2.5-3B-Instruct-GGUF", "qwen2.5-3b-instruct-q4_k_m.gguf")?, models, false),
        other => return Err(format!("téléchargement inconnu : {other}")),
    };
    let (name, size, url) = found;
    let clean = id.starts_with("llama-");
    Ok(Resolved { name, size, url, dest, unzip, clean })
}

fn hub_resolved(repo: &str, file: &str, models: PathBuf) -> Result<Resolved, String> {
    let (name, size, url) = hub_file(repo, file)?;
    Ok(Resolved { name, size, url, dest: models, unzip: false, clean: false })
}

/// Décompresse à plat : le programme et ses DLL côte à côte (jamais de chemin du zip, donc pas d'écriture hors du dossier).
fn unzip_flat(zip_path: &Path, dest: &Path) -> Result<usize, String> {
    let file = std::fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("zip illisible ({e})"))?;
    std::fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    let mut written = 0;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        if !entry.is_file() {
            continue;
        }
        let Some(name) = entry.enclosed_name().and_then(|p| p.file_name().map(|n| n.to_owned())) else { continue };
        let mut out = std::fs::File::create(dest.join(name)).map_err(|e| e.to_string())?;
        std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
        written += 1;
    }
    Ok(written)
}

/// Supprime les programmes et DLL d'un dossier de moteur (rien d'autre : les modèles sont ailleurs).
fn clean_engine_folder(folder: &Path) -> Result<(), String> {
    let Ok(entries) = std::fs::read_dir(folder) else { return Ok(()) };
    for entry in entries.flatten() {
        let path = entry.path();
        let is_engine_file =
            path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("exe") || e.eq_ignore_ascii_case("dll"));
        if is_engine_file {
            std::fs::remove_file(&path)
                .map_err(|e| format!("{} ne peut pas être remplacé ({e}) : le moteur est-il encore lancé ?", path.display()))?;
        }
    }
    Ok(())
}

/// Ce qu'une source rend quand on lui demande un fichier à partir d'un octet.
struct Opened {
    reader: Box<dyn Read>,
    /// L'octet où la réponse commence : celui demandé (réponse partielle), ou 0 si la source ignore la reprise et renvoie tout.
    start: u64,
    /// Taille totale du fichier, 0 si inconnue.
    total: u64,
}

/// D'où viennent les octets (le réseau, ou une fausse source dans les tests).
trait Source {
    fn open(&self, from: u64) -> Result<Opened, String>;
}

struct HttpSource {
    url: String,
}

/// « bytes 100-999/1000 » ou « bytes */1000 » : la taille totale.
fn total_from_content_range(header: &str) -> Option<u64> {
    header.rsplit('/').next()?.trim().parse().ok()
}

impl Source for HttpSource {
    fn open(&self, from: u64) -> Result<Opened, String> {
        let mut request = agent().get(&self.url).set("User-Agent", "scriibe");
        if from > 0 {
            request = request.set("Range", &format!("bytes={from}-"));
        }
        match request.call() {
            Ok(response) => {
                let partial = response.status() == 206;
                let length: u64 = response.header("Content-Length").and_then(|v| v.parse().ok()).unwrap_or(0);
                let total = if partial {
                    response.header("Content-Range").and_then(total_from_content_range).unwrap_or(from + length)
                } else {
                    length
                };
                Ok(Opened { reader: Box::new(response.into_reader()), start: if partial { from } else { 0 }, total })
            }
            // Rien à reprendre : le fichier est déjà entier, ou la reprise n'a pas de sens (fichier changé) : on repart de zéro.
            Err(ureq::Error::Status(416, response)) => {
                let total = response.header("Content-Range").and_then(total_from_content_range).unwrap_or(0);
                if total > 0 && total == from {
                    Ok(Opened { reader: Box::new(std::io::empty()), start: from, total })
                } else {
                    self.open(0)
                }
            }
            Err(e) => Err(format!("téléchargement refusé ({e})")),
        }
    }
}

/// Combien de fois une connexion coupée est reprise (là où elle s'est arrêtée) avant d'abandonner.
const RESUME_ATTEMPTS: u32 = 6;

/// Reçoit le fichier dans `part`. Une coupure de connexion reprend toute seule à l'octet où elle s'est produite (en-tête `Range`) ; un
/// fichier `.part` laissé par un échec précédent est repris de même. En cas d'abandon, `part` est gardé pour la prochaine fois.
/// Rend la taille reçue.
fn receive(source: &dyn Source, part: &Path, expected: u64, pause: Duration, progress: &dyn Fn(u64, u64)) -> Result<u64, String> {
    let mut failures = 0;
    loop {
        let have = std::fs::metadata(part).map(|m| m.len()).unwrap_or(0);
        // Un fichier plus gros que la taille annoncée n'est pas le bon : on repart de zéro.
        let from = if expected > 0 && have > expected { 0 } else { have };
        let outcome = (|| -> Result<u64, String> {
            let mut opened = source.open(from)?;
            let total = if opened.total > 0 { opened.total } else { expected };
            let mut out = if opened.start == 0 {
                std::fs::File::create(part).map_err(|e| e.to_string())?
            } else {
                std::fs::OpenOptions::new().append(true).open(part).map_err(|e| e.to_string())?
            };
            let mut done = opened.start;
            let mut buffer = vec![0u8; 256 * 1024];
            loop {
                let n =
                    opened.reader.read(&mut buffer).map_err(|e| format!("connexion interrompue après {} Mo ({e})", done / 1_048_576))?;
                if n == 0 {
                    break;
                }
                out.write_all(&buffer[..n]).map_err(|e| e.to_string())?;
                done += n as u64;
                progress(done, total);
            }
            out.flush().map_err(|e| e.to_string())?;
            if total > 0 && done < total {
                return Err(format!("fichier incomplet ({} sur {} Mo)", done / 1_048_576, total / 1_048_576));
            }
            Ok(done)
        })();
        match outcome {
            Ok(done) => return Ok(done),
            Err(e) => {
                failures += 1;
                if failures > RESUME_ATTEMPTS {
                    return Err(format!("{e} ; ce qui est déjà reçu est gardé : relancer le téléchargement le reprend"));
                }
                std::thread::sleep(pause * failures);
            }
        }
    }
}

fn fetch(resolved: &Resolved, progress: &dyn Fn(u64, u64)) -> Result<(), String> {
    std::fs::create_dir_all(&resolved.dest).map_err(|e| format!("{} : {e}", resolved.dest.display()))?;
    let part = resolved.dest.join(format!("{}.part", resolved.name));
    let source = HttpSource { url: resolved.url.clone() };
    let received = receive(&source, &part, resolved.size, Duration::from_secs(2), progress)?;
    if resolved.size > 0 && received != resolved.size {
        // Reçu en entier mais pas de la taille annoncée : un fichier changé ou abîmé ne se reprend pas.
        let _ = std::fs::remove_file(&part);
        return Err(format!("taille inattendue ({received} octets au lieu de {})", resolved.size));
    }
    if resolved.unzip {
        if resolved.clean {
            if let Err(e) = clean_engine_folder(&resolved.dest) {
                let _ = std::fs::remove_file(&part);
                return Err(e);
            }
        }
        let count = unzip_flat(&part, &resolved.dest);
        let _ = std::fs::remove_file(&part);
        count.map(|_| ())
    } else {
        std::fs::rename(&part, resolved.dest.join(&resolved.name)).map_err(|e| e.to_string())
    }
}

impl Downloads {
    pub fn progress(&self) -> HashMap<String, Progress> {
        lock(&self.progress).clone()
    }

    /// Lance le téléchargement en arrière-plan. `log` reçoit une ligne au départ et à la fin (nom, taille, source : jamais de clé).
    pub fn start(self: &Arc<Self>, id: &str, resolved: Resolved, log: Arc<dyn scriibe_engines::runtime::Log>) -> Result<(), String> {
        {
            let mut map = lock(&self.progress);
            if matches!(map.get(id), Some(Progress::Running { .. })) {
                return Err("déjà en cours".into());
            }
            map.insert(id.to_string(), Progress::Running { done: 0, total: resolved.size });
        }
        log.write(&format!(
            "téléchargement: {} ({} Mo) depuis {} vers {} — demandé et confirmé dans l'interface",
            resolved.name,
            resolved.size / 1_048_576,
            resolved.url,
            resolved.dest.display()
        ));
        let (me, id) = (Arc::clone(self), id.to_string());
        std::thread::spawn(move || {
            let outcome = fetch(&resolved, &|done, total| {
                lock(&me.progress).insert(id.clone(), Progress::Running { done, total });
            });
            log.write(&format!(
                "téléchargement: {} {}",
                resolved.name,
                outcome.as_ref().map_or_else(|e| format!("échec ({e})"), |()| "terminé".into())
            ));
            lock(&me.progress).insert(id, outcome.map_or_else(Progress::Failed, |()| Progress::Done));
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zip_entries_are_flattened_and_cannot_escape() {
        let dir = std::env::temp_dir().join(format!("scriibe-zip-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let archive = dir.join("a.zip");
        {
            let mut writer = zip::ZipWriter::new(std::fs::File::create(&archive).unwrap());
            let options = zip::write::SimpleFileOptions::default();
            writer.start_file("Release/whisper-server.exe", options).unwrap();
            writer.write_all(b"exe").unwrap();
            writer.start_file("../evil.txt", options).unwrap();
            writer.write_all(b"x").unwrap();
            writer.start_file("Release/sub/ggml.dll", options).unwrap();
            writer.write_all(b"dll").unwrap();
            writer.finish().unwrap();
        }
        let out = dir.join("out");
        let count = unzip_flat(&archive, &out).unwrap();
        assert_eq!(count, 2, "le chemin qui remonte est ignoré");
        assert!(out.join("whisper-server.exe").exists() && out.join("ggml.dll").exists());
        assert!(!dir.join("evil.txt").exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn cleaning_an_engine_folder_removes_programs_and_dlls_only() {
        let dir = std::env::temp_dir().join(format!("scriibe-clean-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for name in ["llama-server.exe", "ggml-vulkan.dll", "notes.txt"] {
            std::fs::write(dir.join(name), b"x").unwrap();
        }
        clean_engine_folder(&dir).unwrap();
        assert!(!dir.join("llama-server.exe").exists() && !dir.join("ggml-vulkan.dll").exists());
        assert!(dir.join("notes.txt").exists());
        assert!(clean_engine_folder(&dir.join("absent")).is_ok());
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Une fausse source : `data`, qui coupe la connexion après `cut_after` octets reçus à chaque ouverture (jusqu'à `cuts` fois).
    struct Flaky {
        data: Vec<u8>,
        cut_after: usize,
        cuts: std::cell::Cell<u32>,
        ignore_range: bool,
        opens: std::cell::RefCell<Vec<u64>>,
    }

    struct Cutting {
        data: std::io::Cursor<Vec<u8>>,
        left: usize,
    }

    impl Read for Cutting {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if self.left == 0 {
                return Err(std::io::Error::other("connexion coupée"));
            }
            let n = buf.len().min(self.left);
            let read = self.data.read(&mut buf[..n])?;
            self.left -= read;
            Ok(read)
        }
    }

    impl Source for Flaky {
        fn open(&self, from: u64) -> Result<Opened, String> {
            self.opens.borrow_mut().push(from);
            let start = if self.ignore_range { 0 } else { from };
            let rest = self.data[start as usize..].to_vec();
            let total = self.data.len() as u64;
            if self.cuts.get() > 0 {
                self.cuts.set(self.cuts.get() - 1);
                return Ok(Opened { reader: Box::new(Cutting { data: std::io::Cursor::new(rest), left: self.cut_after }), start, total });
            }
            Ok(Opened { reader: Box::new(std::io::Cursor::new(rest)), start, total })
        }
    }

    fn flaky(len: usize, cut_after: usize, cuts: u32, ignore_range: bool) -> Flaky {
        let data: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
        Flaky { data, cut_after, cuts: std::cell::Cell::new(cuts), ignore_range, opens: Default::default() }
    }

    fn temp_part(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("scriibe-recv-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("x.part")
    }

    #[test]
    fn a_cut_connection_resumes_where_it_stopped() {
        let part = temp_part("resume");
        let source = flaky(1_000_000, 300_000, 2, false);
        let got = receive(&source, &part, 1_000_000, Duration::ZERO, &|_, _| {}).unwrap();
        assert_eq!(got, 1_000_000);
        assert_eq!(std::fs::read(&part).unwrap(), source.data, "les octets sont les bons, sans trou ni doublon");
        assert_eq!(*source.opens.borrow(), vec![0, 300_000, 600_000], "chaque reprise part de l'octet atteint");
        let _ = std::fs::remove_dir_all(part.parent().unwrap());
    }

    #[test]
    fn a_part_file_left_by_a_failure_is_resumed_next_time() {
        let part = temp_part("left");
        let source = flaky(500_000, 0, 0, false);
        std::fs::write(&part, &source.data[..200_000]).unwrap();
        let got = receive(&source, &part, 500_000, Duration::ZERO, &|_, _| {}).unwrap();
        assert_eq!((got, std::fs::read(&part).unwrap()), (500_000, source.data.clone()));
        assert_eq!(*source.opens.borrow(), vec![200_000]);
        let _ = std::fs::remove_dir_all(part.parent().unwrap());
    }

    #[test]
    fn a_source_that_ignores_the_range_restarts_from_zero_without_corrupting_the_file() {
        let part = temp_part("norange");
        let source = flaky(400_000, 150_000, 1, true);
        let got = receive(&source, &part, 400_000, Duration::ZERO, &|_, _| {}).unwrap();
        assert_eq!((got, std::fs::read(&part).unwrap()), (400_000, source.data.clone()));
        let _ = std::fs::remove_dir_all(part.parent().unwrap());
    }

    #[test]
    fn too_many_cuts_give_up_but_keep_what_was_received() {
        let part = temp_part("giveup");
        let source = flaky(1_000_000, 100_000, 100, false);
        let err = receive(&source, &part, 1_000_000, Duration::ZERO, &|_, _| {}).unwrap_err();
        assert!(err.contains("gardé"), "{err}");
        assert!(std::fs::metadata(&part).unwrap().len() >= 100_000);
        let _ = std::fs::remove_dir_all(part.parent().unwrap());
    }

    #[test]
    fn a_part_file_larger_than_the_announced_size_is_dropped() {
        let part = temp_part("toobig");
        let source = flaky(100_000, 0, 0, false);
        std::fs::write(&part, vec![7u8; 150_000]).unwrap();
        let got = receive(&source, &part, 100_000, Duration::ZERO, &|_, _| {}).unwrap();
        assert_eq!((got, std::fs::read(&part).unwrap()), (100_000, source.data.clone()));
        let _ = std::fs::remove_dir_all(part.parent().unwrap());
    }

    #[test]
    fn the_total_comes_from_the_content_range_header() {
        assert_eq!(total_from_content_range("bytes 100-999/1000"), Some(1000));
        assert_eq!(total_from_content_range("bytes */1000"), Some(1000));
        assert_eq!(total_from_content_range("bytes 0-9/*"), None);
    }

    #[test]
    fn every_item_has_a_distinct_id() {
        let mut ids: Vec<_> = ITEMS.iter().map(|i| i.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), ITEMS.len());
    }
}
