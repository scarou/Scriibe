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
    /// L'adresse qu'une commande (recherche) aurait ouverte : gardée, jamais lancée pendant un essai.
    pub opened: Option<String>,
    /// La longueur (en signes) de l'extrait qu'une commande aurait collé : le texte n'est jamais gardé (il est confidentiel), ni collé.
    pub snippet_chars: Option<usize>,
    /// Une commande aurait ouvert une application de la liste : seul le fait est gardé (ni le nom, ni l'identifiant), rien n'est lancé.
    pub app_opened: bool,
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

    // Pendant une capture d'essai, aucun extrait n'est collé : l'essai n'en garde que la longueur.
    fn paste_transient(&self, text: &str) -> PasteOutcome {
        if self.0.capturing() {
            lock(&self.0.captured).snippet_chars = Some(text.chars().count());
            PasteOutcome::Pasted
        } else {
            self.0.paster.paste_transient(text)
        }
    }

    fn last_report(&self) -> String {
        self.0.paster.last_report()
    }

    fn reads_selection(&self) -> bool {
        self.0.paster.reads_selection()
    }

    // Pendant une capture d'essai, rien ne va dans le presse-papiers (une reformulation copiée faute de pouvoir la coller).
    fn copy(&self, text: &str) -> bool {
        !self.0.capturing() && self.0.paster.copy(text)
    }

    // Pendant une capture d'essai, pas de commande vocale : rien à lire.
    fn selected_text(&self) -> scriibe_platform::paste::SelectedRead {
        if self.0.capturing() {
            scriibe_platform::paste::SelectedRead::NoSelection
        } else {
            self.0.paster.selected_text()
        }
    }

    // Pendant une capture d'essai, aucun raccourci n'est envoyé à l'appli active (et il n'y a pas de commande vocale).
    fn press_shortcut(&self, shortcut: &scriibe_core::shortcut::Shortcut) -> Result<(), &'static str> {
        if self.0.capturing() {
            Err(scriibe_platform::paste::SHORTCUT_UNSUPPORTED)
        } else {
            self.0.paster.press_shortcut(shortcut)
        }
    }

    fn press_return(&self) -> bool {
        !self.0.capturing() && self.0.paster.press_return()
    }

    // Pendant une capture d'essai, aucune adresse ne s'ouvre (recherches) : l'essai garde l'adresse construite au lieu de la lancer.
    fn open_address(&self, url: &str) -> Result<(), &'static str> {
        if self.0.capturing() {
            lock(&self.0.captured).opened = Some(url.to_string());
            Ok(())
        } else {
            self.0.paster.open_address(url)
        }
    }

    // Pendant une capture d'essai, aucune application ne s'ouvre : l'essai ne garde que le fait.
    fn open_app(&self, app_id: &str) -> Result<(), &'static str> {
        if self.0.capturing() {
            lock(&self.0.captured).app_opened = true;
            Ok(())
        } else {
            self.0.paster.open_app(app_id)
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

    /// Chantier 6 : pendant un essai (y compris sous la touche de commande), aucune sortie n'atteint l'application active : ni lecture de la
    /// sélection, ni raccourci, ni touche Entrée, ni collage. Les nouvelles sorties (ouverture, extrait, reformulation) passeront par le même
    /// aiguillage et ce test les couvrira à leur arrivée.
    #[test]
    fn during_a_trial_a_command_reaches_nothing_of_the_active_application() {
        use scriibe_core::shortcut::Shortcut;
        use scriibe_platform::paste::SelectedRead;
        use scriibe_platform::testing::FakeSelected;
        let (sinks, paster, ..) = rig();
        paster.set_selected(FakeSelected::Text("SECRET".into()));
        let switch = SwitchPaster(sinks.clone());
        sinks.begin();
        assert!(matches!(switch.selected_text(), SelectedRead::NoSelection), "la sélection n'est pas lue pendant un essai");
        assert_eq!(paster.selected_reads(), 0);
        assert!(switch.press_shortcut(&Shortcut::parse("Mod+B").unwrap()).is_err());
        assert!(!switch.press_return());
        assert!(!switch.selection().active);
        assert!(switch.open_address("https://www.google.com/search?q=SECRET").is_ok(), "l'essai garde l'adresse, sans la lancer");
        // Un extrait n'est ni collé ni gardé (il est confidentiel) : l'essai n'en retient que la longueur.
        assert_eq!(switch.paste_transient("FR76 SECRET-IBAN"), PasteOutcome::Pasted);
        paster.set_installed_apps(&["com.apple.Safari"]);
        assert!(switch.open_app("com.apple.Safari").is_ok(), "l'essai ne lance pas l'application");
        // Reformulation (étape 4) : la sélection n'étant pas lue, rien ne part au modèle ; une reformulation copiée faute de pouvoir la
        // coller n'atteint pas non plus le presse-papiers.
        assert!(!switch.copy("SECRET reformulé"));
        let captured = sinks.finish();
        assert_eq!(captured.opened.as_deref(), Some("https://www.google.com/search?q=SECRET"));
        assert_eq!(captured.snippet_chars, Some(16));
        assert!(captured.app_opened && paster.opened_apps().is_empty(), "l'essai n'a pas lancé l'application");
        assert!(captured.pasted.is_none(), "l'extrait n'est pas rendu comme un texte collé");
        assert!(!format!("{captured:?}").contains("SECRET-IBAN"), "le texte de l'extrait n'est nulle part dans l'essai");
        assert!(paster.shortcuts().is_empty() && paster.pasted().is_empty() && paster.opened().is_empty());
        assert!(paster.pasted_transient().is_empty(), "rien n'atteint le vrai collage pendant un essai");
        assert!(paster.copied().is_empty(), "rien n'atteint le presse-papiers pendant un essai");
        // Hors essai, la lecture de la sélection et le raccourci passent.
        assert!(matches!(switch.selected_text(), SelectedRead::Text(_)));
        assert!(switch.press_shortcut(&Shortcut::parse("Mod+B").unwrap()).is_ok());
        assert_eq!(paster.shortcuts().len(), 1);
        assert!(switch.open_address("https://www.google.com/search?q=chats").is_ok());
        assert_eq!(paster.opened(), vec!["https://www.google.com/search?q=chats".to_string()]);
        assert_eq!(switch.paste_transient("hors essai"), PasteOutcome::Pasted);
        assert_eq!(paster.pasted_transient(), vec!["hors essai".to_string()]);
        assert!(switch.open_app("com.apple.Safari").is_ok());
        assert_eq!(paster.opened_apps(), vec!["com.apple.Safari".to_string()]);
        assert!(switch.copy("hors essai") && switch.reads_selection());
        assert_eq!(paster.copied(), vec!["hors essai".to_string()]);
    }

    #[test]
    fn the_microphone_level_is_kept_for_the_page() {
        let (sinks, ..) = rig();
        TeeIndicator(sinks.clone()).set_level(0.42);
        assert!((sinks.level() - 0.42).abs() < 1e-6);
    }
}
