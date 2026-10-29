//! Ce que les modules X11 ont en commun : la connexion, le refus d'une session Wayland, les codes de touche.

use x11rb::connection::Connection;
use x11rb::protocol::xproto::{self, ConnectionExt as _};
use x11rb::rust_connection::RustConnection;

/// Ouvre une connexion à l'écran X11 (variable `DISPLAY`). Une session Wayland est refusée avec un message qui l'explique : XWayland
/// connecte bien, mais ne montre à Scriibe ni les touches des autres fenêtres ni leur focus.
pub fn connect() -> Result<(RustConnection, xproto::Window), String> {
    if std::env::var("XDG_SESSION_TYPE").is_ok_and(|t| t.eq_ignore_ascii_case("wayland")) && std::env::var_os("SCRIIBE_FORCE_X11").is_none()
    {
        return Err(
            "session Wayland : la touche globale et le collage ne sont pas encore pris en charge (ouvrez une session « X11 » ou « Xorg »)"
                .into(),
        );
    }
    let (conn, screen) = RustConnection::connect(None).map_err(|e| format!("écran X11 introuvable ({e}) : variable DISPLAY ?"))?;
    let root = conn.setup().roots[screen].root;
    Ok((conn, root))
}

/// Le code de touche qui produit ce symbole (`keysym`) avec la disposition du clavier en vigueur (« v » est à la même place en AZERTY
/// et en QWERTY, pas en Dvorak).
pub fn keycode_for(conn: &RustConnection, keysyms: &[u32]) -> Option<u8> {
    let setup = conn.setup();
    let (min, max) = (setup.min_keycode, setup.max_keycode);
    let mapping = conn.get_keyboard_mapping(min, max - min + 1).ok()?.reply().ok()?;
    let per = usize::from(mapping.keysyms_per_keycode);
    if per == 0 {
        return None;
    }
    keysyms.iter().find_map(|wanted| {
        mapping.keysyms.chunks(per).position(|syms| syms.contains(wanted)).and_then(|i| u8::try_from(usize::from(min) + i).ok())
    })
}
