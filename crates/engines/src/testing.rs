//! Un monde simulé pour les tests : horloge, réseau et processus faux, dans un seul objet. Le temps ne passe que quand on dort :
//! un test qui attend 30 s de chargement dure quelques microsecondes.

use crate::http::{Http, HttpRequest, HttpResponse, NetworkError};
use crate::runtime::{Child, Clock, LogTarget, MemoryLog, Spawner};
use crate::server::Runtime;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

/// Ce que fait un processus simulé.
#[derive(Debug, Clone, Copy)]
pub struct Script {
    /// Le serveur répond au ping ce temps après son lancement.
    pub ready_after: Duration,
    /// Le processus s'arrête tout de suite avec ce code (option refusée, modèle illisible…).
    pub exit_with: Option<i32>,
}

impl Default for Script {
    fn default() -> Self {
        Self { ready_after: Duration::from_secs(2), exit_with: None }
    }
}

#[derive(Debug, Clone)]
pub struct SpawnRecord {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub stderr: Option<LogTarget>,
}

struct Proc {
    args: Vec<String>,
    started: Duration,
    script: Script,
    killed: bool,
}

type Handler = Box<dyn Fn(&HttpRequest) -> Result<HttpResponse, NetworkError> + Send>;
type ScriptFn = Box<dyn Fn(&[String]) -> Script + Send>;

struct World {
    time: Duration,
    procs: Vec<Proc>,
    spawns: Vec<SpawnRecord>,
    kill_requests: Vec<Vec<String>>,
    requests: Vec<HttpRequest>,
    routes: Vec<(String, Handler)>,
    script: ScriptFn,
}

pub struct FakeWorld {
    world: Mutex<World>,
    pub log: Arc<MemoryLog>,
}

impl FakeWorld {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            world: Mutex::new(World {
                time: Duration::ZERO,
                procs: Vec::new(),
                spawns: Vec::new(),
                kill_requests: Vec::new(),
                requests: Vec::new(),
                routes: Vec::new(),
                script: Box::new(|_| Script::default()),
            }),
            log: Arc::new(MemoryLog::default()),
        })
    }

    fn lock(&self) -> MutexGuard<'_, World> {
        self.world.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Le comportement des processus lancés, selon leurs options.
    pub fn set_script(&self, script: impl Fn(&[String]) -> Script + Send + 'static) {
        self.lock().script = Box::new(script);
    }

    /// Répond aux requêtes dont l'adresse contient `url_part`.
    pub fn route(&self, url_part: &str, handler: impl Fn(&HttpRequest) -> Result<HttpResponse, NetworkError> + Send + 'static) {
        self.lock().routes.push((url_part.to_string(), Box::new(handler)));
    }

    /// Répond toujours la même chose.
    pub fn route_fixed(&self, url_part: &str, status: u16, body: &str) {
        let body = body.as_bytes().to_vec();
        self.route(url_part, move |_| Ok(HttpResponse { status, body: body.clone() }));
    }

    pub fn advance(&self, duration: Duration) {
        self.lock().time += duration;
    }

    pub fn spawns(&self) -> Vec<SpawnRecord> {
        self.lock().spawns.clone()
    }

    pub fn requests(&self) -> Vec<HttpRequest> {
        self.lock().requests.clone()
    }

    pub fn kill_requests(&self) -> Vec<Vec<String>> {
        self.lock().kill_requests.clone()
    }

    /// Nombre de processus encore vivants.
    pub fn live_processes(&self) -> usize {
        let w = self.lock();
        w.procs.iter().filter(|p| !p.killed && p.script.exit_with.is_none()).count()
    }

    pub fn runtime(self: &Arc<Self>) -> Runtime {
        Runtime { http: self.clone(), spawner: Arc::new(self.clone()), clock: self.clone(), log: self.log.clone() }
    }
}

impl Clock for FakeWorld {
    fn now(&self) -> Duration {
        self.lock().time
    }

    fn sleep(&self, duration: Duration) {
        self.lock().time += duration;
    }
}

fn port_of(args: &[String]) -> Option<u16> {
    args.iter().position(|a| a == "--port").and_then(|i| args.get(i + 1)).and_then(|p| p.parse().ok())
}

impl Http for FakeWorld {
    fn send(&self, request: HttpRequest) -> Result<HttpResponse, NetworkError> {
        let mut w = self.lock();
        w.requests.push(request.clone());
        // Un serveur lancé par l'appli : il répond une fois chargé.
        if let Some(rest) = request.url.strip_prefix("http://127.0.0.1:") {
            let port: Option<u16> = rest.split('/').next().and_then(|p| p.parse().ok());
            let path = rest.find('/').map_or("/", |i| &rest[i..]);
            let now = w.time;
            if let Some(port) = port {
                if let Some(p) = w.procs.iter().find(|p| !p.killed && p.script.exit_with.is_none() && port_of(&p.args) == Some(port)) {
                    if now < p.started + p.script.ready_after {
                        return Err(NetworkError("connexion refusée".into()));
                    }
                    // Le serveur répond au ping ; les autres adresses (/inference, /v1/chat/completions) vont aux routes du test.
                    if path == "/" || path == "/health" {
                        return Ok(HttpResponse { status: 200, body: b"ok".to_vec() });
                    }
                }
            }
        }
        match w.routes.iter().find(|(part, _)| request.url.contains(part.as_str())) {
            Some((_, handler)) => handler(&request),
            None => Err(NetworkError("aucune route".into())),
        }
    }
}

struct FakeChild {
    world: Arc<FakeWorld>,
    index: usize,
}

impl Child for FakeChild {
    fn is_running(&mut self) -> bool {
        let w = self.world.lock();
        let p = &w.procs[self.index];
        !p.killed && p.script.exit_with.is_none()
    }

    fn exit_code(&mut self) -> Option<i32> {
        let w = self.world.lock();
        let p = &w.procs[self.index];
        p.script.exit_with.or(if p.killed { Some(-9) } else { None })
    }

    fn kill(&mut self) {
        self.world.lock().procs[self.index].killed = true;
    }
}

/// `Spawner` pour un `Arc<FakeWorld>` (le trait prend `&self`, les processus ont besoin du monde partagé).
impl Spawner for Arc<FakeWorld> {
    fn spawn(&self, program: &Path, args: &[String], stderr: Option<&LogTarget>) -> io::Result<Box<dyn Child>> {
        let mut w = self.lock();
        let script = (w.script)(args);
        let started = w.time;
        w.spawns.push(SpawnRecord { program: program.to_path_buf(), args: args.to_vec(), stderr: stderr.cloned() });
        w.procs.push(Proc { args: args.to_vec(), started, script, killed: false });
        let index = w.procs.len() - 1;
        Ok(Box::new(FakeChild { world: self.clone(), index }))
    }

    fn kill_matching(&self, needles: &[String]) {
        self.lock().kill_requests.push(needles.to_vec());
    }
}
