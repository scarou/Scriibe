//! Pause des médias pendant la dictée : qui compte comme « média qui joue ». Port de la partie pure de `MediaPause.swift` (Scribe).
//!
//! La lecture des processus audio et l'envoi de la touche Lecture/Pause sont propres à chaque système (voir `crates/platform`).

use std::collections::HashSet;

/// Un processus qui utilise l'audio.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioProcess {
    pub pid: i64,
    /// Identifiant de l'appli (macOS : identifiant de bundle ; ailleurs : nom de l'exécutable). Peut être vide.
    pub bundle: String,
    /// Le processus sort du son.
    pub output: bool,
    /// Le processus capte le micro.
    pub input: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    NothingPlaying,
    /// Un appel : un processus qui joue et capte le micro. La touche Lecture/Pause n'y a rien à faire.
    InCall,
    /// Il y a un média à mettre en pause : les identifiants (ou « pid N ») de ce qui joue.
    Play(Vec<String>),
}

/// Processus système qui produisent des sons de leur propre chef (alertes, assistant) : ils ne sont pas un média à interrompre.
/// Valeurs de macOS ; sous Windows et Linux, la liste sera celle de la plateforme.
pub const IGNORED_MACOS: [&str; 7] = [
    "com.apple.audiomxd",
    "com.apple.controlcenter",
    "com.apple.CoreSpeech",
    "com.apple.assistantd",
    "com.apple.accessibility.heard",
    "com.apple.universalaccessd",
    "com.apple.mediaremoted",
];

/// Les processus qui sortent du son, hors l'appli elle-même et hors démons système.
pub fn playing<'a>(processes: &'a [AudioProcess], own_pid: i64, ignored: &[&str]) -> Vec<&'a AudioProcess> {
    processes.iter().filter(|p| p.output && p.pid != own_pid && !ignored.contains(&p.bundle.as_str())).collect()
}

fn name(p: &AudioProcess) -> String {
    if p.bundle.is_empty() {
        format!("pid {}", p.pid)
    } else {
        p.bundle.clone()
    }
}

/// Ce qui joue, vu depuis les processus audio : les identifiants des applis qui sortent du son, ou un appel en cours.
pub fn decision(processes: &[AudioProcess], own_pid: i64, ignored: &[&str]) -> Decision {
    let playing = playing(processes, own_pid, ignored);
    if playing.is_empty() {
        return Decision::NothingPlaying;
    }
    if playing.iter().any(|p| p.input) {
        return Decision::InCall;
    }
    Decision::Play(playing.into_iter().map(name).collect())
}

/// Les processus qui sortent du son sans en sortir quand la pause a été envoyée.
pub fn new_players(processes: &[AudioProcess], own_pid: i64, known: &HashSet<i64>, ignored: &[&str]) -> Vec<String> {
    playing(processes, own_pid, ignored).into_iter().filter(|p| !known.contains(&p.pid)).map(name).collect()
}
