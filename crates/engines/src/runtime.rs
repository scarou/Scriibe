//! Ce dont les moteurs ont besoin du système, derrière des traits : horloge, journal, clés, processus. Les tests les remplacent
//! par des faux ; l'appli branche les vrais (`SystemClock`, `RealSpawner`) et ceux de sa plateforme (clés, journal).

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Le temps : pour dormir entre deux essais et mesurer le repos.
pub trait Clock: Send + Sync {
    /// Temps écoulé depuis un point fixe.
    fn now(&self) -> Duration;
    fn sleep(&self, duration: Duration);
}

pub struct SystemClock {
    start: Instant,
}

impl Default for SystemClock {
    fn default() -> Self {
        Self { start: Instant::now() }
    }
}

impl Clock for SystemClock {
    fn now(&self) -> Duration {
        self.start.elapsed()
    }

    fn sleep(&self, duration: Duration) {
        std::thread::sleep(duration);
    }
}

/// Journal technique : étapes et durées, jamais le texte dicté ni les clés.
pub trait Log: Send + Sync {
    fn write(&self, message: &str);
}

pub struct NoLog;

impl Log for NoLog {
    fn write(&self, _: &str) {}
}

/// Un journal en mémoire, pour les tests.
#[derive(Default)]
pub struct MemoryLog(Mutex<Vec<String>>);

impl MemoryLog {
    pub fn lines(&self) -> Vec<String> {
        self.0.lock().map(|l| l.clone()).unwrap_or_default()
    }
}

impl Log for MemoryLog {
    fn write(&self, message: &str) {
        if let Ok(mut lines) = self.0.lock() {
            lines.push(message.to_string());
        }
    }
}

/// Les clés d'API que l'appli sait garder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyKind {
    Deepgram,
    OpenRouter,
}

/// Où sont les clés : le gestionnaire d'identifiants du système, puis `.env` (voir `crates/platform`).
pub trait Secrets: Send + Sync {
    fn get(&self, kind: KeyKind) -> Option<String>;
}

/// Un processus lancé par l'appli.
pub trait Child: Send {
    fn is_running(&mut self) -> bool;
    /// Code de sortie, une fois le processus terminé.
    fn exit_code(&mut self) -> Option<i32>;
    /// Arrête le processus et attend sa fin.
    fn kill(&mut self);
}

/// Où va la sortie d'erreur du serveur (pour comprendre une option refusée après une mise à jour).
#[derive(Debug, Clone)]
pub struct LogTarget {
    pub path: PathBuf,
    /// Prolonge le fichier au lieu de l'écraser.
    pub append: bool,
}

/// Lancer des processus et retrouver ceux d'une session précédente.
pub trait Spawner: Send + Sync {
    fn spawn(&self, program: &Path, args: &[String], stderr: Option<&LogTarget>) -> io::Result<Box<dyn Child>>;
    /// Arrête tout processus dont la ligne de commande contient tous ces morceaux : un serveur resté d'un arrêt brutal de l'appli
    /// garderait son modèle en mémoire et son port.
    fn kill_matching(&self, needles: &[String]);
}

struct RealChild(std::process::Child);

impl Child for RealChild {
    fn is_running(&mut self) -> bool {
        matches!(self.0.try_wait(), Ok(None))
    }

    fn exit_code(&mut self) -> Option<i32> {
        self.0.try_wait().ok().flatten().and_then(|s| s.code())
    }

    fn kill(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

impl Drop for RealChild {
    fn drop(&mut self) {
        if matches!(self.0.try_wait(), Ok(None)) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

/// Les vrais processus du système.
#[derive(Default)]
pub struct RealSpawner;

impl Spawner for RealSpawner {
    fn spawn(&self, program: &Path, args: &[String], stderr: Option<&LogTarget>) -> io::Result<Box<dyn Child>> {
        use std::process::{Command, Stdio};
        let mut command = Command::new(program);
        command.args(args).stdin(Stdio::null()).stdout(Stdio::null());
        let log = stderr.and_then(|t| {
            if let Some(dir) = t.path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            std::fs::OpenOptions::new().create(true).write(true).append(t.append).truncate(!t.append).open(&t.path).ok()
        });
        // llama.cpp écrit une partie de son journal (carte graphique trouvée, couches déchargées) sur la sortie normale : les deux
        // vont dans le même fichier.
        match log.map(|file| file.try_clone().map(|clone| (file, clone))) {
            Some(Ok((err, out))) => {
                command.stderr(Stdio::from(err)).stdout(Stdio::from(out));
            }
            Some(Err(_)) | None => {
                command.stderr(Stdio::null());
            }
        }
        // Linux : les bibliothèques (`libllama.so`, `libggml*.so`) livrées à côté du programme se trouvent même si le zip n'a pas gardé le
        // RPATH d'origine.
        #[cfg(target_os = "linux")]
        if let Some(dir) = program.parent().filter(|d| !d.as_os_str().is_empty()) {
            let mut paths = vec![dir.to_path_buf()];
            if let Some(existing) = std::env::var_os("LD_LIBRARY_PATH") {
                paths.extend(std::env::split_paths(&existing));
            }
            if let Ok(joined) = std::env::join_paths(paths) {
                command.env("LD_LIBRARY_PATH", joined);
            }
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // Pas de fenêtre de console pour un serveur lancé en arrière-plan.
            command.creation_flags(0x0800_0000);
        }
        Ok(Box::new(RealChild(command.spawn()?)))
    }

    fn kill_matching(&self, needles: &[String]) {
        use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};
        let mut system = System::new_with_specifics(RefreshKind::nothing());
        system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing().with_cmd(sysinfo::UpdateKind::Always),
        );
        let own = std::process::id();
        for process in system.processes().values() {
            let command_line = process.cmd().iter().map(|a| a.to_string_lossy()).collect::<Vec<_>>().join(" ");
            if process.pid().as_u32() != own && !needles.is_empty() && needles.iter().all(|n| command_line.contains(n.as_str())) {
                process.kill();
            }
        }
    }
}
