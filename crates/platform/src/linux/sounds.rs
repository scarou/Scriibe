//! Sons de début et de fin : synthétisés (`crate::sounds::synth`, volume compris), écrits une fois dans un fichier WAV du dossier temporaire,
//! puis joués par le lecteur de la session (`paplay` pour PulseAudio et PipeWire, `pw-play`, ou `aplay`), sans attendre la fin du son.

use crate::sounds::{synth, Sounds};
use std::path::PathBuf;
use std::process::{Command, Stdio};

pub struct SessionSounds;

/// Le premier lecteur trouvé dans le `PATH`, avec ses options.
fn player() -> Option<(&'static str, &'static [&'static str])> {
    const PLAYERS: [(&str, &[&str]); 3] = [("paplay", &[]), ("pw-play", &[]), ("aplay", &["-q"])];
    let path = std::env::var_os("PATH")?;
    PLAYERS.iter().copied().find(|(name, _)| std::env::split_paths(&path).any(|dir| dir.join(name).is_file()))
}

fn file_for(name: &str, volume: f64) -> PathBuf {
    std::env::temp_dir().join(format!("scriibe-son-{}-{name}-{}.wav", std::process::id(), (volume * 100.0).round() as i64))
}

impl Sounds for SessionSounds {
    fn play(&self, name: &str, volume: f64) {
        let Some(wav) = synth(name, volume) else { return };
        let Some((program, options)) = player() else { return };
        let path = file_for(name, volume);
        if !path.exists() && std::fs::write(&path, wav).is_err() {
            return;
        }
        if let Ok(mut child) =
            Command::new(program).args(options).arg(&path).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn()
        {
            // Attendu à part : un processus fini mais jamais attendu reste dans la table des processus.
            std::thread::spawn(move || {
                let _ = child.wait();
            });
        }
    }
}
