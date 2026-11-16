//! Les applications installées et leur lancement contrôlé (chantier 6, étape 3 : « ouvre le navigateur » sous la touche de commande).
//!
//! **Liste autorisée** (décision 3 de `docs/CADRAGE_COMMANDES.md`) : les réglages (`voiceApps`) ne gardent qu'un **identifiant** choisi dans
//! [`installed_apps`]. [`launch_app`] **relit** la liste des applications installées à chaque lancement, retrouve l'identifiant dedans et ne
//! lance que ce que **l'entrée de cette liste** désigne : jamais une chaîne venue des réglages, donc un `voiceApps` écrit à la main ne
//! permet rien de plus. Aucune commande système, aucun script, aucun argument, jamais d'interpréteur de commandes.
//!
//! - **Mac** : identifiant de bundle lu dans `Contents/Info.plist` des dossiers d'applications ; lancement `open -b <identifiant>`, jamais un
//!   chemin.
//! - **Windows** : raccourcis `.lnk` des **seuls** dossiers du menu Démarrer (commun et utilisateur) ; identifiant = `commun/…` ou
//!   `utilisateur/…` suivi du chemin relatif ; lancement par `ShellExecuteW` sur le chemin **de la liste** (les applications du Store, sans
//!   raccourci, ne sont pas listées).
//! - **Linux** : fichiers `.desktop` des dossiers d'applications (système, utilisateur, Flatpak, Snap) ; identifiant conforme à
//!   `^[A-Za-z0-9._-]+$` sans tiret initial ; lancement `gtk-launch <identifiant>`.

use std::path::{Path, PathBuf};

/// Pourquoi une application n'a pas été ouverte : des textes fixes, qui peuvent aller dans le journal et la pastille (jamais le nom).
pub const APP_NOT_FOUND: &str = "application introuvable";
pub use crate::open::ADDRESS_NOT_OPENED as APP_NOT_OPENED;

/// Une application installée. `id` est celui que garde `voiceApps` ; `launch` (privé) est ce que le système reçoit au lancement.
#[derive(Clone, PartialEq, Eq)]
pub struct InstalledApp {
    pub id: String,
    pub name: String,
    launch: String,
}

impl std::fmt::Debug for InstalledApp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InstalledApp").field("id", &self.id).finish_non_exhaustive()
    }
}

/// Une application de la liste, par son identifiant.
pub fn find<'a>(list: &'a [InstalledApp], id: &str) -> Option<&'a InstalledApp> {
    list.iter().find(|app| app.id == id)
}

/// Une chaîne qui peut être donnée telle quelle à `open -b` ou `gtk-launch` : lettres, chiffres, `.`, `_`, `-`, jamais de tiret ni de
/// point initial (elle ne peut pas passer pour une option), 255 caractères au plus.
fn is_plain_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 255
        && !id.starts_with(['-', '.'])
        && id.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

fn sorted(mut list: Vec<InstalledApp>) -> Vec<InstalledApp> {
    let mut seen: Vec<String> = Vec::new();
    list.retain(|app| {
        let new = !seen.contains(&app.id);
        if new {
            seen.push(app.id.clone());
        }
        new
    });
    list.sort_by_key(|app| (app.name.to_lowercase(), app.id.clone()));
    list
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Parties pures (testées sur tous les systèmes, utilisées chacune par un seul).
// ---------------------------------------------------------------------------------------------------------------------------------

/// La valeur texte d'une clé d'un `Info.plist` au format XML (`<key>…</key><string>…</string>`).
#[cfg_attr(not(any(target_os = "macos", test)), allow(dead_code))]
fn plist_string(xml: &str, key: &str) -> Option<String> {
    let marker = format!("<key>{key}</key>");
    let after = xml[xml.find(&marker)? + marker.len()..].trim_start();
    let rest = after.strip_prefix("<string>")?;
    Some(rest[..rest.find("</string>")?].trim().to_string())
}

/// Les applications (`*.app`) des dossiers donnés, à un niveau de sous-dossier près (`Utilities`). `read_id` rend l'identifiant de bundle
/// d'un `.app` ; une application dont l'identifiant est illisible ou non conforme est ignorée.
#[cfg_attr(not(any(target_os = "macos", test)), allow(dead_code))]
fn scan_bundles(roots: &[PathBuf], read_id: &dyn Fn(&Path) -> Option<String>) -> Vec<InstalledApp> {
    fn visit(dir: &Path, depth: usize, read_id: &dyn Fn(&Path) -> Option<String>, out: &mut Vec<InstalledApp>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Some(file) = path.file_name().and_then(|n| n.to_str()) else { continue };
            if let Some(name) = file.strip_suffix(".app") {
                if let Some(id) = read_id(&path).filter(|id| is_plain_id(id)) {
                    out.push(InstalledApp { launch: id.clone(), id, name: name.to_string() });
                }
            } else if depth == 0 && !file.starts_with('.') {
                visit(&path, 1, read_id, out);
            }
        }
    }
    let mut out = Vec::new();
    for root in roots {
        visit(root, 0, read_id, &mut out);
    }
    sorted(out)
}

/// Ce qu'un fichier `.desktop` dit d'une application lançable : son nom, ou `None` s'il n'est pas de type `Application`, s'il est caché
/// (`NoDisplay`, `Hidden`) ou sans nom.
#[cfg_attr(not(any(all(unix, not(target_os = "macos")), test)), allow(dead_code))]
fn desktop_name(text: &str) -> Option<String> {
    let (mut in_entry, mut name, mut kind, mut hidden) = (false, None, None, false);
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            in_entry = line == "[Desktop Entry]";
        } else if in_entry {
            if let Some((key, value)) = line.split_once('=') {
                match key.trim() {
                    "Name" if name.is_none() => name = Some(value.trim().to_string()),
                    "Type" => kind = Some(value.trim().to_string()),
                    "NoDisplay" | "Hidden" if value.trim().eq_ignore_ascii_case("true") => hidden = true,
                    _ => {}
                }
            }
        }
    }
    (kind.as_deref() == Some("Application") && !hidden).then_some(name).flatten().filter(|n| !n.is_empty())
}

/// Les fichiers `.desktop` des dossiers donnés (sans sous-dossiers) : l'identifiant est le nom du fichier sans `.desktop`.
#[cfg_attr(not(any(all(unix, not(target_os = "macos")), test)), allow(dead_code))]
fn scan_desktop_files(roots: &[PathBuf]) -> Vec<InstalledApp> {
    let mut out = Vec::new();
    for root in roots {
        let Ok(entries) = std::fs::read_dir(root) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(id) = path.file_name().and_then(|n| n.to_str()).and_then(|n| n.strip_suffix(".desktop")) else { continue };
            if !is_plain_id(id) || !path.is_file() {
                continue;
            }
            let Some(name) = std::fs::read_to_string(&path).ok().and_then(|text| desktop_name(&text)) else { continue };
            out.push(InstalledApp { id: id.to_string(), name, launch: id.to_string() });
        }
    }
    sorted(out)
}

/// Les raccourcis `.lnk` des dossiers du menu Démarrer donnés (`(préfixe de l'identifiant, dossier)`), sur trois niveaux de sous-dossier au
/// plus. Les liens symboliques sont ignorés ; les désinstallateurs aussi.
#[cfg_attr(not(any(windows, test)), allow(dead_code))]
fn scan_start_menu(roots: &[(&str, PathBuf)]) -> Vec<InstalledApp> {
    fn visit(prefix: &str, base: &Path, dir: &Path, depth: usize, out: &mut Vec<InstalledApp>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let Ok(kind) = entry.file_type() else { continue };
            let path = entry.path();
            if kind.is_symlink() {
                continue;
            }
            if kind.is_dir() {
                if depth < 3 {
                    visit(prefix, base, &path, depth + 1, out);
                }
                continue;
            }
            let Some(stem) = path.file_name().and_then(|n| n.to_str()).and_then(|n| n.strip_suffix(".lnk")) else { continue };
            let lower = stem.to_lowercase();
            if lower.contains("uninstall") || lower.contains("désinstall") || lower.contains("desinstall") {
                continue;
            }
            let Ok(relative) = path.strip_prefix(base) else { continue };
            let relative: Vec<&str> = relative.iter().filter_map(|part| part.to_str()).collect();
            if relative.is_empty() {
                continue;
            }
            out.push(InstalledApp {
                id: format!("{prefix}/{}", relative.join("/")),
                name: stem.to_string(),
                launch: path.to_string_lossy().into_owned(),
            });
        }
    }
    let mut out = Vec::new();
    for (prefix, base) in roots {
        visit(prefix, base, base, 0, &mut out);
    }
    sorted(out)
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Par système : énumération fraîche et lancement.
// ---------------------------------------------------------------------------------------------------------------------------------

/// Les applications installées, **énumérées maintenant** (rien n'est gardé d'un appel à l'autre), triées par nom.
pub fn installed_apps() -> Vec<InstalledApp> {
    scan()
}

/// Ouvre l'application `id` : l'identifiant est **retrouvé dans une énumération fraîche** des applications installées ; absent, rien n'est
/// lancé ([`APP_NOT_FOUND`]). Ce qui est lancé vient de l'entrée de la liste, jamais de `id`. Sans attendre, sans fenêtre de console.
pub fn launch_app(id: &str) -> Result<(), &'static str> {
    let list = installed_apps();
    let app = find(&list, id).ok_or(APP_NOT_FOUND)?;
    launch(app)
}

#[cfg(target_os = "macos")]
fn scan() -> Vec<InstalledApp> {
    let mut roots: Vec<PathBuf> =
        ["/Applications", "/System/Applications", "/System/Applications/Utilities", "/Applications/Utilities"].map(PathBuf::from).into();
    if let Some(home) = std::env::var_os("HOME") {
        roots.push(PathBuf::from(home).join("Applications"));
    }
    scan_bundles(&roots, &|app| {
        let plist = app.join("Contents").join("Info.plist");
        let bytes = std::fs::read(&plist).ok()?;
        if bytes.starts_with(b"bplist") {
            // Format binaire (rare) : `plutil` le lit, une application à la fois.
            let out = std::process::Command::new("/usr/bin/plutil")
                .args(["-extract", "CFBundleIdentifier", "raw", "-o", "-"])
                .arg(&plist)
                .output()
                .ok()?;
            return out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_string());
        }
        plist_string(&String::from_utf8_lossy(&bytes), "CFBundleIdentifier")
    })
}

#[cfg(target_os = "macos")]
fn launch(app: &InstalledApp) -> Result<(), &'static str> {
    // `launch` est un identifiant de bundle conforme (`is_plain_id`, tiret initial exclu) : il ne peut pas passer pour une option.
    std::process::Command::new("open").arg("-b").arg(&app.launch).spawn().map(|_| ()).map_err(|_| APP_NOT_OPENED)
}

#[cfg(windows)]
fn scan() -> Vec<InstalledApp> {
    let tail = Path::new("Microsoft").join("Windows").join("Start Menu").join("Programs");
    let mut roots: Vec<(&str, PathBuf)> = Vec::new();
    if let Some(common) = std::env::var_os("ProgramData") {
        roots.push(("commun", PathBuf::from(common).join(&tail)));
    }
    if let Some(user) = std::env::var_os("APPDATA") {
        roots.push(("utilisateur", PathBuf::from(user).join(&tail)));
    }
    scan_start_menu(&roots)
}

#[cfg(windows)]
fn launch(app: &InstalledApp) -> Result<(), &'static str> {
    // `launch` est le chemin d'un raccourci trouvé dans un dossier du menu Démarrer par l'énumération qui vient d'avoir lieu.
    crate::open::shell_execute(&app.launch)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn scan() -> Vec<InstalledApp> {
    let mut roots: Vec<PathBuf> = [
        "/usr/share/applications",
        "/usr/local/share/applications",
        "/var/lib/flatpak/exported/share/applications",
        "/var/lib/snapd/desktop/applications",
    ]
    .map(PathBuf::from)
    .into();
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        roots.push(home.join(".local/share/applications"));
        roots.push(home.join(".local/share/flatpak/exported/share/applications"));
    }
    scan_desktop_files(&roots)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn launch(app: &InstalledApp) -> Result<(), &'static str> {
    // `launch` est un identifiant `.desktop` conforme (`is_plain_id`, tiret initial exclu) : il ne peut pas passer pour une option.
    std::process::Command::new("gtk-launch").arg(&app.launch).spawn().map(|_| ()).map_err(|_| APP_NOT_OPENED)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn temp_dir() -> PathBuf {
        static N: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!("scriibe-apps-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(path: &Path, text: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    fn plist(id: &str) -> String {
        format!("<?xml version=\"1.0\"?>\n<plist version=\"1.0\"><dict>\n<key>CFBundleName</key>\n<string>X</string>\n<key>CFBundleIdentifier</key>\n  <string>{id}</string>\n</dict></plist>")
    }

    /// Chantier 6, étape 3 : un identifiant donné à `open -b` ou `gtk-launch` ne peut pas passer pour une option ni contenir autre chose que
    /// des lettres, chiffres, `.`, `_`, `-`.
    #[test]
    fn commands_551_only_plain_identifiers_are_listed() {
        for ok in ["com.apple.Safari", "org.mozilla.firefox", "firefox", "gimp_2-10", "A"] {
            assert!(is_plain_id(ok), "{ok}");
        }
        for no in ["", "-a Calculator", "--help", ".hidden", "a b", "a;b", "a/b", "a$(b)", "a\nb", "é", "a`b", &"x".repeat(256)] {
            assert!(!is_plain_id(no), "{no:?}");
        }
        assert_eq!(plist_string(&plist("com.apple.Notes"), "CFBundleIdentifier").as_deref(), Some("com.apple.Notes"));
        assert_eq!(plist_string("<dict></dict>", "CFBundleIdentifier"), None);
    }

    /// Mac : les `.app` des dossiers (un niveau de sous-dossier compris) avec un identifiant conforme ; les autres sont ignorés.
    #[test]
    fn commands_552_bundles_are_listed_with_their_identifier() {
        let root = temp_dir();
        write(&root.join("Safari.app/Contents/Info.plist"), &plist("com.apple.Safari"));
        write(&root.join("Utilities/Terminal.app/Contents/Info.plist"), &plist("com.apple.Terminal"));
        write(&root.join("Piégée.app/Contents/Info.plist"), &plist("-a Calculator"));
        write(&root.join("Sans identifiant.app/Contents/Info.plist"), "<dict></dict>");
        write(&root.join("Doublon.app/Contents/Info.plist"), &plist("com.apple.Safari"));
        write(&root.join("A/B/Trop profond.app/Contents/Info.plist"), &plist("com.profond"));
        write(&root.join("pas une appli.txt"), "x");
        let apps = scan_bundles(std::slice::from_ref(&root), &|app| {
            plist_string(&std::fs::read_to_string(app.join("Contents/Info.plist")).ok()?, "CFBundleIdentifier")
        });
        let ids: Vec<&str> = apps.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(ids.len(), 2, "{ids:?}");
        assert!(ids.contains(&"com.apple.Safari") && ids.contains(&"com.apple.Terminal"), "{ids:?}");
        assert_eq!(find(&apps, "com.apple.Terminal").map(|a| a.name.as_str()), Some("Terminal"));
        assert!(find(&apps, "-a Calculator").is_none() && find(&apps, "com.profond").is_none());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Linux : seuls les `.desktop` d'applications visibles, au nom de fichier conforme.
    #[test]
    fn commands_553_desktop_files_are_listed_when_visible_and_well_named() {
        assert_eq!(desktop_name("[Desktop Entry]\nType=Application\nName=Firefox\nName[fr]=Navigateur\n").as_deref(), Some("Firefox"));
        assert_eq!(desktop_name("[Desktop Entry]\nType=Application\nName=X\nNoDisplay=true\n"), None);
        assert_eq!(desktop_name("[Desktop Entry]\nType=Application\nName=X\nHidden=true\n"), None);
        assert_eq!(desktop_name("[Desktop Entry]\nType=Link\nName=X\n"), None);
        assert_eq!(desktop_name("[Desktop Entry]\nType=Application\n"), None);
        assert_eq!(desktop_name("[Desktop Action x]\nName=Autre\n[Desktop Entry]\nType=Application\nName=Vrai\n").as_deref(), Some("Vrai"));
        let root = temp_dir();
        let entry = |name: &str| format!("[Desktop Entry]\nType=Application\nName={name}\nExec=x\n");
        write(&root.join("firefox.desktop"), &entry("Firefox"));
        write(&root.join("org.gnome.Calculator.desktop"), &entry("Calculatrice"));
        write(&root.join("-evil.desktop"), &entry("Piégée"));
        write(&root.join("a b.desktop"), &entry("Espace"));
        write(&root.join("sous/dossier.desktop"), &entry("Sous-dossier"));
        write(&root.join("notes.txt"), "x");
        let apps = scan_desktop_files(std::slice::from_ref(&root));
        let ids: Vec<&str> = apps.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(ids, ["org.gnome.Calculator", "firefox"], "triées par nom : Calculatrice, Firefox");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Windows : seuls les `.lnk` des dossiers donnés ; le chemin lancé est celui de la liste, jamais l'identifiant.
    #[test]
    fn commands_554_start_menu_shortcuts_are_listed_without_uninstallers() {
        let root = temp_dir();
        write(&root.join("Mozilla Firefox.lnk"), "x");
        write(&root.join("Outils/Bloc-notes.lnk"), "x");
        write(&root.join("Outils/Uninstall Outils.lnk"), "x");
        write(&root.join("Outils/Désinstaller.lnk"), "x");
        write(&root.join("Outils/Désinstallation.lnk"), "x");
        write(&root.join("readme.txt"), "x");
        write(&root.join("A/B/C/D/Trop profond.lnk"), "x");
        let apps = scan_start_menu(&[("commun", root.clone())]);
        let ids: Vec<&str> = apps.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(ids, ["commun/Outils/Bloc-notes.lnk", "commun/Mozilla Firefox.lnk"], "{ids:?}");
        let firefox = find(&apps, "commun/Mozilla Firefox.lnk").unwrap();
        assert_eq!(Path::new(&firefox.launch), root.join("Mozilla Firefox.lnk"));
        assert!(
            find(&apps, "commun/../../Windows/System32/cmd.exe").is_none(),
            "un identifiant qui n'est pas dans la liste ne désigne rien"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Une application absente de l'énumération fraîche n'est jamais lancée, quelle que soit la chaîne reçue.
    #[test]
    fn commands_555_an_app_outside_the_fresh_list_is_never_launched() {
        for hostile in ["", "-a Calculator", "../../bin/sh", "calc.exe", "/Applications/Calculator.app", "com.inexistant.application"] {
            assert_eq!(launch_app(hostile), Err(APP_NOT_FOUND), "{hostile:?}");
        }
    }

    /// À la main (`cargo test -p scriibe-platform installed_apps -- --ignored --nocapture`) : l'énumération **réelle** de ce système rend des
    /// applications aux identifiants uniques et sans caractère étrange. Rien n'est lancé.
    #[test]
    #[ignore = "énumère les vraies applications de cette machine"]
    fn commands_556_the_real_system_lists_installed_apps() {
        let list = installed_apps();
        println!("{} applications installées", list.len());
        assert!(!list.is_empty());
        let mut ids: Vec<&str> = list.iter().map(|a| a.id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), list.len(), "identifiants uniques");
        assert!(list.iter().all(|a| !a.name.is_empty() && !a.id.chars().any(char::is_control)));
        #[cfg(target_os = "macos")]
        {
            assert!(find(&list, "com.apple.finder").is_some() || find(&list, "com.apple.Safari").is_some());
            println!("{:?}", list.iter().take(8).map(|a| (&a.name, &a.id)).collect::<Vec<_>>());
        }
    }
}
