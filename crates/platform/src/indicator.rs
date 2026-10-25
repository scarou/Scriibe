//! Indicateur de dictée : ce que la personne voit (icône dans la zone de notification, pastille…).

use scriibe_core::microphone::MicLevel;

#[derive(Debug, Clone, PartialEq)]
pub enum IndicatorState {
    Hidden,
    Listening,
    Working,
    Done,
    Message(String),
}

pub trait Indicator: Send + Sync {
    fn show(&self, state: IndicatorState);
    /// Pendant l'écoute : la langue de traduction annoncée (None : pas de traduction).
    fn set_translation(&self, language: Option<String>);
    /// Pendant l'écoute : niveau du micro de 0 à 1.
    fn set_level(&self, level: f32);
    /// Pendant l'écoute : le micro semble muet ou faible (« Je n'entends rien… », « Je t'entends à peine… »).
    fn set_mic_level(&self, _level: MicLevel) {}
    /// Affiche `state` puis revient à l'état caché au bout de `seconds`.
    fn flash(&self, state: IndicatorState, seconds: f64);
}

/// Écrit les changements d'état sur la sortie d'erreur (essais en ligne de commande, systèmes sans icône).
pub struct ConsoleIndicator;

impl Indicator for ConsoleIndicator {
    fn show(&self, state: IndicatorState) {
        eprintln!("[indicateur] {}", describe(&state));
    }

    fn set_translation(&self, language: Option<String>) {
        if let Some(language) = language {
            eprintln!("[indicateur] traduction vers : {language}");
        }
    }

    fn set_level(&self, _: f32) {}

    fn flash(&self, state: IndicatorState, _: f64) {
        self.show(state);
    }
}

pub fn describe(state: &IndicatorState) -> String {
    match state {
        IndicatorState::Hidden => "repos".into(),
        IndicatorState::Listening => "à l'écoute".into(),
        IndicatorState::Working => "traitement…".into(),
        IndicatorState::Done => "collé".into(),
        IndicatorState::Message(m) => m.clone(),
    }
}
