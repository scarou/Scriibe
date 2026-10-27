//! La fenêtre de Scriibe (Windows) : Tauri 2 et WebView2 affichent l'interface que sert le serveur local. Fermer la fenêtre ne quitte
//! pas Scriibe (la dictée reste active, l'icône de la zone de notification la rouvre) ; « Quitter » est dans l'interface et le menu de l'icône.
//!
//! La boucle de Tauri tourne dans son propre fil (`any_thread`) : le fil principal garde la boucle de messages de l'icône et du
//! crochet clavier. Si WebView2 manque ou refuse de démarrer, l'appelant retombe sur le navigateur.

use scriibe_engines::runtime::Log;
use std::panic::AssertUnwindSafe;
use std::sync::{mpsc, Arc, Mutex, MutexGuard};
use std::time::Duration;
use tauri::{AppHandle, Manager, RunEvent, Url, WebviewUrl, WebviewWindowBuilder};

const LABEL: &str = "main";

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
