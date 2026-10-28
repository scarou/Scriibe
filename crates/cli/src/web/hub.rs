//! Choisir un modèle sur Hugging Face : chercher un dépôt, lister ses fichiers avec leur vraie taille, et gérer les modèles déjà
//! installés. Rien ne se télécharge ici : la page passe ensuite par `downloads` (nom, taille, source, confirmation).

use scriibe_engines::fs::FileSystem;
use scriibe_engines::whisper;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Ce qu'on cherche : un modèle de mise au propre (GGUF pour `llama-server`) ou de transcription (`ggml-*.bin` pour `whisper-server`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Llama,
    Whisper,
}

impl Kind {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "llama" => Some(Kind::Llama),
            "whisper" => Some(Kind::Whisper),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Llama => "llama",
            Kind::Whisper => "whisper",
        }
    }

    /// Le fichier est-il de ce genre ? (même règle que la détection des modèles installés)
    pub fn accepts(self, name: &str) -> bool {
        match self {
            Kind::Llama => name.to_lowercase().ends_with(".gguf"),
            Kind::Whisper => whisper::is_whisper_model(name),
        }
    }
}

/// Ce que la personne a tapé : un dépôt (« owner/nom », ou son adresse, ou l'adresse d'un de ses fichiers), ou des mots à chercher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Query {
    Repo { repo: String, file: Option<String> },
    Search(String),
}

fn valid_part(part: &str) -> bool {
    !part.is_empty() && part != "." && part != ".." && part.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// « owner/nom » valide (lettres, chiffres, « . », « _ », « - ») : seul ce qui passe ce contrôle est mis dans une adresse.
pub fn valid_repo(repo: &str) -> bool {
    let mut parts = repo.split('/');
    matches!((parts.next(), parts.next(), parts.next()), (Some(owner), Some(name), None) if valid_part(owner) && valid_part(name))
}

/// Un nom de fichier seul (pas de dossier) : le fichier est rangé tel quel dans le dossier des modèles.
pub fn valid_file(file: &str) -> bool {
    valid_part(file)
}

pub fn parse_query(input: &str) -> Query {
    let text = input.trim();
    let mut rest = text;
    for prefix in ["https://huggingface.co/", "http://huggingface.co/", "huggingface.co/", "https://hf.co/", "hf.co/"] {
        if let Some(after) = rest.strip_prefix(prefix) {
            rest = after;
            break;
        }
    }
    let rest = rest.split(['?', '#']).next().unwrap_or("");
    let mut parts = rest.split('/').filter(|p| !p.is_empty());
    if let (Some(owner), Some(name)) = (parts.next(), parts.next()) {
        // « owner/nom:Q4_K_M » (écriture d'Ollama) : la quantification se choisit dans la liste des fichiers.
        let name = name.split(':').next().unwrap_or(name);
        if valid_part(owner) && valid_part(name) {
            let file = match parts.next() {
                Some("blob" | "resolve") => {
                    parts.next();
                    let tail: Vec<&str> = parts.collect();
                    (tail.len() == 1 && valid_file(tail[0])).then(|| tail[0].to_string())
                }
                _ => None,
            };
            return Query::Repo { repo: format!("{owner}/{name}"), file };
        }
    }
    Query::Search(text.to_string())
}

/// Un fichier d'un dépôt, tel qu'on le propose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HubFile {
    pub name: String,
    pub size: u64,
    /// Ce qu'il faut savoir avant de le prendre (assistant, vision, plusieurs parties).
    pub note: Option<&'static str>,
    pub downloadable: bool,
}

/// « …-00001-of-00003.gguf » : un modèle coupé en plusieurs fichiers, qu'il faudrait tous avoir.
fn is_split_part(name: &str) -> bool {
    let lower = name.to_lowercase();
    let Some(stem) = lower.strip_suffix(".gguf") else { return false };
    let Some((before, after)) = stem.rsplit_once("-of-") else { return false };
    after.chars().all(|c| c.is_ascii_digit())
        && !after.is_empty()
        && before.rsplit('-').next().is_some_and(|n| n.len() >= 3 && n.chars().all(|c| c.is_ascii_digit()))
}

/// Les fichiers de ce genre dans la liste d'un dépôt (`/api/models/<dépôt>/tree/main`), par ordre alphabétique.
pub fn files_from_tree(tree: &Value, kind: Kind) -> Vec<HubFile> {
    let mut files: Vec<HubFile> = tree
        .as_array()
        .into_iter()
        .flatten()
        .filter(|e| e["type"].as_str().is_none_or(|t| t == "file"))
        .filter_map(|e| {
            let name = e["path"].as_str()?;
            if !valid_file(name) || !kind.accepts(name) {
                return None;
            }
            let lower = name.to_lowercase();
            let size = e["lfs"]["size"].as_u64().or_else(|| e["size"].as_u64()).unwrap_or(0);
            let (note, downloadable) = if kind == Kind::Llama && is_split_part(name) {
                (Some("modèle en plusieurs fichiers : pas encore pris en charge"), false)
            } else if lower.starts_with("mtp-") {
                (Some("assistant de décodage (MTP) : à mettre à côté du modèle du même nom"), true)
            } else if lower.starts_with("mmproj") {
                (Some("partie « vision » : inutile pour dicter"), true)
            } else {
                (None, true)
            };
            Some(HubFile { name: name.to_string(), size, note, downloadable })
        })
        .collect();
    files.sort_by_key(|f| f.name.to_lowercase());
    files
}

/// Un dépôt trouvé par la recherche.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HubRepo {
    pub repo: String,
    pub downloads: u64,
    pub likes: u64,
}

pub fn repos_from_search(list: &Value) -> Vec<HubRepo> {
    list.as_array()
        .into_iter()
        .flatten()
        .filter_map(|m| {
            let repo = m["id"].as_str().or_else(|| m["modelId"].as_str())?;
            valid_repo(repo).then(|| HubRepo {
                repo: repo.to_string(),
                downloads: m["downloads"].as_u64().unwrap_or(0),
                likes: m["likes"].as_u64().unwrap_or(0),
            })
        })
        .collect()
}

/// Le critère de tri de Hugging Face pour le choix de la page (« popular », « trending », « recent »).
pub fn sort_key(choice: &str) -> &'static str {
    match choice {
        "trending" => "trendingScore",
        "recent" => "createdAt",
        _ => "downloads",
    }
}

/// L'identifiant d'un téléchargement choisi dans cette page : `hf:<dépôt>/<fichier>` (relu et revérifié au moment de télécharger).
pub fn download_id(repo: &str, file: &str) -> String {
    format!("hf:{repo}/{file}")
}

/// Relit un identifiant `hf:<dépôt>/<fichier>` : le dépôt et le fichier doivent être valides, et le fichier d'un genre connu.
pub fn parse_download_id(id: &str) -> Option<(String, String, Kind)> {
    let rest = id.strip_prefix("hf:")?;
    let (repo, file) = rest.rsplit_once('/')?;
    if !valid_repo(repo) || !valid_file(file) {
        return None;
    }
    let kind = [Kind::Whisper, Kind::Llama].into_iter().find(|k| k.accepts(file))?;
    Some((repo.to_string(), file.to_string(), kind))
}

/// Un fichier de modèle installé dans le dossier des modèles de l'appli.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installed {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    /// « llama », « whisper » ou « partial » (téléchargement interrompu, repris au prochain essai).
    pub kind: &'static str,
}

/// Les modèles du dossier des modèles de l'appli (les autres dossiers, comme celui de LM Studio, ne sont pas à elle).
pub fn installed(fs: &dyn FileSystem, folder: &Path, size: impl Fn(&Path) -> u64) -> Vec<Installed> {
    let mut found: Vec<Installed> = fs
        .walk(folder)
        .into_iter()
        .filter_map(|path| {
            let name = path.file_name()?.to_string_lossy().into_owned();
            let kind = if name.to_lowercase().ends_with(".part") {
                "partial"
            } else if Kind::Whisper.accepts(&name) {
                "whisper"
            } else if Kind::Llama.accepts(&name) {
                "llama"
            } else {
                return None;
            };
            let size = size(&path);
            Some(Installed { name, path, size, kind })
        })
        .collect();
    found.sort_by(|a, b| (a.kind, a.name.to_lowercase()).cmp(&(b.kind, b.name.to_lowercase())));
    found
}

/// Supprime un modèle installé. Refuse tout ce qui n'est pas un fichier de modèle (ou un `.part`) du dossier des modèles de l'appli.
pub fn delete_installed(folder: &Path, path: &Path) -> Result<(), String> {
    let folder = folder.canonicalize().map_err(|e| format!("dossier des modèles : {e}"))?;
    let target = path.canonicalize().map_err(|e| format!("fichier introuvable ({e})"))?;
    if !target.starts_with(&folder) || !target.is_file() {
        return Err("ce fichier n'est pas dans le dossier des modèles de Scriibe".into());
    }
    let name = target.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    if !(name.ends_with(".gguf") || name.ends_with(".bin") || name.ends_with(".part")) {
        return Err("ce n'est pas un fichier de modèle".into());
    }
    std::fs::remove_file(&target)
        .map_err(|e| format!("suppression impossible ({e}) : le modèle est peut-être chargé par un moteur en marche"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_repo_is_recognised_from_its_name_its_address_or_a_file_address() {
        let repo = |r: &str, f: Option<&str>| Query::Repo { repo: r.into(), file: f.map(Into::into) };
        assert_eq!(parse_query("Qwen/Qwen2.5-3B-Instruct-GGUF"), repo("Qwen/Qwen2.5-3B-Instruct-GGUF", None));
        assert_eq!(parse_query("  https://huggingface.co/unsloth/gemma-3-4b-it-GGUF  "), repo("unsloth/gemma-3-4b-it-GGUF", None));
        assert_eq!(parse_query("https://huggingface.co/unsloth/gemma-3-4b-it-GGUF/tree/main"), repo("unsloth/gemma-3-4b-it-GGUF", None));
        assert_eq!(
            parse_query("https://huggingface.co/unsloth/gemma-3-4b-it-GGUF/blob/main/gemma-3-4b-it-Q4_K_M.gguf?download=true"),
            repo("unsloth/gemma-3-4b-it-GGUF", Some("gemma-3-4b-it-Q4_K_M.gguf"))
        );
        assert_eq!(parse_query("hf.co/bartowski/Llama-3.2-3B-Instruct-GGUF:Q4_K_M"), repo("bartowski/Llama-3.2-3B-Instruct-GGUF", None));
    }

    #[test]
    fn anything_else_is_a_search_and_nothing_unsafe_becomes_an_address() {
        assert_eq!(parse_query("qwen 3 instruct"), Query::Search("qwen 3 instruct".into()));
        assert_eq!(parse_query("gemma"), Query::Search("gemma".into()));
        assert_eq!(parse_query("a b/c d"), Query::Search("a b/c d".into()));
        assert_eq!(parse_query("../etc/passwd"), Query::Search("../etc/passwd".into()));
        assert!(!valid_repo("a/b/c") && !valid_repo("a/..") && !valid_repo("a") && valid_repo("Qwen/Qwen2.5-0.5B-Instruct-GGUF"));
        assert!(!valid_file("../x.gguf") && !valid_file("a/b.gguf") && !valid_file("a b.gguf") && valid_file("model-Q4_K_M.gguf"));
    }

    #[test]
    fn the_files_of_a_repo_are_listed_with_their_real_size_and_what_to_know_about_them() {
        let tree = json!([
            {"type": "file", "path": "README.md", "size": 1200},
            {"type": "file", "path": "gemma-3-4b-it-Q8_0.gguf", "size": 133, "lfs": {"size": 4_130_000_000u64}},
            {"type": "file", "path": "gemma-3-4b-it-Q4_K_M.gguf", "lfs": {"size": 2_490_000_000u64}},
            {"type": "file", "path": "mmproj-F16.gguf", "lfs": {"size": 851_000_000u64}},
            {"type": "file", "path": "mtp-gemma-3-4b-it-Q8_0.gguf", "lfs": {"size": 40_000_000u64}},
            {"type": "file", "path": "big-Q4_K_M-00001-of-00003.gguf", "lfs": {"size": 9_000_000_000u64}},
            {"type": "directory", "path": "BF16"},
        ]);
        let files = files_from_tree(&tree, Kind::Llama);
        let names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "big-Q4_K_M-00001-of-00003.gguf",
                "gemma-3-4b-it-Q4_K_M.gguf",
                "gemma-3-4b-it-Q8_0.gguf",
                "mmproj-F16.gguf",
                "mtp-gemma-3-4b-it-Q8_0.gguf"
            ]
        );
        assert_eq!(files[1].size, 2_490_000_000, "la taille du fichier réel (LFS), pas celle du pointeur");
        assert!(!files[0].downloadable && files[0].note.is_some());
        assert!(files[1].downloadable && files[1].note.is_none());
        assert!(files[3].note.unwrap().contains("vision") && files[4].note.unwrap().contains("MTP"));
        assert!(files_from_tree(&tree, Kind::Whisper).is_empty());
    }

    #[test]
    fn whisper_models_are_the_ggml_files_that_whisper_server_can_load() {
        let tree = json!([
            {"type": "file", "path": "ggml-base.bin", "lfs": {"size": 147_951_465u64}},
            {"type": "file", "path": "ggml-large-v3-turbo-q5_0.bin", "lfs": {"size": 574_041_195u64}},
            {"type": "file", "path": "for-tests-ggml-base.bin", "size": 10},
            {"type": "file", "path": "ggml-silero-v5.1.2.bin", "size": 10},
            {"type": "file", "path": "model.gguf", "size": 10},
        ]);
        let names: Vec<String> = files_from_tree(&tree, Kind::Whisper).into_iter().map(|f| f.name).collect();
        assert_eq!(names, ["ggml-base.bin", "ggml-large-v3-turbo-q5_0.bin"]);
    }

    #[test]
    fn split_models_are_recognised_by_their_part_numbers_only() {
        assert!(is_split_part("Model-Q4_K_M-00001-of-00003.gguf"));
        assert!(!is_split_part("Model-Q4_K_M.gguf"));
        assert!(!is_split_part("model-of-the-year.gguf"));
    }

    #[test]
    fn search_results_keep_valid_repos_with_their_counts() {
        let list = json!([
            {"id": "Qwen/Qwen3-4B-GGUF", "downloads": 120_000, "likes": 300},
            {"modelId": "unsloth/gemma-3-4b-it-GGUF", "downloads": 9},
            {"id": "pas valide", "downloads": 1},
            {"downloads": 1},
        ]);
        assert_eq!(
            repos_from_search(&list),
            vec![
                HubRepo { repo: "Qwen/Qwen3-4B-GGUF".into(), downloads: 120_000, likes: 300 },
                HubRepo { repo: "unsloth/gemma-3-4b-it-GGUF".into(), downloads: 9, likes: 0 },
            ]
        );
        assert_eq!(
            (sort_key("trending"), sort_key("recent"), sort_key("popular"), sort_key("x")),
            ("trendingScore", "createdAt", "downloads", "downloads")
        );
    }

    #[test]
    fn a_download_id_is_read_back_and_rechecked() {
        let id = download_id("Qwen/Qwen3-4B-GGUF", "Qwen3-4B-Q4_K_M.gguf");
        assert_eq!(parse_download_id(&id), Some(("Qwen/Qwen3-4B-GGUF".into(), "Qwen3-4B-Q4_K_M.gguf".into(), Kind::Llama)));
        assert_eq!(parse_download_id("hf:ggerganov/whisper.cpp/ggml-small.bin").map(|t| t.2), Some(Kind::Whisper));
        for bad in ["qwen-3b", "hf:Qwen/x/../y.gguf", "hf:Qwen/Qwen3-4B-GGUF/notes.txt", "hf:a/b/c/d.gguf", "hf:/x.gguf"] {
            assert_eq!(parse_download_id(bad), None, "{bad}");
        }
    }

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("scriibe-hub-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn installed_models_are_listed_by_kind_and_deleted_only_inside_the_folder() {
        let dir = temp("installed");
        let outside = temp("outside");
        for (folder, name) in [
            (&dir, "qwen2.5-1.5b-instruct-q4_k_m.gguf"),
            (&dir, "ggml-base.bin"),
            (&dir, "ggml-small.bin.part"),
            (&dir, "notes.txt"),
            (&outside, "autre.gguf"),
        ] {
            std::fs::write(folder.join(name), b"xxxx").unwrap();
        }
        let list = installed(&scriibe_engines::fs::RealFs, &dir, |p| std::fs::metadata(p).map_or(0, |m| m.len()));
        let seen: Vec<(&str, &str)> = list.iter().map(|m| (m.kind, m.name.as_str())).collect();
        assert_eq!(
            seen,
            [("llama", "qwen2.5-1.5b-instruct-q4_k_m.gguf"), ("partial", "ggml-small.bin.part"), ("whisper", "ggml-base.bin")]
        );
        assert!(list.iter().all(|m| m.size == 4));

        assert!(delete_installed(&dir, &outside.join("autre.gguf")).is_err(), "hors du dossier des modèles");
        assert!(outside.join("autre.gguf").exists());
        assert!(delete_installed(&dir, &dir.join("notes.txt")).is_err(), "pas un modèle");
        assert!(
            delete_installed(&dir, &dir.join("..").join(outside.file_name().unwrap()).join("autre.gguf")).is_err(),
            "chemin qui remonte"
        );
        delete_installed(&dir, &dir.join("ggml-base.bin")).unwrap();
        assert!(!dir.join("ggml-base.bin").exists());
        assert!(delete_installed(&dir, &dir.join("absent.gguf")).is_err());
        let _ = std::fs::remove_dir_all(dir);
        let _ = std::fs::remove_dir_all(outside);
    }
}
