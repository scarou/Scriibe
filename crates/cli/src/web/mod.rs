//! La page d'essais : un petit serveur sur 127.0.0.1 (port libre, adresse secrète) qui sert une page web et ses commandes. Elle permet,
//! sans taper de commande : de voir ce qui est installé, de télécharger ce qui manque (après confirmation), d'enregistrer une phrase du
//! banc de dictée de Scribe et de voir ce que Whisper a entendu, ce qui serait collé, les contrôles et les temps, puis de copier un rapport.

mod downloads;
mod sinks;
mod update;

pub use sinks::{SwitchHistory, SwitchPaster, TeeIndicator, TestSinks};
pub use update::remove_leftovers as remove_update_leftovers;

use crate::app::{detect_whisper_models, models_folder, App};
use downloads::{Downloads, Progress};
use scriibe_core::bench;
use scriibe_core::microphone::MicLevel;
use scriibe_core::settings::{CleanupEngine, TranscriptionEngine};
use scriibe_engines::fs::RealFs;
use scriibe_engines::llama::{self, LlamaLauncher};
use scriibe_engines::runtime::{KeyKind, Log};
use scriibe_engines::server::Launcher;
use scriibe_engines::whisper::{self, WhisperLauncher};
use scriibe_platform::audio::Recording;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;
use tiny_http::{Header, Method, Request, Response, Server};

const PAGE: &str = include_str!("page.html");

pub struct Context {
    app: Arc<App>,
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
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

/// Version affichée dans la page : le commit de la CI, sinon « dev ».
fn build_id() -> String {
    format!("{} ({})", env!("CARGO_PKG_VERSION"), option_env!("GITHUB_SHA").map_or("dev", |s| &s[..s.len().min(7)]))
}

/// Démarre le serveur dans un fil et rend son adresse complète.
pub fn start(app: Arc<App>, sinks: Arc<TestSinks>, keys_error: Option<String>) -> Result<(Arc<Context>, String), String> {
    let server = Server::http("127.0.0.1:0").map_err(|e| format!("serveur local impossible : {e}"))?;
    let port = server.server_addr().to_ip().map(|a| a.port()).ok_or("port inconnu")?;
    let token = scriibe_core::modes::new_id();
    let url = format!("http://127.0.0.1:{port}/{token}/");
    let context = Arc::new(Context {
        app,
        sinks,
        downloads: Arc::new(Downloads::default()),
        updater: Arc::new(update::Updater::default()),
        found: Mutex::new(None),
        token,
        port,
        last: Mutex::new(None),
        recording_since: Mutex::new(None),
        keys_error: Mutex::new(keys_error),
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
    Ok((context, url))
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
        (Method::Get, "") => {
            let _ = request.respond(Response::from_string(PAGE).with_header(header("Content-Type", "text/html; charset=utf-8")));
        }
        (Method::Get, "api/status") => ok(request, status(ctx)),
        (Method::Get, "api/phrases") => ok(request, phrases()),
        (Method::Get, "api/llama/devices") => ok(request, json!({"lines": llama_devices(ctx)})),
        (Method::Get, "api/log") => ok(request, json!({"lines": log_tail(ctx, 60)})),
        (Method::Post, "api/settings") => match save_settings(ctx, &input) {
            Ok(()) => ok(request, json!({"ok": true})),
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
        (Method::Post, "api/transcribe") => match transcribe(ctx, &input) {
            Ok(result) => ok(request, result),
            Err(e) => fail(request, 409, &e),
        },
        (Method::Get, "api/downloads") => ok(request, downloads_state(ctx)),
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
        (Method::Post, "api/update/token") => match save_key(ctx, KeyKind::GitHub, input["token"].as_str().unwrap_or("")) {
            Ok(()) => ok(request, json!({"ok": true})),
            Err(e) => fail(request, 400, &e),
        },
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
            ctx.app.whisper.server.stop();
            ctx.app.llama.stop();
            std::process::exit(0);
        }
        _ => {
            let _ = request.respond(Response::from_string("Introuvable").with_status_code(404));
        }
    }
}

fn phrases() -> Value {
    let list: Vec<Value> = bench::phrases()
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
        "recording": app.controller.is_recording(),
        "busy": app.controller.is_busy(),
        "keys_error": lock(&ctx.keys_error).clone(),
        "hotkey": format!("{:?}", settings.hotkey),
        "paths": {"data": folder.display().to_string(), "settings": app.config.settings_path().display().to_string(), "models": models.display().to_string(),
                  "log": app.log.path().display().to_string(), "engines": scriibe_engines::engines_folder().map(|p| p.display().to_string())},
        "transcription": format!("{:?}", settings.transcription_engine),
        "transcription_engine": match settings.transcription_engine { TranscriptionEngine::Whisper => "whisper", TranscriptionEngine::Deepgram => "deepgram" },
        "cleanup": match settings.cleanup_engine { CleanupEngine::None => "none", CleanupEngine::LlamaCpp => "llamaCpp", CleanupEngine::OpenRouter => "openRouter", _ => "other" },
        "cleanup_model": settings.cleanup_model,
        "llama_log": engine_hints(&folder.join("llama-server.log")),
        "llama_device": ctx.app.llama.launcher().device(),
        "cleanup_label": settings.cleanup_engine.label(),
        "pause_media": settings.pause_media,
        "whisper": {"binary": whisper_binary.map(|p| p.display().to_string()), "models": whisper_models, "chosen": settings.whisper_model_path},
        "llama": {"binary": llama_binary.map(|p| p.display().to_string()), "models": llama_models, "chosen": settings.llama_cpp_model},
        "keys": {"deepgram": key(KeyKind::Deepgram), "openrouter": key(KeyKind::OpenRouter)},
        "microphones": app.platform.recorder.microphones().iter().map(|m| m.label()).collect::<Vec<_>>(),
    })
}

fn save_settings(ctx: &Context, input: &Value) -> Result<(), String> {
    let mut settings = ctx.app.config.settings();
    if let Some(cleanup) = input["cleanup"].as_str() {
        settings.cleanup_engine = match cleanup {
            "none" => CleanupEngine::None,
            "llamaCpp" => CleanupEngine::LlamaCpp,
            "openRouter" => CleanupEngine::OpenRouter,
            other => return Err(format!("moteur inconnu : {other}")),
        };
    }
    if let Some(engine) = input["transcription"].as_str() {
        settings.transcription_engine = match engine {
            "whisper" => TranscriptionEngine::Whisper,
            "deepgram" => TranscriptionEngine::Deepgram,
            other => return Err(format!("moteur de transcription inconnu : {other}")),
        };
    }
    if let Some(model) = input["cleanup_model"].as_str().map(str::trim).filter(|m| !m.is_empty()) {
        settings.cleanup_model = model.to_string();
    }
    if let Some(device) = input["llama_device"].as_str() {
        // Caractères d'un nom d'appareil de llama.cpp (« Vulkan1 », « none ») : rien d'autre ne part en option.
        if !device.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err("appareil invalide".into());
        }
        if device != ctx.app.llama.launcher().device() {
            crate::app::write_llama_device(ctx.app.config.folder(), device)?;
            ctx.app.llama.launcher().set_device(device);
            // Le serveur redémarre à la prochaine dictée avec le nouvel appareil.
            ctx.app.llama.stop();
        }
    }
    if let Some(pause) = input["pause_media"].as_bool() {
        settings.pause_media = pause;
    }
    if let Some(path) = input["whisper_model"].as_str() {
        settings.whisper_model_path = path.to_string();
    }
    if let Some(path) = input["llama_model"].as_str() {
        settings.llama_cpp_model = path.to_string();
    }
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
    ctx.app.controller.process_recording(recording.clone(), translate, "page d'essais");
    let captured = ctx.sinks.finish();
    let phrase = input["phrase"].as_u64().and_then(|i| bench::phrases().get(i as usize));
    let pasted = captured.pasted.clone().unwrap_or_default();
    let checks: Vec<Value> = phrase
        .map(|p| p.checks.iter().map(|c| json!({"label": c.label(), "passed": !pasted.is_empty() && c.passes(&pasted)})).collect())
        .unwrap_or_default();
    let timing = captured.dictation.as_ref().and_then(|d| d.timing.as_ref()).map(|t| {
        json!({"transcription": t.transcription, "cleanup": t.cleanup, "total": t.total, "outcome": format!("{:?}", t.outcome).to_lowercase()})
    });
    Ok(json!({
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
    }))
}

fn downloads_state(ctx: &Context) -> Value {
    let progress = ctx.downloads.progress();
    let items: Vec<Value> = downloads::ITEMS
        .iter()
        .map(|item| {
            let state = match progress.get(item.id) {
                Some(Progress::Running { done, total }) => json!({"state": "running", "done": done, "total": total}),
                Some(Progress::Done) => json!({"state": "done"}),
                Some(Progress::Failed(e)) => json!({"state": "failed", "error": e}),
                None => json!({"state": "idle"}),
            };
            json!({"id": item.id, "label": item.label, "detail": item.detail, "progress": state})
        })
        .collect();
    json!({"items": items})
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
        "has_token": ctx.app.platform.secrets.get(KeyKind::GitHub).is_some(),
        "current": update::current_sha().map(|s| s[..s.len().min(7)].to_string()),
        "repo": update::REPO,
        "progress": state,
    })
}

/// Enregistre une clé ou un jeton dans le Gestionnaire d'identifiants (elle n'est ni affichée, ni journalisée, ni renvoyée à la page).
fn save_key(ctx: &Context, kind: KeyKind, value: &str) -> Result<(), String> {
    let _ = ctx;
    #[cfg(windows)]
    {
        scriibe_platform::windows::set_credential(kind, value)?;
        ctx.app
            .log
            .write(&format!("clés: {} enregistrée dans le Gestionnaire d'identifiants", scriibe_platform::secrets::account_name(kind)));
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = (kind, value);
        Err("l'enregistrement d'une clé ne marche que sous Windows".into())
    }
}

fn github_token(ctx: &Context) -> Result<String, String> {
    ctx.app.platform.secrets.get(KeyKind::GitHub).ok_or_else(|| "aucun jeton GitHub enregistré".to_string())
}

/// Cherche la dernière version, sans rien télécharger : la page montre ce qui sera installé, puis demande confirmation.
fn check_update(ctx: &Context) -> Result<Value, String> {
    let latest = update::latest(&github_token(ctx)?)?;
    let found = json!({
        "ok": true,
        "up_to_date": latest.up_to_date(),
        "sha": latest.short_sha(),
        "title": latest.title,
        "date": latest.date,
        "size": latest.size,
        "run": latest.run_id,
        "source": format!("github.com/{}", update::REPO),
    });
    *lock(&ctx.found) = Some(latest);
    Ok(found)
}

fn install_update(ctx: &Context) -> Result<(), String> {
    let latest = lock(&ctx.found).clone().ok_or("cherche d'abord la dernière version")?;
    let token = github_token(ctx)?;
    let app = ctx.app.clone();
    ctx.updater.start(token, latest, ctx.app.log.clone() as Arc<dyn Log>, move || restart(&app))
}

/// Arrête les moteurs, relance le programme (une nouvelle page d'essais s'ouvre) puis s'arrête.
fn restart(app: &App) {
    app.whisper.server.stop();
    app.llama.stop();
    match std::env::current_exe().and_then(|exe| std::process::Command::new(exe).args(std::env::args().skip(1)).spawn()) {
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
