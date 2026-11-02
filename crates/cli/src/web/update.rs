//! Mise à jour de Scriibe : la dernière Release publiée sur GitHub (le dépôt est public : aucune clé ni aucun jeton). La version installée
//! est celle du programme (`CARGO_PKG_VERSION`), comparée au numéro de l'étiquette de la Release (`v0.2.0`). Sous Windows, le zip
//! `scriibe-windows-x64.zip` de la Release remplace `scriibe.exe` ; Mac et Linux sont seulement prévenus. Comme pour les téléchargements,
//! rien ne part sans que la personne ait vu la version, la taille et la source, puis confirmé.

use scriibe_engines::runtime::Log;
use serde_json::Value;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

pub const REPO: &str = "scarou/Scriibe";
/// Le fichier de la Release qui contient `scriibe.exe`.
const ASSET: &str = "scriibe-windows-x64.zip";
const PROGRAM: &str = "scriibe.exe";

/// La version installée : celle du programme (`Cargo.toml`), qui est aussi le numéro de la Release dont il vient.
pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Le commit compilé (celui de la CI, ou lu dans git pour une compilation à la main ; absent hors d'un dépôt git) : pour l'affichage.
pub fn current_commit() -> Option<&'static str> {
    Some(env!("SCRIIBE_COMMIT")).filter(|s| !s.is_empty())
}

/// « v0.2.0 » ou « 0.2.0 » en trois nombres ; `None` pour tout autre format.
fn parse_version(text: &str) -> Option<(u64, u64, u64)> {
    let mut parts = text.trim().trim_start_matches('v').split('.').map(|p| p.parse::<u64>().ok());
    let version = (parts.next()??, parts.next()??, parts.next()??);
    parts.next().is_none().then_some(version)
}

/// Vrai si `candidate` est un numéro de version strictement plus grand que `current` (un format inconnu n'est jamais « plus récent »).
fn is_newer(candidate: &str, current: &str) -> bool {
    matches!((parse_version(candidate), parse_version(current)), (Some(c), Some(k)) if c > k)
}

/// Une version plus récente que celle qui tourne (trouvée par la vérification).
#[derive(Debug, Clone, PartialEq)]
pub struct Available {
    pub version: String,
    pub title: String,
    pub date: String,
}

/// La version plus récente trouvée par la dernière vérification (la page et le menu de l'icône la lisent).
static AVAILABLE: Mutex<Option<Available>> = Mutex::new(None);

pub fn available() -> Option<Available> {
    lock(&AVAILABLE).clone()
}

pub fn set_available(found: Option<Available>) {
    *lock(&AVAILABLE) = found;
}

/// La dernière Release publiée.
#[derive(Debug, Clone, PartialEq)]
pub struct Latest {
    /// Sans le « v » de l'étiquette : « 0.2.0 ».
    pub version: String,
    /// Le titre de la Release.
    pub title: String,
    pub date: String,
    /// Taille du zip Windows (0 : la Release n'en contient pas).
    pub size: u64,
    /// Adresse du zip Windows (vide : la Release n'en contient pas).
    pub download_url: String,
}

impl Latest {
    pub fn is_newer(&self) -> bool {
        is_newer(&self.version, current_version())
    }

    pub fn available(&self) -> Available {
        Available { version: self.version.clone(), title: self.title.clone(), date: self.date.clone() }
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

/// Les redirections sont suivies à la main (HTTPS seulement, trois au plus).
fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new().redirects(0).timeout_connect(Duration::from_secs(15)).timeout_read(Duration::from_secs(60)).build()
}

/// Une erreur sans l'adresse (celle d'un téléchargement peut contenir une signature temporaire).
fn describe(error: &ureq::Error) -> String {
    match error {
        ureq::Error::Status(code, _) => format!("réponse {code}"),
        ureq::Error::Transport(t) => format!("connexion impossible : {}", t.kind()),
    }
}

/// Lit la Release publiée (`parse_release` en tire ce qui sert). Aucune identification : la Release est publique.
fn fetch_release() -> Result<Value, String> {
    let result = agent()
        .get(&format!("https://api.github.com/repos/{REPO}/releases/latest"))
        .set("Accept", "application/vnd.github+json")
        .set("X-GitHub-Api-Version", "2022-11-28")
        .set("User-Agent", "scriibe")
        .timeout(Duration::from_secs(30))
        .call();
    match result {
        Ok(response) => serde_json::from_reader(response.into_reader()).map_err(|e| format!("réponse de GitHub illisible ({e})")),
        Err(ureq::Error::Status(404, _)) => Err("aucune version n'est publiée sur GitHub pour l'instant".into()),
        Err(ureq::Error::Status(403 | 429, _)) => Err("GitHub limite les demandes pour le moment : réessaie dans une heure".into()),
        Err(e) => Err(format!("GitHub injoignable ({})", describe(&e))),
    }
}

/// La Release : version (sans « v »), titre, date, et le zip Windows s'il existe.
fn parse_release(release: &Value) -> Result<Latest, String> {
    let tag = release["tag_name"].as_str().ok_or("étiquette de version absente")?;
    parse_version(tag).ok_or_else(|| format!("numéro de version illisible : « {tag} »"))?;
    let version = tag.trim().trim_start_matches('v').to_string();
    let title = release["name"].as_str().filter(|n| !n.is_empty()).map_or_else(|| format!("Scriibe {version}"), String::from);
    let date = release["published_at"].as_str().unwrap_or("").to_string();
    let asset = release["assets"].as_array().into_iter().flatten().find(|a| a["name"].as_str() == Some(ASSET));
    let size = asset.and_then(|a| a["size"].as_u64()).unwrap_or(0);
    let download_url = asset.and_then(|a| a["browser_download_url"].as_str()).unwrap_or("").to_string();
    Ok(Latest { version, title, date, size, download_url })
}

/// Cherche la dernière version publiée (aucun téléchargement).
pub fn latest() -> Result<Latest, String> {
    parse_release(&fetch_release()?)
}

/// Télécharge le zip : GitHub renvoie vers un stockage signé (https seulement).
fn download(url: &str, expected: u64, progress: &dyn Fn(u64, u64)) -> Result<Vec<u8>, String> {
    let mut address = url.to_string();
    let mut hops = 0;
    let response = loop {
        if !address.starts_with("https://") {
            return Err("adresse non sécurisée : téléchargement refusé".into());
        }
        let response = match agent().get(&address).set("User-Agent", "scriibe").call() {
            Ok(r) => r,
            Err(e) => return Err(format!("téléchargement refusé ({})", describe(&e))),
        };
        if (300..400).contains(&response.status()) {
            hops += 1;
            if hops > 3 {
                return Err("trop de redirections".into());
            }
            address = response.header("Location").ok_or("redirection sans adresse")?.to_string();
        } else {
            break response;
        }
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

/// Le programme contenu dans le zip de la Release.
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
    pub fn start(self: &Arc<Self>, latest: Latest, log: Arc<dyn Log>, restart: impl FnOnce() + Send + 'static) -> Result<(), String> {
        if !cfg!(windows) {
            return Err("la mise à jour ne marche que sous Windows".into());
        }
        if latest.download_url.is_empty() {
            return Err(format!("cette version ne contient pas de fichier pour Windows ({ASSET})"));
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
            "mise à jour: version {} ({}, {} Mo) depuis GitHub {REPO} vers {} — demandée et confirmée dans l'interface",
            latest.version,
            latest.title,
            latest.size / 1_048_576,
            current.display()
        ));
        let me = Arc::clone(self);
        std::thread::spawn(move || {
            let outcome = download(&latest.download_url, latest.size, &|done, total| me.set(State::Running { done, total }))
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
    fn the_release_gives_version_title_date_and_the_windows_zip() {
        let release = json!({"tag_name": "v0.2.0", "name": "Scriibe 0.2.0", "published_at": "2026-11-02T10:00:00Z", "assets": [
            {"name": "Scriibe-0.2.0-macos.zip", "size": 21359614, "browser_download_url": "https://github.com/x/mac.zip"},
            {"name": ASSET, "size": 18343080, "browser_download_url": "https://github.com/x/win.zip"}]});
        let latest = parse_release(&release).unwrap();
        assert_eq!(
            (latest.version.as_str(), latest.title.as_str(), latest.date.as_str(), latest.size, latest.download_url.as_str()),
            ("0.2.0", "Scriibe 0.2.0", "2026-11-02T10:00:00Z", 18343080, "https://github.com/x/win.zip")
        );
        assert_eq!(
            latest.available(),
            Available { version: "0.2.0".into(), title: "Scriibe 0.2.0".into(), date: "2026-11-02T10:00:00Z".into() }
        );
    }

    #[test]
    fn a_release_without_windows_zip_is_still_a_version_but_not_installable() {
        let release = json!({"tag_name": "v0.1.0", "assets": [{"name": "Scriibe-0.1.0-macos.zip", "size": 1, "browser_download_url": "https://x/m.zip"}]});
        let latest = parse_release(&release).unwrap();
        assert_eq!(
            (latest.version.as_str(), latest.title.as_str(), latest.size, latest.download_url.as_str()),
            ("0.1.0", "Scriibe 0.1.0", 0, "")
        );
        assert!(parse_release(&json!({})).is_err(), "pas d'étiquette");
        assert!(parse_release(&json!({"tag_name": "nightly"})).unwrap_err().contains("illisible"));
    }

    #[test]
    fn only_a_bigger_version_number_is_an_update() {
        assert!(is_newer("0.2.0", "0.1.0"));
        assert!(is_newer("v0.10.0", "0.9.9"), "les nombres se comparent, pas les textes");
        assert!(is_newer("1.0.0", "0.99.99"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.2.0"));
        assert!(!is_newer("nightly", "0.1.0"));
        assert!(!is_newer("0.2", "0.1.0"));
        assert!(!is_newer("0.2.0.1", "0.1.0"));
        assert!(parse_version(current_version()).is_some(), "le numéro du programme est de la forme x.y.z");
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
    fn a_download_refuses_an_address_that_is_not_https() {
        assert!(download("http://exemple.org/scriibe.zip", 0, &|_, _| {}).unwrap_err().contains("non sécurisée"));
    }
}
