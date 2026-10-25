//! Micro : niveau d'une prise et choix du micro. Port de `MicLevel` (`AudioRecorder.swift`) et `Microphones.resolve` (Scribe).

/// Niveau du micro d'après le RMS de la fenêtre la plus forte d'une prise.
/// Repères mesurés sur de vraies dictées : un micro muet reste sous 0,001, une voix normale va de 0,019 à 0,083.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MicLevel {
    Silent,
    Weak,
    Ok,
}

impl MicLevel {
    /// Sous ce niveau, rien n'est envoyé (Whisper inventerait une phrase sur du silence).
    pub const SILENT_BELOW: f32 = 0.004;
    /// Sous ce niveau, la dictée passe mais le micro est signalé comme faible.
    pub const WEAK_BELOW: f32 = 0.012;

    pub fn assess(level: f32) -> Self {
        if level < Self::SILENT_BELOW {
            Self::Silent
        } else if level < Self::WEAK_BELOW {
            Self::Weak
        } else {
            Self::Ok
        }
    }
}

/// Un micro de l'ordinateur, repéré par son identifiant stable.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Microphone {
    pub id: i64,
    pub uid: String,
    pub name: String,
    pub bluetooth: bool,
}

impl Microphone {
    /// Nom affiché dans Réglages : un micro Bluetooth est déconseillé (le casque bascule en mode « téléphone » en 1 à 2 s,
    /// et le début de la phrase est perdu).
    pub fn label(&self) -> String {
        if self.bluetooth {
            format!("{} — Bluetooth, déconseillé", self.name)
        } else {
            self.name.clone()
        }
    }
}

/// Le micro choisi dans l'appli (`uid` ; vide = celui du système). `missing` : il a été choisi mais n'est pas branché.
pub struct Resolved<'a> {
    pub microphone: Option<&'a Microphone>,
    pub missing: bool,
}

pub fn resolve<'a>(uid: &str, among: &'a [Microphone]) -> Resolved<'a> {
    if uid.is_empty() {
        return Resolved { microphone: None, missing: false };
    }
    match among.iter().find(|m| m.uid == uid) {
        Some(found) => Resolved { microphone: Some(found), missing: false },
        None => Resolved { microphone: None, missing: true },
    }
}
