//! Cycle de vie des serveurs de modèle, dans un monde simulé (horloge, réseau et processus faux).

use scriibe_engines::fs::FileSystem;
use scriibe_engines::llama::{self, LlamaLauncher};
use scriibe_engines::server::EngineServer;
use scriibe_engines::testing::{FakeWorld, Script};
use scriibe_engines::whisper::WhisperLauncher;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

struct FakeFs(Vec<PathBuf>);

impl FileSystem for FakeFs {
    fn exists(&self, path: &Path) -> bool {
        self.0.iter().any(|f| f == path)
    }
    fn list_dir(&self, dir: &Path) -> Vec<String> {
        self.0.iter().filter(|f| f.parent() == Some(dir)).filter_map(|f| f.file_name()?.to_str().map(str::to_string)).collect()
    }
    fn walk(&self, root: &Path) -> Vec<PathBuf> {
        self.0.iter().filter(|f| f.starts_with(root)).cloned().collect()
    }
}

fn exe(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

fn path_var() -> OsString {
    std::env::join_paths(["/fake/bin"]).unwrap()
}

fn files(extra: &[&str]) -> Arc<FakeFs> {
    let mut list = vec![Path::new("/fake/bin").join(exe("whisper-server")), Path::new("/fake/bin").join(exe("llama-server"))];
    list.extend(extra.iter().map(PathBuf::from));
    Arc::new(FakeFs(list))
}

fn whisper(world: &Arc<FakeWorld>, fs: Arc<FakeFs>) -> EngineServer<WhisperLauncher> {
    EngineServer::new(WhisperLauncher::new(fs, 8178, Some(&path_var())), world.runtime())
}

fn llama_server(world: &Arc<FakeWorld>, fs: Arc<FakeFs>) -> EngineServer<LlamaLauncher> {
    llama::new_server(LlamaLauncher::new(fs, 8179, Some(PathBuf::from("/fake/data/llama-server.log")), Some(&path_var())), world.runtime())
}

#[test]
fn whisper_starts_once_and_is_reused() {
    let world = FakeWorld::new();
    let server = whisper(&world, files(&["/m/ggml-large-v3-turbo.bin"]));
    server.ensure_running("/m/ggml-large-v3-turbo.bin").unwrap();
    server.ensure_running("/m/ggml-large-v3-turbo.bin").unwrap();
    let spawns = world.spawns();
    assert_eq!(spawns.len(), 1, "le deuxième appel réutilise le serveur");
    assert_eq!(&spawns[0].args[..6], ["-m", "/m/ggml-large-v3-turbo.bin", "--host", "127.0.0.1", "--port", "8178"]);
    assert!(spawns[0].args.contains(&"-l".to_string()) && spawns[0].args.contains(&"auto".to_string()));
    assert!(server.is_ready_for("/m/ggml-large-v3-turbo.bin"));
    // Un serveur d'une session précédente est retiré avant le lancement.
    assert_eq!(world.kill_requests(), vec![vec!["whisper-server".to_string(), "--port 8178 ".to_string()]]);
}

#[test]
fn whisper_restarts_when_the_model_changes() {
    let world = FakeWorld::new();
    let server = whisper(&world, files(&["/m/ggml-a.bin", "/m/ggml-b.bin"]));
    server.ensure_running("/m/ggml-a.bin").unwrap();
    server.ensure_running("/m/ggml-b.bin").unwrap();
    assert_eq!(world.spawns().len(), 2);
    assert_eq!(world.live_processes(), 1, "l'ancien serveur est arrêté");
    assert!(server.is_ready_for("/m/ggml-b.bin") && !server.is_ready_for("/m/ggml-a.bin"));
}

#[test]
fn a_dead_server_is_restarted() {
    let world = FakeWorld::new();
    let server = whisper(&world, files(&["/m/ggml-a.bin"]));
    server.ensure_running("/m/ggml-a.bin").unwrap();
    server.stop();
    assert!(!server.is_running());
    server.ensure_running("/m/ggml-a.bin").unwrap();
    assert_eq!(world.spawns().len(), 2);
}

#[test]
fn missing_binary_and_model_are_reported() {
    let world = FakeWorld::new();
    let no_binary =
        EngineServer::new(WhisperLauncher::new(Arc::new(FakeFs(vec![PathBuf::from("/m/ggml-a.bin")])), 8178, None), world.runtime());
    let error = no_binary.ensure_running("/m/ggml-a.bin").unwrap_err();
    assert!(error.message.starts_with("whisper-server introuvable"), "{error}");
    assert!(world.spawns().is_empty());

    let server = whisper(&world, files(&[]));
    assert_eq!(server.ensure_running("/m/absent.bin").unwrap_err().message, "Modèle Whisper introuvable — voir Réglages");
    assert_eq!(server.ensure_running("").unwrap_err().message, "Modèle Whisper introuvable — voir Réglages");
}

#[test]
fn a_server_that_never_gets_ready_is_given_up_on_and_stopped() {
    let world = FakeWorld::new();
    world.set_script(|_| Script { ready_after: Duration::from_secs(3600), exit_with: None });
    let server = whisper(&world, files(&["/m/ggml-a.bin"]));
    let error = server.ensure_running("/m/ggml-a.bin").unwrap_err();
    assert_eq!(error.message, "Whisper n'a pas pu démarrer (modèle illisible ?)");
    assert_eq!(world.live_processes(), 0, "le serveur qui ne répond pas est arrêté");
    assert!(!server.is_running());
}

#[test]
fn concurrent_calls_share_one_start() {
    let world = FakeWorld::new();
    let server = Arc::new(whisper(&world, files(&["/m/ggml-a.bin"])));
    let threads: Vec<_> = (0..6)
        .map(|_| {
            let s = server.clone();
            std::thread::spawn(move || s.ensure_running("/m/ggml-a.bin"))
        })
        .collect();
    for t in threads {
        t.join().unwrap().unwrap();
    }
    assert_eq!(world.spawns().len(), 1);
}

#[test]
fn llama_uses_the_assistant_then_remembers_it_was_refused() {
    let world = FakeWorld::new();
    // Le serveur « installé » refuse les options de l'assistant : il s'arrête tout de suite.
    world.set_script(|args| {
        if args.iter().any(|a| a == "--spec-type") {
            Script { ready_after: Duration::ZERO, exit_with: Some(1) }
        } else {
            Script::default()
        }
    });
    let model = "/m/gemma-4-E4B-it-Q4_0.gguf";
    let server = llama_server(&world, files(&[model, "/m/mtp-gemma-4-E4B-it-Q8_0.gguf"]));
    server.ensure_running(model).unwrap();
    let spawns = world.spawns();
    assert_eq!(spawns.len(), 2, "un essai avec l'assistant, un autre sans");
    assert!(spawns[0].args.contains(&"--spec-type".to_string()));
    assert!(!spawns[1].args.contains(&"--spec-type".to_string()));
    // La sortie d'erreur est écrasée au premier essai, prolongée au second.
    assert_eq!(spawns[0].stderr.as_ref().map(|t| t.append), Some(false));
    assert_eq!(spawns[1].stderr.as_ref().map(|t| t.append), Some(true));
    assert!(world.log.lines().iter().any(|l| l.contains("nouvel essai sans assistant MTP")));

    // Après un rechargement, l'assistant n'est plus essayé pour ce modèle.
    server.stop();
    server.ensure_running(model).unwrap();
    let spawns = world.spawns();
    assert_eq!(spawns.len(), 3);
    assert!(!spawns[2].args.contains(&"--spec-type".to_string()));
}

#[test]
fn llama_falls_back_to_the_processor_when_the_automatic_device_cannot_load_the_model() {
    let world = FakeWorld::new();
    // La carte graphique n'a pas assez de mémoire : le serveur s'arrête, sauf sur le processeur seul.
    world.set_script(|args| {
        if args.windows(2).any(|w| w[0] == "--device" && w[1] == "none") {
            Script::default()
        } else {
            Script { ready_after: Duration::ZERO, exit_with: Some(1) }
        }
    });
    let model = "/m/chocolatine-3b-instruct-q4_k_m.gguf";
    let server = llama_server(&world, files(&[model]));
    server.ensure_running(model).unwrap();
    let spawns = world.spawns();
    assert_eq!(spawns.len(), 2, "un essai automatique, un autre sur le processeur");
    assert!(!spawns[0].args.contains(&"--device".to_string()));
    assert!(spawns[1].args.windows(2).any(|w| w[0] == "--device" && w[1] == "none"));
    assert!(world.log.lines().iter().any(|l| l.contains("nouvel essai sur le processeur seul")));

    // Après un rechargement, le premier essai se fait directement sur le processeur.
    server.stop();
    server.ensure_running(model).unwrap();
    let spawns = world.spawns();
    assert_eq!(spawns.len(), 3);
    assert!(spawns[2].args.windows(2).any(|w| w[0] == "--device" && w[1] == "none"));

    // Une carte choisie à la main redonne la main à l'appareil automatique.
    server.launcher().set_device("");
    server.stop();
    server.ensure_running(model).unwrap();
    let spawns = world.spawns();
    assert_eq!(spawns.len(), 5, "automatique (échec), puis processeur");
    assert!(!spawns[3].args.contains(&"--device".to_string()));
}

#[test]
fn llama_forgets_the_fallback_when_the_processor_does_not_help_and_never_overrides_a_chosen_device() {
    let world = FakeWorld::new();
    world.set_script(|_| Script { ready_after: Duration::ZERO, exit_with: Some(1) });
    let model = "/m/broken.gguf";
    let server = llama_server(&world, files(&[model]));
    assert!(server.ensure_running(model).is_err());
    assert_eq!(world.spawns().len(), 2, "automatique puis processeur : pas d'autre essai");
    // Le repli n'a pas aidé : l'essai suivant repart de l'appareil automatique.
    assert!(server.ensure_running(model).is_err());
    assert!(!world.spawns()[2].args.contains(&"--device".to_string()));

    let before = world.spawns().len();
    server.launcher().set_device("Vulkan1");
    assert!(server.ensure_running(model).is_err());
    assert_eq!(world.spawns().len(), before + 1, "appareil choisi : jamais de repli");
}

#[test]
fn llama_without_an_assistant_file_starts_plain() {
    let world = FakeWorld::new();
    let model = "/m/Ministral-3-3B-Instruct-2512-Q4_K_M.gguf";
    let server = llama_server(&world, files(&[model]));
    server.ensure_running(model).unwrap();
    assert!(!world.spawns()[0].args.contains(&"--spec-type".to_string()));
    assert!(world.log.lines().iter().any(|l| l.contains("sans assistant MTP")));
}

#[test]
fn llama_needs_a_health_check_to_answer_200() {
    let world = FakeWorld::new();
    let model = "/m/gemma-4-E4B-it-Q4_0.gguf";
    let server = llama_server(&world, files(&[model]));
    server.ensure_running(model).unwrap();
    let health: Vec<_> = world.requests().into_iter().filter(|r| r.url.ends_with("/health")).collect();
    assert!(!health.is_empty() && health.iter().all(|r| r.url == "http://127.0.0.1:8179/health"));
}

#[test]
fn idle_server_is_released_but_never_while_busy_or_before_the_limit() {
    let world = FakeWorld::new();
    let model = "/m/ggml-a.bin";
    let server = whisper(&world, files(&[model]));
    server.set_idle_limit(Some(Duration::from_secs(10 * 60)));
    server.ensure_running(model).unwrap();
    world.advance(Duration::from_secs(9 * 60));
    assert!(!server.release_if_idle(), "pas avant la durée choisie");
    world.advance(Duration::from_secs(2 * 60));
    {
        let _busy = server.begin_use();
        assert!(!server.release_if_idle(), "jamais pendant une transcription");
    }
    assert!(!server.release_if_idle(), "le repos se compte à partir de la fin de la transcription");
    world.advance(Duration::from_secs(11 * 60));
    assert!(server.release_if_idle());
    assert!(!server.is_running());
    assert!(world.log.lines().iter().any(|l| l.contains("serveur arrêté après 10 min sans dictée")));
    // Il revient à l'appui suivant.
    server.ensure_running(model).unwrap();
    assert!(server.is_running());
}

#[test]
fn no_idle_limit_means_never() {
    let world = FakeWorld::new();
    let model = "/m/ggml-a.bin";
    let server = whisper(&world, files(&[model]));
    server.set_idle_limit(None);
    server.ensure_running(model).unwrap();
    world.advance(Duration::from_secs(100_000));
    assert!(!server.release_if_idle());
}

#[test]
fn llama_is_released_after_thirty_minutes() {
    let world = FakeWorld::new();
    let model = "/m/gemma-4-E4B-it-Q4_0.gguf";
    let server = llama_server(&world, files(&[model]));
    server.ensure_running(model).unwrap();
    world.advance(Duration::from_secs(29 * 60));
    assert!(!server.release_if_idle());
    world.advance(Duration::from_secs(2 * 60));
    assert!(server.release_if_idle());
}

#[test]
fn ensure_running_within_returns_the_result() {
    let world = FakeWorld::new();
    let model = "/m/gemma-4-E4B-it-Q4_0.gguf";
    let server = Arc::new(llama_server(&world, files(&[model])));
    llama::ensure_running_within(&server, model, Duration::from_secs(5)).unwrap();
    assert!(server.is_ready_for(model));
    let error = llama::ensure_running_within(&server, "/m/absent.gguf", Duration::from_secs(5)).unwrap_err();
    assert_eq!(error.message, "Modèle llama.cpp introuvable — voir Réglages");
}

/// Le vrai lancement d'un processus : sur tous les systèmes, `cargo` existe puisque les tests tournent avec lui.
#[test]
fn real_spawner_runs_and_reports_a_process() {
    use scriibe_engines::runtime::{RealSpawner, Spawner};
    let program = std::env::var_os("CARGO").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("cargo"));
    let mut child = RealSpawner.spawn(&program, &["--version".to_string()], None).expect("lancer cargo");
    for _ in 0..200 {
        if !child.is_running() {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(!child.is_running());
    assert_eq!(child.exit_code(), Some(0));
}
