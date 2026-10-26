//! Micro : le trait `Recorder` et le traitement du son, qui est le même partout (mono, 16 kHz, 16 bits, niveau, fichier WAV).
//! Port de `AudioRecorder` (Scribe) ; seule l'ouverture du micro est propre à chaque système.

use scriibe_core::microphone::{MicLevel, Microphone};
use std::sync::Arc;

/// Fréquence envoyée à Whisper et Deepgram : le format le plus léger qu'ils acceptent bien.
pub const SAMPLE_RATE: u32 = 16_000;

/// Les premières 150 ms ne comptent pas dans le niveau (son de début, clic de touche).
pub const SKIPPED_SECONDS: f64 = 0.15;
/// Windows : à l'ouverture du flux, certains pilotes (Conexant mesuré sur un portable HP) produisent un « bang » qui décroît pendant
/// ≈ 300 ms, jusqu'à saturer. Il fausserait le niveau (un micro faible passerait pour correct) : on l'ignore plus longtemps.
pub const SKIPPED_SECONDS_WINDOWS: f64 = 0.45;

/// Une prise terminée.
#[derive(Debug, Clone, PartialEq)]
pub struct Recording {
    /// Fichier WAV mono 16 kHz 16 bits.
    pub wav: Vec<u8>,
    /// Secondes de son.
    pub duration: f64,
    /// RMS de la fenêtre la plus forte, **avant** le gain (≈ 0 pour un micro muet, de 0,019 à 0,083 pour une voix normale).
    pub level: f32,
    /// Amplification appliquée au son d'un micro faible (1 : aucune). Le niveau annoncé reste celui du micro.
    pub gain: f32,
}

/// Ce qu'on sait du micro ouvert, pour le journal.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StartInfo {
    /// « Nom du micro, 48000 Hz, 2 canal(aux) ».
    pub description: String,
    /// Pourquoi le micro choisi n'a pas servi (« absent », « échec au démarrage… »), si on a pris celui du système.
    pub fallback: Option<String>,
}

pub type LevelListener = Arc<dyn Fn(f32) + Send + Sync>;

pub trait Recorder: Send + Sync {
    /// Les micros branchés.
    fn microphones(&self) -> Vec<Microphone>;
    /// Ouvre le micro `uid` (vide : celui du système) et commence la prise. Absent ou refusé, le micro choisi laisse la place à celui
    /// du système (`fallback` le dit).
    fn start(&self, microphone_uid: &str) -> Result<StartInfo, String>;
    /// Arrête le micro et rend la prise.
    fn stop(&self) -> Recording;
    /// Niveau de la fenêtre la plus forte depuis le début de la prise (pour l'alerte pendant la dictée).
    fn loudest_so_far(&self) -> f32;
    /// Micro et format de la dernière prise.
    fn input_description(&self) -> String;
    /// Niveau instantané (0 à 1), appelé à chaque tampon pendant la prise.
    fn set_level_listener(&self, listener: Option<LevelListener>);
}

/// Niveau visé pour un micro faible : celui d'une voix normale (0,019 à 0,083).
const TARGET_LEVEL: f32 = 0.06;
/// Au-delà, on amplifierait surtout le bruit.
const MAX_GAIN: f32 = 10.0;

/// Amplification d'un micro faible, pour que Whisper reçoive un son d'un niveau normal. Pas en dessous du seuil « muet » (du bruit,
/// pas une voix), ni au-dessus du niveau visé. Écart avec Scribe (voir `docs/ECARTS.md`) : mesuré sur un portable dont le micro
/// donne 0,006, où le petit modèle Whisper ne comprenait presque rien.
pub fn weak_voice_gain(level: f32) -> f32 {
    if !(MicLevel::SILENT_BELOW..TARGET_LEVEL).contains(&level) {
        1.0
    } else {
        (TARGET_LEVEL / level).min(MAX_GAIN)
    }
}

/// Accumule le son d'une prise : mélange les canaux, passe à 16 kHz (moyenne sur la durée de chaque échantillon d'arrivée, ce qui
/// atténue le repliement) et garde le niveau de la fenêtre la plus forte.
#[derive(Debug)]
pub struct Capture {
    /// Échantillons d'entrée par échantillon de sortie.
    ratio: f64,
    rate: f64,
    /// Poids (en échantillons d'entrée) déjà cumulés pour l'échantillon de sortie en cours.
    weight: f64,
    sum: f64,
    pcm: Vec<i16>,
    loudest: f32,
    seen: f64,
    /// Secondes du début qui ne comptent pas dans le niveau.
    skip: f64,
}

impl Capture {
    pub fn new(input_rate: u32) -> Self {
        Self::with_skip(input_rate, SKIPPED_SECONDS)
    }

    /// `skip` : secondes du début ignorées pour le niveau (le son, lui, est gardé en entier).
    pub fn with_skip(input_rate: u32, skip: f64) -> Self {
        let rate = f64::from(input_rate.max(1));
        Self { ratio: rate / f64::from(SAMPLE_RATE), rate, weight: 0.0, sum: 0.0, pcm: Vec::new(), loudest: 0.0, seen: 0.0, skip }
    }

    /// Ajoute un tampon (échantillons entrelacés, `channels` canaux). Rend le niveau instantané du tampon (RMS × 9, borné à 1) pour
    /// le vumètre, comme Scribe.
    pub fn push(&mut self, interleaved: &[f32], channels: usize) -> f32 {
        let channels = channels.max(1);
        let frames = interleaved.len() / channels;
        if frames == 0 {
            return 0.0;
        }
        // Le niveau se mesure sur le premier canal, comme Scribe (`floatChannelData[0]`).
        let energy: f32 = interleaved.chunks_exact(channels).map(|f| f[0] * f[0]).sum();
        let rms = (energy / frames as f32).sqrt();
        if self.seen / self.rate >= self.skip {
            self.loudest = self.loudest.max(rms);
        }
        self.seen += frames as f64;
        for frame in interleaved.chunks_exact(channels) {
            let mono = frame.iter().sum::<f32>() / channels as f32;
            self.push_mono(mono);
        }
        (rms * 9.0_f32).min(1.0)
    }

    fn push_mono(&mut self, x: f32) {
        let x = f64::from(x);
        if self.ratio <= 1.0 {
            // Entrée à 16 kHz ou moins : un échantillon d'entrée vaut au moins un échantillon de sortie (répété si besoin).
            let outputs = (1.0 / self.ratio).round().max(1.0) as usize;
            for _ in 0..outputs {
                self.emit(x);
            }
            return;
        }
        let mut remaining: f64 = 1.0;
        while remaining > 1e-9 {
            let take = remaining.min(self.ratio - self.weight);
            self.sum += x * take;
            self.weight += take;
            remaining -= take;
            if self.weight >= self.ratio - 1e-9 {
                let value = self.sum / self.weight;
                self.emit(value);
                self.sum = 0.0;
                self.weight = 0.0;
            }
        }
    }

    fn emit(&mut self, value: f64) {
        self.pcm.push((value.clamp(-1.0, 1.0) * f64::from(i16::MAX)).round() as i16);
    }

    pub fn loudest(&self) -> f32 {
        self.loudest
    }

    pub fn duration(&self) -> f64 {
        self.pcm.len() as f64 / f64::from(SAMPLE_RATE)
    }

    pub fn finish(mut self) -> Recording {
        let gain = weak_voice_gain(self.loudest);
        if gain > 1.0 {
            for sample in &mut self.pcm {
                *sample = (f32::from(*sample) * gain).round().clamp(f32::from(i16::MIN), f32::from(i16::MAX)) as i16;
            }
        }
        Recording { duration: self.duration(), level: self.loudest, gain, wav: wav(&self.pcm) }
    }
}

/// Le profil d'une prise : pour voir, sans l'écouter, un début étouffé (pilote qui s'adapte), une saturation ou un silence.
#[derive(Debug, Clone, PartialEq)]
pub struct Profile {
    /// Plus grande valeur absolue (0 à 1).
    pub peak: f32,
    /// Part des échantillons à ≥ 99 % de l'échelle (écrêtage : le son est déformé).
    pub clipped: f32,
    /// RMS de chaque quart de seconde, du début à la fin.
    pub levels: Vec<f32>,
}

/// Profil d'un fichier WAV 16 bits mono 16 kHz tel que produit par [`wav`].
pub fn profile(wav: &[u8]) -> Profile {
    let samples: Vec<f32> =
        wav.get(44..).unwrap_or(&[]).chunks_exact(2).map(|b| f32::from(i16::from_le_bytes([b[0], b[1]])) / 32768.0).collect();
    let peak = samples.iter().fold(0.0_f32, |m, s| m.max(s.abs()));
    let clipped = if samples.is_empty() { 0.0 } else { samples.iter().filter(|s| s.abs() >= 0.99).count() as f32 / samples.len() as f32 };
    let window = (SAMPLE_RATE / 4) as usize;
    let levels = samples.chunks(window).map(|w| (w.iter().map(|s| s * s).sum::<f32>() / w.len() as f32).sqrt()).collect();
    Profile { peak, clipped, levels }
}

/// Fichier WAV mono 16 kHz 16 bits.
pub fn wav(pcm: &[i16]) -> Vec<u8> {
    let data_len = (pcm.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + pcm.len() * 2);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    out.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for sample in pcm {
        out.extend_from_slice(&sample.to_le_bytes());
    }
    out
}

/// Lit un fichier WAV PCM 16 bits (n'importe quelle fréquence, mono ou stéréo) et le ramène au format de la prise : mono 16 kHz.
/// Sert aux essais (`scriibe essai fichier.wav`) : la chaîne se vérifie sans micro.
pub fn read_wav(bytes: &[u8]) -> Result<Recording, String> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("ce n'est pas un fichier WAV".into());
    }
    let (mut channels, mut rate, mut bits, mut format) = (0usize, 0u32, 0u16, 0u16);
    let mut data: Option<&[u8]> = None;
    let mut at = 12;
    while at + 8 <= bytes.len() {
        let size = u32::from_le_bytes([bytes[at + 4], bytes[at + 5], bytes[at + 6], bytes[at + 7]]) as usize;
        let body = &bytes[at + 8..(at + 8 + size).min(bytes.len())];
        match &bytes[at..at + 4] {
            b"fmt " if body.len() >= 16 => {
                format = u16::from_le_bytes([body[0], body[1]]);
                channels = usize::from(u16::from_le_bytes([body[2], body[3]]));
                rate = u32::from_le_bytes([body[4], body[5], body[6], body[7]]);
                bits = u16::from_le_bytes([body[14], body[15]]);
            }
            b"data" => data = Some(body),
            _ => {}
        }
        // Les blocs sont alignés sur 2 octets.
        at += 8 + size + (size & 1);
    }
    if format != 1 || bits != 16 || channels == 0 || rate == 0 {
        return Err("seul le WAV PCM 16 bits est pris en charge".into());
    }
    let data = data.ok_or("le fichier WAV ne contient pas de son")?;
    let samples: Vec<f32> = data.chunks_exact(2).map(|b| f32::from(i16::from_le_bytes([b[0], b[1]])) / 32768.0).collect();
    let mut capture = Capture::new(rate);
    for chunk in samples.chunks(4096 * channels) {
        capture.push(chunk, channels);
    }
    Ok(capture.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(rate: u32, seconds: f64, amplitude: f32) -> Vec<f32> {
        (0..(f64::from(rate) * seconds) as usize)
            .map(|i| amplitude * (2.0 * std::f64::consts::PI * 440.0 * i as f64 / f64::from(rate)).sin() as f32)
            .collect()
    }

    #[test]
    fn wav_header_is_a_16_khz_mono_pcm_file() {
        let file = wav(&[1, -2, 3]);
        assert_eq!(&file[0..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(file[4..8].try_into().unwrap()), 36 + 6);
        assert_eq!(&file[8..16], b"WAVEfmt ");
        assert_eq!(u16::from_le_bytes(file[20..22].try_into().unwrap()), 1);
        assert_eq!(u16::from_le_bytes(file[22..24].try_into().unwrap()), 1);
        assert_eq!(u32::from_le_bytes(file[24..28].try_into().unwrap()), 16_000);
        assert_eq!(u32::from_le_bytes(file[28..32].try_into().unwrap()), 32_000);
        assert_eq!(&file[36..40], b"data");
        assert_eq!(u32::from_le_bytes(file[40..44].try_into().unwrap()), 6);
        assert_eq!(file.len(), 44 + 6);
        assert_eq!(i16::from_le_bytes(file[44..46].try_into().unwrap()), 1);
        assert_eq!(i16::from_le_bytes(file[46..48].try_into().unwrap()), -2);
    }

    #[test]
    fn one_second_at_48_khz_gives_16_000_samples() {
        let mut capture = Capture::new(48_000);
        let signal = sine(48_000, 1.0, 0.5);
        for chunk in signal.chunks(1_024) {
            capture.push(chunk, 1);
        }
        assert!((capture.duration() - 1.0).abs() < 0.001, "durée {}", capture.duration());
    }

    #[test]
    fn non_integer_ratio_keeps_the_duration() {
        let mut capture = Capture::new(44_100);
        let signal = sine(44_100, 2.0, 0.5);
        for chunk in signal.chunks(2_048) {
            capture.push(chunk, 1);
        }
        assert!((capture.duration() - 2.0).abs() < 0.01, "durée {}", capture.duration());
    }

    #[test]
    fn stereo_is_mixed_to_mono_and_level_follows_the_first_channel() {
        let mut capture = Capture::new(16_000);
        // Canal gauche à 0,5, droit à -0,5 : le mélange est nul, le niveau (premier canal) pas.
        let frames: Vec<f32> = (0..16_000).flat_map(|_| [0.5, -0.5]).collect();
        let instant = capture.push(&frames[..8_000], 2);
        assert!((instant - 1.0).abs() < 1e-6, "vumètre borné à 1 : {instant}");
        capture.push(&frames[8_000..], 2);
        assert!((capture.loudest() - 0.5).abs() < 1e-6);
        let recording = capture.finish();
        assert!(recording.wav[44..].chunks_exact(2).all(|s| i16::from_le_bytes([s[0], s[1]]) == 0));
    }

    #[test]
    fn first_150_ms_do_not_count_for_the_level() {
        let mut capture = Capture::new(16_000);
        capture.push(&vec![0.8; 1_600], 1); // 100 ms fortes (clic de touche)
        capture.push(&vec![0.001; 1_600], 1); // puis presque du silence
                                              // Le deuxième tampon commence à 100 ms (< 150 ms) : il ne compte pas non plus.
        assert_eq!(capture.loudest(), 0.0);
        capture.push(&vec![0.001; 1_600], 1);
        assert!((capture.loudest() - 0.001).abs() < 1e-6);
    }

    #[test]
    fn a_driver_bang_at_the_start_does_not_hide_a_weak_voice() {
        use scriibe_core::microphone::MicLevel;
        // Mesuré sur un portable HP (Conexant) : ≈ 0,3 s de « bang » décroissant, puis une voix faible (≈ 0,007).
        // Les tampons de WASAPI font 10 ms.
        let feed = |capture: &mut Capture| {
            let bang: Vec<f32> = (0..4_800).map(|i| 0.9 * (-(i as f32) / 1_000.0).exp()).collect();
            for chunk in bang.chunks(160) {
                capture.push(chunk, 1);
            }
            for _ in 0..100 {
                capture.push(&[0.007; 160], 1);
            }
        };
        let mut windows = Capture::with_skip(16_000, SKIPPED_SECONDS_WINDOWS);
        feed(&mut windows);
        assert_eq!(MicLevel::assess(windows.loudest()), MicLevel::Weak, "niveau {}", windows.loudest());
        // Avec le seuil de 150 ms du Mac, le bang aurait fait passer ce micro faible pour correct.
        let mut mac = Capture::new(16_000);
        feed(&mut mac);
        assert_eq!(MicLevel::assess(mac.loudest()), MicLevel::Ok, "niveau {}", mac.loudest());
    }

    #[test]
    fn a_weak_voice_is_amplified_but_noise_and_good_voices_are_not() {
        assert_eq!(weak_voice_gain(0.0005), 1.0, "du bruit : on ne l'amplifie pas");
        assert!((weak_voice_gain(0.006) - 10.0).abs() < 1e-6, "0,006 → gain plafonné à 10");
        assert!((weak_voice_gain(0.03) - 2.0).abs() < 1e-6);
        assert_eq!(weak_voice_gain(0.06), 1.0);
        assert_eq!(weak_voice_gain(0.2), 1.0);
        let mut capture = Capture::new(16_000);
        for _ in 0..100 {
            capture.push(&[0.01; 160], 1);
        }
        let recording = capture.finish();
        assert!((recording.gain - 6.0).abs() < 1e-4, "gain {}", recording.gain);
        assert!((recording.level - 0.01).abs() < 1e-6, "le niveau annoncé reste celui du micro");
        let sample = i16::from_le_bytes([recording.wav[44 + 4000], recording.wav[44 + 4001]]);
        assert!((f32::from(sample) / 32768.0 - 0.06).abs() < 0.001, "échantillon {sample}");
    }

    #[test]
    fn a_normal_voice_is_ok_and_silence_is_silent() {
        use scriibe_core::microphone::MicLevel;
        let mut capture = Capture::new(48_000);
        for chunk in sine(48_000, 1.0, 0.1).chunks(2_048) {
            capture.push(chunk, 1);
        }
        assert_eq!(MicLevel::assess(capture.loudest()), MicLevel::Ok);
        let mut quiet = Capture::new(48_000);
        for chunk in vec![0.0001f32; 48_000].chunks(2_048) {
            quiet.push(chunk, 1);
        }
        assert_eq!(MicLevel::assess(quiet.loudest()), MicLevel::Silent);
    }

    #[test]
    fn slow_input_is_repeated_up_to_16_khz() {
        let mut capture = Capture::new(8_000);
        capture.push(&vec![0.1; 8_000], 1);
        assert!((capture.duration() - 1.0).abs() < 0.001);
    }

    #[test]
    fn the_profile_shows_a_muffled_start_and_clipping() {
        // 0,5 s presque muettes, puis 0,5 s fortes et écrêtées.
        let mut pcm = vec![300i16; 8_000];
        pcm.extend(vec![i16::MAX; 8_000]);
        let p = profile(&wav(&pcm));
        assert_eq!(p.levels.len(), 4);
        assert!(p.levels[0] < 0.02 && p.levels[3] > 0.9, "{:?}", p.levels);
        assert!((p.clipped - 0.5).abs() < 0.01 && p.peak > 0.99);
        assert_eq!(profile(&wav(&[])).levels.len(), 0);
    }

    #[test]
    fn a_wav_file_is_read_back_at_16_khz_mono() {
        let pcm: Vec<i16> = (0..16_000).map(|i| ((i as f32 / 20.0).sin() * 8000.0) as i16).collect();
        let back = read_wav(&wav(&pcm)).unwrap();
        assert!((back.duration - 1.0).abs() < 0.001);
        assert!(back.level > 0.1, "niveau {}", back.level);
        assert_eq!(back.wav.len(), 44 + 32_000);
    }

    #[test]
    fn a_stereo_48_khz_wav_is_converted() {
        let mut file = wav(&[]);
        let frames: Vec<u8> = (0..48_000).flat_map(|_| [1000i16, 1000i16]).flat_map(i16::to_le_bytes).collect();
        file[22..24].copy_from_slice(&2u16.to_le_bytes());
        file[24..28].copy_from_slice(&48_000u32.to_le_bytes());
        file[40..44].copy_from_slice(&(frames.len() as u32).to_le_bytes());
        file.extend_from_slice(&frames);
        let back = read_wav(&file).unwrap();
        assert!((back.duration - 1.0).abs() < 0.001, "durée {}", back.duration);
    }

    #[test]
    fn something_that_is_not_a_wav_is_refused() {
        assert!(read_wav(b"pas un wav").is_err());
        assert!(read_wav(b"RIFF\0\0\0\0WAVEfmt \x10\0\0\0\x03\0\x01\0\x80\x3e\0\0\0\0\0\0\0\0\x10\0data\0\0\0\0").is_err());
    }
}
