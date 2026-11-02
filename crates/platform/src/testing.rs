//! Faux du système pour tester le chef d'orchestre sans clavier, micro ni fenêtre : ils notent ce qu'on leur demande.

use crate::apps::{AppInfo, ForegroundApp};
use crate::audio::{wav, LevelListener, Recorder, Recording, StartInfo};
use crate::indicator::{Indicator, IndicatorState};
use crate::media::MediaControl;
use crate::paste::{PasteOutcome, Paster};
use crate::sounds::Sounds;
use crate::storage::{ConfigSource, HistorySink};
use scriibe_core::dictionary::UserDictionary;
use scriibe_core::history::Dictation;
use scriibe_core::microphone::{MicLevel, Microphone};
use scriibe_core::settings::AppSettings;
use scriibe_engines::runtime::{KeyKind, Secrets};
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

/// Un micro simulé : rend la prise qu'on lui a préparée.
pub struct FakeRecorder {
    state: Mutex<RecorderState>,
}

struct RecorderState {
    next: Recording,
    fail: Option<String>,
    started: Vec<String>,
    stops: usize,
    open: bool,
    loudest: f32,
    listener: Option<LevelListener>,
}

impl Default for FakeRecorder {
    fn default() -> Self {
        Self::speaking(2.0, 0.05)
    }
}

impl FakeRecorder {
    /// Une prise de `duration` secondes dont la fenêtre la plus forte vaut `level`.
    pub fn speaking(duration: f64, level: f32) -> Self {
        let samples = vec![0i16; (duration * 16_000.0) as usize];
        Self {
            state: Mutex::new(RecorderState {
                next: Recording { wav: wav(&samples), duration, level, gain: 1.0 },
                fail: None,
                started: Vec::new(),
                stops: 0,
                open: false,
                loudest: level,
                listener: None,
            }),
        }
    }

    pub fn set_recording(&self, duration: f64, level: f32) {
        let mut s = lock(&self.state);
        s.next = Recording { wav: wav(&vec![0i16; (duration * 16_000.0) as usize]), duration, level, gain: 1.0 };
        s.loudest = level;
    }

    pub fn fail_to_start(&self, message: &str) {
        lock(&self.state).fail = Some(message.to_string());
    }

    pub fn starts(&self) -> Vec<String> {
        lock(&self.state).started.clone()
    }

    pub fn stops(&self) -> usize {
        lock(&self.state).stops
    }

    pub fn is_open(&self) -> bool {
        lock(&self.state).open
    }

    /// Envoie un niveau au vumètre, comme le ferait le fil du micro.
    pub fn emit_level(&self, level: f32) {
        let listener = lock(&self.state).listener.clone();
        if let Some(listener) = listener {
            listener(level);
        }
    }
}

impl Recorder for FakeRecorder {
    fn microphones(&self) -> Vec<Microphone> {
        Vec::new()
    }

    fn start(&self, microphone_uid: &str) -> Result<StartInfo, String> {
        let mut s = lock(&self.state);
        if let Some(message) = &s.fail {
            return Err(message.clone());
        }
        s.started.push(microphone_uid.to_string());
        s.open = true;
        Ok(StartInfo { description: "micro simulé, 16000 Hz, 1 canal(aux)".into(), fallback: None })
    }

    fn stop(&self) -> Recording {
        let mut s = lock(&self.state);
        s.open = false;
        s.stops += 1;
        s.next.clone()
    }

    fn loudest_so_far(&self) -> f32 {
        lock(&self.state).loudest
    }

    fn input_description(&self) -> String {
        "micro simulé".into()
    }

    fn set_level_listener(&self, listener: Option<LevelListener>) {
        lock(&self.state).listener = listener;
    }
}

pub struct FakePaster {
    pasted: Mutex<Vec<(String, bool)>>,
    outcome: Mutex<PasteOutcome>,
    selection: Mutex<bool>,
}

impl Default for FakePaster {
    fn default() -> Self {
        Self { pasted: Mutex::new(Vec::new()), outcome: Mutex::new(PasteOutcome::Pasted), selection: Mutex::new(false) }
    }
}

impl FakePaster {
    pub fn pasted(&self) -> Vec<(String, bool)> {
        lock(&self.pasted).clone()
    }

    pub fn set_outcome(&self, outcome: PasteOutcome) {
        *lock(&self.outcome) = outcome;
    }

    /// Fait croire qu'un texte est sélectionné dans l'appli active.
    pub fn set_selection(&self, active: bool) {
        *lock(&self.selection) = active;
    }
}

impl Paster for FakePaster {
    fn selection_active(&self) -> bool {
        *lock(&self.selection)
    }

    fn paste(&self, text: &str, restore_clipboard: bool) -> PasteOutcome {
        lock(&self.pasted).push((text.to_string(), restore_clipboard));
        lock(&self.outcome).clone()
    }
}

#[derive(Default)]
pub struct FakeMedia {
    calls: Mutex<Vec<String>>,
}

impl FakeMedia {
    pub fn calls(&self) -> Vec<String> {
        lock(&self.calls).clone()
    }
}

impl MediaControl for FakeMedia {
    fn pause_if_playing(&self, enabled: bool) {
        lock(&self.calls).push(format!("pause({enabled})"));
    }

    fn resume_if_paused(&self) {
        lock(&self.calls).push("resume".into());
    }
}

pub struct FakeApps(pub AppInfo);

impl Default for FakeApps {
    fn default() -> Self {
        Self(AppInfo { name: "Bloc-notes".into(), id: Some("notepad".into()) })
    }
}

impl ForegroundApp for FakeApps {
    fn current(&self) -> AppInfo {
        self.0.clone()
    }
}

/// Note les états montrés, dans l'ordre. Un `flash` s'écrit comme un `show` (sans attendre).
#[derive(Default)]
pub struct FakeIndicator {
    states: Mutex<Vec<IndicatorState>>,
    translation: Mutex<Vec<Option<String>>>,
    levels: Mutex<Vec<f32>>,
    mic: Mutex<Vec<MicLevel>>,
}

impl FakeIndicator {
    pub fn states(&self) -> Vec<IndicatorState> {
        lock(&self.states).clone()
    }

    pub fn last(&self) -> Option<IndicatorState> {
        lock(&self.states).last().cloned()
    }

    pub fn translations(&self) -> Vec<Option<String>> {
        lock(&self.translation).clone()
    }

    pub fn levels(&self) -> Vec<f32> {
        lock(&self.levels).clone()
    }

    pub fn mic_levels(&self) -> Vec<MicLevel> {
        lock(&self.mic).clone()
    }
}

impl Indicator for FakeIndicator {
    fn show(&self, state: IndicatorState) {
        lock(&self.states).push(state);
    }

    fn set_translation(&self, language: Option<String>) {
        lock(&self.translation).push(language);
    }

    fn set_level(&self, level: f32) {
        lock(&self.levels).push(level);
    }

    fn set_mic_level(&self, level: MicLevel) {
        lock(&self.mic).push(level);
    }

    fn flash(&self, state: IndicatorState, _: f64) {
        lock(&self.states).push(state);
    }
}

#[derive(Default)]
pub struct FakeSounds {
    played: Mutex<Vec<String>>,
}

impl FakeSounds {
    pub fn played(&self) -> Vec<String> {
        lock(&self.played).clone()
    }
}

impl Sounds for FakeSounds {
    fn play(&self, name: &str, _: f64) {
        lock(&self.played).push(name.to_string());
    }
}

#[derive(Default)]
pub struct MemoryHistory {
    items: Mutex<Vec<Dictation>>,
}

impl MemoryHistory {
    pub fn items(&self) -> Vec<Dictation> {
        lock(&self.items).clone()
    }
}

impl HistorySink for MemoryHistory {
    fn record(&self, dictation: Dictation) {
        lock(&self.items).push(dictation);
    }
}

/// Des clés fixes.
#[derive(Default)]
pub struct StaticSecrets(pub HashMap<String, String>);

impl StaticSecrets {
    pub fn with(kind: KeyKind, value: &str) -> Self {
        Self(HashMap::from([(crate::secrets::account_name(kind).to_string(), value.to_string())]))
    }
}

impl Secrets for StaticSecrets {
    fn get(&self, kind: KeyKind) -> Option<String> {
        self.0.get(crate::secrets::account_name(kind)).cloned()
    }
}

/// Des réglages et un dictionnaire qu'un test peut changer entre deux dictées.
pub struct FakeConfig {
    settings: Mutex<AppSettings>,
    dictionary: Mutex<UserDictionary>,
}

impl FakeConfig {
    pub fn new(settings: AppSettings) -> Self {
        Self { settings: Mutex::new(settings), dictionary: Mutex::new(UserDictionary::default()) }
    }

    pub fn change(&self, edit: impl FnOnce(&mut AppSettings)) {
        edit(&mut lock(&self.settings));
    }

    pub fn set_dictionary(&self, dictionary: UserDictionary) {
        *lock(&self.dictionary) = dictionary;
    }
}

impl ConfigSource for FakeConfig {
    fn settings(&self) -> AppSettings {
        lock(&self.settings).clone()
    }

    fn dictionary(&self) -> UserDictionary {
        lock(&self.dictionary).clone()
    }
}
