//! La préférence « réduire les animations » du système (chantier 8, logo animé).
//!
//! macOS et la webview de Windows la transmettent déjà à la page (`prefers-reduced-motion`). Sous Linux, WebKitGTK ne suit pas le réglage de
//! GNOME 50 (constaté sur le PC de l'auteur le 13/11/2026 : `reduced-motion = reduce` dans le portail, la page ne le voit pas) : Scriibe
//! lit donc le portail du bureau lui-même (`org.freedesktop.appearance` · `reduced-motion`) et le dit à l'interface par l'état (`reduced_motion`).

use std::time::Duration;

/// Combien de temps on garde la dernière lecture : l'état est relu toutes les 8 s, le portail n'a pas à être interrogé à chaque fois.
pub const KEEP_FOR: Duration = Duration::from_secs(5);

/// La lecture gardée est-elle encore bonne ?
pub fn is_fresh(age: Duration) -> bool {
    age < KEEP_FOR
}

/// Le système demande-t-il de réduire les animations ? Faux quand on ne le sait pas (portail absent, pas de session D-Bus) ou quand la
/// webview le transmet déjà (macOS, Windows).
pub fn reduced() -> bool {
    #[cfg(target_os = "linux")]
    {
        linux::reduced()
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use super::is_fresh;
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    static LAST: Mutex<Option<(Instant, bool)>> = Mutex::new(None);

    pub fn reduced() -> bool {
        let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((at, value)) = *last {
            if is_fresh(at.elapsed()) {
                return value;
            }
        }
        let value = read().unwrap_or(false);
        *last = Some((Instant::now(), value));
        value
    }

    /// Lecture par le portail, dans un fil à part (une boucle `tokio` d'un coup) ; quelques secondes au plus.
    fn read() -> Option<bool> {
        read_detailed().ok()
    }

    /// Comme `read`, avec la raison de l'échec (essai à la main : `cargo test -p scriibe-platform --lib motion -- --ignored --nocapture`).
    pub(super) fn read_detailed() -> Result<bool, String> {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("scriibe-mouvement".into())
            .spawn(move || {
                let value = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                    Err(e) => Err(format!("boucle : {e}")),
                    Ok(runtime) => runtime.block_on(async {
                        let settings = tokio::time::timeout(Duration::from_secs(2), ashpd::desktop::settings::Settings::new())
                            .await
                            .map_err(|_| "portail : pas de réponse".to_string())?
                            .map_err(|e| format!("portail : {e}"))?;
                        let value = tokio::time::timeout(Duration::from_secs(2), settings.reduced_motion())
                            .await
                            .map_err(|_| "lecture : pas de réponse".to_string())?
                            .map_err(|e| format!("lecture : {e}"))?;
                        Ok(matches!(value, ashpd::desktop::settings::ReducedMotion::ReducedMotion))
                    }),
                };
                let _ = tx.send(value);
            })
            .map_err(|e| format!("fil : {e}"))?;
        rx.recv_timeout(Duration::from_secs(5)).map_err(|e| format!("attente : {e}"))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reading_is_kept_for_a_few_seconds_only() {
        assert!(is_fresh(Duration::from_secs(0)));
        assert!(is_fresh(Duration::from_secs(4)));
        assert!(!is_fresh(KEEP_FOR));
        assert!(!is_fresh(Duration::from_secs(60)));
    }

    /// À la main sur un PC Linux avec une session de bureau : montre ce que le portail répond.
    #[cfg(target_os = "linux")]
    #[test]
    #[ignore]
    fn reads_the_portal_on_this_machine() {
        println!("réduire les animations : {:?}", linux::read_detailed());
    }

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn webviews_of_macos_and_windows_pass_the_preference_on_themselves() {
        assert!(!reduced());
    }
}
