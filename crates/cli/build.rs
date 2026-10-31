//! La fenêtre de Scriibe vient de Tauri sous Windows (WebView2) et macOS (WKWebView) : son contexte (icône, identifiant) est généré ici. Linux
//! garde le navigateur jusqu'à son étape.

fn main() {
    // `cfg!(windows)` désignerait la machine qui compile, pas la cible (compilation croisée depuis Linux).
    if matches!(std::env::var("CARGO_CFG_TARGET_OS").as_deref(), Ok("windows" | "macos")) {
        tauri_build::build();
    }
}
