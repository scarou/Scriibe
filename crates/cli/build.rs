//! La fenêtre de Scriibe vient de Tauri sous Windows (WebView2) : son contexte (icône, identifiant) est généré ici. Les autres systèmes
//! gardent le navigateur jusqu'à leur étape (7).

fn main() {
    // `cfg!(windows)` désignerait la machine qui compile, pas la cible (compilation croisée depuis Linux).
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        tauri_build::build();
    }
}
