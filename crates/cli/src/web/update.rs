//! Mise à jour de `scriibe.exe` depuis la page d'essais : la dernière exécution verte de la CI sur GitHub (le dépôt est privé : un jeton
//! en lecture seule, gardé dans le Gestionnaire d'identifiants, voir `KeyKind::GitHub`). Comme pour les téléchargements, rien ne part
//! sans que la personne ait vu la version, la taille et la source, puis confirmé. Le jeton n'est jamais affiché ni journalisé.

use scriibe_engines::runtime::Log;
use serde_json::Value;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

pub const REPO: &str = "scarou/Scriibe";
const WORKFLOW: &str = "ci.yml";
const ARTIFACT: &str = "scriibe-windows-x64";
const PROGRAM: &str = "scriibe.exe";

/// La version installée : le commit compilé (celui de la CI, ou lu dans git pour une compilation à la main ; absent hors d'un dépôt git).
pub fn current_sha() -> Option<&'static str> {
    Some(env!("SCRIIBE_COMMIT")).filter(|s| !s.is_empty())
}

/// Une version plus récente que celle qui tourne (trouvée par la vérification automatique).
#[derive(Debug, Clone, PartialEq)]
pub struct Available {
    pub sha: String,
    pub title: String,
    pub date: String,
}

impl Available {
    pub fn short_sha(&self) -> &str {
        &self.sha[..self.sha.len().min(7)]
    }
}

/// GitHub dit si le commit `latest` est en avance sur `current` (« ahead ») ; « identical », « behind » ou « diverged » (une compilation
/// qui a des commits à elle) ne déclenchent pas d'alerte.
fn is_ahead(compare: &Value) -> bool {
    compare["status"].as_str() == Some("ahead")
}

/// Cherche s'il existe une version plus récente que la version installée : la dernière exécution verte de la CI sur `main`, comparée à
/// `current`. Rien n'est téléchargé ; `None` quand la version installée est la dernière, ou plus récente.
pub fn newer(token: &str, current: &str) -> Result<Option<Available>, String> {
    let runs = get_json(
        &format!("https://api.github.com/repos/{REPO}/actions/workflows/{WORKFLOW}/runs?branch=main&status=success&per_page=1"),
        token,
    )?;
    let (_, sha, title, date) = parse_run(&runs)?;
    if sha == current {
        return Ok(None);
    }
    let compare = get_json(&format!("https://api.github.com/repos/{REPO}/compare/{current}...{sha}"), token)?;
    Ok(is_ahead(&compare).then_some(Available { sha, title, date }))
}

/// La dernière version disponible.
#[derive(Debug, Clone, PartialEq)]
pub struct Latest {
    pub run_id: u64,
    pub sha: String,
    /// Première ligne du message du commit.
    pub title: String,
    pub date: String,
    pub size: u64,
    pub download_url: String,
}

impl Latest {
    pub fn short_sha(&self) -> &str {
        &self.sha[..self.sha.len().min(7)]
    }

    pub fn up_to_date(&self) -> bool {
        current_sha().is_some_and(|current| current == self.sha)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum State {
    Idle,
    Running {
        done: u64,
        total: u64,
    },
    /// Installée : le programme va se relancer.
    Installed,
    Failed(String),
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

/// Sans suivre les redirections : l'en-tête d'autorisation ne doit jamais partir vers un autre serveur que GitHub.
fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new().redirects(0).timeout_connect(Duration::from_secs(15)).timeout_read(Duration::from_secs(60)).build()
}

/// Une erreur sans l'adresse (celle d'un téléchargement contient une signature temporaire).
fn describe(error: &ureq::Error) -> String {
    match error {
        ureq::Error::Status(code, _) => format!("réponse {code}"),
        ureq::Error::Transport(t) => format!("connexion impossible : {}", t.kind()),
    }
}

/// La forme d'un jeton, jamais sa valeur : de quoi voir qu'un collage l'a abîmé (trop court, trop long, caractères étrangers).
fn token_shape(token: &str) -> String {
    let kind = if token.starts_with("github_pat_") {
        "type github_pat_"
    } else if token.starts_with("ghp_") {
        "type ghp_"
    } else {
        "type inconnu : un jeton GitHub commence par github_pat_ ou ghp_"
    };
    let odd = token.chars().filter(|c| !(c.is_ascii_alphanumeric() || *c == '_')).count();
    format!("jeton de {} caractères, {kind}, {odd} caractère(s) inhabituel(s)", token.chars().count())
}

fn get_authorized(url: &str, token: &str) -> Result<ureq::Response, String> {
    let result = agent()
        .get(url)
        .set("Authorization", &format!("Bearer {token}"))
        .set("Accept", "application/vnd.github+json")
        .set("X-GitHub-Api-Version", "2022-11-28")
        .set("User-Agent", "scriibe")
        .timeout(Duration::from_secs(30))
        .call();
    match result {
        Ok(response) => Ok(response),
        Err(ureq::Error::Status(401, _)) => {
            Err(format!("GitHub refuse le jeton (401) : faux, expiré ou à recréer ({})", token_shape(token)))
        }
        Err(ureq::Error::Status(code @ (403 | 404), response)) => {
            // GitHub dit lui-même ce qui manque : permissions exigées par la route et message (jamais le jeton).
            let needed = response.header("x-accepted-github-permissions").unwrap_or("").to_string();
            let message = response
                .into_string()
                .ok()
                .and_then(|b| serde_json::from_str::<serde_json::Value>(&b).ok())
                .and_then(|v| v["message"].as_str().map(String::from))
                .unwrap_or_default();
            Err(format!(
                "GitHub refuse l'accès ({code}) : le jeton doit pouvoir lire « Actions » du dépôt {REPO} (lecture seule suffit). Réponse de GitHub : « {message} »{}",
                if needed.is_empty() { String::new() } else { format!(" ; permissions demandées : {needed}") }
            ))
        }
        Err(e) => Err(format!("GitHub injoignable ({})", describe(&e))),
    }
}

fn get_json(url: &str, token: &str) -> Result<Value, String> {
    let response = get_authorized(url, token)?;
    if response.status() != 200 {
        return Err(format!("GitHub a répondu {} à une demande de liste", response.status()));
    }
    serde_json::from_reader(response.into_reader()).map_err(|e| format!("réponse de GitHub illisible ({e})"))
}

/// Première ligne d'un message de commit.
fn title_of(message: &str) -> String {
    message.lines().next().unwrap_or("").trim().to_string()
}

/// La dernière exécution verte de la CI : son commit, son titre, sa date.
fn parse_run(runs: &Value) -> Result<(u64, String, String, String), String> {
    let run = runs["workflow_runs"].get(0).ok_or("aucune exécution verte de la CI trouvée sur la branche main")?;
    let id = run["id"].as_u64().ok_or("numéro d'exécution absent")?;
    let sha = run["head_sha"].as_str().ok_or("commit absent")?.to_string();
    let title = title_of(run["head_commit"]["message"].as_str().unwrap_or(""));
    let date = run["created_at"].as_str().unwrap_or("").to_string();
    Ok((id, sha, title, date))
}

/// La pièce jointe `scriibe-windows-x64` : sa taille et l'adresse de son zip.
fn parse_artifact(artifacts: &Value) -> Result<(u64, String), String> {
    let found = artifacts["artifacts"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|a| a["name"].as_str() == Some(ARTIFACT))
        .ok_or_else(|| format!("pas de pièce jointe « {ARTIFACT} » dans la dernière exécution"))?;
    if found["expired"].as_bool() == Some(true) {
        return Err("la pièce jointe a expiré (GitHub les garde 30 jours) : pousse un nouveau commit".into());
    }
    let url = found["archive_download_url"].as_str().ok_or("adresse de téléchargement absente")?.to_string();
    Ok((found["size_in_bytes"].as_u64().unwrap_or(0), url))
}

/// Cherche la dernière version (aucun téléchargement).
pub fn latest(token: &str) -> Result<Latest, String> {
    if !cfg!(windows) {
        return Err("la mise à jour ne marche que sous Windows (c'est scriibe.exe qui est publié)".into());
    }
    let runs = get_json(
        &format!("https://api.github.com/repos/{REPO}/actions/workflows/{WORKFLOW}/runs?branch=main&status=success&per_page=1"),
        token,
    )?;
    let (run_id, sha, title, date) = parse_run(&runs)?;
    let artifacts = get_json(&format!("https://api.github.com/repos/{REPO}/actions/runs/{run_id}/artifacts"), token)?;
    let (size, download_url) = parse_artifact(&artifacts)?;
    Ok(Latest { run_id, sha, title, date, size, download_url })
}

/// Télécharge le zip : GitHub renvoie vers un stockage signé (sans le jeton, qui n'y va pas).
fn download(token: &str, url: &str, expected: u64, progress: &dyn Fn(u64, u64)) -> Result<Vec<u8>, String> {
    let first = get_authorized(url, token)?;
    let response = if (300..400).contains(&first.status()) {
        let location = first.header("Location").ok_or("redirection sans adresse")?.to_string();
        if !location.starts_with("https://") {
            return Err("redirection vers une adresse non sécurisée : refusée".into());
        }
        agent().get(&location).set("User-Agent", "scriibe").call().map_err(|e| format!("téléchargement refusé ({})", describe(&e)))?
    } else {
        first
    };
    let total = response.header("Content-Length").and_then(|v| v.parse().ok()).unwrap_or(expected);
    let mut reader = response.into_reader();
    let mut data = Vec::with_capacity(total as usize);
    let mut buffer = vec![0u8; 256 * 1024];
    loop {
        let n = reader.read(&mut buffer).map_err(|e| format!("connexion interrompue après {} Mo ({e})", data.len() / 1_048_576))?;
        if n == 0 {
            break;
        }
        data.extend_from_slice(&buffer[..n]);
        progress(data.len() as u64, total);
    }
    if total > 0 && (data.len() as u64) < total {
        return Err(format!("fichier incomplet ({} sur {} Mo)", data.len() / 1_048_576, total / 1_048_576));
    }
    Ok(data)
}

/// Le programme contenu dans le zip de la CI.
fn extract_program(zip_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(zip_bytes)).map_err(|e| format!("zip illisible ({e})"))?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let is_program = entry.enclosed_name().and_then(|p| p.file_name().map(|n| n.to_owned())).is_some_and(|n| n == PROGRAM);
        if entry.is_file() && is_program {
            let mut program = Vec::new();
            entry.read_to_end(&mut program).map_err(|e| e.to_string())?;
            // Un exécutable Windows commence par « MZ » ; un fichier minuscule n'est pas un programme.
            if program.len() < 100_000 || !program.starts_with(b"MZ") {
                return Err("le fichier du zip ne ressemble pas à un programme Windows".into());
            }
            return Ok(program);
        }
    }
    Err(format!("{PROGRAM} n'est pas dans le zip"))
}

fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| PROGRAM.into());
    path.with_file_name(format!("{name}{suffix}"))
}

/// Remplace le programme : Windows interdit d'écraser un exécutable en cours, mais permet de le renommer. L'ancien devient
/// `scriibe.exe.old` (supprimé au prochain lancement) ; si quelque chose échoue, l'ancien est remis en place.
fn replace_program(current: &Path, program: &[u8]) -> Result<(), String> {
    let (fresh, old) = (sibling(current, ".new"), sibling(current, ".old"));
    let folder = current.parent().map(|p| p.display().to_string()).unwrap_or_default();
    std::fs::File::create(&fresh)
        .and_then(|mut f| f.write_all(program).and_then(|()| f.flush()))
        .map_err(|e| format!("écriture impossible dans {folder} ({e}) : place scriibe.exe dans un dossier où tu peux écrire"))?;
    let _ = std::fs::remove_file(&old);
    if let Err(e) = std::fs::rename(current, &old) {
        let _ = std::fs::remove_file(&fresh);
        return Err(format!("l'ancienne version ne peut pas être renommée ({e})"));
    }
    if let Err(e) = std::fs::rename(&fresh, current) {
        let _ = std::fs::rename(&old, current);
        let _ = std::fs::remove_file(&fresh);
        return Err(format!("la nouvelle version ne peut pas être mise en place ({e}) ; l'ancienne est remise"));
    }
    Ok(())
}

/// Au lancement : supprime l'ancienne version laissée par une mise à jour. Elle tourne encore quelques secondes (le temps qu'elle
/// s'arrête), donc on réessaie un peu, sans rien bloquer.
pub fn remove_leftovers() {
    let Ok(current) = std::env::current_exe() else { return };
    let old = sibling(&current, ".old");
    if !old.exists() {
        return;
    }
    let _ = std::thread::Builder::new().name("scriibe-nettoyage".into()).spawn(move || {
        for _ in 0..15 {
            if std::fs::remove_file(&old).is_ok() || !old.exists() {
                return;
            }
            std::thread::sleep(Duration::from_secs(2));
        }
    });
}

#[derive(Default)]
pub struct Updater {
    state: Mutex<Option<State>>,
}

impl Updater {
    pub fn state(&self) -> State {
        lock(&self.state).clone().unwrap_or(State::Idle)
    }

    fn set(&self, state: State) {
        *lock(&self.state) = Some(state);
    }

    /// Télécharge, installe, puis appelle `restart` (qui relance le programme). Le journal reçoit le commit, la taille et la source.
    pub fn start(
        self: &Arc<Self>,
        token: String,
        latest: Latest,
        log: Arc<dyn Log>,
        restart: impl FnOnce() + Send + 'static,
    ) -> Result<(), String> {
        if !cfg!(windows) {
            return Err("la mise à jour ne marche que sous Windows".into());
        }
        let current = std::env::current_exe().map_err(|e| format!("emplacement du programme inconnu ({e})"))?;
        {
            let mut state = lock(&self.state);
            if matches!(*state, Some(State::Running { .. } | State::Installed)) {
                return Err("déjà en cours".into());
            }
            *state = Some(State::Running { done: 0, total: latest.size });
        }
        log.write(&format!(
            "mise à jour: {} ({}, {} Mo) depuis GitHub {REPO} (exécution {}) vers {} — demandée et confirmée dans l'interface",
            latest.short_sha(),
            latest.title,
            latest.size / 1_048_576,
            latest.run_id,
            current.display()
        ));
        let me = Arc::clone(self);
        std::thread::spawn(move || {
            let outcome = download(&token, &latest.download_url, latest.size, &|done, total| me.set(State::Running { done, total }))
                .and_then(|zip| extract_program(&zip))
                .and_then(|program| replace_program(&current, &program));
            match outcome {
                Ok(()) => {
                    log.write("mise à jour: installée, redémarrage");
                    me.set(State::Installed);
                    // De quoi laisser la page lire « installée » avant que le programme s'arrête.
                    std::thread::sleep(Duration::from_millis(1500));
                    restart();
                }
                Err(e) => {
                    log.write(&format!("mise à jour: échec ({e})"));
                    me.set(State::Failed(e));
                }
            }
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("scriibe-maj-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn fake_program(tag: u8) -> Vec<u8> {
        let mut bytes = b"MZ".to_vec();
        bytes.resize(200_000, tag);
        bytes
    }

    fn zip_with(entries: &[(&str, Vec<u8>)]) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        for (name, bytes) in entries {
            writer.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    #[test]
    fn the_latest_green_run_gives_commit_title_and_date() {
        let runs = json!({"workflow_runs": [{"id": 7, "head_sha": "767010f3661de6fd", "created_at": "2026-10-26T19:27:55Z",
            "head_commit": {"message": "Préchargement des modèles\n\nCo-Authored-By: quelqu'un"}}]});
        let (id, sha, title, date) = parse_run(&runs).unwrap();
        assert_eq!(
            (id, sha.as_str(), title.as_str(), date.as_str()),
            (7, "767010f3661de6fd", "Préchargement des modèles", "2026-10-26T19:27:55Z")
        );
        assert!(parse_run(&json!({"workflow_runs": []})).is_err(), "aucune exécution verte");
    }

    #[test]
    fn the_artifact_is_found_by_name_and_must_not_be_expired() {
        let list = |expired| {
            json!({"artifacts": [
                {"name": "autre", "size_in_bytes": 1, "expired": false, "archive_download_url": "https://x/autre"},
                {"name": ARTIFACT, "size_in_bytes": 18343080, "expired": expired, "archive_download_url": "https://api.github.com/a/zip"}]})
        };
        assert_eq!(parse_artifact(&list(false)).unwrap(), (18343080, "https://api.github.com/a/zip".to_string()));
        assert!(parse_artifact(&list(true)).unwrap_err().contains("expiré"));
        assert!(parse_artifact(&json!({"artifacts": []})).is_err());
    }

    #[test]
    fn the_program_is_taken_from_the_zip_and_checked() {
        let good = zip_with(&[("LISEZMOI.txt", b"x".to_vec()), ("scriibe.exe", fake_program(1))]);
        assert_eq!(extract_program(&good).unwrap(), fake_program(1));
        assert!(extract_program(&zip_with(&[("autre.exe", fake_program(1))])).is_err(), "pas de scriibe.exe");
        assert!(extract_program(&zip_with(&[("scriibe.exe", b"MZ".to_vec())])).unwrap_err().contains("programme"), "trop petit");
        assert!(extract_program(&zip_with(&[("scriibe.exe", vec![b'x'; 200_000])])).is_err(), "pas un exécutable");
        assert!(extract_program(b"pas un zip").is_err());
    }

    #[test]
    fn the_program_is_replaced_and_the_old_one_kept_aside() {
        let dir = temp("remplace");
        let current = dir.join("scriibe.exe");
        std::fs::write(&current, fake_program(1)).unwrap();
        replace_program(&current, &fake_program(2)).unwrap();
        assert_eq!(std::fs::read(&current).unwrap(), fake_program(2));
        assert_eq!(std::fs::read(dir.join("scriibe.exe.old")).unwrap(), fake_program(1));
        assert!(!dir.join("scriibe.exe.new").exists());
        // Une deuxième mise à jour remplace l'ancienne copie sans erreur.
        replace_program(&current, &fake_program(3)).unwrap();
        assert_eq!(std::fs::read(dir.join("scriibe.exe.old")).unwrap(), fake_program(2));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_failed_replacement_leaves_the_current_program_untouched() {
        let dir = temp("echec");
        let current = dir.join("introuvable").join("scriibe.exe");
        assert!(replace_program(&current, &fake_program(2)).unwrap_err().contains("écriture impossible"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn only_a_latest_commit_ahead_of_the_installed_one_is_an_update() {
        for (status, ahead) in [("ahead", true), ("identical", false), ("behind", false), ("diverged", false)] {
            assert_eq!(is_ahead(&serde_json::json!({"status": status})), ahead, "{status}");
        }
        assert!(!is_ahead(&serde_json::json!({})));
        assert_eq!(Available { sha: "abcdef1234".into(), title: String::new(), date: String::new() }.short_sha(), "abcdef1");
    }

    #[test]
    fn up_to_date_only_when_the_commit_is_the_same() {
        let latest =
            Latest { run_id: 1, sha: "abcdef1234".into(), title: String::new(), date: String::new(), size: 0, download_url: String::new() };
        assert_eq!(latest.short_sha(), "abcdef1");
        assert!(!latest.up_to_date());
        if let Some(current) = current_sha() {
            assert!(Latest { sha: current.to_string(), ..latest }.up_to_date());
        }
    }
}
