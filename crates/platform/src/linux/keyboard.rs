//! Touche globale : les événements « bruts » de XInput2 (`XI_RawKeyPress` / `XI_RawKeyRelease`) sur la fenêtre racine donnent chaque touche
//! du clavier, quelle que soit la fenêtre qui a le focus, sans rien retenir : l'appli active reçoit les touches comme avant.
//!
//! Écart avec Windows (`docs/ECARTS.md`) : une touche ordinaire de la combinaison (F9, « < > ») n'est pas retenue, elle écrit donc aussi
//! dans l'appli active ; les touches de modification (Ctrl droit, par défaut) n'écrivent rien. Les touches envoyées par programme
//! (notre propre Ctrl+V, via XTest) sont ignorées.

use super::evdev::vk_of_keycode;
use super::x11;
use crate::keys::{translate_vk, KeyEvent, KeyListener, Subscription};
use scriibe_core::hotkey::Key;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::Arc;
use std::thread::JoinHandle;
use x11rb::connection::Connection;
use x11rb::protocol::xinput::{self, ConnectionExt as _, XIEventMask};
use x11rb::protocol::xproto::{self, ConnectionExt as _};
use x11rb::protocol::Event;
use x11rb::rust_connection::RustConnection;

/// `keyboard`: écoute des touches ; `accept_synthetic` : prend aussi celles de XTest (essais sur un écran virtuel).
#[derive(Default)]
pub struct LinuxKeys {
    pub accept_synthetic: bool,
}

const STOP_ATOM: &str = "SCRIIBE_STOP_KEYS";

struct Listening {
    stop: Arc<AtomicBool>,
    /// Le fil a fini : on ne l'attend pas plus de 2 s à l'arrêt (un message perdu ne doit pas bloquer la fermeture de l'appli).
    finished: Arc<AtomicBool>,
    window: xproto::Window,
    handle: Option<JoinHandle<()>>,
}

impl Subscription for Listening {}

impl Drop for Listening {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Réveille le fil, bloqué en attente d'un événement : un message envoyé à sa fenêtre cachée.
        if let Ok((conn, _)) = x11::connect() {
            if let Ok(atom) =
                conn.intern_atom(false, STOP_ATOM.as_bytes()).map_err(|e| e.to_string()).and_then(|c| c.reply().map_err(|e| e.to_string()))
            {
                let event = xproto::ClientMessageEvent::new(32, self.window, atom.atom, [0u32; 5]);
                // Attendu jusqu'au bout (`check`) : la connexion ne doit pas se fermer avant que le message soit parti.
                let _ = conn
                    .send_event(false, self.window, xproto::EventMask::NO_EVENT, event)
                    .map_err(|e| e.to_string())
                    .and_then(|c| c.check().map_err(|e| e.to_string()));
            }
        }
        let started = std::time::Instant::now();
        while !self.finished.load(Ordering::SeqCst) && started.elapsed() < std::time::Duration::from_secs(2) {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        if let Some(handle) = self.handle.take() {
            if self.finished.load(Ordering::SeqCst) {
                let _ = handle.join();
            }
        }
    }
}

/// L'identifiant du clavier « virtuel » de XTest (où arrivent les touches envoyées par programme), s'il existe.
fn xtest_device(conn: &RustConnection) -> Option<u16> {
    let devices = conn.xinput_xi_query_device(u16::from(xinput::Device::ALL)).ok()?.reply().ok()?;
    devices
        .infos
        .iter()
        .find(|d| String::from_utf8_lossy(&d.name).contains("XTEST") && d.type_ == xinput::DeviceType::SLAVE_KEYBOARD)
        .map(|d| d.deviceid)
}

impl KeyListener for LinuxKeys {
    fn start(&self, sink: Sender<KeyEvent>) -> Result<Box<dyn Subscription>, String> {
        let (conn, root) = x11::connect()?;
        let version =
            conn.xinput_xi_query_version(2, 0).map_err(|e| e.to_string())?.reply().map_err(|e| format!("XInput2 absent ({e})"))?;
        if version.major_version < 2 {
            return Err("XInput2 absent : la touche globale est impossible avec ce serveur X".into());
        }
        let ignored_device = if self.accept_synthetic { None } else { xtest_device(&conn) };

        // Une fenêtre sans contenu, seulement pour recevoir le message qui arrête l'écoute.
        let window = conn.generate_id().map_err(|e| e.to_string())?;
        conn.create_window(0, window, root, 0, 0, 1, 1, 0, xproto::WindowClass::INPUT_ONLY, 0, &xproto::CreateWindowAux::new())
            .map_err(|e| e.to_string())?;
        let mask = xinput::EventMask {
            deviceid: u16::from(xinput::Device::ALL_MASTER),
            mask: vec![XIEventMask::RAW_KEY_PRESS | XIEventMask::RAW_KEY_RELEASE],
        };
        conn.xinput_xi_select_events(root, &[mask])
            .map_err(|e| e.to_string())?
            .check()
            .map_err(|e| format!("événements du clavier refusés ({e})"))?;
        conn.flush().map_err(|e| e.to_string())?;

        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let finished = Arc::new(AtomicBool::new(false));
        let done = finished.clone();
        let (ready_tx, ready_rx) = mpsc::channel::<()>();
        let handle = std::thread::Builder::new()
            .name("scriibe-clavier".into())
            .spawn(move || {
                let _ = ready_tx.send(());
                let mut held: Vec<Key> = Vec::new();
                while !flag.load(Ordering::SeqCst) {
                    let Ok(event) = conn.wait_for_event() else { break };
                    let (keycode, down, source) = match event {
                        Event::XinputRawKeyPress(e) => (e.detail, true, e.sourceid),
                        Event::XinputRawKeyRelease(e) => (e.detail, false, e.sourceid),
                        _ => continue,
                    };
                    if Some(source) == ignored_device {
                        continue;
                    }
                    if let Some(event) = translate_vk(&mut held, vk_of_keycode(keycode), down) {
                        let _ = sink.send(event);
                    }
                }
                done.store(true, Ordering::SeqCst);
            })
            .map_err(|e| format!("fil du clavier : {e}"))?;
        let _ = ready_rx.recv();
        Ok(Box::new(Listening { stop, finished, window, handle: Some(handle) }))
    }
}
