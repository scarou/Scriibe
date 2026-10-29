//! La fenêtre de Scriibe (Windows) : Tauri 2 et WebView2 affichent l'interface que sert le serveur local. Fermer la fenêtre ne quitte
//! pas Scriibe (la dictée reste active, l'icône de la zone de notification la rouvre) ; « Quitter » est dans l'interface et le menu de l'icône.
//!
//! La boucle de Tauri tourne dans son propre fil (`any_thread`) : le fil principal garde la boucle de messages de l'icône et du
//! crochet clavier. Si WebView2 manque ou refuse de démarrer, l'appelant retombe sur le navigateur.

use crate::overlay::{origin_x, OverlayModel, BOTTOM_GAP, WINDOW_SIZE};
use scriibe_engines::runtime::Log;
use scriibe_platform::storage::FileConfig;
use std::panic::AssertUnwindSafe;
use std::sync::{mpsc, Arc, Mutex, MutexGuard};
use std::time::Duration;
use tauri::{AppHandle, Manager, PhysicalPosition, RunEvent, Url, WebviewUrl, WebviewWindowBuilder};

const LABEL: &str = "main";
const OVERLAY_LABEL: &str = "overlay";

static HANDLE: Mutex<Option<AppHandle>> = Mutex::new(None);

fn handle() -> MutexGuard<'static, Option<AppHandle>> {
    HANDLE.lock().unwrap_or_else(|p| p.into_inner())
}

/// Montre la fenêtre (la crée si elle a été fermée) sur cette adresse.
fn show(app: &AppHandle, url: &str) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.unminimize();
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
                // Sans l'activer : `show()` prendrait le focus quand Scriibe est au premier plan, et la fenêtre perdrait la touche tenue.
                match window.hwnd() {
                    Ok(hwnd) => scriibe_platform::windows::show_without_activating(hwnd.0 as isize),
                    Err(_) => {
                        let _ = window.show();
                    }
                }
            } else {
                let _ = window.hide();
            }
            shown = want;
        }
    });
}
