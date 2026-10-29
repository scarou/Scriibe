//! L'appli au premier plan : le processus de la fenêtre active (`_NET_ACTIVE_WINDOW`, puis `_NET_WM_PID`), nommé comme son exécutable.

use super::x11;
use crate::apps::{AppInfo, ForegroundApp};
use std::sync::Mutex;
use x11rb::protocol::xproto::{self, AtomEnum, ConnectionExt as _};
use x11rb::rust_connection::RustConnection;

#[derive(Default)]
pub struct X11Foreground {
    /// La connexion est gardée d'un appel à l'autre (un appel par appui de touche) ; refaite si elle tombe.
    conn: Mutex<Option<(RustConnection, xproto::Window)>>,
}

fn property(conn: &RustConnection, window: xproto::Window, name: &str, kind: AtomEnum) -> Option<u32> {
    let atom = conn.intern_atom(true, name.as_bytes()).ok()?.reply().ok()?.atom;
    if atom == 0 {
        return None;
    }
    let reply = conn.get_property(false, window, atom, kind, 0, 1).ok()?.reply().ok()?;
    let value = reply.value32()?.next();
    value
}

fn query(conn: &RustConnection, root: xproto::Window) -> Option<AppInfo> {
    let window = property(conn, root, "_NET_ACTIVE_WINDOW", AtomEnum::WINDOW).filter(|w| *w != 0)?;
    let pid = property(conn, window, "_NET_WM_PID", AtomEnum::CARDINAL)?;
    let name = std::fs::read_to_string(format!("/proc/{pid}/comm")).ok()?.trim().to_string();
    (!name.is_empty()).then(|| AppInfo { id: Some(name.to_lowercase()), name })
}

impl ForegroundApp for X11Foreground {
    fn current(&self) -> AppInfo {
        let mut slot = self.conn.lock().unwrap_or_else(|p| p.into_inner());
        for _ in 0..2 {
            if slot.is_none() {
                *slot = x11::connect().ok();
            }
            let Some((conn, root)) = slot.as_ref() else { return AppInfo::unknown() };
            if let Some(info) = query(conn, *root) {
                return info;
            }
            // Pas de réponse : la connexion est peut-être tombée (le serveur X a redémarré) ; un seul nouvel essai.
            if conn.get_input_focus().ok().and_then(|c| c.reply().ok()).is_some() {
                return AppInfo::unknown();
            }
            *slot = None;
        }
        AppInfo::unknown()
    }
}
