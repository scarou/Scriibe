//! La page d'essais passe par le même chef d'orchestre que la touche : ces trois adaptateurs détournent le collage, l'historique et les
//! messages de l'indicateur pendant un essai (on garde ce qui serait collé, sans coller ni écrire dans l'historique).

use scriibe_core::history::Dictation;
use scriibe_platform::indicator::{Indicator, IndicatorState};
use scriibe_platform::paste::{PasteOutcome, Paster, Selection};
use scriibe_platform::storage::HistorySink;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

/// Ce qu'un essai a produit.
#[derive(Debug, Default, Clone)]
pub struct Captured {
    pub pasted: Option<String>,
    pub dictation: Option<Dictation>,
    /// Messages de l'indicateur (« Rien entendu », « Collé sans mise au propre (…) »…).
    pub messages: Vec<String>,
}

pub struct TestSinks {
    capturing: AtomicBool,
    captured: Mutex<Captured>,
    level: AtomicU32,
    paster: Arc<dyn Paster>,
    history: Arc<dyn HistorySink>,
    indicator: Arc<dyn Indicator>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

impl TestSinks {
    pub fn new(paster: Arc<dyn Paster>, history: Arc<dyn HistorySink>, indicator: Arc<dyn Indicator>) -> Arc<Self> {
        Arc::new(Self {
            capturing: AtomicBool::new(false),
            captured: Mutex::new(Captured::default()),
            level: AtomicU32::new(0),
            paster,
            history,
            indicator,
        })
    }

    /// Les essais qui suivent sont gardés au lieu d'être collés.
    pub fn begin(&self) {
        *lock(&self.captured) = Captured::default();
        self.capturing.store(true, Ordering::SeqCst);
    }

    pub fn finish(&self) -> Captured {
        self.capturing.store(false, Ordering::SeqCst);
        std::mem::take(&mut *lock(&self.captured))
    }

    /// Dernier niveau du micro envoyé à l'indicateur (0 à 1).
    pub fn level(&self) -> f32 {
        f32::from_bits(self.level.load(Ordering::SeqCst))
    }

    fn capturing(&self) -> bool {
        self.capturing.load(Ordering::SeqCst)
    }
}

pub struct SwitchPaster(pub Arc<TestSinks>);

impl Paster for SwitchPaster {
    fn paste(&self, text: &str, restore_clipboard: bool) -> PasteOutcome {
        if self.0.capturing() {
            lock(&self.0.captured).pasted = Some(text.to_string());
            PasteOutcome::Pasted
        } else {
            self.0.paster.paste(text, restore_clipboard)
        }
    }

    // Pendant une capture d'essai rien n'est collé : il n'y a pas de sélection à remplacer.
    fn selection(&self) -> Selection {
        if self.0.capturing() {
            Selection::default()
        } else {
            self.0.paster.selection()
        }
    }
}

pub struct SwitchHistory(pub Arc<TestSinks>);

impl HistorySink for SwitchHistory {
    fn record(&self, dictation: Dictation) {
        if self.0.capturing() {
            lock(&self.0.captured).dictation = Some(dictation);
        } else {
            self.0.history.record(dictation);
        }
    }
}

pub struct TeeIndicator(pub Arc<TestSinks>);

impl Indicator for TeeIndicator {
    fn show(&self, state: IndicatorState) {
        self.note(&state);
        self.0.indicator.show(state);
    }

    fn set_translation(&self, language: Option<String>) {
        self.0.indicator.set_translation(language);
    }

    fn set_level(&self, level: f32) {
        self.0.level.store(level.to_bits(), Ordering::SeqCst);
        self.0.indicator.set_level(level);
    }

    fn set_mic_level(&self, level: scriibe_core::microphone::MicLevel) {
        self.0.indicator.set_mic_level(level);
    }

    fn flash(&self, state: IndicatorState, seconds: f64) {
        self.note(&state);
        self.0.indicator.flash(state, seconds);
    }
}

impl TeeIndicator {
    fn note(&self, state: &IndicatorState) {
        if let (true, IndicatorState::Message(text)) = (self.0.capturing(), state) {
            lock(&self.0.captured).messages.push(text.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scriibe_platform::testing::{FakeIndicator, FakePaster, MemoryHistory};

    fn rig() -> (Arc<TestSinks>, Arc<FakePaster>, Arc<MemoryHistory>, Arc<FakeIndicator>) {
        let (paster, history, indicator) =
            (Arc::new(FakePaster::default()), Arc::new(MemoryHistory::default()), Arc::new(FakeIndicator::default()));
        (TestSinks::new(paster.clone(), history.clone(), indicator.clone()), paster, history, indicator)
    }

    #[test]
    fn during_a_trial_nothing_is_pasted_or_recorded_but_everything_is_kept() {
        let (sinks, paster, history, indicator) = rig();
        sinks.begin();
        SwitchPaster(sinks.clone()).paste("Bonjour.", true);
        SwitchHistory(sinks.clone()).record(Dictation::new(chrono::Utc::now(), "page", "bonjour", "Bonjour.", 1, 1.0, 0));
        TeeIndicator(sinks.clone()).flash(IndicatorState::Message("Rien entendu".into()), 1.0);
        let captured = sinks.finish();
        assert_eq!(captured.pasted.as_deref(), Some("Bonjour."));
        assert_eq!(captured.dictation.map(|d| d.raw_text), Some("bonjour".to_string()));
        assert_eq!(captured.messages, vec!["Rien entendu"]);
        assert!(paster.pasted().is_empty() && history.items().is_empty());
        // L'indicateur réel, lui, voit tout.
        assert_eq!(indicator.states().len(), 1);
    }

    #[test]
    fn outside_a_trial_the_real_paste_and_history_are_used() {
        let (sinks, paster, history, _) = rig();
        SwitchPaster(sinks.clone()).paste("Réel", false);
        SwitchHistory(sinks.clone()).record(Dictation::new(chrono::Utc::now(), "bloc-notes", "réel", "Réel", 1, 1.0, 0));
        assert_eq!(paster.pasted(), vec![("Réel".to_string(), false)]);
        assert_eq!(history.items().len(), 1);
        assert_eq!(sinks.finish().pasted, None);
    }

    #[test]
    fn the_microphone_level_is_kept_for_the_page() {
        let (sinks, ..) = rig();
        TeeIndicator(sinks.clone()).set_level(0.42);
        assert!((sinks.level() - 0.42).abs() < 1e-6);
    }
}
