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

/// Le genre d'une erreur qui reste affichée (chantier 5) : il choisit le titre de la pastille et, pour les deux premiers, le son d'erreur.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureKind {
    /// Sous la touche de commande, la phrase n'est (ou n'est plus) une commande.
    NotRecognised,
    /// Une commande reconnue qui n'a pas abouti.
    NotDone,
    /// La dictée elle-même a échoué (micro, transcription).
    Dictation,
}

impl FailureKind {
    /// Le son d'erreur ne concerne que les commandes.
    pub fn is_command(self) -> bool {
        self != Self::Dictation
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::NotRecognised => "not_recognised",
            Self::NotDone => "not_done",
            Self::Dictation => "dictation",
        }
    }
}

/// Une erreur qui reste affichée jusqu'à un clic sur la croix ou la dictée suivante. `message` est à texte fixe (jamais les mots dits).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    pub kind: FailureKind,
    pub message: String,
}

impl Failure {
    pub fn new(kind: FailureKind, message: impl Into<String>) -> Self {
        Self { kind, message: message.into() }
    }
}

pub trait Indicator: Send + Sync {
    fn show(&self, state: IndicatorState);
    /// Pendant l'écoute : la langue de traduction annoncée (None : pas de traduction).
    fn set_translation(&self, language: Option<String>);
    /// Pendant l'écoute : la dictée est une commande (touche de commande), la pastille change de signe. Sans effet par défaut (icône, console).
    fn set_command(&self, _on: bool) {}
    /// Pendant l'écoute : niveau du micro de 0 à 1.
    fn set_level(&self, level: f32);
    /// Pendant l'écoute : le micro semble muet ou faible (« Je n'entends rien… », « Je t'entends à peine… »).
    fn set_mic_level(&self, _level: MicLevel) {}
    /// Affiche `state` puis revient à l'état caché au bout de `seconds`.
    fn flash(&self, state: IndicatorState, seconds: f64);
    /// Une erreur qui reste affichée. Par défaut (icône, console) : un affichage temporaire comme avant ; la pastille à l'écran la garde.
    fn fail(&self, failure: Failure) {
        self.flash(IndicatorState::Message(failure.message), 4.0);
    }
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
    describe_in(state, false)
}

/// L'état en deux langues (l'infobulle de l'icône) ; un message du chef d'orchestre reste tel qu'il a été écrit.
pub fn describe_in(state: &IndicatorState, english: bool) -> String {
    match (state, english) {
        (IndicatorState::Hidden, false) => "repos".into(),
        (IndicatorState::Hidden, true) => "idle".into(),
        (IndicatorState::Listening, false) => "à l'écoute".into(),
        (IndicatorState::Listening, true) => "listening".into(),
        (IndicatorState::Working, false) => "traitement…".into(),
        (IndicatorState::Working, true) => "working…".into(),
        (IndicatorState::Done, false) => "collé".into(),
        (IndicatorState::Done, true) => "pasted".into(),
        (IndicatorState::Message(m), _) => m.clone(),
    }
}
