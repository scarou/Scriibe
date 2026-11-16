//! Ouvrir une adresse web ou un dossier avec l'appli par défaut du système.

use std::process::Command;
use std::sync::OnceLock;

/// Ce qui ouvre l'interface de Scriibe dans sa propre fenêtre (rend faux si elle ne peut pas) plutôt que dans le navigateur.
type WindowOpener = Box<dyn Fn(&str) -> bool + Send + Sync>;

static WINDOW_OPENER: OnceLock<WindowOpener> = OnceLock::new();

/// Enregistre la fenêtre de Scriibe : les adresses locales (`http://127.0.0.1:…`) s'ouvrent désormais dedans.
pub fn set_window_opener(opener: impl Fn(&str) -> bool + Send + Sync + 'static) {
    let _ = WINDOW_OPENER.set(Box::new(opener));
}

/// Ouvre `target` (adresse `http://…` ou chemin de dossier). Sans attendre, sans fenêtre de console. L'adresse locale de l'interface
/// s'ouvre dans la fenêtre de Scriibe quand elle existe.
pub fn open(target: &str) {
    if target.starts_with("http://127.0.0.1:") && WINDOW_OPENER.get().is_some_and(|opener| opener(target)) {
        return;
    }
    #[cfg(windows)]
    let mut command = {
        use std::os::windows::process::CommandExt;
        // `start` comprend les adresses comme les dossiers ; le premier argument vide est le titre de fenêtre.
        let mut c = Command::new("cmd");
        c.args(["/C", "start", "", target]).creation_flags(0x0800_0000);
        c
    };
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut c = Command::new("open");
        c.arg(target);
        c
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut c = Command::new("xdg-open");
        c.arg(target);
        c
    };
    let _ = command.spawn();
}

/// Pourquoi une adresse n'a pas été ouverte : des textes fixes, qui peuvent aller dans le journal et la pastille (jamais l'adresse).
pub const ADDRESS_REFUSED: &str = "adresse refusée";
pub const ADDRESS_NOT_OPENED: &str = "ouverture impossible";

/// Ouvre une adresse construite par une **commande vocale** (recherches, nouveau mail) : la fonction d'ouverture contrôlée du chantier 6.
///
/// Distincte de [`open`], qui reste aux usages internes (interface, dossiers). Elle n'accepte que `https://` à hôte nommé et `mailto:`
/// ([`scriibe_core::web_search::is_openable`]) et **ne passe jamais par un interpréteur de commandes** : `ShellExecuteW` sous Windows (pas
/// `cmd /C start`, qui relirait `&` et `^`), `open` sur Mac et `xdg-open` sous Linux, lancés avec l'adresse pour seul argument. Une
/// adresse refusée ne lance rien. Rend `Ok` quand l'ouverture est partie, sans attendre.
pub fn open_address(url: &str) -> Result<(), &'static str> {
    if !scriibe_core::web_search::is_openable(url) {
        return Err(ADDRESS_REFUSED);
    }
    launch_address(url)
}

#[cfg(windows)]
fn launch_address(url: &str) -> Result<(), &'static str> {
    shell_execute(url)
}

/// `ShellExecuteW` « open » sur `target` (adresse contrôlée, ou raccourci du menu Démarrer trouvé par l'énumération des applications),
/// sans interpréteur de commandes. Rend `Ok` quand l'ouverture est partie, sans attendre.
#[cfg(windows)]
pub(crate) fn shell_execute(target: &str) -> Result<(), &'static str> {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    let wide = |text: &str| text.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
    let (verb, file) = (wide("open"), wide(target));
    // SAFETY: les deux chaînes se terminent par un zéro et vivent jusqu'au retour ; les autres paramètres sont nuls (permis par l'API).
    let result =
        unsafe { ShellExecuteW(std::ptr::null_mut(), verb.as_ptr(), file.as_ptr(), std::ptr::null(), std::ptr::null(), SW_SHOWNORMAL) };
    // Au-delà de 32, l'ouverture est partie ; en dessous, c'est un code d'erreur.
    if result as usize > 32 {
        Ok(())
    } else {
        Err(ADDRESS_NOT_OPENED)
    }
}

#[cfg(not(windows))]
fn launch_address(url: &str) -> Result<(), &'static str> {
    #[cfg(target_os = "macos")]
    let program = "open";
    #[cfg(not(target_os = "macos"))]
    let program = "xdg-open";
    // L'adresse commence par `https:` ou `mailto:` (contrôlé plus haut) : jamais par un tiret, donc jamais prise pour une option.
    run_launcher(program, &[url], ADDRESS_NOT_OPENED)
}

/// Combien de temps on attend le code de sortie d'un lanceur du système (`open`, `xdg-open`, `gtk-launch`) avant de le laisser partir.
#[cfg(not(windows))]
const LAUNCHER_PATIENCE: std::time::Duration = std::time::Duration::from_secs(2);

/// Lance `program` avec ces arguments, sans fenêtre ni sortie, et **regarde son code de sortie** : un lanceur qui échoue (par exemple
/// `xdg-open mailto:` sans messagerie : code 4) donne `failure`, au lieu d'une réussite affichée alors que rien ne s'est ouvert (essai du
/// lot 2, Linux, 16/11/2026). Un lanceur encore en route après [`LAUNCHER_PATIENCE`] (certains attendent la fin de l'application) compte
/// comme parti : on ne bloque jamais plus longtemps.
#[cfg(not(windows))]
pub(crate) fn run_launcher(program: &str, args: &[&str], failure: &'static str) -> Result<(), &'static str> {
    run_launcher_within(program, args, failure, LAUNCHER_PATIENCE)
}

#[cfg(not(windows))]
fn run_launcher_within(program: &str, args: &[&str], failure: &'static str, patience: std::time::Duration) -> Result<(), &'static str> {
    use std::process::Stdio;
    use std::time::Instant;
    let mut child =
        Command::new(program).args(args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().map_err(|_| failure)?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return if status.success() { Ok(()) } else { Err(failure) },
            Ok(None) if start.elapsed() < patience => std::thread::sleep(std::time::Duration::from_millis(25)),
            Ok(None) => return Ok(()),
            Err(_) => return Err(failure),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Chantier 6, étape 1b : seules des adresses `https://` (ou `mailto:`) contrôlées s'ouvrent ; le reste est refusé **avant** tout lancement.
    #[test]
    fn open_address_refuses_everything_but_checked_https_and_mailto() {
        for refused in [
            "http://example.com/",
            "javascript:alert(1)",
            "file:///etc/passwd",
            "ftp://example.com/",
            "data:text/html,<script>1</script>",
            "ms-msdt:/id PCWDiagnostic",
            "calc.exe",
            "/Applications/Calculator.app",
            "-a Calculator",
            "https://user@example.com/",
            "https://example.com@evil.com/",
            "https://localhost/",
            "https://127.0.0.1/",
            "https://example.com/a b",
            "https://example.com/\"x",
            "https://example.com/\nnotepad",
            "https://exämple.com/",
            "https://{q}/",
            "mailto:a@b.fr",
            "mailto:?to=a@b.fr",
            "mailto:?subject=a&to=a@b.fr",
            "mailto:?subject=a b",
            "mailto:?subject=",
            "mailto:?body=SECRET",
            "mailto:?subject=a%20b&body=x",
            "",
            " https://example.com/",
        ] {
            assert_eq!(open_address(refused), Err(ADDRESS_REFUSED), "{refused:?}");
        }
        assert_eq!(open_address(&format!("https://example.com/{}", "a".repeat(2000))), Err(ADDRESS_REFUSED), "trop longue");
    }

    /// Un lanceur du système qui échoue (code de sortie non nul), ou qui n'existe pas, n'est plus affiché comme une réussite ; un lanceur
    /// qui réussit l'est ; un lanceur encore en route après la patience est laissé partir (on ne bloque pas).
    #[cfg(unix)]
    #[test]
    fn a_launcher_that_fails_is_reported_as_not_opened() {
        use std::time::Duration;
        let patience = Duration::from_secs(2);
        assert_eq!(run_launcher_within("true", &[], "échec", patience), Ok(()));
        assert_eq!(run_launcher_within("false", &[], "échec", patience), Err("échec"), "code de sortie non nul");
        assert_eq!(run_launcher_within("sh", &["-c", "exit 4"], "échec", patience), Err("échec"), "comme xdg-open sans messagerie");
        assert_eq!(run_launcher_within("scriibe-lanceur-inexistant", &[], "échec", patience), Err("échec"), "programme absent");
        let started = std::time::Instant::now();
        assert_eq!(run_launcher_within("sleep", &["5"], "échec", Duration::from_millis(200)), Ok(()), "toujours en route : parti");
        assert!(started.elapsed() < Duration::from_secs(3), "n'attend pas la fin du lanceur");
    }

    #[test]
    fn the_checked_addresses_are_the_ones_the_searches_build() {
        use scriibe_core::web_search::is_openable;
        assert!(is_openable("https://www.google.com/search?q=chats%20mignons"));
        assert!(is_openable("HTTPS://duckduckgo.com/?q=a"));
        assert!(is_openable("mailto:?subject=r%C3%A9union"));
        assert!(is_openable("mailto:"), "message vierge : « nouveau mail » sans objet");
        // `&` est un caractère d'adresse comme un autre : aucun interpréteur de commandes ne la relit (pas de `cmd /C start`).
        assert!(is_openable("https://example.com/?a=1&b=calc.exe"));
        assert!(!is_openable("https://www.google.com/search?q=a&x={q}"));
    }
}
