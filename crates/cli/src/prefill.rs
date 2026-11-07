//! « Ajouter au dictionnaire » par la voix : le texte sélectionné attend, en mémoire seulement, que la page le prenne.
//!
//! Jamais écrit sur disque, jamais journalisé ; il disparaît dès que la page l'a pris, ou au bout de [`KEEP`] s'il n'a pas été pris.

use scriibe_pipeline::CommandSink;
use scriibe_platform::paste::SelectedText;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

/// Combien de temps la demande attend que la page vienne la chercher.
const KEEP: Duration = Duration::from_secs(60);

#[derive(Default)]
pub struct PrefillSink {
    pending: Mutex<Option<(SelectedText, Instant)>>,
    /// L'adresse de l'interface (avec son jeton), connue quand le serveur local est démarré.
    url: Mutex<Option<String>>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

impl PrefillSink {
    pub fn set_url(&self, url: String) {
        *lock(&self.url) = Some(url);
    }

    /// Une demande attend-elle la page ? (Lue chaque seconde par la page : ne rend jamais le texte.)
    pub fn has_pending(&self) -> bool {
        let mut pending = lock(&self.pending);
        if pending.as_ref().is_some_and(|(_, at)| at.elapsed() > KEEP) {
            *pending = None;
        }
        pending.is_some()
    }

    /// La page prend le texte : il n'est plus gardé ici.
    pub fn take(&self) -> Option<SelectedText> {
        lock(&self.pending).take().filter(|(_, at)| at.elapsed() <= KEEP).map(|(text, _)| text)
    }
}

impl CommandSink for PrefillSink {
    fn add_to_dictionary(&self, heard: SelectedText) -> Result<(), &'static str> {
        let Some(url) = lock(&self.url).clone() else {
            return Err("Scriibe n'a pas pu ouvrir sa fenêtre");
        };
        *lock(&self.pending) = Some((heard, Instant::now()));
        scriibe_platform::open::open(&url);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_text_is_taken_once_and_never_shown_by_debug() {
        let sink = PrefillSink::default();
        assert_eq!(
            sink.add_to_dictionary(SelectedText::new("cri be")),
            Err("Scriibe n'a pas pu ouvrir sa fenêtre"),
            "sans adresse, rien n'est gardé"
        );
        assert!(!sink.has_pending());
        *lock(&sink.pending) = Some((SelectedText::new("cri be"), Instant::now()));
        assert!(sink.has_pending());
        let text = sink.take().unwrap();
        assert_eq!(text.expose(), "cri be");
        assert!(!format!("{text:?}").contains("cri be"));
        assert!(!sink.has_pending() && sink.take().is_none());
    }

    #[test]
    fn an_old_request_is_dropped() {
        let sink = PrefillSink::default();
        *lock(&sink.pending) = Some((SelectedText::new("mot"), Instant::now() - KEEP - Duration::from_secs(1)));
        assert!(!sink.has_pending());
        assert!(sink.take().is_none());
    }
}
