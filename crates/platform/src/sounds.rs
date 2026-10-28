//! Sons de début et de fin de dictée.

pub trait Sounds: Send + Sync {
    /// Joue le son `name` (celui des réglages) à `volume` (0 à 1) sans attendre sa fin. « Aucun son » ne joue rien.
    fn play(&self, name: &str, volume: f64);
}

pub struct Silent;

impl Sounds for Silent {
    fn play(&self, _: &str, _: f64) {}
}

/// Les sons proposés, du plus discret au plus marqué : les noms de ceux de Scribe (macOS), pour que le fichier de réglages passe d'un
/// système à l'autre. Les sons eux-mêmes sont synthétisés ici (`synth`), pas copiés.
pub const NAMES: [&str; 12] = ["Tink", "Pop", "Purr", "Morse", "Bottle", "Frog", "Glass", "Ping", "Submarine", "Blow", "Funk", "Hero"];

const RATE: u32 = 44_100;

/// Une note : début (s), fréquence de départ et d'arrivée (Hz), durée (s), vitesse d'extinction, part de bruit (0 à 1).
struct Note {
    at: f64,
    from: f64,
    to: f64,
    length: f64,
    decay: f64,
    noise: f64,
    /// Durée de la montée du son (s) : longue, le son est doux.
    attack: f64,
}

const fn note(at: f64, from: f64, to: f64, length: f64, decay: f64) -> Note {
    Note { at, from, to, length, decay, noise: 0.0, attack: 0.003 }
}

fn notes(name: &str) -> Option<Vec<Note>> {
    Some(match name {
        "Tink" => vec![note(0.0, 1760.0, 1760.0, 0.09, 45.0)],
        "Pop" => vec![note(0.0, 520.0, 260.0, 0.10, 38.0)],
        "Purr" => vec![note(0.0, 240.0, 200.0, 0.22, 11.0), note(0.0, 480.0, 400.0, 0.18, 14.0)],
        "Morse" => vec![note(0.0, 880.0, 880.0, 0.05, 20.0), note(0.09, 880.0, 880.0, 0.05, 20.0)],
        "Bottle" => vec![note(0.0, 420.0, 400.0, 0.22, 16.0)],
        "Frog" => vec![note(0.0, 300.0, 430.0, 0.07, 22.0), note(0.08, 330.0, 480.0, 0.09, 22.0)],
        "Glass" => vec![note(0.0, 2093.0, 2093.0, 0.45, 9.0), note(0.0, 3136.0, 3136.0, 0.30, 14.0)],
        "Ping" => vec![note(0.0, 1318.0, 1318.0, 0.30, 14.0)],
        "Submarine" => vec![note(0.0, 330.0, 190.0, 0.40, 8.0)],
        "Blow" => vec![Note { noise: 0.3, attack: 0.03, ..note(0.0, 330.0, 250.0, 0.24, 9.0) }],
        "Funk" => vec![note(0.0, 330.0, 330.0, 0.09, 18.0), note(0.10, 440.0, 440.0, 0.14, 14.0)],
        "Hero" => vec![note(0.0, 523.0, 523.0, 0.12, 12.0), note(0.11, 659.0, 659.0, 0.12, 12.0), note(0.22, 784.0, 784.0, 0.30, 8.0)],
        _ => return None,
    })
}

/// Le son `name` à `volume` (0 à 1) en fichier WAV (16 bits, mono, 44,1 kHz), ou rien pour « Aucun » et les noms inconnus.
pub fn synth(name: &str, volume: f64) -> Option<Vec<u8>> {
    let notes = notes(name)?;
    let end = notes.iter().map(|n| n.at + n.length).fold(0.0, f64::max);
    let total = (end * f64::from(RATE)) as usize + 1;
    let mut mix = vec![0.0f64; total];
    let mut seed = 0x2545_F491_4F6C_DD1Du64;
    for n in &notes {
        let first = (n.at * f64::from(RATE)) as usize;
        let count = (n.length * f64::from(RATE)) as usize;
        let mut phase = 0.0f64;
        let mut low = 0.0f64;
        for i in 0..count.min(total - first) {
            let t = i as f64 / f64::from(RATE);
            let freq = n.from + (n.to - n.from) * (t / n.length);
            phase += 2.0 * std::f64::consts::PI * freq / f64::from(RATE);
            let attack = (t / n.attack).min(1.0);
            let release = ((n.length - t) / 0.01).clamp(0.0, 1.0);
            let envelope = attack * release * (-n.decay * t).exp();
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let white = (seed >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0;
            low += 0.15 * (white - low);
            mix[first + i] += envelope * (phase.sin() * (1.0 - n.noise) + low * 3.0 * n.noise);
        }
    }
    let peak = mix.iter().fold(0.0f64, |m, x| m.max(x.abs())).max(1e-9);
    // Normalisé à 0,8 de la pleine échelle au volume 1, puis proportionnel au volume.
    let gain = 0.8 * volume.clamp(0.0, 1.0) / peak;
    let pcm: Vec<i16> = mix.iter().map(|x| (x * gain * f64::from(i16::MAX)) as i16).collect();
    Some(wav(&pcm))
}

fn wav(pcm: &[i16]) -> Vec<u8> {
    let data = (pcm.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + pcm.len() * 2);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&RATE.to_le_bytes());
    out.extend_from_slice(&(RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data.to_le_bytes());
    for sample in pcm {
        out.extend_from_slice(&sample.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peak(wav: &[u8]) -> i32 {
        wav[44..].chunks(2).map(|b| i32::from(i16::from_le_bytes([b[0], b[1]])).abs()).max().unwrap_or(0)
    }

    #[test]
    fn every_name_makes_a_short_wav_and_none_makes_nothing() {
        for name in NAMES {
            let sound = synth(name, 0.5).unwrap_or_else(|| panic!("{name}"));
            assert_eq!(&sound[..4], b"RIFF");
            let seconds = (sound.len() - 44) as f64 / 2.0 / f64::from(RATE);
            assert!((0.05..0.6).contains(&seconds), "{name} : {seconds} s");
            assert!(peak(&sound) > 1000, "{name} est audible");
        }
        assert!(synth(scriibe_core::settings::NO_SOUND, 0.5).is_none());
        assert!(synth("Inconnu", 0.5).is_none());
    }

    #[test]
    fn the_volume_scales_the_amplitude() {
        let loud = peak(&synth("Ping", 1.0).unwrap()) as f64;
        let soft = peak(&synth("Ping", 0.25).unwrap()) as f64;
        assert!((soft / loud - 0.25).abs() < 0.02, "{soft} / {loud}");
        assert_eq!(peak(&synth("Ping", 0.0).unwrap()), 0);
    }
}
