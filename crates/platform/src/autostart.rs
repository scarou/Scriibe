//! Lancement de Scriibe à l'ouverture de session (C7, tâche 3 ; `docs/SYSTEME.md` § Cadrage, question 6).
//!
//! La vérité est le système : rien n'est rangé dans `settings.json`. Une entrée simple par système, créée quand on coche, retirée
//! quand on décoche, qui lance `lancer --sans-navigateur` (icône seule, touche prête, sans fenêtre) :
//!
//! - macOS : `~/Library/LaunchAgents/fr.scriibe.app.plist`, qui appelle `open -g -a Scriibe.app` (les autorisations restent celles de l'app) ;
//! - Windows : valeur `Scriibe` de `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` ;
//! - Linux : `$XDG_CONFIG_HOME/autostart/fr.scriibe.Scriibe.desktop` (sinon `~/.config/autostart`).
//!
//! Les fonctions de fichiers prennent leurs dossiers en paramètres : les tests travaillent sur des dossiers factices, jamais sur
//! ceux de l'ordinateur.

use std::path::{Path, PathBuf};

/// Identifiant de l'application (macOS : nom de l'entrée et `AssociatedBundleIdentifiers`).
pub const BUNDLE_ID: &str = "fr.scriibe.app";
/// Nom de la valeur du registre de Windows.
pub const WINDOWS_VALUE: &str = "Scriibe";
/// Nom du fichier de démarrage de Linux.
pub const LINUX_FILE: &str = "fr.scriibe.Scriibe.desktop";
/// Ce que l'entrée lance : icône seule, touche prête, sans fenêtre.
const ARGS: [&str; 2] = ["lancer", "--sans-navigateur"];

/// L'état du réglage, tel que le système le montre.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    /// Ce système sait faire (faux : Scriibe lancée hors de `Scriibe.app` sur Mac, ou système non pris en charge).
    pub supported: bool,
    /// Pourquoi ce n'est pas pris en charge, quand `supported` est faux.
    pub reason: Option<String>,
    /// Une entrée existe **et** n'est pas désactivée dans les réglages du système : la case est cochée.
    pub enabled: bool,
    /// Une entrée existe mais le système la désactive (Gestionnaire des tâches, Réglages Système…) : la case est décochée.
    pub disabled_by_system: bool,
}

// ───────────────────────────── Contenus des entrées (purs, testés) ─────────────────────────────

/// Échappe un texte placé dans un fichier XML (plist).
fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Le contenu du LaunchAgent de macOS pour `Scriibe.app` rangée à `app`.
pub fn mac_plist(app: &Path) -> String {
    let app = xml_escape(&app.to_string_lossy());
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
<plist version=\"1.0\">\n\
<dict>\n\
\t<key>Label</key>\n\t<string>{BUNDLE_ID}</string>\n\
\t<key>ProgramArguments</key>\n\t<array>\n\
\t\t<string>/usr/bin/open</string>\n\t\t<string>-g</string>\n\t\t<string>-a</string>\n\t\t<string>{app}</string>\n\
\t\t<string>--args</string>\n\t\t<string>{}</string>\n\t\t<string>{}</string>\n\t</array>\n\
\t<key>RunAtLoad</key>\n\t<true/>\n\
\t<key>AssociatedBundleIdentifiers</key>\n\t<array>\n\t\t<string>{BUNDLE_ID}</string>\n\t</array>\n\
</dict>\n\
</plist>\n",
        ARGS[0], ARGS[1]
    )
}

/// Le chemin de `Scriibe.app` contenu dans un LaunchAgent écrit par [`mac_plist`] (le premier argument après `-a`).
pub fn mac_plist_target(plist: &str) -> Option<String> {
    let after = plist.split("<string>-a</string>").nth(1)?;
    let value = after.split("<string>").nth(1)?.split("</string>").next()?;
    Some(value.replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&"))
}

/// Vrai si `launchctl print-disabled gui/<uid>` marque `label` comme désactivé (ligne `"fr.scriibe.app" => disabled`).
pub fn mac_label_disabled(print_disabled: &str, label: &str) -> bool {
    print_disabled.lines().any(|line| {
        let line = line.trim();
        line.starts_with(&format!("\"{label}\"")) && line.ends_with("disabled") && !line.ends_with("enabled")
    })
}

/// La commande de la valeur du registre de Windows : le programme entre guillemets, puis les arguments.
pub fn windows_command(exe: &Path) -> String {
    format!("\"{}\" {} {}", exe.display(), ARGS[0], ARGS[1])
}

/// Le programme contenu dans une commande écrite par [`windows_command`] (ce qui est entre les premiers guillemets).
pub fn windows_command_target(command: &str) -> Option<String> {
    let rest = command.strip_prefix('"')?;
    Some(rest.split('"').next()?.to_string())
}

/// `StartupApproved\Run` : le Gestionnaire des tâches y écrit 12 octets par entrée. Premier octet pair (2, 6) : activée ; impair (3, 7) :
/// désactivée par l'utilisateur. Une donnée absente ou vide : activée.
pub fn windows_approved_enabled(data: &[u8]) -> bool {
    data.first().is_none_or(|b| b % 2 == 0)
}

/// Le contenu du fichier de démarrage de Linux pour le programme `exe`.
pub fn linux_desktop(exe: &Path) -> String {
    // `Exec` : le chemin entre guillemets, `\` et `"` échappés (règles de la spécification Desktop Entry).
    let quoted = exe.to_string_lossy().replace('\\', "\\\\").replace('"', "\\\"");
    format!(
        "[Desktop Entry]\nType=Application\nName=Scriibe\nComment=Dictée vocale\nExec=\"{quoted}\" {} {}\nTerminal=false\nX-GNOME-Autostart-enabled=true\n",
        ARGS[0], ARGS[1]
    )
}

/// Le programme contenu dans un fichier écrit par [`linux_desktop`] (ligne `Exec=`, premier mot entre guillemets).
pub fn linux_desktop_target(content: &str) -> Option<String> {
    let exec = content.lines().find_map(|l| l.strip_prefix("Exec="))?;
    let rest = exec.strip_prefix('"')?;
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => out.push(chars.next()?),
            '"' => return Some(out),
            c => out.push(c),
        }
    }
    None
}

/// Vrai si le fichier de démarrage est désactivé (`Hidden=true` ou `X-GNOME-Autostart-enabled=false`).
pub fn linux_desktop_disabled(content: &str) -> bool {
    content.lines().any(|l| {
        let l = l.trim();
        l.eq_ignore_ascii_case("Hidden=true") || l.eq_ignore_ascii_case("X-GNOME-Autostart-enabled=false")
    })
}

/// Le dossier `autostart` de Linux d'après les variables d'environnement données.
pub fn linux_autostart_dir(xdg_config_home: Option<&str>, home: Option<&Path>) -> Option<PathBuf> {
    match xdg_config_home.filter(|v| Path::new(v).is_absolute()) {
        Some(x) => Some(Path::new(x).join("autostart")),
        None => home.filter(|h| h.is_absolute()).map(|h| h.join(".config").join("autostart")),
    }
}

/// `Scriibe.app` qui contient `exe` (`…/Scriibe.app/Contents/MacOS/scriibe`), s'il y en a une.
pub fn app_bundle_of(exe: &Path) -> Option<PathBuf> {
    exe.ancestors().find(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("app"))).map(Path::to_path_buf)
}

// ───────────────────────────── Fichiers (dossiers en paramètres, testés) ─────────────────────────────

/// Écrit `content` dans `file` (dossier créé au besoin), par un fichier voisin puis un renommage : jamais d'entrée à moitié écrite.
fn write_entry(file: &Path, content: &str) -> Result<(), String> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("dossier {} : {e}", dir.display()))?;
    }
    let tmp = file.with_extension("tmp-scriibe");
    std::fs::write(&tmp, content).map_err(|e| format!("écriture de {} : {e}", tmp.display()))?;
    std::fs::rename(&tmp, file).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("{} : {e}", file.display())
    })
}

/// Retire `file` ; un fichier absent n'est pas une erreur.
fn remove_entry(file: &Path) -> Result<(), String> {
    match std::fs::remove_file(file) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("{} : {e}", file.display())),
    }
}

/// Le fichier de démarrage de Linux est-il là, et que dit-il ? `(présent, désactivé, programme visé)`.
pub fn read_linux_entry(file: &Path) -> (bool, bool, Option<String>) {
    match std::fs::read_to_string(file) {
        Ok(content) => (true, linux_desktop_disabled(&content), linux_desktop_target(&content)),
        Err(_) => (false, false, None),
    }
}

/// Le LaunchAgent de macOS est-il là, et que vise-t-il ? `(présent, app visée)`.
pub fn read_mac_entry(file: &Path) -> (bool, Option<String>) {
    match std::fs::read_to_string(file) {
        Ok(content) => (true, mac_plist_target(&content)),
        Err(_) => (false, None),
    }
}

/// Crée (ou réécrit) le fichier de démarrage de Linux pour `exe`.
pub fn write_linux_entry(file: &Path, exe: &Path) -> Result<(), String> {
    write_entry(file, &linux_desktop(exe))
}

/// Crée (ou réécrit) le LaunchAgent de macOS pour `app`.
pub fn write_mac_entry(file: &Path, app: &Path) -> Result<(), String> {
    write_entry(file, &mac_plist(app))
}

/// Retire l'entrée (fichier) ; absente : rien à faire.
pub fn remove_file_entry(file: &Path) -> Result<(), String> {
    remove_entry(file)
}

// ───────────────────────────── Ce système ─────────────────────────────

/// Le fichier d'entrée de ce système (macOS, Linux) ; `None` si le dossier personnel est inconnu.
#[cfg(any(target_os = "macos", target_os = "linux"))]
fn entry_file() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var_os("HOME").map(PathBuf::from).filter(|h| h.is_absolute())?;
        Some(home.join("Library").join("LaunchAgents").join(format!("{BUNDLE_ID}.plist")))
    }
    #[cfg(target_os = "linux")]
    {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let dir = linux_autostart_dir(std::env::var("XDG_CONFIG_HOME").ok().as_deref(), home.as_deref())?;
        Some(dir.join(LINUX_FILE))
    }
}

/// Ce que l'entrée doit viser : `Scriibe.app` sur Mac, le programme en cours ailleurs.
fn target_program() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| format!("programme en cours introuvable : {e}"))?;
    let program = if cfg!(target_os = "macos") {
        app_bundle_of(&exe).ok_or_else(|| {
            "Scriibe n'est pas lancée depuis Scriibe.app (programme compilé ?) : rien n'est ajouté au démarrage".to_string()
        })?
    } else {
        exe
    };
    match launch_location_problem(&program) {
        Some(problem) => Err(problem.to_string()),
        None => Ok(program),
    }
}

/// Pourquoi ce programme ne doit pas être mis au démarrage : programme de développement (`target/`, dépôt git), image disque, volume externe
/// ou copie temporaire de macOS (`AppTranslocation`). Une entrée vers un tel programme cesserait de marcher (ou lancerait une version de
/// développement) au démarrage suivant.
pub fn launch_location_problem(program: &Path) -> Option<&'static str> {
    if crate::uninstall::is_dev_location(program) {
        return Some("Scriibe est lancée depuis un dossier de développement (target/ ou dépôt git) : rien n'est ajouté au démarrage");
    }
    let text = program.to_string_lossy();
    if text.contains("/AppTranslocation/") || text.starts_with("/Volumes/") {
        return Some("Scriibe est lancée depuis une image disque, un volume externe ou un emplacement temporaire : copie-la dans le dossier Applications, puis relance-la");
    }
    None
}

/// L'état du réglage, relu dans le système.
pub fn status() -> Status {
    let (supported, reason) = match target_program() {
        Ok(_) if cfg!(any(windows, target_os = "macos", target_os = "linux")) => (true, None),
        Ok(_) => (false, Some("pas encore pris en charge sur ce système".to_string())),
        Err(e) => (false, Some(e)),
    };
    let (present, disabled) = system_state();
    Status { supported, reason, enabled: present && !disabled, disabled_by_system: present && disabled }
}

/// Coche ou décoche : crée l'entrée (et lève une désactivation du système) ou la retire.
pub fn set(enabled: bool) -> Result<(), String> {
    #[cfg(any(windows, target_os = "macos", target_os = "linux"))]
    {
        if enabled {
            system_enable(&target_program()?)
        } else {
            system_disable()
        }
    }
    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    {
        let _ = enabled;
        Err("pas encore pris en charge sur ce système".to_string())
    }
}

/// À chaque lancement : si l'entrée existe et vise un autre programme que celui qui tourne (Scriibe déplacée), la réécrire.
/// Rend vrai si elle a été réécrite. Ne crée jamais une entrée absente et ne touche pas à une désactivation du système.
pub fn refresh() -> bool {
    let Ok(program) = target_program() else { return false };
    let Some(visee) = system_target() else { return false };
    if visee == program.to_string_lossy() {
        return false;
    }
    system_rewrite(&program).is_ok()
}

#[cfg(target_os = "macos")]
fn system_state() -> (bool, bool) {
    let Some(file) = entry_file() else { return (false, false) };
    let (present, _) = read_mac_entry(&file);
    if !present {
        return (false, false);
    }
    (true, mac_disabled_now())
}

#[cfg(target_os = "macos")]
fn mac_uid() -> u32 {
    // SAFETY: getuid n'a pas d'effet de bord.
    unsafe { libc::getuid() }
}

#[cfg(target_os = "macos")]
fn mac_disabled_now() -> bool {
    let out = std::process::Command::new("/bin/launchctl").arg("print-disabled").arg(format!("gui/{}", mac_uid())).output();
    out.ok().is_some_and(|o| mac_label_disabled(&String::from_utf8_lossy(&o.stdout), BUNDLE_ID))
}

#[cfg(target_os = "macos")]
fn system_target() -> Option<String> {
    read_mac_entry(&entry_file()?).1
}

#[cfg(target_os = "macos")]
fn system_rewrite(program: &Path) -> Result<(), String> {
    write_mac_entry(&entry_file().ok_or("dossier personnel inconnu")?, program)
}

#[cfg(target_os = "macos")]
fn system_enable(app: &Path) -> Result<(), String> {
    system_rewrite(app)?;
    if mac_disabled_now() {
        // Réactive l'élément désactivé dans les Réglages Système ; un échec n'empêche pas l'entrée d'exister.
        let _ = std::process::Command::new("/bin/launchctl").arg("enable").arg(format!("gui/{}/{BUNDLE_ID}", mac_uid())).output();
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn system_disable() -> Result<(), String> {
    remove_file_entry(&entry_file().ok_or("dossier personnel inconnu")?)
}

#[cfg(target_os = "linux")]
fn system_state() -> (bool, bool) {
    entry_file().map_or((false, false), |f| {
        let (present, disabled, _) = read_linux_entry(&f);
        (present, disabled)
    })
}

#[cfg(target_os = "linux")]
fn system_target() -> Option<String> {
    read_linux_entry(&entry_file()?).2
}

#[cfg(target_os = "linux")]
fn system_rewrite(program: &Path) -> Result<(), String> {
    write_linux_entry(&entry_file().ok_or("dossier personnel inconnu")?, program)
}

#[cfg(target_os = "linux")]
fn system_enable(program: &Path) -> Result<(), String> {
    // Réécrire le fichier lève aussi une désactivation (`Hidden`, `X-GNOME-Autostart-enabled=false`).
    system_rewrite(program)
}

#[cfg(target_os = "linux")]
fn system_disable() -> Result<(), String> {
    remove_file_entry(&entry_file().ok_or("dossier personnel inconnu")?)
}

#[cfg(windows)]
use windows_registry as registry;

#[cfg(windows)]
fn system_state() -> (bool, bool) {
    let present = registry::read_run().is_some();
    (present, present && !registry::approved_enabled())
}

#[cfg(windows)]
fn system_target() -> Option<String> {
    registry::read_run().and_then(|c| windows_command_target(&c))
}

#[cfg(windows)]
fn system_rewrite(program: &Path) -> Result<(), String> {
    registry::write_run(&windows_command(program))
}

#[cfg(windows)]
fn system_enable(program: &Path) -> Result<(), String> {
    system_rewrite(program)?;
    // Lève la désactivation du Gestionnaire des tâches (la valeur de `StartupApproved` est simplement effacée).
    registry::clear_approved()
}

#[cfg(windows)]
fn system_disable() -> Result<(), String> {
    registry::delete_run()?;
    registry::clear_approved()
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
fn system_state() -> (bool, bool) {
    (false, false)
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
fn system_target() -> Option<String> {
    None
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
fn system_rewrite(_program: &Path) -> Result<(), String> {
    Err("pas encore pris en charge sur ce système".to_string())
}

/// Windows, programme console : en démarrage discret, Scriibe se détache de sa console (sinon une fenêtre noire s'ouvrirait à chaque
/// connexion, et la fermer tuerait Scriibe). Le double-clic ordinaire garde sa console. Ailleurs : rien.
pub fn detach_console() {
    #[cfg(windows)]
    // SAFETY: FreeConsole ne prend aucun argument ; sans console attachée, elle échoue sans effet.
    unsafe {
        windows_sys::Win32::System::Console::FreeConsole();
    }
}

/// L'accès au registre de Windows (clé `Run` de l'utilisateur et `StartupApproved\Run`).
#[cfg(windows)]
mod windows_registry {
    use super::WINDOWS_VALUE;
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_READ,
        KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ,
    };

    const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const APPROVED: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// Lit une valeur (octets et type) de `key`, ou `None` si la clé ou la valeur manque.
    fn read(key: &str, value: &str) -> Option<(Vec<u8>, u32)> {
        let mut handle: HKEY = std::ptr::null_mut();
        // SAFETY: pointeurs vers des tampons locaux valides pendant l'appel ; le handle est refermé avant de sortir.
        unsafe {
            if RegOpenKeyExW(HKEY_CURRENT_USER, wide(key).as_ptr(), 0, KEY_READ, &mut handle) != ERROR_SUCCESS {
                return None;
            }
            let name = wide(value);
            let (mut kind, mut size) = (0u32, 0u32);
            let mut data = None;
            if RegQueryValueExW(handle, name.as_ptr(), std::ptr::null(), &mut kind, std::ptr::null_mut(), &mut size) == ERROR_SUCCESS {
                let mut buffer = vec![0u8; size as usize];
                if RegQueryValueExW(handle, name.as_ptr(), std::ptr::null(), &mut kind, buffer.as_mut_ptr(), &mut size) == ERROR_SUCCESS {
                    buffer.truncate(size as usize);
                    data = Some((buffer, kind));
                }
            }
            RegCloseKey(handle);
            data
        }
    }

    /// Efface une valeur de `key` ; clé ou valeur absente : rien à faire.
    fn delete(key: &str, value: &str) -> Result<(), String> {
        let mut handle: HKEY = std::ptr::null_mut();
        // SAFETY: comme `read`.
        unsafe {
            if RegOpenKeyExW(HKEY_CURRENT_USER, wide(key).as_ptr(), 0, KEY_SET_VALUE, &mut handle) != ERROR_SUCCESS {
                return Ok(());
            }
            let code = RegDeleteValueW(handle, wide(value).as_ptr());
            RegCloseKey(handle);
            // 2 : ERROR_FILE_NOT_FOUND, la valeur n'existe pas.
            if code == ERROR_SUCCESS || code == 2 {
                Ok(())
            } else {
                Err(format!("registre : code {code}"))
            }
        }
    }

    /// La commande de la valeur `Scriibe` de la clé `Run`.
    pub fn read_run() -> Option<String> {
        let (data, kind) = read(RUN, WINDOWS_VALUE)?;
        if kind != REG_SZ {
            return None;
        }
        let units: Vec<u16> = data.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        Some(String::from_utf16_lossy(&units).trim_end_matches('\0').to_string())
    }

    /// Écrit la valeur `Scriibe` de la clé `Run`.
    pub fn write_run(command: &str) -> Result<(), String> {
        let mut handle: HKEY = std::ptr::null_mut();
        let data = wide(command);
        // SAFETY: comme `read` ; `data` vit jusqu'à la fin de l'appel et sa taille est donnée en octets.
        unsafe {
            let code = RegCreateKeyExW(
                HKEY_CURRENT_USER,
                wide(RUN).as_ptr(),
                0,
                std::ptr::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_SET_VALUE,
                std::ptr::null(),
                &mut handle,
                std::ptr::null_mut(),
            );
            if code != ERROR_SUCCESS {
                return Err(format!("registre : code {code}"));
            }
            let code = RegSetValueExW(handle, wide(WINDOWS_VALUE).as_ptr(), 0, REG_SZ, data.as_ptr().cast(), (data.len() * 2) as u32);
            RegCloseKey(handle);
            if code == ERROR_SUCCESS {
                Ok(())
            } else {
                Err(format!("registre : code {code}"))
            }
        }
    }

    pub fn delete_run() -> Result<(), String> {
        delete(RUN, WINDOWS_VALUE)
    }

    /// Le Gestionnaire des tâches laisse-t-il l'entrée activée ? (Pas de marque : oui.)
    pub fn approved_enabled() -> bool {
        read(APPROVED, WINDOWS_VALUE).is_none_or(|(data, _)| super::windows_approved_enabled(&data))
    }

    pub fn clear_approved() -> Result<(), String> {
        delete(APPROVED, WINDOWS_VALUE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un dossier factice propre à ce test, effacé à la fin.
    struct Scratch(PathBuf);
    impl Scratch {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("scriibe-autostart-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn mac_plist_lance_open_avec_le_demarrage_discret() {
        let p = mac_plist(Path::new("/Applications/Scriibe.app"));
        assert!(p.contains("<string>fr.scriibe.app</string>"));
        assert!(p.contains("<string>/usr/bin/open</string>"));
        assert!(p.contains("<string>-g</string>"));
        assert!(p.contains("<string>/Applications/Scriibe.app</string>"));
        assert!(
            p.contains("<string>--args</string>")
                && p.contains("<string>lancer</string>")
                && p.contains("<string>--sans-navigateur</string>")
        );
        assert!(p.contains("<key>RunAtLoad</key>\n\t<true/>"));
        assert!(p.contains("AssociatedBundleIdentifiers"));
    }

    #[test]
    fn mac_plist_echappe_et_relit_le_chemin() {
        let app = Path::new("/Users/a&b/<Mes apps>/Scriibe.app");
        let p = mac_plist(app);
        assert!(p.contains("a&amp;b/&lt;Mes apps&gt;"));
        assert_eq!(mac_plist_target(&p).as_deref(), Some("/Users/a&b/<Mes apps>/Scriibe.app"));
    }

    #[test]
    fn mac_desactivation_lue_dans_launchctl() {
        let sortie = "disabled services = {\n\t\"com.exemple.autre\" => enabled\n\t\"fr.scriibe.app\" => disabled\n}\n";
        assert!(mac_label_disabled(sortie, "fr.scriibe.app"));
        assert!(!mac_label_disabled(sortie, "com.exemple.autre"));
        assert!(!mac_label_disabled("\t\"fr.scriibe.app\" => enabled\n", "fr.scriibe.app"));
        assert!(!mac_label_disabled("", "fr.scriibe.app"));
        // Un autre identifiant qui commence pareil n'est pas le nôtre.
        assert!(!mac_label_disabled("\t\"fr.scriibe.app.autre\" => disabled\n", "fr.scriibe.app"));
    }

    #[test]
    fn windows_commande_et_activation() {
        let c = windows_command(Path::new(r"C:\Users\A B\Scriibe\scriibe.exe"));
        assert_eq!(c, "\"C:\\Users\\A B\\Scriibe\\scriibe.exe\" lancer --sans-navigateur");
        assert_eq!(windows_command_target(&c).as_deref(), Some(r"C:\Users\A B\Scriibe\scriibe.exe"));
        assert_eq!(windows_command_target("scriibe.exe lancer"), None);
        // Gestionnaire des tâches : 02/06 activée, 03/07 désactivée ; pas de donnée : activée.
        assert!(windows_approved_enabled(&[2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]));
        assert!(windows_approved_enabled(&[6, 0, 0, 0]));
        assert!(!windows_approved_enabled(&[3, 0, 0, 0, 0xde, 0xad, 0xbe, 0xef, 0, 0, 0, 0]));
        assert!(!windows_approved_enabled(&[7, 0, 0, 0]));
        assert!(windows_approved_enabled(&[]));
    }

    #[test]
    fn linux_fichier_contenu_cible_et_desactivation() {
        let d = linux_desktop(Path::new("/opt/Mes apps/scriibe"));
        assert!(d.contains("Exec=\"/opt/Mes apps/scriibe\" lancer --sans-navigateur"));
        assert!(d.contains("Terminal=false") && d.contains("X-GNOME-Autostart-enabled=true"));
        assert_eq!(linux_desktop_target(&d).as_deref(), Some("/opt/Mes apps/scriibe"));
        assert!(!linux_desktop_disabled(&d));
        assert!(linux_desktop_disabled(&d.replace("enabled=true", "enabled=false")));
        assert!(linux_desktop_disabled(&format!("{d}Hidden=true\n")));
        let guillemet = linux_desktop(Path::new("/opt/a\"b\\c/scriibe"));
        assert_eq!(linux_desktop_target(&guillemet).as_deref(), Some("/opt/a\"b\\c/scriibe"));
    }

    #[test]
    fn linux_dossier_autostart() {
        // Chemins absolus propres à chaque système (sous Windows, « /home/a » n'est pas absolu).
        let base = std::env::temp_dir();
        let (home, cfg) = (base.join("home-a"), base.join("cfg"));
        let defaut = home.join(".config").join("autostart");
        assert_eq!(linux_autostart_dir(None, Some(&home)), Some(defaut.clone()));
        assert_eq!(linux_autostart_dir(Some(cfg.to_str().unwrap()), Some(&home)), Some(cfg.join("autostart")));
        // Une valeur relative est ignorée ; sans dossier personnel absolu : rien.
        assert_eq!(linux_autostart_dir(Some("cfg"), Some(&home)), Some(defaut));
        assert_eq!(linux_autostart_dir(None, None), None);
        assert_eq!(linux_autostart_dir(None, Some(Path::new("rel"))), None);
    }

    #[test]
    fn pas_d_entree_vers_un_programme_de_developpement_ni_un_emplacement_temporaire() {
        assert!(launch_location_problem(Path::new("/Users/a/dev/Scriibe/target/release/scriibe")).is_some());
        assert!(launch_location_problem(Path::new("/private/var/folders/x/AppTranslocation/ABC/d/Scriibe.app")).is_some());
        assert!(launch_location_problem(Path::new("/Volumes/Scriibe 0.3.0/Scriibe.app")).is_some());
        assert!(launch_location_problem(Path::new("/Users/a/Applications/Scriibe.app")).is_none());
        assert!(launch_location_problem(Path::new("/opt/scriibe/scriibe")).is_none());
    }

    #[test]
    fn app_bundle_trouve_scriibe_app_et_refuse_un_programme_compile() {
        assert_eq!(
            app_bundle_of(Path::new("/Applications/Scriibe.app/Contents/MacOS/scriibe")),
            Some(PathBuf::from("/Applications/Scriibe.app"))
        );
        assert_eq!(app_bundle_of(Path::new("/dev/Scriibe/target/release/scriibe")), None);
    }

    #[test]
    fn entree_creee_quand_on_coche_retiree_quand_on_decoche() {
        let s = Scratch::new("cycle");
        let linux = s.0.join("config").join("autostart").join(LINUX_FILE);
        assert_eq!(read_linux_entry(&linux), (false, false, None));
        write_linux_entry(&linux, Path::new("/opt/scriibe")).unwrap();
        assert_eq!(read_linux_entry(&linux), (true, false, Some("/opt/scriibe".to_string())));
        // Déplacée : réécrite, un seul fichier, aucun reste temporaire.
        write_linux_entry(&linux, Path::new("/opt/ailleurs/scriibe")).unwrap();
        assert_eq!(read_linux_entry(&linux).2.as_deref(), Some("/opt/ailleurs/scriibe"));
        assert_eq!(std::fs::read_dir(linux.parent().unwrap()).unwrap().count(), 1);
        remove_file_entry(&linux).unwrap();
        assert_eq!(read_linux_entry(&linux), (false, false, None));
        // Retirer une entrée absente n'est pas une erreur.
        remove_file_entry(&linux).unwrap();

        let mac = s.0.join("LaunchAgents").join("fr.scriibe.app.plist");
        write_mac_entry(&mac, Path::new("/Applications/Scriibe.app")).unwrap();
        assert_eq!(read_mac_entry(&mac), (true, Some("/Applications/Scriibe.app".to_string())));
        remove_file_entry(&mac).unwrap();
        assert_eq!(read_mac_entry(&mac), (false, None));
    }

    #[test]
    fn recocher_leve_une_desactivation_du_fichier_linux() {
        let s = Scratch::new("desactive");
        let f = s.0.join(LINUX_FILE);
        write_linux_entry(&f, Path::new("/opt/scriibe")).unwrap();
        let contenu = std::fs::read_to_string(&f).unwrap().replace("enabled=true", "enabled=false");
        std::fs::write(&f, contenu).unwrap();
        assert!(read_linux_entry(&f).1);
        write_linux_entry(&f, Path::new("/opt/scriibe")).unwrap();
        assert!(!read_linux_entry(&f).1);
    }
}
