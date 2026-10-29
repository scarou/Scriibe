//! Pause des médias : ce qui joue ou capte le micro se lit dans PulseAudio (ou PipeWire, qui parle la même langue) avec `pactl`, comme
//! Windows lit les sessions audio ; la pause est envoyée à la main au lecteur concerné par `playerctl` (MPRIS), sinon par la touche
//! « Lecture/Pause » du clavier. Les délais et la détection d'un appel sont ceux de `MediaPauser` (communs à tous les systèmes).

use super::x11;
use crate::media::AudioSource;
use scriibe_core::media_pause::AudioProcess;
use std::process::Command;
use x11rb::connection::Connection;
use x11rb::protocol::xproto;
use x11rb::protocol::xtest::ConnectionExt as _;

pub struct PulseAudio;

/// Le texte de `pactl list sink-inputs` ou `source-outputs` (en anglais : `LC_ALL=C`) : un processus par flux qui n'est pas en pause
/// (« Corked: no »), avec le nom de son exécutable.
pub fn parse_streams(text: &str, output: bool) -> Vec<AudioProcess> {
    let mut found = Vec::new();
    let mut corked = true;
    let (mut pid, mut binary): (Option<i64>, String) = (None, String::new());
    let flush = |corked: bool, pid: Option<i64>, binary: &str, found: &mut Vec<AudioProcess>| {
        if let (false, Some(pid)) = (corked, pid) {
            found.push(AudioProcess { pid, bundle: binary.to_string(), output, input: !output });
        }
    };
    for line in text.lines() {
        if !line.starts_with(char::is_whitespace) && (line.starts_with("Sink Input #") || line.starts_with("Source Output #")) {
            flush(corked, pid, &binary, &mut found);
            (corked, pid, binary) = (true, None, String::new());
            continue;
        }
        let line = line.trim();
        if let Some(value) = line.strip_prefix("Corked:") {
            corked = value.trim() != "no";
        } else if let Some(value) = line.strip_prefix("application.process.id = ") {
            pid = value.trim().trim_matches('"').parse().ok();
        } else if let Some(value) = line.strip_prefix("application.process.binary = ") {
            binary = value.trim().trim_matches('"').to_string();
        }
    }
    flush(corked, pid, &binary, &mut found);
    found
}

/// Un processus par pid : un navigateur qui joue et capte le micro (appel) est un seul processus qui fait les deux.
pub fn merge(streams: Vec<AudioProcess>) -> Vec<AudioProcess> {
    let mut merged: Vec<AudioProcess> = Vec::new();
    for stream in streams {
        match merged.iter_mut().find(|p| p.pid == stream.pid) {
            Some(p) => {
                p.output |= stream.output;
                p.input |= stream.input;
            }
            None => merged.push(stream),
        }
    }
    merged
}

fn pactl(kind: &str) -> Option<String> {
    let out = Command::new("pactl").args(["list", kind]).env("LC_ALL", "C").output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

fn playerctl(args: &[&str]) -> Option<String> {
    let out = Command::new("playerctl").args(args).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// La touche « Lecture/Pause » du clavier (`XF86AudioPlay`), envoyée par XTest.
fn media_key() -> Result<(), String> {
    let (conn, root) = x11::connect()?;
    let code = x11::keycode_for(&conn, &[0x1008_FF14]).ok_or("touche Lecture/Pause absente de la disposition du clavier")?;
    for kind in [xproto::KEY_PRESS_EVENT, xproto::KEY_RELEASE_EVENT] {
        conn.xtest_fake_input(kind, code, 0, root, 0, 0, 0).map_err(|e| e.to_string())?;
    }
    conn.flush().map_err(|e| e.to_string())
}

impl AudioSource for PulseAudio {
    fn processes(&self) -> Vec<AudioProcess> {
        let mut all = parse_streams(&pactl("sink-inputs").unwrap_or_default(), true);
        all.extend(parse_streams(&pactl("source-outputs").unwrap_or_default(), false));
        merge(all)
    }

    fn play_pause(&self) {
        if media_key().is_err() {
            let _ = playerctl(&["play-pause"]);
        }
    }

    fn play_pause_for(&self, players: &[AudioProcess]) -> String {
        // Un lecteur MPRIS porte le nom de son exécutable (« vlc », « spotify », « firefox »…) : on vise celui qui joue.
        let reached: Vec<&str> = players
            .iter()
            .map(|p| p.bundle.as_str())
            .filter(|name| !name.is_empty() && playerctl(&["-p", name, "play-pause"]).is_some())
            .collect();
        if reached.is_empty() {
            self.play_pause();
            return String::new();
        }
        format!(" — commande MPRIS envoyée à {}", reached.join(", "))
    }

    fn ignored(&self) -> &'static [&'static str] {
        &[]
    }

    fn describe(&self) -> String {
        let all = self.processes();
        if all.is_empty() {
            return "aucun flux audio actif (pactl)".into();
        }
        all.iter()
            .map(|p| format!("{} (pid {}) {}{}", p.bundle, p.pid, if p.output { "son" } else { "" }, if p.input { "micro" } else { "" }))
            .collect::<Vec<_>>()
            .join(" ; ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SINKS: &str = "Sink Input #52\n\tDriver: PipeWire\n\tCorked: no\n\tMute: no\n\tProperties:\n\t\tmedia.name = \"Playback\"\n\t\tapplication.name = \"Firefox\"\n\t\tapplication.process.id = \"2345\"\n\t\tapplication.process.binary = \"firefox\"\n\nSink Input #60\n\tCorked: yes\n\tProperties:\n\t\tapplication.process.id = \"999\"\n\t\tapplication.process.binary = \"vlc\"\n\nSink Input #61\n\tCorked: no\n\tProperties:\n\t\tapplication.name = \"sans pid\"\n";
    const SOURCES: &str = "Source Output #7\n\tCorked: no\n\tProperties:\n\t\tapplication.process.id = \"2345\"\n\t\tapplication.process.binary = \"firefox\"\n\nSource Output #8\n\tCorked: no\n\tProperties:\n\t\tapplication.process.id = \"77\"\n\t\tapplication.process.binary = \"obs\"\n";

    #[test]
    fn only_the_streams_that_are_not_corked_and_have_a_process_count() {
        let out = parse_streams(SINKS, true);
        assert_eq!(out, vec![AudioProcess { pid: 2345, bundle: "firefox".into(), output: true, input: false }]);
    }

    #[test]
    fn a_process_that_plays_and_captures_the_microphone_is_one_process_in_a_call() {
        let mut all = parse_streams(SINKS, true);
        all.extend(parse_streams(SOURCES, false));
        let merged = merge(all);
        assert_eq!(merged.len(), 2);
        let firefox = merged.iter().find(|p| p.pid == 2345).unwrap();
        assert!(firefox.output && firefox.input, "joue et capte le micro : un appel");
        let obs = merged.iter().find(|p| p.pid == 77).unwrap();
        assert!(!obs.output && obs.input, "capte seulement : pas un média");
        assert_eq!(scriibe_core::media_pause::decision(&merged, 1, &[]), scriibe_core::media_pause::Decision::InCall);
    }

    #[test]
    fn nothing_is_found_in_empty_output() {
        assert!(parse_streams("", true).is_empty());
        assert!(merge(Vec::new()).is_empty());
    }
}
