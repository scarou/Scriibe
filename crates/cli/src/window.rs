//! La fenêtre de Scriibe (Windows, macOS) : Tauri 2 (WebView2, WKWebView) affiche l'interface que sert le serveur local. Fermer la fenêtre
//! ne quitte pas Scriibe (la dictée reste active, l'icône la rouvre) ; « Quitter » est dans l'interface et le menu de l'icône.
//!
//! Windows : la boucle de Tauri tourne dans son propre fil (`any_thread`) ; le fil principal garde la boucle de messages de l'icône et du
//! crochet clavier. Si WebView2 manque ou refuse de démarrer, l'appelant retombe sur le navigateur.
//!
//! macOS : Tauri (comme AppKit) exige le fil principal ; il le garde donc (`run_main`) et porte aussi l'icône de la barre des menus
//! (`mac_tray`). L'appli est « accessoire » : pas d'icône dans le Dock, comme Scribe.
//!
//! Linux : comme macOS (`run_main`, fil principal, icône par `mac_tray`, qui passe par AppIndicator), sans pastille pour l'instant (Wayland
//! ne laisse pas une appli placer une fenêtre ni la garder devant sans prendre le focus).

use crate::overlay::OverlayModel;
#[cfg(any(windows, target_os = "macos"))]
use crate::overlay::{origin_x, BOTTOM_GAP, WINDOW_SIZE};
use scriibe_engines::runtime::Log;
use scriibe_platform::storage::FileConfig;
#[cfg(windows)]
use std::panic::AssertUnwindSafe;
#[cfg(windows)]
use std::sync::mpsc;
use std::sync::{Arc, Mutex, MutexGuard};
#[cfg(any(windows, target_os = "macos"))]
use std::time::Duration;
#[cfg(any(windows, target_os = "macos"))]
use tauri::PhysicalPosition;
use tauri::{AppHandle, Manager, RunEvent, Url, WebviewUrl, WebviewWindowBuilder};

const LABEL: &str = "main";
#[cfg(any(windows, target_os = "macos"))]
const OVERLAY_LABEL: &str = "overlay";

static HANDLE: Mutex<Option<AppHandle>> = Mutex::new(None);

fn handle() -> MutexGuard<'static, Option<AppHandle>> {
    HANDLE.lock().unwrap_or_else(|p| p.into_inner())
}

/// Montre la fenêtre (la crée si elle a été fermée) sur cette adresse.
fn show(app: &AppHandle, url: &str) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.unminimize();
        // Wayland : une appli ne peut ni sortir sa fenêtre de la barre des tâches ni la passer devant ; cachée puis remontrée,
        // la fenêtre revient comme une nouvelle, devant.
        #[cfg(target_os = "linux")]
        let _ = window.hide();
        let _ = window.show();
        return window.set_focus().map_err(|e| e.to_string());
    }
    let url: Url = url.parse().map_err(|e| format!("adresse illisible ({e})"))?;
    WebviewWindowBuilder::new(app, LABEL, WebviewUrl::External(url))
        .title("Scriibe")
        .inner_size(1040.0, 760.0)
        .min_inner_size(640.0, 480.0)
        .build()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Ouvre la fenêtre et rend la main quand elle est affichée (20 s au plus). Les ouvertures suivantes de l'adresse locale (menu de
/// l'icône) passent par elle.
#[cfg(windows)]
pub fn start(url: String, log: Arc<dyn Log>) -> Result<(), String> {
    let (ready, waiting) = mpsc::channel::<Result<(), String>>();
    let thread_log = log.clone();
    std::thread::Builder::new()
        .name("scriibe-fenetre".into())
        .spawn(move || {
            let failed = ready.clone();
            let outcome = std::panic::catch_unwind(AssertUnwindSafe(|| run(url, ready)));
            match outcome {
                Ok(Ok(())) => thread_log.write("fenêtre: fermée définitivement"),
                Ok(Err(e)) => {
                    let _ = failed.send(Err(e));
                }
                Err(_) => {
                    let _ = failed.send(Err("la fenêtre a planté (WebView2 absent ?)".into()));
                }
            }
        })
        .map_err(|e| e.to_string())?;
    match waiting.recv_timeout(Duration::from_secs(20)) {
        Ok(Ok(())) => {
            log.write("fenêtre: Tauri (WebView2) ouverte");
            scriibe_platform::open::set_window_opener(move |target| match handle().as_ref() {
                Some(app) => show(app, target).is_ok(),
                None => false,
            });
            Ok(())
        }
        Ok(Err(e)) => Err(e),
        Err(_) => Err("la fenêtre ne s'est pas ouverte en 20 s".into()),
    }
}

#[cfg(windows)]
fn run(url: String, ready: mpsc::Sender<Result<(), String>>) -> Result<(), String> {
    let app = tauri::Builder::default()
        .any_thread()
        .setup(move |app| {
            show(app.handle(), &url)?;
            *handle() = Some(app.handle().clone());
            let _ = ready.send(Ok(()));
            Ok(())
        })
        .build(tauri::generate_context!())
        .map_err(|e| e.to_string())?;
    app.run(|_, event| {
        // Fermer la dernière fenêtre ne quitte pas Scriibe.
        if let RunEvent::ExitRequested { api, code: None, .. } = event {
            api.prevent_exit();
        }
    });
    Ok(())
}

#[cfg(any(windows, target_os = "macos"))]
/// La fenêtre de la pastille : sans cadre, transparente, toujours devant, jamais activée (le texte doit se coller dans l'appli qui a le focus)
/// et insensible à la souris. Elle charge la même interface avec « #overlay ».
fn build_overlay(app: &AppHandle, url: &str) -> Result<tauri::WebviewWindow, String> {
    let url: Url = format!("{url}#overlay").parse().map_err(|e| format!("adresse illisible ({e})"))?;
    let window = WebviewWindowBuilder::new(app, OVERLAY_LABEL, WebviewUrl::External(url))
        .title("Scriibe — indicateur")
        .inner_size(WINDOW_SIZE.0, WINDOW_SIZE.1)
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .focused(false)
        .focusable(false)
        .visible(false)
        .build()
        .map_err(|e| e.to_string())?;
    let _ = window.set_ignore_cursor_events(true);
    Ok(window)
}

#[cfg(any(windows, target_os = "macos"))]
/// Pose la fenêtre en bas de la zone de travail de l'écran où se trouve la souris, selon la position choisie.
fn place_overlay(app: &AppHandle, window: &tauri::WebviewWindow, position: scriibe_core::settings::CapturePosition) {
    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|p| app.monitor_from_point(p.x, p.y).ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else { return };
    let (area, scale) = (monitor.work_area(), monitor.scale_factor());
    let (width, height) = (WINDOW_SIZE.0 * scale, WINDOW_SIZE.1 * scale);
    let x = origin_x(position, f64::from(area.position.x), f64::from(area.size.width), width);
    let y = f64::from(area.position.y) + f64::from(area.size.height) - BOTTOM_GAP * scale - height;
    let _ = window.set_position(PhysicalPosition::new(x.round() as i32, y.round() as i32));
}

/// Montre la pastille sans l'activer : `show()` prendrait le focus quand Scriibe est au premier plan, et la fenêtre perdrait la touche tenue
/// (Windows) ou le texte ne se collerait plus dans l'appli où la personne écrit (macOS).
#[cfg(windows)]
fn reveal(_: &AppHandle, window: &tauri::WebviewWindow) {
    match window.hwnd() {
        Ok(hwnd) => scriibe_platform::windows::show_without_activating(hwnd.0 as isize),
        Err(_) => {
            let _ = window.show();
        }
    }
}

#[cfg(target_os = "macos")]
fn reveal(app: &AppHandle, window: &tauri::WebviewWindow) {
    // Les fenêtres d'AppKit ne se touchent que depuis le fil principal.
    let window = window.clone();
    let _ = app.run_on_main_thread(move || match window.ns_window() {
        Ok(ns_window) => scriibe_platform::macos::order_front_without_activating(ns_window),
        Err(_) => {
            let _ = window.show();
        }
    });
}

#[cfg(any(windows, target_os = "macos"))]
/// Surveille l'état de la pastille : la montre et la cache avec l'état de la dictée quand le réglage « Afficher la pastille » est coché.
/// La fenêtre est créée (cachée) au premier besoin, puis réutilisée.
pub fn start_overlay(url: String, model: Arc<OverlayModel>, config: Arc<FileConfig>, log: Arc<dyn Log>) {
    let _ = std::thread::Builder::new().name("scriibe-pastille".into()).spawn(move || {
        let mut shown = false;
        let mut failed = false;
        loop {
            std::thread::sleep(Duration::from_millis(40));
            let settings = config.settings();
            let want = settings.overlay_enabled && model.snapshot().visible();
            if failed || want == shown {
                continue;
            }
            let Some(app) = handle().clone() else { continue };
            let window = match app.get_webview_window(OVERLAY_LABEL) {
                Some(window) => window,
                None if want => match build_overlay(&app, &url) {
                    Ok(window) => {
                        log.write("pastille: fenêtre créée");
                        window
                    }
                    Err(e) => {
                        log.write(&format!("pastille: impossible ({e})"));
                        failed = true;
                        continue;
                    }
                },
                None => {
                    shown = false;
                    continue;
                }
            };
            if want {
                place_overlay(&app, &window, settings.capture_position);
                reveal(&app, &window);
            } else {
                let _ = window.hide();
            }
            shown = want;
        }
    });
}

/// macOS : ouvre la fenêtre, l'icône de la barre des menus et la pastille, puis fait tourner Tauri sur le fil principal. Ne rend la main
/// qu'en cas d'échec avant le démarrage de la boucle (l'appelant retombe alors sur le navigateur) : « Quitter » arrête le programme.
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub struct MainSetup {
    pub url: String,
    /// Ouvrir la fenêtre au démarrage (pas avec `--sans-navigateur`).
    pub show_window: bool,
    pub log: Arc<dyn Log>,
    pub overlay: Arc<OverlayModel>,
    pub config: Arc<FileConfig>,
    pub tray: crate::mac_tray::Shell,
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub fn run_main(setup: MainSetup) -> Result<(), String> {
    let MainSetup { url, show_window, log, overlay, config, tray } = setup;
    #[cfg(target_os = "macos")]
    let reopen_url = url.clone();
    let app = tauri::Builder::default()
        .on_menu_event(|_, event| crate::mac_tray::on_menu_event(event.id().as_ref()))
        .setup(move |app| {
            // Pas d'icône dans le Dock ni de menu en haut de l'écran : l'icône de la barre des menus suffit (comme Scribe).
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            // Sans menu « Édition », ⌘C, ⌘V et ⌘A ne marchent pas dans les champs de la page.
            #[cfg(target_os = "macos")]
            app.set_menu(crate::mac_tray::app_menu(app.handle())?).map_err(|e| e.to_string())?;
            *handle() = Some(app.handle().clone());
            scriibe_platform::open::set_window_opener(move |target| match handle().as_ref() {
                Some(app) => show(app, target).is_ok(),
                None => false,
            });
            if show_window {
                show(app.handle(), &url)?;
            }
            // Linux : sans AppIndicator (bureau sans zone d'icônes), la fenêtre reste ; elle se rouvre par le lanceur.
            if let Err(e) = crate::mac_tray::install(app.handle(), tray) {
                if cfg!(target_os = "macos") {
                    return Err(e.into());
                }
                log.write(&format!("icône: impossible ({e})"));
            }
            #[cfg(target_os = "macos")]
            start_overlay(url.clone(), overlay, config, log.clone());
            #[cfg(target_os = "linux")]
            let _ = (overlay, config);
            log.write("fenêtre: Tauri prête, icône installée");
            Ok(())
        })
        .build(tauri::generate_context!())
        .map_err(|e| e.to_string())?;
    #[cfg_attr(not(target_os = "macos"), allow(unused_variables))]
    app.run(move |app, event| match event {
        // Fermer la dernière fenêtre ne quitte pas Scriibe.
        RunEvent::ExitRequested { api, code: None, .. } => api.prevent_exit(),
        // Un second double-clic sur Scriibe.app, ou « Ouvrir » dans le Finder : montrer la fenêtre.
        #[cfg(target_os = "macos")]
        RunEvent::Reopen { has_visible_windows: false, .. } => {
            let _ = show(app, &reopen_url);
        }
        _ => {}
    });
    Ok(())
}
