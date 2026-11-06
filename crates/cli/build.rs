//! La fenêtre de Scriibe vient de Tauri sous Windows (WebView2), macOS (WKWebView) et Linux (WebKitGTK) : son contexte (icône, identifiant)
//! est généré ici. Le commit compilé est aussi noté ici (`SCRIIBE_COMMIT`) : la CI le donne par `GITHUB_SHA`, une
//! compilation à la main le lit dans git ; il sert à reconnaître la version installée (alerte de nouvelle version).

fn main() {
    // `cfg!(windows)` désignerait la machine qui compile, pas la cible (compilation croisée depuis Linux).
    if matches!(std::env::var("CARGO_CFG_TARGET_OS").as_deref(), Ok("windows" | "macos" | "linux")) {
        tauri_build::build();
    }
    let commit = std::env::var("GITHUB_SHA").ok().filter(|s| !s.is_empty()).or_else(|| {
        let out = std::process::Command::new("git").args(["rev-parse", "HEAD"]).output().ok()?;
        let sha = String::from_utf8(out.stdout).ok()?.trim().to_string();
        (out.status.success() && sha.len() == 40 && sha.chars().all(|c| c.is_ascii_hexdigit())).then_some(sha)
    });
    println!("cargo:rustc-env=SCRIIBE_COMMIT={}", commit.unwrap_or_default());
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../.git/refs/heads");
}
