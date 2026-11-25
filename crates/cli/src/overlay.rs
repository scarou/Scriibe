//! L'indicateur à l'écran (pastille, point ou vumètre) : ce que Scriibe montre pendant une dictée, en plus de l'icône de la zone de
//! notification. Ce module garde l'état (port de `OverlayModel` de Scribe) ; la fenêtre qui l'affiche est `window.rs` (Windows) et l'interface
//! web (`ui/src/overlay/`) dessine la pastille.

use scriibe_core::microphone::MicLevel;
use scriibe_core::settings::CapturePosition;
use scriibe_platform::indicator::{Failure, Indicator, IndicatorState};
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
    /// Dictée de commande en cours (touche de commande tenue).
    command: bool,
    mic: MicLevel,
    level: f32,
    /// Affichage temporaire (`flash`) : l'état redevient caché à cet instant.
    until: Option<Instant>,
    /// Une erreur qui reste affichée (jusqu'à `dismiss` ou un autre état) : l'état est alors `Message`, sans `until`.
    failure: Option<Failure>,
    /// Taille (px logiques) de la capsule d'erreur mesurée par la page : la fenêtre s'y réduit pour devenir cliquable.
    shape: Option<(f64, f64)>,
}

/// Ce que la fenêtre d'indicateur doit montrer à cet instant.
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub state: IndicatorState,
    pub translation: Option<String>,
    pub command: bool,
    pub mic: MicLevel,
    pub level: f32,
    pub failure: Option<Failure>,
    /// Taille de la capsule d'erreur (une fois mesurée) : la fenêtre se réduit à elle et accepte la souris.
    pub shape: Option<(f64, f64)>,
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
        let error = self.failure.as_ref().map(|f| json!({"kind": f.kind.id(), "message": f.message}));
        json!({"visible": self.visible(), "state": state, "message": message, "translation": self.translation, "command": self.command, "mic": mic, "level": self.level, "error": error})
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
            inner: Mutex::new(Inner {
                state: IndicatorState::Hidden,
                translation: None,
                command: false,
                mic: MicLevel::Ok,
                level: 0.0,
                until: None,
                failure: None,
                shape: None,
            }),
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
            inner.command = false;
        }
        inner.state = state;
        inner.until = None;
        inner.failure = None;
        inner.shape = None;
    }

    /// Une erreur qui reste affichée jusqu'à `dismiss` ou jusqu'à l'état suivant (la dictée suivante, un autre message).
    ///
    /// D'un seul verrou (la page ne voit jamais un message ordinaire entre les deux). Une erreur qui en remplace une garde la dernière
    /// mesure de la capsule : la page ne renvoie sa taille que si elle change, et sans cela deux erreurs identiques de suite (micro
    /// absent, appui répété) laisseraient la fenêtre agrandie et insensible à la souris sous une capsule qui n'a pas bougé.
    pub fn fail(&self, failure: Failure) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        let mut inner = self.lock();
        let shape = if inner.failure.is_some() { inner.shape } else { None };
        inner.state = IndicatorState::Message(failure.message.clone());
        inner.until = None;
        inner.failure = Some(failure);
        inner.shape = shape;
    }

    /// La croix de la pastille : cache l'erreur qui reste affichée. Sans effet sur un autre état (une dictée déjà repartie).
    pub fn dismiss(&self) {
        let mut inner = self.lock();
        if inner.failure.take().is_some() {
            self.generation.fetch_add(1, Ordering::SeqCst);
            inner.state = IndicatorState::Hidden;
            inner.shape = None;
        }
    }

    /// La page a mesuré la capsule d'erreur (px logiques) ; ignoré s'il n'y a pas d'erreur affichée.
    pub fn set_shape(&self, width: f64, height: f64) {
        let mut inner = self.lock();
        if inner.failure.is_some() && width.is_finite() && height.is_finite() {
            inner.shape = Some((width.clamp(40.0, WINDOW_SIZE.0), height.clamp(20.0, WINDOW_SIZE.1)));
        }
    }

    pub fn flash(&self, state: IndicatorState, seconds: f64) {
        self.show(state);
        self.lock().until = Some(Instant::now() + Duration::from_secs_f64(seconds.max(0.0)));
    }

    pub fn set_translation(&self, language: Option<String>) {
        self.lock().translation = language;
    }

    pub fn set_command(&self, on: bool) {
        self.lock().command = on;
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
        Snapshot {
            state: inner.state.clone(),
            translation: inner.translation.clone(),
            command: inner.command,
            mic: inner.mic,
            level: inner.level,
            failure: inner.failure.clone(),
            shape: inner.shape,
        }
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

    fn set_command(&self, on: bool) {
        self.model.set_command(on);
        self.inner.set_command(on);
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

    /// La pastille garde l'erreur ; l'icône de la zone de notification, elle, la montre 4 s comme un message.
    fn fail(&self, failure: Failure) {
        self.inner.flash(IndicatorState::Message(failure.message.clone()), 4.0);
        self.model.fail(failure);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scriibe_platform::indicator::FailureKind;
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
    fn a_command_dictation_is_shown_until_the_next_listening() {
        let model = OverlayModel::default();
        model.show(IndicatorState::Listening);
        assert_eq!(model.snapshot().json()["command"], false);
        model.set_command(true);
        model.show(IndicatorState::Working);
        assert_eq!(model.snapshot().json()["command"], true, "le traitement garde le signe");
        model.show(IndicatorState::Listening);
        assert_eq!(model.snapshot().json()["command"], false, "l'écoute suivante repart de zéro");
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

    fn failure(text: &str) -> Failure {
        Failure::new(FailureKind::NotDone, text)
    }

    /// Relecture C5 (important I1) : une erreur qui en remplace une garde la mesure de la capsule (sinon la fenêtre redevient insensible à la
    /// souris sous une capsule inchangée) ; une erreur après un autre état repart sans mesure ; la mesure ne s'applique qu'à une erreur.
    #[test]
    fn a_failure_replacing_a_failure_keeps_the_measured_shape_but_any_other_state_drops_it() {
        let model = OverlayModel::default();
        model.fail(failure("Le micro ne répond pas"));
        model.set_shape(300.0, 60.0);
        model.fail(failure("Le micro ne répond pas"));
        assert_eq!(model.snapshot().shape, Some((300.0, 60.0)), "deux erreurs identiques de suite : la croix reste cliquable");
        assert!(model.snapshot().failure.is_some());
        model.fail(failure("Aucune sélection"));
        assert_eq!(model.snapshot().shape, Some((300.0, 60.0)), "la page renvoie sa mesure si la taille change");
        model.show(IndicatorState::Listening);
        model.fail(failure("Aucune sélection"));
        assert_eq!(model.snapshot().shape, None, "après un autre état, la capsule est neuve : rien n'est gardé");
        // Une première erreur sur une pastille ordinaire, ou après une croix, repart aussi sans mesure.
        model.set_shape(300.0, 60.0);
        model.dismiss();
        model.fail(failure("Aucune sélection"));
        assert_eq!(model.snapshot().shape, None);
        // Un flash ordinaire efface l'erreur et sa mesure.
        model.set_shape(200.0, 50.0);
        model.flash(IndicatorState::Done, 1.0);
        assert_eq!((model.snapshot().failure, model.snapshot().shape), (None, None));
    }

    #[test]
    fn a_failure_stays_until_dismissed_unlike_a_flash() {
        let model = OverlayModel::default();
        model.fail(failure("Aucune sélection"));
        let now = Instant::now();
        let later = now + Duration::from_secs(3600);
        assert!(model.snapshot_at(later).visible(), "aucun délai : l'erreur reste");
        let json = model.snapshot_at(later).json();
        assert_eq!((json["error"]["kind"].as_str(), json["error"]["message"].as_str()), (Some("not_done"), Some("Aucune sélection")));
        model.dismiss();
        assert!(!model.snapshot().visible());
        assert!(model.snapshot().json()["error"].is_null());
    }

    #[test]
    fn the_next_dictation_or_any_other_state_replaces_a_failure() {
        let model = OverlayModel::default();
        model.fail(failure("Aucune sélection"));
        model.show(IndicatorState::Listening);
        let snapshot = model.snapshot();
        assert_eq!((snapshot.state, snapshot.failure), (IndicatorState::Listening, None));
        model.fail(failure("Mail non ouvert"));
        model.flash(IndicatorState::Done, 0.9);
        assert!(model.snapshot().failure.is_none() && model.snapshot().state == IndicatorState::Done);
    }

    #[test]
    fn dismissing_does_nothing_to_a_dictation_that_has_started() {
        let model = OverlayModel::default();
        model.fail(failure("Aucune sélection"));
        model.show(IndicatorState::Listening);
        model.dismiss();
        assert_eq!(model.snapshot().state, IndicatorState::Listening, "la croix lue en retard ne cache pas l'écoute");
    }

    #[test]
    fn the_measured_shape_counts_only_for_a_failure_and_is_bounded() {
        let model = OverlayModel::default();
        model.set_shape(200.0, 50.0);
        assert!(model.snapshot().shape.is_none(), "pas d'erreur : rien à réduire");
        model.fail(failure("Aucune sélection"));
        model.set_shape(200.0, 50.0);
        assert_eq!(model.snapshot().shape, Some((200.0, 50.0)));
        model.set_shape(5000.0, 5000.0);
        assert_eq!(model.snapshot().shape, Some(WINDOW_SIZE), "jamais plus grand que la fenêtre normale");
        model.set_shape(f64::NAN, 10.0);
        assert_eq!(model.snapshot().shape, Some(WINDOW_SIZE), "une valeur absurde est ignorée");
        model.show(IndicatorState::Listening);
        assert!(model.snapshot().shape.is_none(), "la forme ne survit pas à l'erreur");
    }

    #[test]
    fn the_overlay_indicator_keeps_the_failure_on_screen_and_flashes_the_icon() {
        let icon = Arc::new(FakeIndicator::default());
        let model = Arc::new(OverlayModel::default());
        let both = OverlayIndicator { inner: icon.clone(), model: model.clone() };
        both.fail(failure("Mail non ouvert"));
        assert_eq!(model.snapshot().failure, Some(failure("Mail non ouvert")));
        assert_eq!(icon.states(), vec![IndicatorState::Message("Mail non ouvert".into())], "l'icône reçoit le message temporaire");
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
