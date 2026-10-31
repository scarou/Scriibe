//! Sons de début et de fin : synthétisés (`crate::sounds::synth`, volume compris), écrits une fois dans un fichier WAV du dossier temporaire,
//! puis joués par `afplay` (livré avec macOS), sans attendre la fin du son.

use crate::sounds::{synth, Sounds};
use std::path::PathBuf;
use std::process::{Command, Stdio};

pub struct AfplaySounds;

fn file_for(name: &str, volume: f64) -> PathBuf {
    std::env::temp_dir().join(format!("scriibe-son-{}-{name}-{}.wav", std::process::id(), (volume * 100.0).round() as i64))
}

impl Sounds for AfplaySounds {
    fn play(&self, name: &str, volume: f64) {
        let Some(wav) = synth(name, volume) else { return };
        let path = file_for(name, volume);
        if !path.exists() && std::fs::write(&path, wav).is_err() {
            return;
        }
        if let Ok(mut child) = Command::new("afplay").arg(&path).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn() {
            // Attendu à part : un processus fini mais jamais attendu reste dans la table des processus.
            std::thread::spawn(move || {
                let _ = child.wait();
            });
        }
    }
}
