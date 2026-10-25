//! Pause des médias pendant la dictée (YouTube, musique…) et reprise ensuite.

pub trait MediaControl: Send + Sync {
    /// Met en pause ce qui joue, si `enabled` et si quelque chose joue (et pas un appel).
    fn pause_if_playing(&self, enabled: bool);
    /// Relance seulement ce que `pause_if_playing` a mis en pause.
    fn resume_if_paused(&self);
}

/// Aucune pause de média (système où elle n'est pas encore portée).
pub struct NoMedia;

impl MediaControl for NoMedia {
    fn pause_if_playing(&self, _: bool) {}
    fn resume_if_paused(&self) {}
}
