//! Le cycle de vie d'un serveur de modèle (`whisper-server`, `llama-server`) : démarrage partagé, attente de « prêt », arrêt,
//! libération au repos, reprise sans assistant MTP. Port de `WhisperServer.swift` et `LlamaServer.swift` (Scribe).
//!
//! Les appels bloquent (attente du chargement du modèle, jusqu'à 30 s pour Whisper et 2 min pour llama.cpp) : ils se font sur un fil de
//! travail, jamais sur celui de l'interface.

use crate::file_name;
use crate::http::{Http, HttpRequest, ServiceError};
use crate::runtime::{Child, Clock, Log, LogTarget, Spawner};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::Duration;

/// Ce qui change d'un serveur à l'autre.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Nom dans le journal (« whisper », « llama.cpp »).
    pub name: &'static str,
    pub port: u16,
    /// Adresse interrogée pour savoir si le serveur est prêt.
    pub ping_path: &'static str,
    /// Prêt seulement si la réponse est un 200 (llama.cpp répond 503 pendant le chargement) ; sinon toute réponse suffit.
    pub ping_needs_ok: bool,
    /// Nombre d'interrogations avant d'abandonner (Whisper : 120 × 0,25 s ; llama.cpp : 480 × 0,25 s).
    pub max_polls: u32,
    pub poll: Duration,
    /// Morceaux de la ligne de commande d'un serveur resté d'une session précédente.
    pub orphan_needles: Vec<String>,
    /// Fichier où garder la sortie d'erreur du serveur.
    pub log_file: Option<PathBuf>,
}

/// Comment lancer un serveur en particulier.
pub trait Launcher: Send + Sync {
    fn config(&self) -> &ServerConfig;
    fn binary(&self) -> Option<PathBuf>;
    fn model_exists(&self, model: &str) -> bool;
    /// Les options, et si l'assistant MTP y est vraiment (il peut manquer à côté du modèle).
    fn args(&self, model: &str, use_assistant: bool) -> (Vec<String>, bool);
    /// Précision ajoutée à la ligne du journal (« , avec assistant MTP »).
    fn describe(&self, _assistant: bool) -> String {
        String::new()
    }
    fn missing_binary(&self) -> String;
    fn missing_model(&self) -> String;
    fn start_failed(&self) -> String;
    /// Après un démarrage raté (le processus s'est arrêté), un seul autre essai avec un réglage de repli (llama.cpp : sans carte
    /// graphique). Dit lequel, pour le journal ; `None` : pas de repli, ou déjà en cours pour ce modèle.
    fn begin_fallback(&self, _model: &str) -> Option<&'static str> {
        None
    }
    /// Ce modèle tourne-t-il avec le repli ?
    fn in_fallback(&self, _model: &str) -> bool {
        false
    }
    /// Le repli n'a pas aidé : on l'oublie, le choix de départ reste le bon pour la suite.
    fn cancel_fallback(&self, _model: &str) {}
}

/// Ce que les moteurs demandent au système.
#[derive(Clone)]
pub struct Runtime {
    pub http: Arc<dyn Http>,
    pub spawner: Arc<dyn Spawner>,
    pub clock: Arc<dyn Clock>,
    pub log: Arc<dyn Log>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum StartError {
    /// Un autre démarrage (autre modèle) a pris la place de celui-ci.
    Cancelled,
    Failed(ServiceError),
}

struct Running {
    serial: u64,
    child: Box<dyn Child>,
}

#[derive(Default)]
struct State {
    child: Option<Running>,
    next_serial: u64,
    running_model: String,
    /// Le serveur a répondu « prêt » et n'a pas été arrêté depuis.
    ready: bool,
    /// Dernier usage (démarrage demandé ou fin d'une transcription).
    last_use: Duration,
    /// Démarrage en cours : tous les appels attendent le même, au lieu de se tuer mutuellement le serveur pendant le chargement.
    starting: Option<(u64, String)>,
    next_id: u64,
    last_outcome: Option<(u64, Result<(), StartError>)>,
    /// Modèles dont l'assistant MTP a été refusé par le serveur installé : relancés directement sans lui.
    assistant_refused: HashSet<String>,
    in_flight: usize,
    /// Repos au-delà duquel le serveur s'arrête (`None` : jamais).
    idle_limit: Option<Duration>,
}

pub struct EngineServer<L: Launcher> {
    launcher: L,
    rt: Runtime,
    state: Mutex<State>,
    changed: Condvar,
}

/// Tant que ce témoin vit, le serveur est « occupé » : il ne se relance ni ne se libère. Le repos se compte à partir de sa fin.
pub struct UseGuard<'a, L: Launcher> {
    server: &'a EngineServer<L>,
}

impl<L: Launcher> Drop for UseGuard<'_, L> {
    fn drop(&mut self) {
        let mut st = self.server.lock();
        st.in_flight = st.in_flight.saturating_sub(1);
        st.last_use = self.server.rt.clock.now();
    }
}

impl<L: Launcher> EngineServer<L> {
    pub fn new(launcher: L, rt: Runtime) -> Self {
        Self { launcher, rt, state: Mutex::new(State::default()), changed: Condvar::new() }
    }

    pub fn launcher(&self) -> &L {
        &self.launcher
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn log(&self, message: &str) {
        self.rt.log.write(&format!("{}: {message}", self.launcher.config().name));
    }

    /// Le serveur tourne (processus vivant).
    pub fn is_running(&self) -> bool {
        self.lock().child.as_mut().is_some_and(|r| r.child.is_running())
    }

    /// Le serveur tourne avec ce modèle et a répondu « prêt ».
    pub fn is_ready_for(&self, model: &str) -> bool {
        let mut st = self.lock();
        let same = st.running_model == model;
        st.ready && same && st.child.as_mut().is_some_and(|r| r.child.is_running())
    }

    pub fn begin_use(&self) -> UseGuard<'_, L> {
        self.lock().in_flight += 1;
        UseGuard { server: self }
    }

    /// Le repos au-delà duquel le serveur s'arrête (`None` : jamais). Pris en compte tout de suite, sans attendre la prochaine dictée.
    pub fn set_idle_limit(&self, limit: Option<Duration>) {
        let mut st = self.lock();
        st.idle_limit = limit;
        st.last_use = self.rt.clock.now();
    }

    /// À appeler régulièrement (chaque minute) : arrête le serveur s'il est resté inutilisé trop longtemps, jamais pendant un démarrage
    /// ni une transcription. Renvoie vrai s'il l'a arrêté.
    pub fn release_if_idle(&self) -> bool {
        let (release, minutes) = {
            let mut st = self.lock();
            let running = st.child.as_mut().is_some_and(|r| r.child.is_running());
            let idle = self.rt.clock.now().saturating_sub(st.last_use);
            let over = st.idle_limit.is_some_and(|limit| idle > limit);
            (running && st.starting.is_none() && st.in_flight == 0 && over, st.idle_limit.map_or(0, |l| l.as_secs() / 60))
        };
        if release {
            self.log(&format!("serveur arrêté après {minutes} min sans dictée"));
            self.terminate_process();
        }
        release
    }

    /// Arrêt définitif (changement de moteur, sortie de l'appli) : annule un démarrage en cours et attend la fin du processus.
    pub fn stop(&self) {
        {
            let mut st = self.lock();
            st.starting = None;
        }
        self.changed.notify_all();
        self.terminate_process();
    }

    /// Le serveur répond-il ?
    pub fn ping(&self) -> bool {
        let cfg = self.launcher.config();
        let request = HttpRequest::get(format!("http://127.0.0.1:{}{}", cfg.port, cfg.ping_path), Duration::from_millis(500));
        match self.rt.http.send(request) {
            Ok(response) => !cfg.ping_needs_ok || response.status == 200,
            Err(_) => false,
        }
    }

    /// Démarre le serveur avec ce modèle s'il ne tourne pas déjà avec lui. Un démarrage en cours se rejoint.
    pub fn ensure_running(&self, model: &str) -> Result<(), ServiceError> {
        let mut ping_failed_once = false;
        loop {
            let mut st = self.lock();
            st.last_use = self.rt.clock.now();
            // Un démarrage en cours pour ce modèle se rejoint avant toute vérification de santé : le serveur ne répond pas pendant le chargement.
            if let Some((id, starting_model)) = st.starting.clone() {
                if starting_model == model {
                    while st.starting.as_ref().map(|s| s.0) == Some(id) {
                        st = self.changed.wait(st).unwrap_or_else(|p| p.into_inner());
                    }
                    match st.last_outcome.clone() {
                        Some((outcome_id, Ok(()))) if outcome_id == id => return Ok(()),
                        Some((outcome_id, Err(StartError::Failed(e)))) if outcome_id == id => return Err(e),
                        _ => continue,
                    }
                }
            }
            let alive = st.child.as_mut().is_some_and(|r| r.child.is_running());
            if alive && st.running_model == model && st.ready {
                // Un serveur qui transcrit peut mettre plus de 0,5 s à répondre au ping : on ne le relance jamais en pleine transcription.
                if st.in_flight > 0 {
                    return Ok(());
                }
                drop(st);
                if self.ping() {
                    return Ok(());
                }
                // Un seul échec ne condamne pas un serveur occupé : on réessaie une fois, et un démarrage lancé entre-temps est rejoint.
                if !ping_failed_once {
                    ping_failed_once = true;
                    self.rt.clock.sleep(Duration::from_millis(300));
                    continue;
                }
                st = self.lock();
                if st.starting.as_ref().is_some_and(|s| s.1 == model) {
                    continue;
                }
            }
            // C'est à nous de démarrer : un démarrage pour un autre modèle est annulé.
            st.next_id += 1;
            let id = st.next_id;
            st.starting = Some((id, model.to_string()));
            drop(st);
            self.changed.notify_all();
            let outcome = self.launch(model, true, id);
            let mut st = self.lock();
            if st.starting.as_ref().map(|s| s.0) == Some(id) {
                st.starting = None;
            }
            st.last_outcome = Some((id, outcome.clone()));
            drop(st);
            self.changed.notify_all();
            return match outcome {
                Ok(()) => Ok(()),
                Err(StartError::Failed(e)) => Err(e),
                Err(StartError::Cancelled) => Err(ServiceError::new("Démarrage annulé")),
            };
        }
    }

    fn cancelled(&self, id: u64) -> bool {
        self.lock().starting.as_ref().map(|s| s.0) != Some(id)
    }

    fn child_alive(&self, serial: u64) -> bool {
        self.lock().child.as_mut().is_some_and(|r| r.serial == serial && r.child.is_running())
    }

    fn is_current(&self, serial: u64) -> bool {
        self.lock().child.as_ref().is_some_and(|r| r.serial == serial)
    }

    fn launch(&self, model: &str, use_assistant: bool, id: u64) -> Result<(), StartError> {
        self.terminate_process();
        let cfg = self.launcher.config().clone();
        let Some(binary) = self.launcher.binary() else {
            return Err(StartError::Failed(ServiceError::new(self.launcher.missing_binary())));
        };
        if model.is_empty() || !self.launcher.model_exists(model) {
            return Err(StartError::Failed(ServiceError::new(self.launcher.missing_model())));
        }
        self.rt.spawner.kill_matching(&cfg.orphan_needles);

        let refused = self.lock().assistant_refused.contains(model);
        let (args, used_assistant) = self.launcher.args(model, use_assistant && !refused);
        // Sortie d'erreur gardée (écrasée à chaque lancement sauf le nouvel essai sans assistant, qui la prolonge) : sans elle,
        // une option refusée après une mise à jour du serveur ne laisserait aucune trace.
        let target = cfg.log_file.as_ref().map(|path| LogTarget { path: path.clone(), append: !use_assistant });
        let child = self
            .rt
            .spawner
            .spawn(&binary, &args, target.as_ref())
            .map_err(|e| StartError::Failed(ServiceError::new(format!("{} n'a pas pu être lancé ({e})", cfg.name))))?;
        let serial = {
            let mut st = self.lock();
            st.next_serial += 1;
            let serial = st.next_serial;
            st.child = Some(Running { serial, child });
            st.running_model = model.to_string();
            st.ready = false;
            serial
        };
        self.log(&format!("démarrage du serveur ({}{})", file_name(model), self.launcher.describe(used_assistant)));

        for _ in 0..cfg.max_polls {
            if self.cancelled(id) {
                break;
            }
            let alive = self.child_alive(serial);
            // Le processus doit aussi être vivant : un ancien serveur qui répond encore pendant son arrêt ne valide pas ce démarrage.
            if alive && self.ping() && self.child_alive(serial) {
                let mut st = self.lock();
                if st.child.as_ref().is_some_and(|r| r.serial == serial) {
                    st.ready = true;
                }
                drop(st);
                self.log("serveur prêt");
                return Ok(());
            }
            if !alive {
                break;
            }
            self.rt.clock.sleep(cfg.poll);
        }
        let exit_code = {
            let mut st = self.lock();
            match st.child.as_mut() {
                Some(r) if r.serial == serial => {
                    if r.child.is_running() {
                        None
                    } else {
                        r.child.exit_code().or(Some(-1))
                    }
                }
                _ => None,
            }
        };
        // On n'arrête que notre propre serveur, jamais celui qu'un autre démarrage aurait lancé entre-temps.
        if self.is_current(serial) {
            self.terminate_process();
        }
        if self.cancelled(id) {
            return Err(StartError::Cancelled);
        }
        if let Some(code) = exit_code {
            self.log(&format!("le serveur s'est arrêté (code {code})"));
            // Une mise à jour peut refuser les options de l'assistant : on retente une fois sans lui, et si cela démarre on s'en
            // souvient pour ce modèle (sinon chaque rechargement paierait un échec de plus).
            if used_assistant {
                self.log("nouvel essai sans assistant MTP");
                let retry = self.launch(model, false, id);
                // Un démarrage rendu possible par le repli (et non par l'abandon de l'assistant) ne prouve rien sur l'assistant.
                if retry.is_ok() && !self.launcher.in_fallback(model) {
                    self.lock().assistant_refused.insert(model.to_string());
                }
                return retry;
            }
            // Repli (llama.cpp : la carte graphique n'a pas assez de mémoire pour le modèle) : un autre essai, avec le même choix
            // d'assistant. S'il démarre, le repli reste en vigueur pour ce modèle ; sinon il est oublié.
            if let Some(what) = self.launcher.begin_fallback(model) {
                self.log(&format!("nouvel essai {what}"));
                let retry = self.launch(model, use_assistant, id);
                if retry.is_err() {
                    self.launcher.cancel_fallback(model);
                }
                return retry;
            }
        }
        Err(StartError::Failed(ServiceError::new(self.launcher.start_failed())))
    }

    fn terminate_process(&self) {
        let running = {
            let mut st = self.lock();
            st.ready = false;
            st.running_model.clear();
            st.child.take()
        };
        if let Some(mut r) = running {
            if r.child.is_running() {
                r.child.kill();
            }
        }
    }
}
