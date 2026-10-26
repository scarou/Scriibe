//! Pause des médias pendant la dictée (YouTube, musique…) et reprise ensuite.
//!
//! `MediaPauser` est le port de la partie « minuteries » de `MediaPause.swift` (Scribe) ; il est commun aux systèmes. Chaque système
//! fournit seulement une `AudioSource` : quels processus sortent du son ou captent le micro, et l'envoi de la touche Lecture/Pause.

use scriibe_core::media_pause::{self, AudioProcess, Decision};
use scriibe_engines::runtime::Log;
use std::collections::HashSet;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

pub trait MediaControl: Send + Sync {
    /// Met en pause ce qui joue, si `enabled` et si quelque chose joue (et pas un appel).
    fn pause_if_playing(&self, enabled: bool);
    /// Relance seulement ce que `pause_if_playing` a mis en pause.
    fn resume_if_paused(&self);
    /// Où écrire ce que fait la pause (pour comprendre une pause ratée). Facultatif.
    fn set_log(&self, _log: Arc<dyn Log>) {}
}

/// Aucune pause de média (système où elle n'est pas encore portée).
pub struct NoMedia;

impl MediaControl for NoMedia {
    fn pause_if_playing(&self, _: bool) {}
    fn resume_if_paused(&self) {}
}

/// Ce que `MediaPauser` demande au système.
pub trait AudioSource: Send + Sync + 'static {
    /// Les processus qui sortent du son ou captent le micro, à cet instant.
    fn processes(&self) -> Vec<AudioProcess>;
    /// La touche Lecture/Pause du clavier, appuyée puis relâchée.
    fn play_pause(&self);
    /// Processus système qui produisent des sons de leur propre chef : ils ne sont pas un média à interrompre.
    fn ignored(&self) -> &'static [&'static str];
    /// Ce que le système voit, en une ligne, pour le journal quand rien n'est mis en pause (comprendre pourquoi).
    fn describe(&self) -> String {
        String::new()
    }
}

/// Les délais de Scribe (`MediaPause.swift`) ; les tests les raccourcissent.
#[derive(Debug, Clone, Copy)]
pub struct Timing {
    /// La pause part ce temps après l'ouverture du micro : un appui bref ou une annulation ne fait pas clignoter la vidéo.
    pub start_delay: Duration,
    /// Deux touches ne partent jamais à moins de ce temps l'une de l'autre.
    pub min_gap: Duration,
    /// Un nouveau processus doit encore sortir du son après ce temps pour compter (une alerte de notification ne compte pas).
    pub confirm: Duration,
    /// Vérification de la pause : nombre de mesures et pas entre deux mesures.
    pub verify_steps: u32,
    pub verify_step: Duration,
    /// Un nouveau processus doit être vu sur ce nombre de mesures de suite pour compter comme « autre chose lancée par la touche ».
    pub streak: u32,
}

impl Default for Timing {
    fn default() -> Self {
        Self {
            start_delay: Duration::from_millis(250),
            min_gap: Duration::from_millis(350),
            confirm: Duration::from_millis(200),
            verify_steps: 20,
            verify_step: Duration::from_millis(50),
            streak: 3,
        }
    }
}

#[derive(Default)]
struct State {
    /// Une pause est demandée (en attente) ou envoyée ; faux dès la relance ou l'annulation.
    active: bool,
    sent_at: Option<Instant>,
    /// Les processus qui jouaient quand la pause a été envoyée. Un navigateur peut garder son flux audio ouvert tant que la vidéo est
    /// en pause : seul un processus nouveau montre qu'autre chose joue.
    paused: Vec<AudioProcess>,
    /// Change à chaque pause et à chaque reprise : une tâche en retard d'une pause déjà annulée ne fait plus rien.
    generation: u64,
}

struct Shared<S: AudioSource> {
    source: S,
    timing: Timing,
    own_pid: i64,
    state: Mutex<State>,
    log: Mutex<Option<Arc<dyn Log>>>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

impl<S: AudioSource> Shared<S> {
    fn log(&self, line: &str) {
        if let Some(log) = lock(&self.log).as_ref() {
            log.write(line);
        }
    }

    fn current(&self, mine: u64) -> bool {
        let state = lock(&self.state);
        state.generation == mine && state.active
    }

    fn send_pause(&self, mine: u64) {
        let snapshot = self.source.processes();
        let ignored = self.source.ignored();
        match media_pause::decision(&snapshot, self.own_pid, ignored) {
            Decision::NothingPlaying => {
                {
                    let mut state = lock(&self.state);
                    if state.generation != mine {
                        return;
                    }
                    state.active = false;
                }
                self.log(&format!("médias: rien ne joue — {}", self.source.describe()));
            }
            Decision::InCall => {
                {
                    let mut state = lock(&self.state);
                    if state.generation != mine {
                        return;
                    }
                    state.active = false;
                }
                self.log("médias: appel en cours (son et micro dans le même processus), pas de pause");
            }
            Decision::Play(names) => {
                {
                    let mut state = lock(&self.state);
                    if state.generation != mine || !state.active {
                        return;
                    }
                    state.paused = media_pause::playing(&snapshot, self.own_pid, ignored).into_iter().cloned().collect();
                    state.sent_at = Some(Instant::now());
                }
                self.source.play_pause();
                self.log(&format!("médias: pause demandée ({})", names.join(", ")));
                self.verify(mine);
            }
        }
    }

    /// Mesure le délai de la pause (journal) et vérifie que la touche n'a pas lancé une autre appli (« Lecture » reçue par un lecteur
    /// qui n'était pas celui qui jouait) : dans ce cas la touche est renvoyée pour annuler.
    fn verify(&self, mine: u64) {
        let known: HashSet<i64> = lock(&self.state).paused.iter().map(|p| p.pid).collect();
        let ignored = self.source.ignored();
        let mut stopped = false;
        let mut streak: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
        for _ in 0..self.timing.verify_steps {
            std::thread::sleep(self.timing.verify_step);
            if !self.current(mine) {
                return;
            }
            let snapshot = self.source.processes();
            let others = media_pause::new_players(&snapshot, self.own_pid, &known, ignored);
            streak = others.into_iter().map(|name| (name.clone(), streak.get(&name).copied().unwrap_or(0) + 1)).collect();
            let playing = media_pause::playing(&snapshot, self.own_pid, ignored);
            // Un appel qui démarre n'est pas « autre chose » que la touche aurait lancé.
            if playing.iter().any(|p| p.input && !known.contains(&p.pid)) {
                continue;
            }
            if let Some((started, _)) = streak.iter().find(|(_, count)| **count >= self.timing.streak) {
                {
                    let mut state = lock(&self.state);
                    if state.generation != mine {
                        return;
                    }
                    state.active = false;
                    state.generation += 1;
                    state.sent_at = None;
                }
                self.source.play_pause();
                self.log(&format!("médias: la touche a lancé autre chose ({started}), renvoyée pour annuler"));
                return;
            }
            if !stopped && playing.iter().all(|p| !known.contains(&p.pid)) {
                stopped = true;
                let sent = lock(&self.state).sent_at;
                if let Some(sent) = sent {
                    self.log(&format!("médias: son arrêté {} ms après la pause", sent.elapsed().as_millis()));
                }
            }
        }
        if !stopped {
            self.log("médias: le processus garde son flux audio ouvert après la pause (normal pour un navigateur)");
        }
    }

    /// Relance ce qui a été mis en pause, sauf si une autre appli joue (vraiment) entre-temps.
    fn resume(&self, players: &[AudioProcess]) {
        let known: HashSet<i64> = players.iter().map(|p| p.pid).collect();
        let ignored = self.source.ignored();
        let mut others = media_pause::new_players(&self.source.processes(), self.own_pid, &known, ignored);
        if !others.is_empty() {
            std::thread::sleep(self.timing.confirm);
            let later: HashSet<String> =
                media_pause::new_players(&self.source.processes(), self.own_pid, &known, ignored).into_iter().collect();
            others.retain(|name| later.contains(name));
        }
        if !others.is_empty() {
            self.log(&format!("médias: autre chose joue ({}), pas de relance", others.join(", ")));
            return;
        }
        self.source.play_pause();
        self.log("médias: relance demandée");
    }
}

/// La pause des médias, pour tout système qui sait dire ce qui joue et envoyer la touche Lecture/Pause.
pub struct MediaPauser<S: AudioSource> {
    shared: Arc<Shared<S>>,
}

impl<S: AudioSource> MediaPauser<S> {
    pub fn new(source: S) -> Self {
        Self::with_timing(source, Timing::default(), i64::from(std::process::id()))
    }

    pub fn with_timing(source: S, timing: Timing, own_pid: i64) -> Self {
        Self { shared: Arc::new(Shared { source, timing, own_pid, state: Mutex::new(State::default()), log: Mutex::new(None) }) }
    }
}

impl<S: AudioSource> MediaControl for MediaPauser<S> {
    /// À l'ouverture du micro : si `enabled`, demande une pause. Elle part un peu plus tard et seulement si la dictée dure. Si un
    /// média joue alors, envoie Lecture/Pause et retient qu'il faudra le relancer.
    fn pause_if_playing(&self, enabled: bool) {
        if !enabled {
            return;
        }
        let mine = {
            let mut state = lock(&self.shared.state);
            if state.active {
                return;
            }
            state.active = true;
            state.generation += 1;
            state.generation
        };
        let shared = Arc::clone(&self.shared);
        std::thread::spawn(move || {
            std::thread::sleep(shared.timing.start_delay);
            if shared.current(mine) {
                shared.send_pause(mine);
            }
        });
    }

    /// Au relâchement ou à l'annulation : relance ce qui a été mis en pause.
    fn resume_if_paused(&self) {
        let (sent, players) = {
            let mut state = lock(&self.shared.state);
            if !state.active {
                return;
            }
            state.active = false;
            state.generation += 1;
            // La pause n'était pas encore partie : rien à relancer.
            let Some(sent) = state.sent_at.take() else { return };
            (sent, state.paused.clone())
        };
        let shared = Arc::clone(&self.shared);
        std::thread::spawn(move || {
            std::thread::sleep(shared.timing.min_gap.saturating_sub(sent.elapsed()));
            shared.resume(&players);
        });
    }

    fn set_log(&self, log: Arc<dyn Log>) {
        *lock(&self.shared.log) = Some(log);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OWN: i64 = 1;
    const BROWSER: i64 = 10;
    const OTHER: i64 = 20;

    fn process(pid: i64, output: bool, input: bool) -> AudioProcess {
        AudioProcess { pid, bundle: format!("app{pid}.exe"), output, input }
    }

    /// Ce que le système dit à un instant, et les touches envoyées. La touche Lecture/Pause arrête le navigateur (ou le relance).
    #[derive(Default)]
    struct Fake {
        processes: Mutex<Vec<AudioProcess>>,
        keys: Mutex<usize>,
        /// Ce que la touche lance : rien (le navigateur s'arrête), ou un autre lecteur qui démarre.
        key_starts_other: Mutex<bool>,
    }

    impl Fake {
        fn set(&self, processes: Vec<AudioProcess>) {
            *lock(&self.processes) = processes;
        }
        fn keys(&self) -> usize {
            *lock(&self.keys)
        }
    }

    struct Source(Arc<Fake>);

    impl AudioSource for Source {
        fn processes(&self) -> Vec<AudioProcess> {
            lock(&self.0.processes).clone()
        }
        fn play_pause(&self) {
            *lock(&self.0.keys) += 1;
            let mut list = lock(&self.0.processes);
            if *lock(&self.0.key_starts_other) {
                list.push(process(OTHER, true, false));
            } else {
                list.retain(|p| p.pid != BROWSER);
            }
        }
        fn ignored(&self) -> &'static [&'static str] {
            &["system.exe"]
        }
    }

    fn fast() -> Timing {
        Timing {
            start_delay: Duration::from_millis(30),
            min_gap: Duration::from_millis(60),
            confirm: Duration::from_millis(30),
            verify_steps: 6,
            verify_step: Duration::from_millis(10),
            streak: 3,
        }
    }

    fn pauser(fake: &Arc<Fake>) -> MediaPauser<Source> {
        MediaPauser::with_timing(Source(fake.clone()), fast(), OWN)
    }

    /// Les délais de `fast()` sont de quelques dizaines de millisecondes : on attend large pour ne pas dépendre de la charge de la machine.
    fn wait(ms: u64) {
        std::thread::sleep(Duration::from_millis(ms * 5));
    }

    #[test]
    fn nothing_is_done_when_the_setting_is_off_or_nothing_plays() {
        let fake = Arc::new(Fake::default());
        fake.set(vec![process(BROWSER, true, false)]);
        let media = pauser(&fake);
        media.pause_if_playing(false);
        wait(150);
        assert_eq!(fake.keys(), 0, "réglage coupé");
        fake.set(vec![process(BROWSER, false, false), process(OWN, true, true)]);
        media.pause_if_playing(true);
        wait(150);
        media.resume_if_paused();
        wait(150);
        assert_eq!(fake.keys(), 0, "rien ne joue (et Scriibe lui-même ne compte pas)");
    }

    #[test]
    fn a_playing_media_is_paused_after_a_short_delay_and_resumed_once() {
        let fake = Arc::new(Fake::default());
        fake.set(vec![process(BROWSER, true, false)]);
        let media = pauser(&fake);
        media.pause_if_playing(true);
        assert_eq!(fake.keys(), 0, "pas tout de suite : un appui bref ne fait pas clignoter la vidéo");
        wait(60);
        assert_eq!(fake.keys(), 1, "pause envoyée");
        wait(100);
        media.resume_if_paused();
        wait(150);
        assert_eq!(fake.keys(), 2, "relancé une fois");
        media.resume_if_paused();
        wait(100);
        assert_eq!(fake.keys(), 2, "pas deux fois");
    }

    #[test]
    fn a_very_short_press_never_touches_the_media() {
        let fake = Arc::new(Fake::default());
        fake.set(vec![process(BROWSER, true, false)]);
        let media = pauser(&fake);
        media.pause_if_playing(true);
        media.resume_if_paused();
        wait(150);
        assert_eq!(fake.keys(), 0);
    }

    #[test]
    fn a_call_is_never_paused() {
        let fake = Arc::new(Fake::default());
        fake.set(vec![process(BROWSER, true, true)]);
        let media = pauser(&fake);
        media.pause_if_playing(true);
        wait(150);
        media.resume_if_paused();
        wait(150);
        assert_eq!(fake.keys(), 0);
    }

    #[test]
    fn system_sounds_are_not_a_media() {
        let fake = Arc::new(Fake::default());
        fake.set(vec![AudioProcess { pid: 99, bundle: "system.exe".into(), output: true, input: false }]);
        let media = pauser(&fake);
        media.pause_if_playing(true);
        wait(150);
        assert_eq!(fake.keys(), 0);
    }

    #[test]
    fn something_else_that_plays_meanwhile_is_not_started_again() {
        let fake = Arc::new(Fake::default());
        fake.set(vec![process(BROWSER, true, false)]);
        let media = pauser(&fake);
        media.pause_if_playing(true);
        wait(150);
        assert_eq!(fake.keys(), 1);
        // Un autre lecteur se met à jouer pendant la dictée : on ne relance pas, on laisserait deux sons.
        fake.set(vec![process(OTHER, true, false)]);
        media.resume_if_paused();
        wait(200);
        assert_eq!(fake.keys(), 1);
    }

    #[test]
    fn a_key_that_started_another_player_is_sent_again_to_cancel() {
        let fake = Arc::new(Fake::default());
        fake.set(vec![process(BROWSER, true, false)]);
        *lock(&fake.key_starts_other) = true;
        let media = pauser(&fake);
        media.pause_if_playing(true);
        wait(250);
        assert_eq!(fake.keys(), 2, "pause, puis la même touche pour annuler ce que la première a lancé");
    }
}
