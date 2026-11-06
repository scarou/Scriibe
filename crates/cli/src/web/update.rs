//! Mise à jour de Scriibe : la dernière Release publiée sur GitHub (le dépôt est public : aucune clé ni aucun jeton). La version installée
//! est celle du programme (`CARGO_PKG_VERSION`), comparée au numéro de l'étiquette de la Release (`v0.2.0`). Sous Windows, le zip
//! `scriibe-windows-x64.zip` de la Release remplace `scriibe.exe` ; sur Mac, `Scriibe-x.y.z-macos.zip` remplace `Scriibe.app` (S15) ;
//! sous Linux, `scriibe-linux-x64.zip` remplace le programme `scriibe` (S15). Comme pour les téléchargements, rien ne part sans que la personne ait vu la version, la taille et la
//! source, puis confirmé.

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
/// Le fichier de la Release qui contient le programme Linux `scriibe` (`release-linux.yml`).
const LINUX_ASSET: &str = "scriibe-linux-x64.zip";
const LINUX_PROGRAM: &str = "scriibe";
/// Le programme à l'intérieur de `Scriibe.app`, et l'identifiant que `tools/faire_app_macos.sh` donne à l'application.
const MAC_PROGRAM: &str = "Contents/MacOS/Scriibe";
const MAC_BUNDLE_ID: &str = "fr.scriibe.app";

/// Le système dont on cherche le fichier dans la Release.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Platform {
    Windows,
    Mac,
    Linux,
}

impl Platform {
    fn this() -> Platform {
        if cfg!(target_os = "macos") {
            Platform::Mac
        } else if cfg!(target_os = "linux") {
            Platform::Linux
        } else {
            Platform::Windows
        }
    }

    /// Windows : `scriibe-windows-x64.zip` ; Mac : `Scriibe-0.2.1-macos.zip` (le numéro change avec la version).
    fn owns(self, asset: &str) -> bool {
        match self {
            Platform::Windows => asset == ASSET,
            Platform::Mac => asset.starts_with("Scriibe-") && asset.ends_with("-macos.zip"),
            Platform::Linux => asset == LINUX_ASSET,
        }
    }

    /// Le programme dans le zip, et le début de fichier qui prouve que c'en est un (Windows : « MZ » ; Linux : ELF).
    fn program(self) -> (&'static str, &'static [u8]) {
        match self {
            Platform::Linux => (LINUX_PROGRAM, b"\x7fELF"),
            _ => (PROGRAM, b"MZ"),
        }
    }
}

/// Vrai là où Scriibe sait s'installer seul (Windows, Mac et Linux) ; ailleurs il prévient seulement.
pub fn can_install() -> bool {
    cfg!(any(windows, target_os = "macos", target_os = "linux"))
}

/// Le programme en cours, tel qu'il faut le relancer. Linux : une fois l'ancien renommé en `scriibe.old`, le système donne ce nom-là
/// (ou « … (deleted) ») pour le programme qui tourne ; la nouvelle version est à l'emplacement d'origine.
pub fn program_path() -> std::io::Result<PathBuf> {
    std::env::current_exe().map(original_program)
}

fn original_program(exe: PathBuf) -> PathBuf {
    let name = exe.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let original = name.strip_suffix(" (deleted)").unwrap_or(&name);
    let original = original.strip_suffix(".old").unwrap_or(original);
    if original == name {
        exe
    } else {
        exe.with_file_name(original)
    }
}

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
    /// Taille du zip de ce système (0 : la Release n'en contient pas).
    pub size: u64,
    /// Adresse du zip de ce système (vide : la Release n'en contient pas).
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

/// La Release : version (sans « v »), titre, date, et le zip du système qui tourne s'il existe.
fn parse_release(release: &Value) -> Result<Latest, String> {
    parse_release_for(release, Platform::this())
}

fn parse_release_for(release: &Value, platform: Platform) -> Result<Latest, String> {
    let tag = release["tag_name"].as_str().ok_or("étiquette de version absente")?;
    parse_version(tag).ok_or_else(|| format!("numéro de version illisible : « {tag} »"))?;
    let version = tag.trim().trim_start_matches('v').to_string();
    let title = release["name"].as_str().filter(|n| !n.is_empty()).map_or_else(|| format!("Scriibe {version}"), String::from);
    let date = release["published_at"].as_str().unwrap_or("").to_string();
    let asset = release["assets"].as_array().into_iter().flatten().find(|a| a["name"].as_str().is_some_and(|n| platform.owns(n)));
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
    extract_program_for(zip_bytes, Platform::this())
}

fn extract_program_for(zip_bytes: &[u8], platform: Platform) -> Result<Vec<u8>, String> {
    let (program_name, magic) = platform.program();
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(zip_bytes)).map_err(|e| format!("zip illisible ({e})"))?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let is_program = entry.enclosed_name().and_then(|p| p.file_name().map(|n| n.to_owned())).is_some_and(|n| n == program_name);
        if entry.is_file() && is_program {
            let mut program = Vec::new();
            entry.read_to_end(&mut program).map_err(|e| e.to_string())?;
            // Un exécutable Windows commence par « MZ », un exécutable Linux par « \x7fELF » ; un fichier minuscule n'est pas un programme.
            if program.len() < 100_000 || !program.starts_with(magic) {
                return Err(format!(
                    "le fichier du zip ne ressemble pas à un programme {}",
                    if magic == b"MZ" { "Windows" } else { "Linux" }
                ));
            }
            return Ok(program);
        }
    }
    Err(format!("{program_name} n'est pas dans le zip"))
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
        .map_err(|e| format!("écriture impossible dans {folder} ({e}) : place Scriibe dans un dossier où tu peux écrire"))?;
    // Linux : le programme écrit doit pouvoir s'exécuter.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&fresh, std::fs::Permissions::from_mode(0o755));
    }
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

/// `Scriibe.app` d'après l'emplacement du programme (`…/Scriibe.app/Contents/MacOS/Scriibe`) ; `None` hors d'une application (`cargo run`).
pub fn app_bundle() -> Option<PathBuf> {
    let current = std::env::current_exe().ok()?;
    bundle_of(&current)
}

fn bundle_of(exe: &Path) -> Option<PathBuf> {
    let bundle = exe.parent()?.parent()?.parent()?;
    let is_app = bundle.extension().is_some_and(|e| e == "app") && exe.ends_with(MAC_PROGRAM);
    is_app.then(|| bundle.to_path_buf())
}

/// Lance un outil du système et renvoie sa sortie d'erreur s'il échoue.
fn run_tool(program: &str, args: &[&Path]) -> Result<(), String> {
    let output = std::process::Command::new(program).args(args).output().map_err(|e| format!("{program} introuvable ({e})"))?;
    if output.status.success() {
        return Ok(());
    }
    let said = String::from_utf8_lossy(&output.stderr);
    Err(format!("{program} a échoué : {}", said.trim().chars().take(200).collect::<String>()))
}

/// Décompresse le zip de la Release (par `ditto`, l'outil de macOS qui l'a fabriqué : liens, droits et signature intacts) dans `stage`
/// et vérifie que `Scriibe.app` y est, complète et correctement signée. Renvoie son emplacement.
fn unpack_app(zip_bytes: &[u8], stage: &Path) -> Result<PathBuf, String> {
    let zip = stage.join("release.zip");
    std::fs::write(&zip, zip_bytes).map_err(|e| format!("écriture impossible dans {} ({e})", stage.display()))?;
    run_tool("/usr/bin/ditto", &[Path::new("-x"), Path::new("-k"), &zip, stage])?;
    let app = stage.join("Scriibe.app");
    if !app.join(MAC_PROGRAM).is_file() {
        return Err("Scriibe.app n'est pas dans le zip".into());
    }
    let plist = std::fs::read_to_string(app.join("Contents/Info.plist")).unwrap_or_default();
    if !plist.contains(MAC_BUNDLE_ID) {
        return Err("le zip ne contient pas Scriibe".into());
    }
    run_tool("/usr/bin/codesign", &[Path::new("--verify"), Path::new("--deep"), Path::new("--strict"), &app])
        .map_err(|e| format!("la signature de la nouvelle version n'est pas valide ({e})"))?;
    Ok(app)
}

/// Met `fresh` à la place de `bundle` : l'ancienne application devient `Scriibe.app.old` (supprimée au prochain lancement) et, si
/// quelque chose échoue, elle est remise en place. macOS laisse renommer une application qui tourne.
fn swap_bundle(bundle: &Path, fresh: &Path) -> Result<(), String> {
    let old = sibling(bundle, ".old");
    let _ = std::fs::remove_dir_all(&old);
    if let Err(e) = std::fs::rename(bundle, &old) {
        return Err(format!("l'ancienne version ne peut pas être renommée ({e})"));
    }
    if let Err(e) = std::fs::rename(fresh, bundle) {
        let _ = std::fs::rename(&old, bundle);
        return Err(format!("la nouvelle version ne peut pas être mise en place ({e}) ; l'ancienne est remise"));
    }
    Ok(())
}

/// Installe la Release sur Mac : décompressée à côté de `Scriibe.app` (même disque, donc un simple renommage), vérifiée, puis échangée.
fn install_app(exe: &Path, zip_bytes: &[u8]) -> Result<(), String> {
    let bundle = bundle_of(exe).ok_or("Scriibe ne tourne pas depuis Scriibe.app : rien à remplacer")?;
    let folder = bundle.parent().ok_or("emplacement de Scriibe.app inconnu")?;
    let stage = folder.join(".Scriibe-mise-a-jour");
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir(&stage)
        .map_err(|e| format!("écriture impossible dans {} ({e}) : place Scriibe.app dans le dossier Applications", folder.display()))?;
    let outcome = unpack_app(zip_bytes, &stage).and_then(|fresh| swap_bundle(&bundle, &fresh));
    let _ = std::fs::remove_dir_all(&stage);
    outcome
}

/// Au lancement : supprime l'ancienne version laissée par une mise à jour. Elle tourne encore quelques secondes (le temps qu'elle
/// s'arrête), donc on réessaie un peu, sans rien bloquer.
pub fn remove_leftovers() {
    let Ok(current) = std::env::current_exe() else { return };
    // Sur Mac, c'est l'ancienne application entière (`Scriibe.app.old`) ; ailleurs, l'ancien programme (`scriibe.exe.old`).
    let old = sibling(&bundle_of(&current).unwrap_or(current), ".old");
    if !old.exists() {
        return;
    }
    let _ = std::thread::Builder::new().name("scriibe-nettoyage".into()).spawn(move || {
        for _ in 0..15 {
            if std::fs::remove_file(&old).is_ok() || std::fs::remove_dir_all(&old).is_ok() || !old.exists() {
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
        if !can_install() {
            return Err("la mise à jour intégrée ne marche que sous Windows, sur Mac et sous Linux".into());
        }
        if latest.download_url.is_empty() {
            return Err(if cfg!(windows) {
                format!("cette version ne contient pas de fichier pour Windows ({ASSET})")
            } else if cfg!(target_os = "linux") {
                format!("cette version ne contient pas de fichier pour Linux ({LINUX_ASSET})")
            } else {
                "cette version ne contient pas de fichier pour Mac (Scriibe-…-macos.zip)".into()
            });
        }
        let current = program_path().map_err(|e| format!("emplacement du programme inconnu ({e})"))?;
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
            let outcome =
                download(&latest.download_url, latest.size, &|done, total| me.set(State::Running { done, total })).and_then(|zip| {
                    if cfg!(any(windows, target_os = "linux")) {
                        extract_program(&zip).and_then(|program| replace_program(&current, &program))
                    } else {
                        install_app(&current, &zip)
                    }
                });
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
        let latest = parse_release_for(&release, Platform::Windows).unwrap();
        assert_eq!(
            (latest.version.as_str(), latest.title.as_str(), latest.date.as_str(), latest.size, latest.download_url.as_str()),
            ("0.2.0", "Scriibe 0.2.0", "2026-11-02T10:00:00Z", 18343080, "https://github.com/x/win.zip")
        );
        let mac = parse_release_for(&release, Platform::Mac).unwrap();
        assert_eq!(
            (mac.size, mac.download_url.as_str()),
            (21359614, "https://github.com/x/mac.zip"),
            "le zip Mac, quel que soit son numéro"
        );
        assert_eq!(
            latest.available(),
            Available { version: "0.2.0".into(), title: "Scriibe 0.2.0".into(), date: "2026-11-02T10:00:00Z".into() }
        );
    }

    #[test]
    fn a_release_without_windows_zip_is_still_a_version_but_not_installable() {
        let release = json!({"tag_name": "v0.1.0", "assets": [{"name": "Scriibe-0.1.0-macos.zip", "size": 1, "browser_download_url": "https://x/m.zip"}]});
        let latest = parse_release_for(&release, Platform::Windows).unwrap();
        assert_eq!(
            (latest.version.as_str(), latest.title.as_str(), latest.size, latest.download_url.as_str()),
            ("0.1.0", "Scriibe 0.1.0", 0, "")
        );
        let windows_only = json!({"tag_name": "v0.1.0", "assets": [{"name": ASSET, "size": 1, "browser_download_url": "https://x/w.zip"}]});
        assert_eq!(
            parse_release_for(&windows_only, Platform::Mac).unwrap().download_url,
            "",
            "une Release sans zip Mac n'est pas installable sur Mac"
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
        assert_eq!(extract_program_for(&good, Platform::Windows).unwrap(), fake_program(1));
        assert!(extract_program_for(&zip_with(&[("autre.exe", fake_program(1))]), Platform::Windows).is_err(), "pas de scriibe.exe");
        assert!(
            extract_program_for(&zip_with(&[("scriibe.exe", b"MZ".to_vec())]), Platform::Windows).unwrap_err().contains("programme"),
            "trop petit"
        );
        assert!(extract_program_for(&zip_with(&[("scriibe.exe", vec![b'x'; 200_000])]), Platform::Windows).is_err(), "pas un exécutable");
        assert!(extract_program_for(b"pas un zip", Platform::Windows).is_err());
    }

    #[test]
    fn after_the_swap_the_new_program_is_relaunched_not_the_old_one() {
        let at = |p: &str| original_program(PathBuf::from(p));
        assert_eq!(at("/home/moi/scriibe/scriibe.old"), PathBuf::from("/home/moi/scriibe/scriibe"));
        assert_eq!(at("/home/moi/scriibe/scriibe.old (deleted)"), PathBuf::from("/home/moi/scriibe/scriibe"));
        assert_eq!(at("/home/moi/scriibe/scriibe"), PathBuf::from("/home/moi/scriibe/scriibe"));
        assert_eq!(at("C:\\scriibe\\scriibe.exe"), PathBuf::from("C:\\scriibe\\scriibe.exe"));
    }

    #[test]
    fn the_linux_program_is_taken_from_its_zip() {
        let elf = |tag: u8| {
            let mut p = b"\x7fELF".to_vec();
            p.extend(vec![tag; 200_000]);
            p
        };
        let release = json!({"tag_name": "v0.3.0", "assets": [
            {"name": ASSET, "size": 1, "browser_download_url": "https://x/w.zip"},
            {"name": LINUX_ASSET, "size": 19, "browser_download_url": "https://x/l.zip"}]});
        assert_eq!(parse_release_for(&release, Platform::Linux).unwrap().download_url, "https://x/l.zip");
        let good = zip_with(&[("scriibe", elf(1))]);
        assert_eq!(extract_program_for(&good, Platform::Linux).unwrap(), elf(1));
        assert!(
            extract_program_for(&zip_with(&[("scriibe", fake_program(1))]), Platform::Linux).is_err(),
            "un .exe n'est pas un programme Linux"
        );
        assert!(extract_program_for(&zip_with(&[("scriibe.exe", elf(1))]), Platform::Linux).is_err(), "pas de scriibe");
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
    fn the_app_is_found_from_the_program_inside_it() {
        let exe = Path::new("/Users/moi/Applications/Scriibe.app/Contents/MacOS/Scriibe");
        assert_eq!(bundle_of(exe), Some(PathBuf::from("/Users/moi/Applications/Scriibe.app")));
        assert_eq!(bundle_of(Path::new("/code/Scriibe/target/release/scriibe")), None, "cargo run : pas d'application à remplacer");
        assert_eq!(bundle_of(Path::new("/Applications/Autre.app/Contents/MacOS/Scriibe")).unwrap().file_name().unwrap(), "Autre.app");
        assert_eq!(bundle_of(Path::new("/x/Dossier/Contents/MacOS/Scriibe")), None, "pas un .app");
        assert_eq!(sibling(Path::new("/a/Scriibe.app"), ".old"), Path::new("/a/Scriibe.app.old"));
    }

    fn fake_app(folder: &Path, tag: &str) -> PathBuf {
        let app = folder.join("Scriibe.app");
        std::fs::create_dir_all(app.join("Contents/MacOS")).unwrap();
        std::fs::write(app.join(MAC_PROGRAM), tag).unwrap();
        std::fs::write(app.join("Contents/Info.plist"), format!(
                r#"<?xml version="1.0" encoding="UTF-8"?><plist version="1.0"><dict><key>CFBundleExecutable</key><string>Scriibe</string><key>CFBundleIdentifier</key><string>{MAC_BUNDLE_ID}</string><key>CFBundlePackageType</key><string>APPL</string></dict></plist>"#
            )).unwrap();
        app
    }

    #[test]
    fn the_app_is_replaced_and_the_old_one_kept_aside() {
        let dir = temp("app-remplace");
        let (current, fresh_dir) = (fake_app(&dir, "ancienne"), dir.join("pret"));
        let fresh = fake_app(&fresh_dir, "nouvelle");
        swap_bundle(&current, &fresh).unwrap();
        assert_eq!(std::fs::read_to_string(current.join(MAC_PROGRAM)).unwrap(), "nouvelle");
        assert_eq!(std::fs::read_to_string(dir.join("Scriibe.app.old").join(MAC_PROGRAM)).unwrap(), "ancienne");
        assert!(!fresh.exists());
        // Une deuxième mise à jour remplace l'ancienne copie sans erreur.
        let again = fake_app(&dir.join("encore"), "troisieme");
        swap_bundle(&current, &again).unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("Scriibe.app.old").join(MAC_PROGRAM)).unwrap(), "nouvelle");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_failed_swap_puts_the_old_app_back() {
        let dir = temp("app-echec");
        let current = fake_app(&dir, "ancienne");
        let error = swap_bundle(&current, &dir.join("absente")).unwrap_err();
        assert!(error.contains("l'ancienne est remise"), "{error}");
        assert_eq!(std::fs::read_to_string(current.join(MAC_PROGRAM)).unwrap(), "ancienne");
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Une vraie `Scriibe.app` (signée ad hoc, comme le fait `faire_app_macos.sh` sans certificat) zippée par `ditto`, comme la Release.
    #[cfg(target_os = "macos")]
    fn signed_zip(dir: &Path, tag: &str, sign: bool) -> Vec<u8> {
        let app = fake_app(dir, tag);
        // `codesign` ne signe qu'un vrai exécutable : une copie d'un programme du système, et la marque dans les ressources.
        std::fs::copy("/usr/bin/true", app.join(MAC_PROGRAM)).unwrap();
        std::fs::create_dir_all(app.join("Contents/Resources")).unwrap();
        std::fs::write(app.join("Contents/Resources/marque"), tag).unwrap();
        if sign {
            run_tool(
                "/usr/bin/codesign",
                &[Path::new("--force"), Path::new("--sign"), Path::new("-"), Path::new("--identifier"), Path::new(MAC_BUNDLE_ID), &app],
            )
            .unwrap();
        }
        let zip = dir.join("Scriibe.zip");
        run_tool("/usr/bin/ditto", &[Path::new("-c"), Path::new("-k"), Path::new("--keepParent"), &app, &zip]).unwrap();
        std::fs::read(zip).unwrap()
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn a_signed_release_replaces_the_installed_app_and_leaves_no_trace() {
        let dir = temp("app-installe");
        let installed = dir.join("Applications");
        let current = fake_app(&installed, "ancienne");
        let zip = signed_zip(&dir.join("release"), "nouvelle", true);
        install_app(&current.join(MAC_PROGRAM), &zip).unwrap();
        assert_eq!(std::fs::read_to_string(current.join("Contents/Resources/marque")).unwrap(), "nouvelle");
        assert_eq!(std::fs::read_to_string(installed.join("Scriibe.app.old").join(MAC_PROGRAM)).unwrap(), "ancienne");
        assert!(!installed.join(".Scriibe-mise-a-jour").exists(), "le dossier de travail est supprimé");
        run_tool("/usr/bin/codesign", &[Path::new("--verify"), Path::new("--deep"), Path::new("--strict"), &current]).unwrap();
        let _ = std::fs::remove_dir_all(dir);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn an_unsigned_or_foreign_release_is_refused_and_the_app_stays() {
        let dir = temp("app-refuse");
        let installed = dir.join("Applications");
        let current = fake_app(&installed, "ancienne");
        let unsigned = signed_zip(&dir.join("sans-signature"), "nouvelle", false);
        let error = install_app(&current.join(MAC_PROGRAM), &unsigned).unwrap_err();
        assert!(error.contains("signature"), "{error}");
        assert!(install_app(&current.join(MAC_PROGRAM), b"pas un zip").is_err());
        assert_eq!(std::fs::read_to_string(current.join(MAC_PROGRAM)).unwrap(), "ancienne");
        assert!(!installed.join("Scriibe.app.old").exists() && !installed.join(".Scriibe-mise-a-jour").exists());
        assert!(install_app(&dir.join("target/release/scriibe"), &unsigned).unwrap_err().contains("Scriibe.app"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_download_refuses_an_address_that_is_not_https() {
        assert!(download("http://exemple.org/scriibe.zip", 0, &|_, _| {}).unwrap_err().contains("non sécurisée"));
    }
}
