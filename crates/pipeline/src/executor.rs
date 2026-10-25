//! Où s'exécutent les tâches de fond (préchauffage, traitement d'une dictée).

pub type Job = Box<dyn FnOnce() + Send>;

pub trait Executor: Send + Sync {
    fn spawn(&self, name: &str, job: Job);
}

/// Un fil par tâche.
pub struct ThreadExecutor;

impl Executor for ThreadExecutor {
    fn spawn(&self, name: &str, job: Job) {
        let _ = std::thread::Builder::new().name(format!("scriibe-{name}")).spawn(job);
    }
}

/// Exécute la tâche tout de suite, sur le fil appelant : les tests sont déterministes.
pub struct InlineExecutor;

impl Executor for InlineExecutor {
    fn spawn(&self, _: &str, job: Job) {
        job();
    }
}
