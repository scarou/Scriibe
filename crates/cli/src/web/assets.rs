//! L'interface (`ui/`, construite par Vite dans `ui/dist`) embarquée dans le programme : le serveur local la sert sous l'adresse secrète.

use std::borrow::Cow;

#[derive(rust_embed::RustEmbed)]
#[folder = "../../ui/dist"]
#[allow_missing = true]
struct Ui;

/// Ce qu'on voit quand le programme a été compilé sans avoir construit l'interface (`cd ui && npm ci && npm run build`).
const MISSING: &str = "<!doctype html><meta charset=\"utf-8\"><title>Scriibe</title>\
<body style=\"font:16px system-ui;max-width:40em;margin:3em auto\"><h1>Interface absente</h1>\
<p>Ce programme a été compilé sans l’interface. Dans le dossier du dépôt : <code>cd ui &amp;&amp; npm ci &amp;&amp; npm run build</code>, \
puis recompiler.</p></body>";

fn content_type(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "wav" => "audio/wav",
        _ => "application/octet-stream",
    }
}

/// Le fichier de l'interface à cette adresse relative (« » = la page d'accueil) : contenu et type.
pub fn get(path: &str) -> Option<(Cow<'static, [u8]>, &'static str)> {
    let path = if path.is_empty() { "index.html" } else { path };
    // Rien ne remonte hors du dossier de l'interface.
    if path.split('/').any(|part| part == ".." || part.is_empty()) {
        return None;
    }
    match Ui::get(path) {
        Some(file) => Some((file.data, content_type(path))),
        None if path == "index.html" => Some((Cow::Borrowed(MISSING.as_bytes()), content_type(path))),
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_home_page_is_always_answered_and_paths_cannot_climb() {
        let (body, kind) = get("").expect("page d'accueil (ou message d'interface absente)");
        assert!(!body.is_empty());
        assert!(kind.starts_with("text/html"));
        assert!(get("../Cargo.toml").is_none());
        assert!(get("assets//x.js").is_none());
        assert!(get("assets/nexiste-pas.js").is_none());
    }

    #[test]
    fn content_types_follow_the_extension() {
        assert_eq!(content_type("assets/app.js"), "text/javascript; charset=utf-8");
        assert_eq!(content_type("a.css"), "text/css; charset=utf-8");
        assert_eq!(content_type("inconnu.xyz"), "application/octet-stream");
    }
}
