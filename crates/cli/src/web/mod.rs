//! L'interface locale : un petit serveur sur 127.0.0.1 (port libre, adresse secrète) qui sert une page web et ses commandes. Elle permet,
//! sans taper de commande : de voir ce qui est installé, de télécharger ce qui manque (après confirmation), d'enregistrer une phrase du
//! banc de dictée de Scribe et de voir ce que Whisper a entendu, ce qui serait collé, les contrôles et les temps, puis de copier un rapport.

mod assets;
mod bench_view;
mod data;
mod downloads;
mod hub;
mod sinks;
mod trial;
pub(crate) mod update;

pub use sinks::{SwitchHistory, SwitchPaster, TeeIndicator, TestSinks};
pub use update::remove_leftovers as remove_update_leftovers;

/// Les identifiants des éléments de la page Installation (l'assistant vérifie que ce qu'il demande existe).
#[cfg(test)]
pub fn download_ids() -> Vec<&'static str> {
    downloads::ITEMS.iter().map(|i| i.id).collect()
}

use crate::app::{detect_whisper_models, models_folder, App};
use crate::assistant;
use crate::overlay::OverlayModel;
use downloads::{Downloads, Progress};
use scriibe_core::bench;
use scriibe_core::bench_archive::{self, BenchRun, BenchSetup};
use scriibe_core::dictionary::{Correction, UserDictionary};
use scriibe_core::history::Cleanup;
use scriibe_core::microphone::MicLevel;
use scriibe_core::settings::{is_device_name, AppSettings, CleanupEngine, TranscriptionEngine};
use scriibe_engines::fs::RealFs;
use scriibe_engines::llama::{self, LlamaLauncher};
use scriibe_engines::runtime::{KeyKind, Log};
use scriibe_engines::server::Launcher;
use scriibe_engines::whisper::{self, WhisperLauncher};
use scriibe_platform::audio::Recording;
use scriibe_platform::bench_store::BenchStore;
use scriibe_platform::storage::FileHistory;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};
use tiny_http::{Header, Method, Request, Response, Server};

pub struct Context {
    app: Arc<App>,
    history: Arc<FileHistory>,
    bench: BenchStore,
    overlay: Arc<OverlayModel>,
    sinks: Arc<TestSinks>,
    downloads: Arc<Downloads>,
    updater: Arc<update::Updater>,
    /// Dernière version trouvée par « Chercher » : c'est elle, vue et confirmée, que « Installer » télécharge.
    found: Mutex<Option<update::Latest>>,
    token: String,
    port: u16,
    last: Mutex<Option<Recording>>,
    recording_since: Mutex<Option<Instant>>,
    /// Ce que la touche globale n'a pas pu démarrer (affiché dans l'état).
    keys_error: Mutex<Option<String>>,
    /// La machine, mesurée à la première demande de l'assistant (la recherche des cartes graphiques prend quelques secondes).
    machine: Mutex<Option<scriibe_platform::machine::MachineInfo>>,
    /// Ce que la capture de touche en cours règle : « dictation » ou « translate ».
    capture_kind: Mutex<String>,
    /// Les touches que la page dit tenues pendant une capture (la fenêtre de Scriibe ne reçoit rien du crochet quand elle a le focus).
    capture_held: Mutex<Vec<scriibe_core::hotkey::Key>>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

/// Version affichée dans la page : le numéro et le commit compilé, sinon « dev ».
pub fn build_id() -> String {
    format!("{} ({})", update::current_version(), update::current_commit().map_or("dev", |s| &s[..s.len().min(7)]))
}

/// Démarre le serveur dans un fil et rend son adresse complète.
pub fn start(
    app: Arc<App>,
    history: Arc<FileHistory>,
    sinks: Arc<TestSinks>,
    overlay: Arc<OverlayModel>,
    keys_error: Option<String>,
) -> Result<(Arc<Context>, String), String> {
    let server = Server::http("127.0.0.1:0").map_err(|e| format!("serveur local impossible : {e}"))?;
    let port = server.server_addr().to_ip().map(|a| a.port()).ok_or("port inconnu")?;
    let token = scriibe_core::modes::new_id();
    let url = format!("http://127.0.0.1:{port}/{token}/");
    let bench = BenchStore::open(BenchStore::standard_folder(app.config.folder()));
    let context = Arc::new(Context {
        app,
        history,
        bench,
        overlay,
        sinks,
        downloads: Arc::new(Downloads::default()),
        updater: Arc::new(update::Updater::default()),
        found: Mutex::new(None),
        token,
        port,
        last: Mutex::new(None),
        recording_since: Mutex::new(None),
        keys_error: Mutex::new(keys_error),
        machine: Mutex::new(None),
        capture_kind: Mutex::new("dictation".into()),
        capture_held: Mutex::new(Vec::new()),
    });
    let shared = context.clone();
    std::thread::Builder::new()
        .name("scriibe-page".into())
        .spawn(move || {
            for request in server.incoming_requests() {
                let context = shared.clone();
                // Un fil par requête : un essai de plusieurs secondes ne bloque pas le vumètre ni la progression.
                std::thread::spawn(move || handle(&context, request));
            }
        })
        .map_err(|e| e.to_string())?;
    spawn_update_checker(&context);
    Ok((context, url))
}

/// Le délai entre deux vérifications automatiques de nouvelle version.
const UPDATE_CHECK_EVERY: Duration = Duration::from_secs(6 * 3600);

/// Cherche une nouvelle version toutes les 6 heures (d'abord 30 s après le lancement), si le réglage est actif. Une requête légère vers
/// GitHub (la dernière Release publique), rien d'autre ne part ; une erreur (réseau coupé) est notée au journal et réessayée à l'heure.
fn spawn_update_checker(ctx: &Arc<Context>) {
    let ctx = ctx.clone();
    let _ = std::thread::Builder::new().name("scriibe-update".into()).spawn(move || {
        std::thread::sleep(Duration::from_secs(30));
        let mut last: Option<Instant> = None;
        loop {
            if ctx.app.config.settings().check_updates && last.is_none_or(|t| t.elapsed() >= UPDATE_CHECK_EVERY) {
                last = Some(Instant::now());
                match update::latest() {
                    Ok(latest) => {
                        let found = latest.is_newer().then(|| latest.available());
                        if let Some(v) = &found {
                            ctx.app.log.write(&format!("mise à jour: nouvelle version disponible ({})", v.version));
                        }
                        update::set_available(found);
                    }
                    Err(e) => ctx.app.log.write(&format!("mise à jour: vérification impossible ({e})")),
                }
            }
            std::thread::sleep(Duration::from_secs(60));
        }
    });
}

fn header(name: &str, value: &str) -> Header {
    Header::from_bytes(name, value).expect("en-tête valide")
}

fn reply_json(request: Request, status: u16, body: &Value) {
    let response = Response::from_string(body.to_string())
        .with_status_code(status)
        .with_header(header("Content-Type", "application/json; charset=utf-8"))
        .with_header(header("Cache-Control", "no-store"));
    let _ = request.respond(response);
}

fn ok(request: Request, body: Value) {
    reply_json(request, 200, &body);
}

fn fail(request: Request, status: u16, message: &str) {
    reply_json(request, status, &json!({"ok": false, "error": message}));
}

fn handle(ctx: &Context, mut request: Request) {
    // Seule la page ouverte depuis cette machine, avec l'adresse secrète, peut parler au serveur.
    let host_ok = request.headers().iter().any(|h| h.field.equiv("Host") && h.value.as_str() == format!("127.0.0.1:{}", ctx.port));
    let url = request.url().to_string();
    let prefix = format!("/{}/", ctx.token);
    let Some(path) = url.strip_prefix(&prefix).map(|p| p.split('?').next().unwrap_or("").to_string()).filter(|_| host_ok) else {
        let _ = request.respond(Response::from_string("Introuvable").with_status_code(404));
        return;
    };
    let mut body = String::new();
    let _ = request.as_reader().read_to_string(&mut body);
    let input: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
    match (request.method().clone(), path.as_str()) {
        (Method::Get, "api/status") => ok(request, status(ctx)),
        // L'état de la dictée seul, léger : la ligne d'état de la barre latérale le relit chaque seconde.
        (Method::Get, "api/activity") => ok(
            request,
            json!({"recording": ctx.app.controller.is_recording(), "busy": ctx.app.controller.is_busy(), "keys_error": lock(&ctx.keys_error).clone()}),
        ),
        // Une deuxième Scriibe lancée par l'utilisateur demande de montrer cette fenêtre, puis s'arrête.
        (Method::Post, "api/instance/show") => {
            scriibe_platform::open::open(&format!("http://127.0.0.1:{}/{}/", ctx.port, ctx.token));
            ok(request, json!({"ok": true}));
        }
        (Method::Get, "api/phrases") => ok(request, phrases(ctx.app.config.settings().language)),
        (Method::Get, "api/signs") => ok(
            request,
            json!({"signs": scriibe_core::spoken_signs::ALL.iter().map(|s| json!({"said": s.said, "written": s.written})).collect::<Vec<_>>()}),
        ),
        (Method::Get, "api/trial/samples") => {
            ok(request, json!({"samples": scriibe_core::clean_test::samples_for(ctx.app.config.settings().language)}))
        }
        (Method::Post, "api/trial/sample") => ok(request, trial::sample(ctx, &input)),
        (Method::Post, "api/trial/candidate") => ok(request, trial::candidate(ctx, &input)),
        (Method::Post, "api/pick-file") => ok(request, pick_file(input["kind"].as_str().unwrap_or(""))),
        (Method::Post, "api/sounds/play") => {
            let volume = input["volume"].as_f64().unwrap_or(0.5);
            ctx.app.platform.sounds.play(input["name"].as_str().unwrap_or(""), volume);
            ok(request, json!({"ok": true}))
        }
        (Method::Post, "api/local-models") => ok(request, local_models(ctx, input["engine"].as_str().unwrap_or(""))),
        (Method::Get, "api/llama/devices") => ok(request, json!({"lines": llama_devices(ctx)})),
        (Method::Get, "api/log") => ok(request, json!({"lines": log_tail(ctx, 60)})),
        (Method::Get, "api/bench/runs") => {
            ok(request, json!({"runs": ctx.bench.runs().iter().map(bench_view::run_summary).collect::<Vec<_>>()}))
        }
        (Method::Post, "api/bench/save") => match BenchRun::from_value(&input) {
            Some(run) => {
                ctx.bench.save(run);
                ok(request, json!({"ok": true, "deleted": ctx.bench.is_deleted(input["id"].as_str().unwrap_or(""))}))
            }
            None => fail(request, 400, "passage illisible"),
        },
        (Method::Post, "api/bench/discard") => {
            ctx.bench.discard(input["id"].as_str().unwrap_or(""));
            ok(request, json!({"ok": true}))
        }
        (Method::Post, "api/bench/delete") => {
            ctx.bench.delete(input["id"].as_str().unwrap_or(""));
            ok(request, json!({"ok": true}))
        }
        (Method::Post, "api/bench/compare") => {
            let runs = ctx.bench.runs();
            let pick = |key: &str| input[key].as_str().and_then(|id| ctx.bench.get(id));
            let pair = match (pick("older"), pick("newer")) {
                (Some(older), Some(newer)) => Some((older, newer)),
                _ => bench_archive::default_pair(&runs).map(|(o, n)| (o.clone(), n.clone())),
            };
            match pair {
                Some((older, newer)) => ok(request, bench_view::comparison(&older, &newer)),
                None => fail(request, 404, "il faut deux passages pour comparer"),
            }
        }
        (Method::Post, "api/keys/capture/start") => {
            *lock(&ctx.capture_kind) = if input["kind"] == "translate" { "translate".into() } else { "dictation".into() };
            lock(&ctx.capture_held).clear();
            ctx.app.controller.start_capture();
            ok(request, json!({"ok": true}))
        }
        (Method::Post, "api/keys/capture/key") => {
            let key = input["code"].as_str().and_then(scriibe_platform::keys::key_of_web_code);
            if let Some(key) = key {
                let down = input["down"].as_bool().unwrap_or(false);
                let event = scriibe_platform::keys::translate_key(&mut lock(&ctx.capture_held), key, down);
                if let Some(event) = event {
                    ctx.app.controller.feed_capture(event);
                }
            }
            ok(request, json!({"ok": true}))
        }
        // Le clavier de la fenêtre de Scriibe, relayé par la page : le crochet du système n'y reçoit rien.
        (Method::Post, "api/keys/page") => {
            if input["release"].as_bool() == Some(true) {
                ctx.app.controller.release_page_keys();
            } else if let Some(key) = input["code"].as_str().and_then(scriibe_platform::keys::key_of_web_code) {
                ctx.app.controller.handle_page_key(key, input["down"].as_bool().unwrap_or(false));
            }
            ok(request, json!({"ok": true}))
        }
        (Method::Post, "api/keys/capture/stop") => {
            ctx.app.controller.stop_capture();
            ok(request, json!({"ok": true}))
        }
        (Method::Get, "api/keys/capture") => ok(request, capture_view(ctx)),
        (Method::Get, "api/overlay") => ok(request, overlay_view(ctx)),
        (Method::Post, "api/overlay/preview") => {
            ctx.overlay.preview();
            ok(request, json!({"ok": true}))
        }
        (Method::Get, "api/assistant") => ok(request, assistant_view(ctx)),
        (Method::Post, "api/assistant/apply") => match assistant_apply(ctx, input["chain"].as_str().unwrap_or("")) {
            Ok(()) => ok(request, json!({"ok": true})),
            Err(e) => fail(request, 400, &e),
        },
        (Method::Post, "api/assistant/skip") => match assistant_done(ctx) {
            Ok(()) => ok(request, json!({"ok": true})),
            Err(e) => fail(request, 500, &e),
        },
        (Method::Get, "api/modes") => ok(request, data::modes(&ctx.app.config.settings())),
        (Method::Post, "api/modes/choose") => match choose_mode(ctx, input["choice"].as_str().unwrap_or("")) {
            Ok(view) => ok(request, view),
            Err(e) => fail(request, 400, &e),
        },
        (Method::Get, "api/dictionary") => ok(request, dictionary_view(&ctx.app.config.dictionary())),
        (Method::Post, "api/dictionary") => match save_dictionary(ctx, &input) {
            Ok(view) => ok(request, view),
            Err(e) => fail(request, 400, &e),
        },
        (Method::Get, "api/history") => ok(request, ctx.history.read(|store, _, _| data::history(store))),
        (Method::Get, "api/stats") => ok(request, ctx.history.read(data::stats)),
        (Method::Post, "api/history/delete") => {
            let id = input["id"].as_str().unwrap_or("").to_string();
            ctx.history.update(|store, _, _| store.delete(&id));
            ok(request, json!({"ok": true}))
        }
        (Method::Post, "api/history/clear") => {
            if input["confirmed"].as_bool() != Some(true) {
                return fail(request, 400, "confirmation manquante");
            }
            ctx.history.update(|store, _, _| store.clear_history());
            ok(request, json!({"ok": true}))
        }
        (Method::Post, "api/history/preview") => {
            let (keep_text, keep_days) = history_choice(&input);
            let change = ctx.history.read(|store, calendar, now| store.preview_history_change(calendar, now, keep_text, keep_days));
            ok(request, json!({"ok": true, "purged": change.purged, "erased": change.erased}))
        }
        (Method::Post, "api/history/apply") => {
            if input["confirmed"].as_bool() != Some(true) {
                return fail(request, 400, "confirmation manquante");
            }
            let (keep_text, keep_days) = history_choice(&input);
            match apply_history_change(ctx, keep_text, keep_days) {
                Ok(()) => ok(request, json!({"ok": true})),
                Err(e) => fail(request, 500, &e),
            }
        }
        (Method::Get, "api/settings") => ok(request, json!({"settings": ctx.app.config.settings()})),
        (Method::Post, "api/settings") => match patch_settings(ctx, &input) {
            Ok(settings) => ok(request, json!({"ok": true, "settings": settings})),
            Err(e) => fail(request, 400, &e),
        },
        (Method::Post, "api/record/start") => match record_start(ctx) {
            Ok(description) => ok(request, json!({"ok": true, "description": description})),
            Err(e) => fail(request, 409, &e),
        },
        (Method::Get, "api/record/level") => {
            let seconds = lock(&ctx.recording_since).map_or(0.0, |t| t.elapsed().as_secs_f64());
            ok(request, json!({"seconds": seconds, "level": ctx.sinks.level(), "loudest": ctx.app.platform.recorder.loudest_so_far()}));
        }
        (Method::Post, "api/record/stop") => ok(request, record_stop(ctx)),
        (Method::Get, "api/record/audio") => match lock(&ctx.last).as_ref() {
            Some(recording) => {
                let _ = request.respond(
                    Response::from_data(recording.wav.clone())
                        .with_header(header("Content-Type", "audio/wav"))
                        .with_header(header("Cache-Control", "no-store")),
                );
            }
            None => fail(request, 404, "aucune prise"),
        },
        (Method::Post, "api/bench/model") => ok(request, bench_model(ctx, &input)),
        (Method::Post, "api/transcribe") => match transcribe(ctx, &input) {
            Ok(result) => ok(request, result),
            Err(e) => fail(request, 409, &e),
        },
        (Method::Get, "api/downloads") => ok(request, downloads_state(ctx)),
        (Method::Get, "api/models") => ok(request, models_state(ctx)),
        (Method::Post, "api/models/delete") => {
            let folder = models_folder(ctx.app.config.folder());
            match hub::delete_installed(&folder, std::path::Path::new(input["path"].as_str().unwrap_or(""))) {
                Ok(()) => {
                    ctx.app.log.write(&format!("modèles: {} supprimé à la demande de l'interface", input["name"].as_str().unwrap_or("?")));
                    ok(request, json!({"ok": true}))
                }
                Err(e) => fail(request, 409, &e),
            }
        }
        (Method::Post, "api/hub/look") => match hub_look(&input) {
            Ok(result) => ok(request, result),
            Err(e) => fail(request, 502, &e),
        },
        (Method::Post, "api/downloads/resolve") => {
            let id = input["id"].as_str().unwrap_or("");
            match downloads::resolve(id, ctx.app.config.folder()) {
                Ok(r) => {
                    ok(request, json!({"ok": true, "name": r.name, "size": r.size, "url": r.url, "dest": r.dest.display().to_string()}))
                }
                Err(e) => fail(request, 502, &e),
            }
        }
        (Method::Post, "api/downloads/start") => {
            let id = input["id"].as_str().unwrap_or("").to_string();
            if input["confirmed"].as_bool() != Some(true) {
                return fail(request, 400, "confirmation manquante");
            }
            // Les fichiers d'un moteur en marche ne peuvent pas être remplacés.
            if id.starts_with("llama-") {
                ctx.app.llama.stop();
            }
            let result = downloads::resolve(&id, ctx.app.config.folder())
                .and_then(|resolved| ctx.downloads.start(&id, resolved, ctx.app.log.clone() as Arc<dyn Log>));
            match result {
                Ok(()) => ok(request, json!({"ok": true})),
                Err(e) => fail(request, 502, &e),
            }
        }
        (Method::Get, "api/update") => ok(request, update_state(ctx)),
        (Method::Post, "api/keys") => {
            let kind = match input["kind"].as_str() {
                Some("deepgram") => KeyKind::Deepgram,
                Some("openrouter") => KeyKind::OpenRouter,
                _ => return fail(request, 400, "clé inconnue"),
            };
            match save_key(ctx, kind, input["value"].as_str().unwrap_or("")) {
                Ok(()) => ok(request, json!({"ok": true})),
                Err(e) => fail(request, 400, &e),
            }
        }
        (Method::Post, "api/update/check") => match check_update(ctx) {
            Ok(found) => ok(request, found),
            Err(e) => fail(request, 502, &e),
        },
        (Method::Post, "api/update/install") => {
            if input["confirmed"].as_bool() != Some(true) {
                return fail(request, 400, "confirmation manquante");
            }
            match install_update(ctx) {
                Ok(()) => ok(request, json!({"ok": true})),
                Err(e) => fail(request, 409, &e),
            }
        }
        (Method::Post, "api/quit") => {
            ok(request, json!({"ok": true}));
            ctx.app.whisper.stop();
            ctx.app.llama.stop();
            std::process::exit(0);
        }
        (Method::Get, asset) if !asset.starts_with("api/") => match assets::get(asset) {
            Some((bytes, kind)) => {
                let _ = request.respond(
                    Response::from_data(bytes.into_owned())
                        .with_header(header("Content-Type", kind))
                        .with_header(header("Cache-Control", "no-store")),
                );
            }
            None => {
                let _ = request.respond(Response::from_string("Introuvable").with_status_code(404));
            }
        },
        _ => {
            let _ = request.respond(Response::from_string("Introuvable").with_status_code(404));
        }
    }
}

fn phrases(language: scriibe_core::settings::DictationLanguage) -> Value {
    let list: Vec<Value> = bench::phrases_for(language)
        .iter()
        .map(|p| json!({"category": p.category, "text": p.text, "expect": p.expect, "checks": p.checks.iter().map(|c| c.label()).collect::<Vec<_>>()}))
        .collect();
    json!({"phrases": list})
}

fn log_tail(ctx: &Context, count: usize) -> Vec<String> {
    let text = std::fs::read_to_string(ctx.app.log.path()).unwrap_or_default();
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(count)..].iter().map(|l| (*l).to_string()).collect()
}

fn profile_json(recording: &Recording) -> Value {
    let p = scriibe_platform::audio::profile(&recording.wav);
    json!({"peak": p.peak, "clipped": p.clipped, "levels": p.levels})
}

fn verdict(level: f32) -> &'static str {
    match MicLevel::assess(level) {
        MicLevel::Silent => "muet",
        MicLevel::Weak => "faible",
        MicLevel::Ok => "correct",
    }
}

/// Les modèles que LM Studio ou Ollama proposent, avec la phrase qui dit où on en est (les messages de Scribe).
fn local_models(ctx: &Context, engine: &str) -> Value {
    let engine = match engine {
        "lmStudio" => CleanupEngine::LmStudio,
        "ollama" => CleanupEngine::Ollama,
        _ => return json!({"ok": false, "error": "moteur inconnu", "models": [], "status": ""}),
    };
    match ctx.app.cleaner.local_models(engine, &ctx.app.config.settings()) {
        Ok(models) => {
            let status = if models.is_empty() {
                format!("{} répond, mais aucun modèle n'est installé", engine.name())
            } else {
                let plural = if models.len() > 1 { "s" } else { "" };
                format!(
                    "{} en marche · {} modèle{plural} · le premier appel charge le modèle (quelques secondes)",
                    engine.name(),
                    models.len()
                )
            };
            json!({"ok": true, "models": models, "status": status})
        }
        Err(_) => {
            let status = if engine == CleanupEngine::LmStudio {
                "LM Studio ne répond pas : ouvre-le et démarre son serveur (onglet Developer)"
            } else {
                "Ollama ne répond pas : installe-le depuis ollama.com, puis lance-le"
            };
            json!({"ok": false, "models": [], "status": status})
        }
    }
}

/// « Parcourir… » : la boîte d'ouverture de fichier du système (Windows et macOS), `{"path": null}` si on annule.
fn pick_file(kind: &str) -> Value {
    #[cfg(windows)]
    {
        let (title, filter): (&str, &[(&str, &str)]) = match kind {
            "whisper" => ("Choisir un modèle Whisper (fichier ggml .bin)", &[("Modèles Whisper (*.bin)", "*.bin")]),
            _ => return json!({"ok": false, "error": "type de fichier inconnu"}),
        };
        json!({"ok": true, "path": scriibe_platform::windows::file_dialog::pick_file(title, filter)})
    }
    #[cfg(target_os = "macos")]
    {
        let (title, filter): (&str, &[(&str, &str)]) = match kind {
            "whisper" => ("Choisir un modèle Whisper (fichier ggml .bin)", &[("Modèles Whisper (*.bin)", "*.bin")]),
            _ => return json!({"ok": false, "error": "type de fichier inconnu"}),
        };
        json!({"ok": true, "path": scriibe_platform::macos::file_dialog::pick_file(title, filter)})
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = kind;
        json!({"ok": false, "error": "le choix d'un fichier n'est pas encore pris en charge sur ce système"})
    }
}

fn status(ctx: &Context) -> Value {
    let app = &ctx.app;
    let settings = app.config.settings();
    let folder = app.config.folder().to_path_buf();
    let models = models_folder(&folder);
    let fs = Arc::new(RealFs);
    let path_var = std::env::var_os("PATH");
    let whisper_binary = WhisperLauncher::new(fs.clone(), whisper::PORT, path_var.as_deref()).binary();
    let llama_binary = LlamaLauncher::new(fs.clone(), llama::PORT, None, path_var.as_deref()).binary();
    let whisper_models = detect_whisper_models(fs.as_ref(), &models);
    let llama_models = llama::detect_models(fs.as_ref(), std::slice::from_ref(&models), Some(&models));
    let key = |kind| app.platform.secrets.get(kind).is_some();
    json!({
        "version": build_id(),
        "windows": cfg!(windows),
        "os": std::env::consts::OS,
        "first_name": scriibe_platform::machine::first_name(),
        "recording": app.controller.is_recording(),
        "busy": app.controller.is_busy(),
        "keys_error": lock(&ctx.keys_error).clone(),
        "update": update::available().as_ref().map(|v| json!({"version": v.version, "title": v.title, "date": v.date})),
        "hotkey": format!("{:?}", settings.hotkey),
        "dictation_label": if settings.dictation_keys.is_empty() { scriibe_platform::keys::default_dictation_label().to_string() } else { scriibe_platform::keys::label(&settings.dictation_keys) },
        "translate_label": if settings.translate_keys.is_empty() { scriibe_platform::keys::default_translate_label().to_string() } else { scriibe_platform::keys::label(&settings.translate_keys) },
        "key_choices": scriibe_platform::keys::layout().choices().iter().map(|c| json!({"id": c.id, "label": c.label, "modifier": c.modifier})).collect::<Vec<_>>(),
        "default_dictation_label": scriibe_platform::keys::default_dictation_label(),
        "default_translate_label": scriibe_platform::keys::default_translate_label(),
        "default_dictation_id": scriibe_platform::keys::layout().default_dictation_id(&settings),
        "immediate_keys": scriibe_platform::keys::layout().immediate_ids(),
        "paths": {"data": folder.display().to_string(), "settings": app.config.settings_path().display().to_string(), "models": models.display().to_string(),
                  "log": app.log.path().display().to_string(), "engines": scriibe_engines::engines_folder().map(|p| p.display().to_string())},
        "transcription": format!("{:?}", settings.transcription_engine),
        "transcription_engine": match settings.transcription_engine { TranscriptionEngine::Whisper => "whisper", TranscriptionEngine::Deepgram => "deepgram" },
        "cleanup": match settings.cleanup_engine {
            CleanupEngine::None => "none",
            CleanupEngine::LlamaCpp => "llamaCpp",
            CleanupEngine::OpenRouter => "openRouter",
            CleanupEngine::LmStudio => "lmStudio",
            CleanupEngine::Ollama => "ollama",
        },
        "cleanup_model": settings.cleanup_model,
        "sounds": scriibe_platform::sounds::NAMES,
        "llama_log": engine_hints(&folder.join("llama-server.log")),
        "llama_device": ctx.app.llama.launcher().device(),
        "llama_idle_minutes": settings.llama_idle_minutes,
        "cleanup_label": settings.cleanup_engine.label(),
        "pause_media": settings.pause_media,
        "whisper": {"binary": whisper_binary.map(|p| p.display().to_string()), "models": whisper_models, "chosen": settings.whisper_model_path},
        "llama": {"binary": llama_binary.map(|p| p.display().to_string()), "models": llama_models, "chosen": settings.llama_cpp_model},
        "keys": {"deepgram": key(KeyKind::Deepgram), "openrouter": key(KeyKind::OpenRouter)},
        "microphones": app.platform.recorder.microphones().iter().map(|m| m.label()).collect::<Vec<_>>(),
        "microphone_list": app.platform.recorder.microphones().iter().map(|m| json!({"uid": m.uid, "label": m.label()})).collect::<Vec<_>>(),
    })
}

/// Les réglages après des changements (clés et valeurs du fichier de réglages, comme dans Scribe) : lecture tolérante du résultat.
fn merge_settings(before: &AppSettings, changes: &Value) -> Result<AppSettings, String> {
    let Some(changes) = changes.as_object() else { return Err("changements illisibles".into()) };
    let mut merged = serde_json::to_value(before).map_err(|e| e.to_string())?;
    let object = merged.as_object_mut().ok_or("réglages illisibles")?;
    for (key, value) in changes {
        if !object.contains_key(key) && !OPTIONAL_KEYS.contains(&key.as_str()) {
            return Err(format!("réglage inconnu : {key}"));
        }
        object.insert(key.clone(), value.clone());
    }
    if changes.get("llamaDevice").and_then(Value::as_str).is_some_and(|d| !is_device_name(d)) {
        return Err("appareil invalide".into());
    }
    let merged = AppSettings::from_value(&merged).ok_or_else(|| "réglages illisibles".to_string())?;
    scriibe_platform::keys::validate(&merged.dictation_keys, &merged.translate_keys)?;
    Ok(merged)
}

/// Applique des changements aux réglages, puis fait tout de suite ce qu'il faut pour `llama-server` (appareil, durée de conservation).
/// Rend les réglages enregistrés.
fn patch_settings(ctx: &Context, changes: &Value) -> Result<Value, String> {
    let before = ctx.app.config.settings();
    let after = merge_settings(&before, changes)?;
    if after.llama_device != before.llama_device {
        ctx.app.llama.launcher().set_device(&after.llama_device);
        // Le serveur redémarre à la prochaine dictée avec le nouvel appareil.
        ctx.app.llama.stop();
    }
    if after.whisper_idle_minutes != before.whisper_idle_minutes {
        ctx.app.whisper.set_idle_limit(crate::app::idle_limit(after.whisper_idle_minutes));
    }
    if after.llama_idle_minutes != before.llama_idle_minutes {
        ctx.app.llama.set_idle_limit(crate::app::idle_limit(after.llama_idle_minutes));
    }
    ctx.app.config.save_settings(&after).map_err(|e| e.to_string())?;
    serde_json::to_value(&after).map_err(|e| e.to_string())
}

/// Ce que la personne tient pendant la capture d'une touche : les touches, leur nom, et si la combinaison peut servir.
fn capture_view(ctx: &Context) -> Value {
    use scriibe_platform::keys;
    let Some(capture) = ctx.app.controller.capture() else { return json!({"active": false}) };
    let kind = lock(&ctx.capture_kind).clone();
    let labels: Vec<String> = capture.keys.iter().map(|k| keys::label_of(*k)).collect();
    let ids: Option<Vec<String>> = capture.keys.iter().map(|k| keys::id_of(*k).map(str::to_string)).collect();
    let mut error = None;
    if capture.finished {
        let settings = ctx.app.config.settings();
        error = match &ids {
            None => Some(format!("{} ne peut pas servir de touche de dictée : elle sert à taper (ou à AltGr).", labels.join(" + "))),
            Some(ids) if kind == "translate" => match ids.as_slice() {
                [one] => keys::validate(&settings.dictation_keys, std::slice::from_ref(one)).err(),
                _ => Some("une seule touche pour traduire".into()),
            },
            Some(ids) => keys::validate(ids, &settings.translate_keys).err(),
        };
    }
    json!({"active": true, "kind": kind, "finished": capture.finished, "labels": labels, "ids": ids, "error": error})
}

/// Réglages facultatifs, absents du fichier tant qu'ils n'ont pas de valeur.
const OPTIONAL_KEYS: [&str; 2] = ["historyKeepDays", "customModeID"];

/// Ce que la pastille à l'écran montre (état, niveau, alertes) avec l'apparence choisie.
fn overlay_view(ctx: &Context) -> Value {
    let settings = ctx.app.config.settings();
    let mut view = ctx.overlay.snapshot().json();
    view["os"] = json!(std::env::consts::OS);
    view["appearance"] = json!({
        "enabled": settings.overlay_enabled,
        "style": settings.capture_style,
        "position": settings.capture_position,
        "color": settings.capture_color,
        "opacity": settings.capture_opacity,
    });
    view
}

/// L'assistant de premier lancement : la machine mesurée (une fois), les chaînes proposées et celle qui est conseillée.
fn assistant_view(ctx: &Context) -> Value {
    let machine = lock(&ctx.machine).get_or_insert_with(scriibe_platform::machine::probe).clone();
    let has_key = ctx.app.platform.secrets.get(KeyKind::OpenRouter).is_some();
    json!({
        "done": ctx.app.config.settings().setup_done,
        "machine": assistant::machine_json(&machine),
        "chains": assistant::offered(&machine).into_iter().map(assistant::chain_json).collect::<Vec<_>>(),
        "recommended": assistant::recommended(&machine, has_key),
        "has_openrouter_key": has_key,
    })
}

fn assistant_apply(ctx: &Context, id: &str) -> Result<(), String> {
    let chain = assistant::chain(id).ok_or("chaîne inconnue")?;
    let mut settings = ctx.app.config.settings();
    assistant::apply(chain, &mut settings);
    ctx.app.config.save_settings(&settings).map_err(|e| e.to_string())?;
    ctx.app.log.write(&format!("assistant: chaîne « {} » appliquée", chain.id));
    Ok(())
}

/// « Garder mes réglages » : l'assistant ne se représente plus.
fn assistant_done(ctx: &Context) -> Result<(), String> {
    let mut settings = ctx.app.config.settings();
    settings.setup_done = true;
    ctx.app.config.save_settings(&settings).map_err(|e| e.to_string())
}

/// Le mode choisi : un mode livré (« formal ») ou l'identifiant d'un mode personnel.
fn choose_mode(ctx: &Context, choice: &str) -> Result<Value, String> {
    let mut settings = ctx.app.config.settings();
    settings.set_mode_choice(choice);
    if settings.mode_choice() != choice {
        return Err("mode inconnu".into());
    }
    ctx.app.config.save_settings(&settings).map_err(|e| e.to_string())?;
    Ok(data::modes(&settings))
}

fn dictionary_view(dictionary: &UserDictionary) -> Value {
    json!({
        "words": dictionary.words,
        "corrections": dictionary.corrections.iter().map(|c| json!({"heard": c.heard, "written": c.written})).collect::<Vec<_>>(),
    })
}

/// Enregistre le dictionnaire tel que la page l'envoie : mots et corrections nettoyés (espaces, lignes vides, doublons de mots).
fn clean_dictionary(input: &Value) -> UserDictionary {
    let mut words: Vec<String> = Vec::new();
    for word in input["words"].as_array().into_iter().flatten().filter_map(Value::as_str) {
        let word = word.trim();
        if !word.is_empty() && !words.iter().any(|w| w.eq_ignore_ascii_case(word)) {
            words.push(word.to_string());
        }
    }
    let corrections = input["corrections"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|c| {
            let (heard, written) = (c["heard"].as_str()?.trim(), c["written"].as_str()?.trim());
            (!heard.is_empty() && !written.is_empty()).then(|| Correction { heard: heard.to_string(), written: written.to_string() })
        })
        .collect();
    UserDictionary { words, corrections }
}

fn save_dictionary(ctx: &Context, input: &Value) -> Result<Value, String> {
    let dictionary = clean_dictionary(input);
    ctx.app.config.save_dictionary(&dictionary).map_err(|e| format!("impossible d'écrire le dictionnaire ({e})"))?;
    Ok(dictionary_view(&dictionary))
}

fn history_choice(input: &Value) -> (bool, Option<i64>) {
    (input["keep_text"].as_bool().unwrap_or(true), input["keep_days"].as_i64())
}

/// Applique « garder le texte » et la durée de conservation : l'historique est purgé ou effacé (définitif, l'aperçu a été montré),
/// puis les réglages sont enregistrés.
fn apply_history_change(ctx: &Context, keep_text: bool, keep_days: Option<i64>) -> Result<(), String> {
    let settings = ctx.history.update(|store, calendar, now| {
        store.apply_history_change(calendar, now, keep_text, keep_days);
        store.settings.clone()
    });
    ctx.app.config.save_settings(&settings).map_err(|e| e.to_string())
}

fn record_start(ctx: &Context) -> Result<String, String> {
    if ctx.app.controller.is_recording() || ctx.app.controller.is_busy() {
        return Err("une dictée est en cours avec la touche".into());
    }
    let info = ctx.app.platform.recorder.start(&ctx.app.config.settings().microphone_uid)?;
    *lock(&ctx.recording_since) = Some(Instant::now());
    Ok(info.description)
}

fn record_stop(ctx: &Context) -> Value {
    *lock(&ctx.recording_since) = None;
    let recording = ctx.app.platform.recorder.stop();
    let summary = json!({"ok": true, "duration": recording.duration, "level": recording.level, "verdict": verdict(recording.level), "gain": recording.gain,
        "profile": profile_json(&recording)});
    *lock(&ctx.last) = Some(recording);
    summary
}

/// Passe la dernière prise par la vraie chaîne, sans coller ni toucher à l'historique.
fn transcribe(ctx: &Context, input: &Value) -> Result<Value, String> {
    let recording = lock(&ctx.last).clone().ok_or("aucune prise : enregistre d'abord")?;
    if ctx.app.controller.is_recording() || ctx.app.controller.is_busy() {
        return Err("une dictée est en cours avec la touche".into());
    }
    let translate = input["translate"].as_bool().unwrap_or(false);
    ctx.sinks.begin();
    ctx.app.controller.process_recording(recording.clone(), translate, "banc d'essai");
    let captured = ctx.sinks.finish();
    let phrase = input["phrase"].as_u64().and_then(|i| bench::phrases_for(ctx.app.config.settings().language).get(i as usize));
    let pasted = captured.pasted.clone().unwrap_or_default();
    let checks: Vec<Value> = phrase
        .map(|p| p.checks.iter().map(|c| json!({"label": c.label(), "passed": !pasted.is_empty() && c.passes(&pasted)})).collect())
        .unwrap_or_default();
    let timing = captured.dictation.as_ref().and_then(|d| d.timing.as_ref()).and_then(|t| serde_json::to_value(t).ok());
    // Banc : ce que l'option « Sauter si déjà propre » aurait décidé, et ce qui serait collé sans modèle. Une phrase sautée est
    // aussi demandée au modèle (`api/bench/model`), sans rien coller, pour voir ce qu'il aurait écrit.
    let judgement = phrase
        .and(captured.dictation.as_ref())
        .map(|d| bench_view::judge(&ctx.app.config.settings(), &ctx.app.config.dictionary(), &d.raw_text, translate));
    let skipped = captured.dictation.as_ref().and_then(|d| d.timing.as_ref()).is_some_and(|t| t.outcome == Cleanup::Skipped);
    Ok(json!({
        "would_skip": judgement.as_ref().is_some_and(|j| j.would_skip),
        "without_model": judgement.as_ref().map(|j| j.without_model.clone()).unwrap_or_default(),
        "cleanup_input": judgement.as_ref().map(|j| j.cleanup_input.clone()),
        "compare": judgement.is_some() && skipped,
        "ok": captured.pasted.is_some(),
        "raw": captured.dictation.as_ref().map(|d| d.raw_text.clone()),
        "text": captured.pasted,
        "timing": timing,
        "messages": captured.messages,
        "checks": checks,
        "duration": recording.duration,
        "level": recording.level,
        "gain": recording.gain,
        "verdict": verdict(recording.level),
        "profile": profile_json(&recording),
        "setup": current_setup(ctx),
    }))
}

/// Ce que le modèle aurait écrit pour une phrase du banc collée sans lui (mêmes réglages, mêmes corrections, même typographie).
fn bench_model(ctx: &Context, input: &Value) -> Value {
    let settings = ctx.app.config.settings();
    let dictionary = ctx.app.config.dictionary();
    let text = input["text"].as_str().unwrap_or("");
    let start = Instant::now();
    match ctx.app.cleaner.cleanup(text, &settings, &dictionary.words) {
        Ok(cleaned) => {
            let mut out = scriibe_core::dictionary::apply(&dictionary, &cleaned, settings.list_bullet, settings.lists_on()).text;
            if settings.language == scriibe_core::settings::DictationLanguage::Fr {
                out = scriibe_core::typography::french_typography(&out);
            }
            json!({"text": out, "seconds": start.elapsed().as_secs_f64()})
        }
        Err(e) => json!({"error": bench_view::model_error(e.message == scriibe_core::cleaner::REWRITE_REJECTED, &e.to_string())}),
    }
}

/// Les modèles que Scriibe utilise vraiment : celui des réglages, ou à défaut le premier trouvé (Whisper, mise au propre).
fn effective_models(ctx: &Context, settings: &AppSettings) -> (String, String) {
    let models = models_folder(ctx.app.config.folder());
    let fs = RealFs;
    let whisper = if settings.whisper_model_path.is_empty() {
        detect_whisper_models(&fs, &models).into_iter().next().unwrap_or_default()
    } else {
        settings.whisper_model_path.clone()
    };
    let llama_model = if settings.llama_cpp_model.is_empty() {
        llama::detect_models(&fs, std::slice::from_ref(&models), Some(&models)).into_iter().next().unwrap_or_default()
    } else {
        settings.llama_cpp_model.clone()
    };
    (whisper, llama_model)
}

/// Les modèles du dossier de l'appli, avec ce qui sert en ce moment.
fn models_state(ctx: &Context) -> Value {
    let settings = ctx.app.config.settings();
    let (whisper, llama_model) = effective_models(ctx, &settings);
    let folder = models_folder(ctx.app.config.folder());
    let same = |a: &std::path::Path, b: &str| a.to_string_lossy().eq_ignore_ascii_case(b);
    let list: Vec<Value> = hub::installed(&RealFs, &folder, |p| std::fs::metadata(p).map_or(0, |m| m.len()))
        .into_iter()
        .map(|m| {
            let used = (m.kind == "whisper" && same(&m.path, &whisper)) || (m.kind == "llama" && same(&m.path, &llama_model));
            json!({"name": m.name, "path": m.path.display().to_string(), "size": m.size, "kind": m.kind, "used": used})
        })
        .collect();
    json!({"folder": folder.display().to_string(), "models": list})
}

/// Cherche sur Hugging Face : un dépôt (adresse ou « propriétaire/nom ») donne ses fichiers, des mots donnent des dépôts.
fn hub_look(input: &Value) -> Result<Value, String> {
    let kind = hub::Kind::parse(input["kind"].as_str().unwrap_or("llama")).ok_or("genre de modèle inconnu")?;
    let sort = input["sort"].as_str().unwrap_or("popular");
    let text = input["query"].as_str().unwrap_or("").trim();
    if text.is_empty() {
        return Err("tapez un nom de dépôt (propriétaire/nom), une adresse Hugging Face ou des mots à chercher".into());
    }
    match hub::parse_query(text) {
        hub::Query::Repo { repo, file } => {
            let files: Vec<Value> = downloads::hub_list(&repo, kind)?
                .into_iter()
                .filter(|f| file.as_ref().is_none_or(|wanted| &f.name == wanted))
                .map(|f| {
                    json!({"name": f.name, "size": f.size, "note": f.note, "downloadable": f.downloadable, "id": hub::download_id(&repo, &f.name)})
                })
                .collect();
            Ok(json!({"mode": "files", "repo": repo, "kind": kind.as_str(), "files": files}))
        }
        hub::Query::Search(words) => {
            let repos: Vec<Value> = downloads::hub_search(&words, kind, sort)?
                .into_iter()
                .map(|r| json!({"repo": r.repo, "downloads": r.downloads, "likes": r.likes}))
                .collect();
            Ok(json!({"mode": "search", "repos": repos}))
        }
    }
}

/// Les réglages d'un passage du banc, d'après les réglages d'aujourd'hui et ce que Scriibe utilise vraiment (modèles trouvés).
fn current_setup(ctx: &Context) -> BenchSetup {
    let settings = ctx.app.config.settings();
    let (whisper, llama_model) = effective_models(ctx, &settings);
    let microphone = ctx
        .app
        .platform
        .recorder
        .microphones()
        .iter()
        .find(|m| m.uid == settings.microphone_uid)
        .map(|m| m.name.clone())
        .unwrap_or_default();
    bench_view::setup_for(&settings, &build_id(), &whisper, &llama_model, &microphone)
}

fn progress_json(progress: Option<&Progress>) -> Value {
    match progress {
        Some(Progress::Running { done, total }) => json!({"state": "running", "done": done, "total": total}),
        Some(Progress::Done) => json!({"state": "done"}),
        Some(Progress::Failed(e)) => json!({"state": "failed", "error": e}),
        None => json!({"state": "idle"}),
    }
}

fn downloads_state(ctx: &Context) -> Value {
    let progress = ctx.downloads.progress();
    let items: Vec<Value> = downloads::ITEMS
        .iter()
        .filter(|item| downloads::offered(item.id))
        .map(|item| {
            json!({"id": item.id, "label": item.label, "detail": item.detail, "kind": item.kind, "progress": progress_json(progress.get(item.id))})
        })
        .collect();
    // Les téléchargements choisis sur Hugging Face (`hf:<dépôt>/<fichier>`) : la page les retrouve par leur identifiant.
    let chosen: serde_json::Map<String, Value> =
        progress.iter().filter(|(id, _)| id.starts_with("hf:")).map(|(id, p)| (id.clone(), progress_json(Some(p)))).collect();
    json!({"items": items, "chosen": chosen})
}

fn update_state(ctx: &Context) -> Value {
    let state = match ctx.updater.state() {
        update::State::Idle => json!({"state": "idle"}),
        update::State::Running { done, total } => json!({"state": "running", "done": done, "total": total}),
        update::State::Installed => json!({"state": "installed"}),
        update::State::Failed(e) => json!({"state": "failed", "error": e}),
    };
    json!({
        "windows": cfg!(windows),
        "mac": cfg!(target_os = "macos"),
        "installs": update::can_install(),
        "current": update::current_version(),
        "commit": update::current_commit().map(|s| s[..s.len().min(7)].to_string()),
        "repo": update::REPO,
        "progress": state,
    })
}

/// Enregistre une clé ou un jeton dans le coffre du système (elle n'est ni affichée, ni journalisée, ni renvoyée à la page).
fn save_key(ctx: &Context, kind: KeyKind, value: &str) -> Result<(), String> {
    // Un collage ramène souvent un retour à la ligne ou une espace : une clé n'en contient jamais.
    let store = scriibe_platform::save_secret(kind, value.trim())?;
    ctx.app.log.write(&format!("clés: {} enregistrée dans le {store}", scriibe_platform::secrets::account_name(kind)));
    Ok(())
}

/// Cherche la dernière version publiée, sans rien télécharger : la page montre ce qui sera installé, puis demande confirmation. Sous Linux,
/// Scriibe dit seulement s'il existe une version plus récente ; sous Windows et sur Mac il peut l'installer si la Release contient son zip.
fn check_update(ctx: &Context) -> Result<Value, String> {
    let latest = update::latest()?;
    let newer = latest.is_newer();
    update::set_available(newer.then(|| latest.available()));
    let found = json!({
        "ok": true,
        "up_to_date": !newer,
        "installable": update::can_install() && !latest.download_url.is_empty(),
        "version": latest.version,
        "title": latest.title,
        "date": latest.date,
        "size": latest.size,
        "source": format!("github.com/{}", update::REPO),
    });
    *lock(&ctx.found) = Some(latest);
    Ok(found)
}

fn install_update(ctx: &Context) -> Result<(), String> {
    let latest = lock(&ctx.found).clone().ok_or("cherche d'abord la dernière version")?;
    let app = ctx.app.clone();
    ctx.updater.start(latest, ctx.app.log.clone() as Arc<dyn Log>, move || restart(&app))
}

/// Arrête les moteurs, relance le programme (la fenêtre se rouvre) puis s'arrête.
fn restart(app: &App) {
    app.whisper.stop();
    app.llama.stop();
    // Mac : l'application se rouvre par `open`, comme un double-clic, pour que macOS lui rattache ses autorisations (micro, touche) ; un
    // programme lancé par un autre n'en a aucune. L'aide attend la fin de ce processus : `open` réactiverait sinon l'ancienne instance.
    if let Some(bundle) = update::app_bundle().filter(|_| cfg!(target_os = "macos")) {
        let script = r#"while kill -0 "$1" 2>/dev/null; do sleep 0.2; done; exec /usr/bin/open "$2""#;
        let pid = std::process::id().to_string();
        match std::process::Command::new("/bin/sh").args(["-c", script, "sh", &pid]).arg(&bundle).spawn() {
            Ok(_) => app.log.write("mise à jour: nouvelle version lancée"),
            Err(e) => app.log.write(&format!("mise à jour: relance impossible ({e}) : ouvre Scriibe à la main")),
        }
        std::process::exit(0);
    }
    let mut args: Vec<String> = std::env::args().skip(1).filter(|a| a != crate::commands::RESTARTED).collect();
    args.push(crate::commands::RESTARTED.into());
    match std::env::current_exe().and_then(|exe| std::process::Command::new(exe).args(args).spawn()) {
        Ok(_) => app.log.write("mise à jour: nouvelle version lancée"),
        Err(e) => app.log.write(&format!("mise à jour: relance impossible ({e}) : relance scriibe.exe à la main")),
    }
    std::process::exit(0);
}

/// Le journal d'un moteur : sa taille, les lignes qui disent sur quoi il tourne (carte graphique, couches déchargées) et les dernières
/// lignes, pour voir ce qu'il contient quand rien ne correspond.
fn engine_hints(path: &std::path::Path) -> Value {
    let Ok(bytes) = std::fs::read(path) else { return json!({"bytes": 0, "hints": [], "head": [], "tail": []}) };
    let tail_bytes = &bytes[bytes.len().saturating_sub(300_000)..];
    let text = String::from_utf8_lossy(tail_bytes);
    let short = |l: &str| -> String { l.trim().chars().take(180).collect() };
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let hints: Vec<String> = lines
        .iter()
        .filter(|l| {
            let l = l.to_lowercase();
            l.contains("vulkan") || l.contains("offload") || l.contains("device") || l.contains("gpu") || l.contains("backend")
        })
        .map(|l| short(l))
        .collect();
    json!({
        "bytes": bytes.len(),
        "hints": hints[hints.len().saturating_sub(8)..],
        "head": lines.iter().take(14).map(|l| short(l)).collect::<Vec<_>>(),
        "tail": lines[lines.len().saturating_sub(4)..].iter().map(|l| short(l)).collect::<Vec<_>>(),
    })
}

/// Ce que `llama-server --list-devices` dit des cartes graphiques qu'il sait utiliser (quelques lignes, 20 s au plus).
fn llama_devices(ctx: &Context) -> Vec<String> {
    let path_var = std::env::var_os("PATH");
    let Some(binary) = LlamaLauncher::new(Arc::new(RealFs), llama::PORT, None, path_var.as_deref()).binary() else {
        return vec!["llama-server introuvable".into()];
    };
    let mut command = std::process::Command::new(&binary);
    command
        .arg("--list-devices")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let (sender, receiver) = std::sync::mpsc::channel();
    match command.spawn() {
        Ok(child) => {
            std::thread::spawn(move || {
                let _ = sender.send(child.wait_with_output());
            });
        }
        Err(e) => return vec![format!("llama-server ne démarre pas ({e})")],
    }
    match receiver.recv_timeout(std::time::Duration::from_secs(20)) {
        Ok(Ok(output)) => {
            let text = format!("{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
            let lines: Vec<String> =
                text.lines().map(|l| l.trim().chars().take(180).collect::<String>()).filter(|l| !l.is_empty()).collect();
            ctx.app.log.write(&format!("llama-server --list-devices: {} lignes", lines.len()));
            lines.into_iter().take(14).collect()
        }
        Ok(Err(e)) => vec![format!("lecture impossible ({e})")],
        Err(_) => vec!["pas de réponse en 20 s".into()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_dictionary_sent_by_the_page_is_cleaned() {
        let cleaned = clean_dictionary(&json!({
            "words": ["  Scriibe ", "", "scriibe", "Tauri"],
            "corrections": [{"heard": " cri be ", "written": "Scriibe"}, {"heard": "", "written": "x"}, {"heard": "a"}],
        }));
        assert_eq!(cleaned.words, vec!["Scriibe", "Tauri"], "espaces retirés, mot vide et doublon (majuscules ignorées) écartés");
        assert_eq!(cleaned.corrections, vec![Correction { heard: "cri be".into(), written: "Scriibe".into() }]);
    }

    #[test]
    fn settings_changes_are_merged_by_key_and_unknown_or_unsafe_ones_are_refused() {
        let before = AppSettings::default();
        let after = merge_settings(&before, &json!({"pauseMedia": true, "llamaIdleMinutes": 120, "language": "en"})).unwrap();
        assert!(after.pause_media);
        assert_eq!(after.llama_idle_minutes, 120);
        assert_eq!(after.language.raw(), "en");
        assert_eq!(after.cleanup_model, before.cleanup_model, "le reste ne bouge pas");
        assert!(merge_settings(&before, &json!({"inconnu": 1})).unwrap_err().contains("inconnu"));
        assert!(merge_settings(&before, &json!({"llamaDevice": "Vulkan0; calc"})).is_err());
        assert!(merge_settings(&before, &json!([1])).is_err());
        let kept = merge_settings(&before, &json!({"historyKeepDays": 30})).unwrap();
        assert_eq!(kept.history_keep_days, Some(30), "réglage facultatif absent du fichier tant qu'il n'a pas de valeur");
        // Les touches de dictée : une combinaison valide passe, une touche inconnue ou dangereuse est refusée.
        let keys = merge_settings(&before, &json!({"dictationKeys": ["left_control", "f9"], "translateKeys": ["left_shift"]})).unwrap();
        assert_eq!((keys.dictation_keys.len(), keys.translate_keys.len()), (2, 1));
        assert!(merge_settings(&before, &json!({"dictationKeys": ["a"]})).is_err());
        if cfg!(target_os = "macos") {
            // Sur Mac : Option, Commande et Fn sont proposées, pas la touche Windows ; ⌘ ne peut pas dicter et traduire à la fois.
            assert!(merge_settings(&before, &json!({"dictationKeys": ["right_option"]})).is_ok());
            assert!(merge_settings(&before, &json!({"dictationKeys": ["fn"], "translateKeys": ["left_command"]})).is_ok());
            assert!(merge_settings(&before, &json!({"dictationKeys": ["left_win"]})).is_err());
            assert!(merge_settings(&before, &json!({"dictationKeys": ["left_command", "right_command"]})).unwrap_err().contains("⌘"));
        } else {
            // Des touches de modification seules sont permises (l'écoute attend 0,3 s) ; Maj ne peut pas dicter et traduire à la fois.
            assert!(merge_settings(&before, &json!({"dictationKeys": ["left_win"]})).is_ok());
            assert!(merge_settings(&before, &json!({"dictationKeys": ["left_shift"]})).unwrap_err().contains("Maj"));
        }
        assert!(merge_settings(&before, &json!({"dictationKeys": ["a"]})).is_err(), "une lettre ne dicte pas");
        assert!(merge_settings(&before, &json!({"dictationKeys": ["f9"], "translateKeys": ["f9"]})).is_err());
    }
}
