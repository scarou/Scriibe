//! Micro : `cpal` (WASAPI sous Windows, ALSA sous Linux). Le flux vit sur un fil à lui (`Stream` n'est pas `Send` sur tous les systèmes) et verse ses tampons dans
//! un `Capture`, qui mélange les canaux, passe à 16 kHz et mesure le niveau.

use crate::audio::{Capture, LevelListener, Recorder, Recording, StartInfo, SKIPPED_SECONDS, SKIPPED_SECONDS_WINDOWS};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream};
use scriibe_core::microphone::Microphone;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::Duration;

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

#[derive(Default)]
struct Shared {
    capture: Mutex<Option<Capture>>,
    listener: Mutex<Option<LevelListener>>,
    description: Mutex<String>,
}

impl Shared {
    fn push(&self, samples: &[f32], channels: usize) {
        let level = lock(&self.capture).as_mut().map(|c| c.push(samples, channels));
        if let Some(level) = level {
            let listener = lock(&self.listener).clone();
            if let Some(listener) = listener {
                listener(level);
            }
        }
    }
}

struct Running {
    stop: Sender<()>,
    thread: JoinHandle<()>,
}

#[derive(Default)]
pub struct CpalRecorder {
    shared: Arc<Shared>,
    running: Mutex<Option<Running>>,
    /// La dernière liste des micros et son heure (Linux : chaque parcours d'ALSA écrit des lignes d'erreur sur le terminal et la page
    /// redemande la liste à chaque rafraîchissement ; quelques secondes de mémoire suffisent).
    listed: Mutex<Option<(std::time::Instant, Vec<Microphone>)>>,
}

/// Durée pendant laquelle la liste des micros est gardée en mémoire (Linux seulement).
const MICROPHONES_CACHE: Duration = Duration::from_secs(5);

fn device_name(device: &Device) -> String {
    device.name().unwrap_or_else(|_| "micro sans nom".into())
}

fn is_bluetooth(name: &str) -> bool {
    let n = name.to_lowercase();
    n.contains("bluetooth") || n.contains("hands-free") || n.contains("airpods")
}

/// Ouvre le flux d'un micro. Rend le flux et ce qu'on peut en dire dans le journal.
fn open(device: &Device, shared: &Arc<Shared>) -> Result<(Stream, String), String> {
    let config = device.default_input_config().map_err(|e| format!("format du micro illisible ({e})"))?;
    let channels = usize::from(config.channels());
    let rate = config.sample_rate().0;
    // Windows : le « bang » de certains pilotes à l'ouverture du flux (voir `audio.rs`) ; ailleurs, le seuil de Scribe.
    let skip = if cfg!(windows) { SKIPPED_SECONDS_WINDOWS } else { SKIPPED_SECONDS };
    *lock(&shared.capture) = Some(Capture::with_skip(rate, skip));
    let on_error = |e| eprintln!("micro : erreur du flux ({e})");
    let target = Arc::clone(shared);
    let stream = match config.sample_format() {
        SampleFormat::F32 => {
            device.build_input_stream(&config.clone().into(), move |data: &[f32], _| target.push(data, channels), on_error, None)
        }
        SampleFormat::I16 => device.build_input_stream(
            &config.clone().into(),
            move |data: &[i16], _| {
                let floats: Vec<f32> = data.iter().map(|s| f32::from(*s) / 32768.0).collect();
                target.push(&floats, channels);
            },
            on_error,
            None,
        ),
        SampleFormat::U16 => device.build_input_stream(
            &config.clone().into(),
            move |data: &[u16], _| {
                let floats: Vec<f32> = data.iter().map(|s| (f32::from(*s) - 32768.0) / 32768.0).collect();
                target.push(&floats, channels);
            },
            on_error,
            None,
        ),
        other => return Err(format!("format audio non pris en charge ({other:?})")),
    }
    .map_err(|e| format!("micro refusé par le système ({e})"))?;
    stream.play().map_err(|e| format!("démarrage du micro refusé ({e})"))?;
    Ok((stream, format!("{}, {rate} Hz, {channels} canal(aux)", device_name(device))))
}

impl Recorder for CpalRecorder {
    fn microphones(&self) -> Vec<Microphone> {
        if cfg!(target_os = "linux") {
            if let Some((when, list)) = lock(&self.listed).as_ref() {
                if when.elapsed() < MICROPHONES_CACHE {
                    return list.clone();
                }
            }
        }
        let host = cpal::default_host();
        let list: Vec<Microphone> = host
            .input_devices()
            .map(|devices| {
                devices
                    .enumerate()
                    .map(|(i, d)| {
                        let name = device_name(&d);
                        Microphone { id: i as i64, uid: name.clone(), bluetooth: is_bluetooth(&name), name }
                    })
                    .collect()
            })
            .unwrap_or_default();
        if cfg!(target_os = "linux") {
            *lock(&self.listed) = Some((std::time::Instant::now(), list.clone()));
        }
        list
    }

    fn start(&self, microphone_uid: &str) -> Result<StartInfo, String> {
        let _ = self.stop();
        let uid = microphone_uid.to_string();
        let shared = Arc::clone(&self.shared);
        let (ready_tx, ready_rx) = mpsc::channel::<Result<StartInfo, String>>();
        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let thread = std::thread::Builder::new()
            .name("scriibe-micro".into())
            .spawn(move || {
                let host = cpal::default_host();
                let mut fallback = None;
                let chosen = if uid.is_empty() {
                    None
                } else {
                    let found = host.input_devices().ok().and_then(|mut all| all.find(|d| device_name(d) == uid));
                    if found.is_none() {
                        fallback = Some("absent".to_string());
                    }
                    found
                };
                let attempt = chosen.as_ref().map(|d| open(d, &shared));
                let opened = match attempt {
                    Some(Ok(ok)) => Ok(ok),
                    other => {
                        if let Some(Err(e)) = other {
                            fallback = Some(format!("en échec au démarrage ({e})"));
                        }
                        match host.default_input_device() {
                            Some(device) => open(&device, &shared),
                            None => Err("Aucun micro disponible".to_string()),
                        }
                    }
                };
                match opened {
                    Ok((stream, description)) => {
                        *lock(&shared.description) = description.clone();
                        let _ = ready_tx.send(Ok(StartInfo { description, fallback }));
                        let _ = stop_rx.recv();
                        drop(stream);
                    }
                    Err(e) => {
                        let _ = ready_tx.send(Err(e));
                    }
                }
            })
            .map_err(|e| format!("fil du micro : {e}"))?;
        match ready_rx.recv_timeout(Duration::from_secs(10)) {
            Ok(Ok(info)) => {
                *lock(&self.running) = Some(Running { stop: stop_tx, thread });
                Ok(info)
            }
            Ok(Err(e)) => {
                let _ = thread.join();
                Err(e)
            }
            Err(_) => Err("le micro met trop de temps à s'ouvrir".into()),
        }
    }

    fn stop(&self) -> Recording {
        let running = lock(&self.running).take();
        if let Some(running) = running {
            let _ = running.stop.send(());
            let _ = running.thread.join();
        }
        match lock(&self.shared.capture).take() {
            Some(capture) => capture.finish(),
            None => Recording { wav: crate::audio::wav(&[]), duration: 0.0, level: 0.0, gain: 1.0 },
        }
    }

    fn loudest_so_far(&self) -> f32 {
        lock(&self.shared.capture).as_ref().map_or(0.0, Capture::loudest)
    }

    fn input_description(&self) -> String {
        lock(&self.shared.description).clone()
    }

    fn set_level_listener(&self, listener: Option<LevelListener>) {
        *lock(&self.shared.listener) = listener;
    }
}
