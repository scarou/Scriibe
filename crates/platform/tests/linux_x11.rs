//! Linux (X11) essayé sur un écran virtuel (Xvfb) : touche globale, collage, fenêtre active. Sans `Xvfb`, `xdotool` et `xclip` dans le `PATH`,
//! l'essai se saute (il le dit). Un seul test : les essais partagent la variable `DISPLAY`.
#![cfg(target_os = "linux")]

use scriibe_core::hotkey::Key;
use scriibe_platform::apps::ForegroundApp;
use scriibe_platform::keys::{KeyEvent, KeyListener};
use scriibe_platform::linux::{LinuxKeys, X11Paster};
use scriibe_platform::paste::{PasteOutcome, Paster};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

fn have(program: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|path| std::env::split_paths(&path).any(|dir| dir.join(program).is_file()))
}

struct Screen {
    child: Child,
}

impl Screen {
    fn start() -> Screen {
        let number = 90 + std::process::id() % 200;
        let display = format!(":{number}");
        let child = Command::new("Xvfb")
            .args([&display, "-screen", "0", "800x600x24", "-nolisten", "tcp"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Xvfb se lance");
        let socket = format!("/tmp/.X11-unix/X{number}");
        let started = Instant::now();
        while !std::path::Path::new(&socket).exists() {
            assert!(started.elapsed() < Duration::from_secs(10), "Xvfb ne démarre pas");
            std::thread::sleep(Duration::from_millis(50));
        }
        std::env::set_var("DISPLAY", &display);
        std::env::remove_var("XDG_SESSION_TYPE");
        std::thread::sleep(Duration::from_millis(200));
        Screen { child }
    }
}

impl Drop for Screen {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn xdotool(args: &[&str]) {
    let status = Command::new("xdotool").args(args).stdout(Stdio::null()).stderr(Stdio::null()).status().expect("xdotool");
    assert!(status.success(), "xdotool {args:?}");
}

fn clipboard() -> String {
    let out = Command::new("xclip").args(["-o", "-selection", "clipboard"]).output().expect("xclip");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn set_clipboard(text: &str) {
    use std::io::Write;
    let mut child = Command::new("xclip")
        .args(["-i", "-selection", "clipboard"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("xclip");
    child.stdin.take().unwrap().write_all(text.as_bytes()).unwrap();
    // xclip reste propriétaire de la sélection en arrière-plan.
    std::mem::forget(child);
    std::thread::sleep(Duration::from_millis(200));
}

fn next(rx: &Receiver<KeyEvent>) -> Option<(Key, bool)> {
    match rx.recv_timeout(Duration::from_secs(3)) {
        Ok(KeyEvent::Changed { changed, down, .. }) => Some((changed, down)),
        Err(_) => None,
    }
}

#[test]
fn keys_paste_and_foreground_on_a_virtual_screen() {
    if !(have("Xvfb") && have("xdotool") && have("xclip")) {
        eprintln!("essai X11 sauté : Xvfb, xdotool et xclip sont nécessaires");
        return;
    }
    let _screen = Screen::start();

    // Touche globale : Ctrl droit et F9, vus avec leurs codes de Windows.
    let (tx, rx) = mpsc::channel();
    let subscription = LinuxKeys { accept_synthetic: true }.start(tx).expect("écoute des touches");
    xdotool(&["keydown", "105"]); // Ctrl droit (évdev 97 + 8) : un code, pas un nom, pour ne pas dépendre de la disposition de Xvfb
    assert_eq!(next(&rx), Some((Key::RightControl, true)));
    xdotool(&["keyup", "105"]);
    assert_eq!(next(&rx), Some((Key::RightControl, false)));
    xdotool(&["key", "75"]); // F9
    assert_eq!(next(&rx), Some((Key::Vk(0x78), true)));
    assert_eq!(next(&rx), Some((Key::Vk(0x78), false)));
    xdotool(&["key", "94"]); // « < > »
    assert!(matches!(next(&rx), Some((Key::Vk(_), true))), "une touche ordinaire est vue aussi");
    drop(subscription); // l'écoute s'arrête sans rester bloquée

    // Les touches envoyées par programme (notre propre Ctrl+V) sont ignorées par défaut.
    let (tx, rx) = mpsc::channel();
    let subscription = LinuxKeys::default().start(tx).expect("écoute des touches");
    xdotool(&["key", "75"]);
    assert!(rx.recv_timeout(Duration::from_millis(600)).is_err(), "les touches de XTest ne comptent pas");
    drop(subscription);

    // Collage : le texte est dans le presse-papiers, Ctrl+V part, l'ancien contenu revient ensuite.
    let (tx, rx) = mpsc::channel();
    let _watch = LinuxKeys { accept_synthetic: true }.start(tx).expect("écoute des touches");
    set_clipboard("ancien contenu");
    let paster = X11Paster::default();
    assert_eq!(paster.paste("Bonjour, ça va ? é ï €", true), PasteOutcome::Pasted);
    assert_eq!(next(&rx), Some((Key::LeftControl, true)));
    assert!(matches!(next(&rx), Some((Key::Vk(0x56), true))), "la touche V");
    assert_eq!(clipboard(), "Bonjour, ça va ? é ï €");
    std::thread::sleep(Duration::from_millis(2200));
    assert_eq!(clipboard(), "ancien contenu", "l'ancien presse-papiers est remis");
    assert!(paster.copy("copié seulement"));
    assert_eq!(clipboard(), "copié seulement");

    // Fenêtre active : le processus de la fenêtre que le gestionnaire de fenêtres dit active.
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{self, AtomEnum, ConnectionExt as _};
    use x11rb::wrapper::ConnectionExt as _;
    let (conn, screen) = x11rb::connect(None).unwrap();
    let root = conn.setup().roots[screen].root;
    let window = conn.generate_id().unwrap();
    conn.create_window(0, window, root, 0, 0, 10, 10, 0, xproto::WindowClass::INPUT_OUTPUT, 0, &xproto::CreateWindowAux::new()).unwrap();
    let atom = |name: &str| conn.intern_atom(false, name.as_bytes()).unwrap().reply().unwrap().atom;
    conn.change_property32(xproto::PropMode::REPLACE, window, atom("_NET_WM_PID"), AtomEnum::CARDINAL, &[std::process::id()]).unwrap();
    let foreground = scriibe_platform::linux::foreground_for_tests();
    assert_eq!(foreground.current().name, "Inconnue", "aucune fenêtre active : le gestionnaire de fenêtres n'a rien dit");
    conn.change_property32(xproto::PropMode::REPLACE, root, atom("_NET_ACTIVE_WINDOW"), AtomEnum::WINDOW, &[window]).unwrap();
    conn.flush().unwrap();
    let app = foreground.current();
    let own = std::fs::read_to_string("/proc/self/comm").unwrap().trim().to_string();
    assert_eq!((app.name.as_str(), app.id.as_deref()), (own.as_str(), Some(own.to_lowercase().as_str())));

    // Wayland : refusé avec une explication, sans rien tenter.
    std::env::set_var("XDG_SESSION_TYPE", "wayland");
    let (tx, _rx) = mpsc::channel();
    let error = LinuxKeys::default().start(tx).err().expect("refusé sous Wayland");
    assert!(error.contains("Wayland"), "{error}");
    std::env::remove_var("XDG_SESSION_TYPE");
}
