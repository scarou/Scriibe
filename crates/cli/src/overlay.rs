//! L'indicateur à l'écran (pastille, point ou vumètre) : ce que Scriibe montre pendant une dictée, en plus de l'icône de la zone de
//! notification. Ce module garde l'état (port de `OverlayModel` de Scribe) ; la fenêtre qui l'affiche est `window.rs` (Windows) et l'interface
//! web (`ui/src/overlay/`) dessine la pastille.

use scriibe_core::microphone::MicLevel;
use scriibe_core::settings::CapturePosition;
use scriibe_platform::indicator::{Indicator, IndicatorState};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

/// Taille de la fenêtre d'indicateur : assez haute pour poser une légende au-dessus du point ou du vumètre.
#[cfg_attr(not(windows), allow(dead_code))] // utilisé par la fenêtre de la pastille (Windows) et ses tests
pub const WINDOW_SIZE: (f64, f64) = (440.0, 110.0);
/// Écart entre la fenêtre et le bord de l'écran quand l'indicateur est à gauche ou à droite.
#[cfg_attr(not(windows), allow(dead_code))] // utilisé par la fenêtre de la pastille (Windows) et ses tests
pub const MARGIN: f64 = 12.0;
/// Écart entre le bas de la fenêtre et le bas de la zone de travail (au-dessus de la barre des tâches).
#[cfg_attr(not(windows), allow(dead_code))] // utilisé par la fenêtre de la pastille (Windows) et ses tests
pub const BOTTOM_GAP: f64 = 36.0;

/// Bord gauche de la fenêtre dans une zone de travail de départ `area_x` et de largeur `area_width` (`ui_logic-001` à `-003`, comme
/// `OverlayController.originX` de Scribe).
#[cfg_attr(not(windows), allow(dead_code))] // utilisé par la fenêtre de la pastille (Windows) et ses tests
pub fn origin_x(position: CapturePosition, area_x: f64, area_width: f64, width: f64) -> f64 {
    match position {
        CapturePosition::Left => area_x + MARGIN,
        CapturePosition::Center => area_x + area_width / 2.0 - width / 2.0,
        CapturePosition::Right => area_x + area_width - MARGIN - width,
    }
}

struct Inner {
    state: IndicatorState,
    translation: Option<String>,
    mic: MicLevel,
    level: f32,
    /// Affichage temporaire (`flash`) : l'état redevient caché à cet instant.
    until: Option<Instant>,
}

/// Ce que la fenêtre d'indicateur doit montrer à cet instant.
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub state: IndicatorState,
    pub translation: Option<String>,
    pub mic: MicLevel,
    pub level: f32,
}

impl Snapshot {
    pub fn visible(&self) -> bool {
        self.state != IndicatorState::Hidden
    }

    pub fn json(&self) -> Value {
        let (state, message) = match &self.state {
            IndicatorState::Hidden => ("hidden", None),
            IndicatorState::Listening => ("listening", None),
            IndicatorState::Working => ("working", None),
            IndicatorState::Done => ("done", None),
            IndicatorState::Message(text) => ("message", Some(text.clone())),
        };
        let mic = match self.mic {
            MicLevel::Ok => "ok",
            MicLevel::Weak => "weak",
            MicLevel::Silent => "silent",
        };
        json!({"visible": self.visible(), "state": state, "message": message, "translation": self.translation, "mic": mic, "level": self.level})
    }
}

pub struct OverlayModel {
    inner: Mutex<Inner>,
    /// Change à chaque nouvel état : un aperçu ou un affichage temporaire remplacé s'arrête de lui-même.
    generation: AtomicU64,
}

impl Default for OverlayModel {
    fn default() -> Self {
        Self {
            inner: Mutex::new(Inner { state: IndicatorState::Hidden, translation: None, mic: MicLevel::Ok, level: 0.0, until: None }),
            generation: AtomicU64::new(0),
        }
    }
}

impl OverlayModel {
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn show(&self, state: IndicatorState) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        let mut inner = self.lock();
        if state == IndicatorState::Listening {
            // Une nouvelle écoute repart de zéro (niveau, alerte de micro, langue de traduction).
            inner.level = 0.0;
            inner.mic = MicLevel::Ok;
            inner.translation = None;
        }
        inner.state = state;
        inner.until = None;
    }

    pub fn flash(&self, state: IndicatorState, seconds: f64) {
        self.show(state);
        self.lock().until = Some(Instant::now() + Duration::from_secs_f64(seconds.max(0.0)));
    }

    pub fn set_translation(&self, language: Option<String>) {
        self.lock().translation = language;
    }

    pub fn set_level(&self, level: f32) {
        self.lock().level = level.clamp(0.0, 1.0);
    }

    pub fn set_mic(&self, level: MicLevel) {
        self.lock().mic = level;
    }

    pub fn snapshot(&self) -> Snapshot {
        self.snapshot_at(Instant::now())
    }

    pub fn snapshot_at(&self, now: Instant) -> Snapshot {
        let mut inner = self.lock();
        if inner.until.is_some_and(|until| now >= until) {
            inner.state = IndicatorState::Hidden;
            inner.until = None;
        }
        Snapshot { state: inner.state.clone(), translation: inner.translation.clone(), mic: inner.mic, level: inner.level }
    }

    /// Réglages → Apparence → Aperçu : une fausse écoute de 2,5 s avec une voix simulée (niveaux toutes les 45 ms, comme le micro), puis
    /// « Collé » une seconde. Une vraie dictée qui démarre pendant l'aperçu l'arrête.
    pub fn preview(self: &Arc<Self>) {
        self.show(IndicatorState::Listening);
        let (model, generation) = (Arc::clone(self), self.generation.load(Ordering::SeqCst));
        std::thread::spawn(move || {
            let started = Instant::now();
            loop {
                if model.generation.load(Ordering::SeqCst) != generation {
                    return;
                }
                let t = started.elapsed().as_secs_f64();
                if t >= 2.5 {
                    model.flash(IndicatorState::Done, 1.0);
                    return;
                }
                model.set_level((0.35 + 0.3 * (t * 7.0).sin() + 0.25 * (t * 2.3).sin()).clamp(0.0, 1.0) as f32);
                std::thread::sleep(Duration::from_millis(45));
            }
        });
    }
}

/// Branche l'état de l'indicateur à l'écran sur l'indicateur existant (l'icône de la zone de notification) : tout passe par les deux.
pub struct OverlayIndicator {
    pub inner: Arc<dyn Indicator>,
    pub model: Arc<OverlayModel>,
}

impl Indicator for OverlayIndicator {
    fn show(&self, state: IndicatorState) {
        self.model.show(state.clone());
        self.inner.show(state);
    }

    fn set_translation(&self, language: Option<String>) {
        self.model.set_translation(language.clone());
        self.inner.set_translation(language);
    }

    fn set_level(&self, level: f32) {
        self.model.set_level(level);
        self.inner.set_level(level);
    }

    fn set_mic_level(&self, level: MicLevel) {
        self.model.set_mic(level);
        self.inner.set_mic_level(level);
    }

    fn flash(&self, state: IndicatorState, seconds: f64) {
        self.model.flash(state.clone(), seconds);
        self.inner.flash(state, seconds);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scriibe_platform::testing::FakeIndicator;

    #[test]
    fn ui_logic_001_to_003_the_window_sits_left_centre_or_right_of_the_screen() {
        // Écran de 1 000 de large à partir de 100, fenêtre de 440 : 112, 380 et 648.
        assert_eq!(origin_x(CapturePosition::Left, 100.0, 1000.0, 440.0), 112.0);
        assert_eq!(origin_x(CapturePosition::Center, 100.0, 1000.0, 440.0), 380.0);
        assert_eq!(origin_x(CapturePosition::Right, 100.0, 1000.0, 440.0), 648.0);
    }

    #[test]
    fn a_flash_goes_back_to_hidden_when_its_time_is_up() {
        let model = OverlayModel::default();
        model.flash(IndicatorState::Message("Rien entendu".into()), 2.0);
        let now = Instant::now();
        assert_eq!(model.snapshot_at(now).state, IndicatorState::Message("Rien entendu".into()));
        assert!(model.snapshot_at(now + Duration::from_secs(1)).visible());
        assert!(!model.snapshot_at(now + Duration::from_secs(3)).visible());
    }

    #[test]
    fn a_new_listening_starts_from_zero_and_keeps_what_is_set_after_it() {
        let model = OverlayModel::default();
        model.show(IndicatorState::Listening);
        model.set_level(0.8);
        model.set_mic(MicLevel::Weak);
        model.set_translation(Some("anglais".into()));
        let s = model.snapshot();
        assert_eq!((s.level, s.mic, s.translation.as_deref()), (0.8, MicLevel::Weak, Some("anglais")));
        model.show(IndicatorState::Working);
        assert_eq!(model.snapshot().translation.as_deref(), Some("anglais"), "le traitement garde la langue annoncée");
        model.show(IndicatorState::Listening);
        let s = model.snapshot();
        assert_eq!((s.level, s.mic, s.translation), (0.0, MicLevel::Ok, None), "l'écoute suivante repart de zéro");
    }

    #[test]
    fn the_snapshot_is_what_the_page_reads() {
        let model = OverlayModel::default();
        assert_eq!(model.snapshot().json()["state"], "hidden");
        model.show(IndicatorState::Message("Collé en anglais".into()));
        let json = model.snapshot().json();
        assert_eq!((json["state"].as_str(), json["message"].as_str()), (Some("message"), Some("Collé en anglais")));
        model.show(IndicatorState::Listening);
        model.set_mic(MicLevel::Silent);
        assert_eq!(model.snapshot().json()["mic"], "silent");
    }

    #[test]
    fn the_overlay_indicator_feeds_both_the_screen_and_the_icon() {
        let icon = Arc::new(FakeIndicator::default());
        let model = Arc::new(OverlayModel::default());
        let both = OverlayIndicator { inner: icon.clone(), model: model.clone() };
        both.show(IndicatorState::Listening);
        both.set_level(0.5);
        both.flash(IndicatorState::Done, 0.9);
        assert_eq!(model.snapshot().state, IndicatorState::Done);
        assert_eq!(icon.states().len(), 2, "l'icône a reçu les deux changements d'état");
    }

    #[test]
    fn a_real_dictation_stops_the_preview() {
        let model = Arc::new(OverlayModel::default());
        model.preview();
        assert_eq!(model.snapshot().state, IndicatorState::Listening);
        model.show(IndicatorState::Working);
        std::thread::sleep(Duration::from_millis(200));
        assert_eq!(model.snapshot().state, IndicatorState::Working, "l'aperçu n'écrase pas l'état d'une vraie dictée");
    }
}
