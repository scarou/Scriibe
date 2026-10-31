//! Clients Deepgram, Whisper et mise au propre, contre un réseau simulé.

use scriibe_core::lang::NoGuesser;
use scriibe_core::modes::RewriteMode;
use scriibe_core::settings::{AppSettings, CleanupEngine, DictationLanguage, TranscriptionEngine};
use scriibe_core::translation::TranslationLanguage;
use scriibe_engines::fs::FileSystem;
use scriibe_engines::http::{HttpRequest, HttpResponse, Method, NetworkError};
use scriibe_engines::llama::{self, LlamaLauncher};
use scriibe_engines::runtime::{KeyKind, NoLog, Secrets};
use scriibe_engines::services::{deepgram, Cleaner, Transcriber};
use scriibe_engines::testing::FakeWorld;
use scriibe_engines::whisper::{self, Whisper, WhisperLauncher};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

struct Keys {
    deepgram: Option<&'static str>,
    openrouter: Option<&'static str>,
}

impl Secrets for Keys {
    fn get(&self, kind: KeyKind) -> Option<String> {
        match kind {
            KeyKind::Deepgram => self.deepgram,
            KeyKind::OpenRouter => self.openrouter,
            KeyKind::GitHub => None,
        }
        .map(str::to_string)
    }
}

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

/// Le programme factice, au nom qu'il a sur ce système (`.exe` sous Windows), dans un dossier du PATH factice.
fn bin(name: &str) -> PathBuf {
    Path::new("/fake/bin").join(if cfg!(windows) { format!("{name}.exe") } else { name.to_string() })
}

fn reply(content: &str) -> String {
    json!({"choices": [{"message": {"content": content}, "finish_reason": "stop"}]}).to_string()
}

fn cleaner(world: &Arc<FakeWorld>, keys: Keys) -> Cleaner {
    let model = "/m/gemma-4-E4B-it-Q4_0.gguf";
    let fs: Arc<dyn FileSystem> = Arc::new(FakeFs(vec![PathBuf::from(model), bin("llama-server")]));
    let launcher = LlamaLauncher::new(fs.clone(), 8179, None, Some(&std::env::join_paths(["/fake/bin"]).unwrap()));
    Cleaner {
        http: world.clone(),
        secrets: Arc::new(keys),
        log: world.log.clone(),
        guesser: Arc::new(NoGuesser),
        llama: Some(Arc::new(llama::new_server(launcher, world.runtime()))),
        llama_models: Arc::new(move || vec![model.to_string()]),
        fs,
        warm_ups: Default::default(),
    }
}

fn no_keys() -> Keys {
    Keys { deepgram: None, openrouter: None }
}

fn lm_studio() -> AppSettings {
    AppSettings { cleanup_engine: CleanupEngine::LmStudio, ..AppSettings::default() }
}

fn body(request: &HttpRequest) -> Value {
    serde_json::from_slice(&request.body).unwrap()
}

fn header<'a>(request: &'a HttpRequest, name: &str) -> Option<&'a str> {
    request.headers.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str())
}

// MARK: Deepgram

#[test]
fn deepgram_url_uses_keyterm_for_nova_3_and_keywords_otherwise() {
    let terms = vec!["Claude Code".to_string(), "  ".to_string(), "to-do".to_string()];
    let nova3 = deepgram::url("nova-3", DictationLanguage::Fr, &terms);
    assert_eq!(
        nova3,
        "https://api.deepgram.com/v1/listen?model=nova-3&language=fr&smart_format=true&punctuate=true&keyterm=Claude%20Code&keyterm=to-do"
    );
    let nova2 = deepgram::url("nova-2", DictationLanguage::Multi, &terms);
    assert!(nova2.contains("language=multi") && nova2.contains("keywords=Claude%20Code%3A2&keywords=to-do%3A2"), "{nova2}");
    let many: Vec<String> = (0..150).map(|i| format!("mot{i}")).collect();
    assert_eq!(deepgram::url("nova-3", DictationLanguage::Fr, &many).matches("keyterm=").count(), 100, "100 termes au plus");
}

#[test]
fn deepgram_returns_the_transcript_and_falls_back_to_nova_2_on_400() {
    let world = FakeWorld::new();
    world.route("model=nova-3", |_| Ok(HttpResponse { status: 400, body: Vec::new() }));
    world.route("model=nova-2", |_| {
        Ok(HttpResponse {
            status: 200,
            body: json!({"results": {"channels": [{"alternatives": [{"transcript": "bonjour à tous"}]}]}}).to_string().into_bytes(),
        })
    });
    let text = deepgram::transcribe(world.as_ref(), b"RIFF", "secret", "nova-3", DictationLanguage::Fr, &[]).unwrap();
    assert_eq!(text, "bonjour à tous");
    let requests = world.requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(header(&requests[0], "Authorization"), Some("Token secret"));
    assert_eq!(header(&requests[0], "Content-Type"), Some("audio/wav"));
    assert_eq!(requests[0].body, b"RIFF");
}

#[test]
fn deepgram_errors_are_explained() {
    let world = FakeWorld::new();
    world.route_fixed("deepgram.com", 401, "");
    assert_eq!(
        deepgram::transcribe(world.as_ref(), b"x", "k", "nova-2", DictationLanguage::Fr, &[]).unwrap_err().message,
        "Clé Deepgram refusée"
    );
    let world = FakeWorld::new();
    assert_eq!(
        deepgram::transcribe(world.as_ref(), b"x", "k", "nova-2", DictationLanguage::Fr, &[]).unwrap_err().message,
        "Deepgram injoignable (réseau)"
    );
    let world = FakeWorld::new();
    world.route_fixed("deepgram.com", 200, "pas du json");
    assert_eq!(
        deepgram::transcribe(world.as_ref(), b"x", "k", "nova-2", DictationLanguage::Fr, &[]).unwrap_err().message,
        "Réponse Deepgram illisible"
    );
    for (status, message) in [(402, "Crédit Deepgram épuisé"), (429, "Deepgram : trop de requêtes"), (500, "Deepgram : erreur 500")] {
        let world = FakeWorld::new();
        world.route_fixed("deepgram.com", status, "");
        assert_eq!(deepgram::test(world.as_ref(), "k").unwrap_err().message, message);
    }
}

// MARK: Transcription

fn transcriber(world: &Arc<FakeWorld>, keys: Keys, models: &'static [&'static str]) -> Transcriber {
    let mut files = vec![bin("whisper-server")];
    files.extend(models.iter().map(PathBuf::from));
    let launcher = WhisperLauncher::new(Arc::new(FakeFs(files)), 8178, Some(&std::env::join_paths(["/fake/bin"]).unwrap()));
    Transcriber {
        http: world.clone(),
        secrets: Arc::new(keys),
        whisper: Arc::new(Whisper::new(launcher, world.runtime())),
        detected_whisper_models: Arc::new(move || models.iter().map(|m| m.to_string()).collect()),
    }
}

#[test]
fn deepgram_needs_a_key() {
    let world = FakeWorld::new();
    let t = transcriber(&world, no_keys(), &[]);
    assert_eq!(t.transcribe(b"x", &AppSettings::default(), &[]).unwrap_err().message, "Clé Deepgram manquante — voir Réglages");
}

#[test]
fn whisper_transcribes_through_the_local_server() {
    let world = FakeWorld::new();
    world.route("/inference", |_| {
        Ok(HttpResponse { status: 200, body: json!({"text": " [BLANK_AUDIO] Bonjour   le\nmonde [Musique] "}).to_string().into_bytes() })
    });
    let t = transcriber(&world, no_keys(), &["/m/ggml-large-v3-turbo.bin"]);
    let settings = AppSettings { transcription_engine: TranscriptionEngine::Whisper, ..AppSettings::default() };
    let text = t.transcribe(b"WAVDATA", &settings, &["Claude Code".to_string(), String::new()]).unwrap();
    assert_eq!(text, "Bonjour le monde");
    // Le serveur est lancé avec le meilleur modèle repéré, puis interrogé.
    assert_eq!(world.spawns().len(), 1);
    assert_eq!(world.spawns()[0].args[1], "/m/ggml-large-v3-turbo.bin");
    let request = world.requests().into_iter().find(|r| r.url.ends_with("/inference")).unwrap();
    assert_eq!(request.method, Method::Post);
    assert_eq!(request.url, "http://127.0.0.1:8178/inference");
    let content_type = header(&request, "Content-Type").unwrap();
    let boundary = content_type.strip_prefix("multipart/form-data; boundary=").unwrap();
    let text_body = String::from_utf8_lossy(&request.body).into_owned();
    assert!(text_body.starts_with(&format!("--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"audio.wav\"")));
    assert!(text_body.contains("WAVDATA"));
    for field in ["response_format\"\r\n\r\njson", "temperature\"\r\n\r\n0.0", "language\"\r\n\r\nfr", "prompt\"\r\n\r\nClaude Code"] {
        assert!(text_body.contains(field), "champ absent : {field}");
    }
    assert!(text_body.ends_with(&format!("--{boundary}--\r\n")));
}

/// Un WAV 16 kHz mono 16 bits de `seconds` secondes de silence.
fn wav_of(seconds: f64) -> Vec<u8> {
    let data = (seconds * 32_000.0) as u32;
    let mut out = b"RIFF".to_vec();
    out.extend((36 + data).to_le_bytes());
    out.extend(b"WAVEfmt ");
    out.extend(16u32.to_le_bytes());
    out.extend([1, 0, 1, 0]);
    out.extend(16_000u32.to_le_bytes());
    out.extend(32_000u32.to_le_bytes());
    out.extend([2, 0, 16, 0]);
    out.extend(b"data");
    out.extend(data.to_le_bytes());
    out.extend(vec![0u8; data as usize]);
    out
}

fn whisper_world() -> Arc<FakeWorld> {
    let world = FakeWorld::new();
    world.route("/inference", |_| Ok(HttpResponse { status: 200, body: json!({"text": "Bonjour"}).to_string().into_bytes() }));
    world
}

#[test]
fn a_short_recording_goes_to_the_short_server_with_a_reduced_audio_context() {
    let world = whisper_world();
    let t = transcriber(&world, no_keys(), &["/m/ggml-small.bin"]);
    let settings = AppSettings { transcription_engine: TranscriptionEngine::Whisper, ..AppSettings::default() };
    assert_eq!(t.transcribe(&wav_of(4.0), &settings, &[]).unwrap(), "Bonjour");
    let spawn = &world.spawns()[0];
    assert!(spawn.args.windows(2).any(|w| w == ["-ac", "512"]), "contexte réduit : {:?}", spawn.args);
    assert!(spawn.args.windows(2).any(|w| w == ["--port", "8177"]), "port court : {:?}", spawn.args);
    assert!(world.requests().iter().any(|r| r.url == "http://127.0.0.1:8177/inference"));
}

#[test]
fn a_long_recording_goes_to_the_normal_server_and_both_servers_stop_together() {
    let world = whisper_world();
    let t = transcriber(&world, no_keys(), &["/m/ggml-small.bin"]);
    let settings = AppSettings { transcription_engine: TranscriptionEngine::Whisper, ..AppSettings::default() };
    t.transcribe(&wav_of(12.0), &settings, &[]).unwrap();
    let spawns = world.spawns();
    assert_eq!(spawns.len(), 1, "le serveur court ne démarre pas pour une prise longue");
    assert!(!spawns[0].args.iter().any(|a| a == "-ac"), "30 s de contexte : {:?}", spawns[0].args);
    assert!(spawns[0].args.windows(2).any(|w| w == ["--port", "8178"]));
    t.transcribe(&wav_of(3.0), &settings, &[]).unwrap();
    assert_eq!(world.spawns().len(), 2, "la prise courte démarre son propre serveur");
    assert_eq!(world.live_processes(), 2);
    t.whisper.stop();
    assert_eq!(world.live_processes(), 0);
}

#[test]
fn a_recording_of_unknown_length_goes_to_the_normal_server() {
    let world = whisper_world();
    let t = transcriber(&world, no_keys(), &["/m/ggml-small.bin"]);
    let settings = AppSettings { transcription_engine: TranscriptionEngine::Whisper, ..AppSettings::default() };
    t.transcribe(b"pas un WAV", &settings, &[]).unwrap();
    assert!(world.requests().iter().any(|r| r.url == "http://127.0.0.1:8178/inference"));
}

#[test]
fn the_length_of_a_wav_is_read_from_its_header() {
    assert_eq!(whisper::wav_seconds(&wav_of(2.0)), Some(2.0));
    assert_eq!(whisper::wav_seconds(&wav_of(0.0)), Some(0.0));
    assert_eq!(whisper::wav_seconds(b"RIFF"), None);
    assert_eq!(whisper::wav_seconds(&[0u8; 100]), None);
    // Un en-tête de flux annonce une taille démesurée : seul ce qui est reçu compte.
    let mut stream = wav_of(1.0);
    stream[40..44].copy_from_slice(&u32::MAX.to_le_bytes());
    assert_eq!(whisper::wav_seconds(&stream), Some(1.0));
}

#[test]
fn whisper_language_multi_is_sent_as_auto_and_no_prompt_without_words() {
    let body = whisper::multipart_body("B", b"x", DictationLanguage::Multi, &[]);
    let text = String::from_utf8(body).unwrap();
    assert!(text.contains("name=\"language\"\r\n\r\nauto"));
    assert!(!text.contains("name=\"prompt\""));
}

#[test]
fn whisper_clean_removes_annotations() {
    assert_eq!(whisper::clean("[Musique] Salut  [rires]\ttout   le monde\n"), "Salut tout le monde");
    assert_eq!(whisper::clean("[BLANK_AUDIO]"), "");
}

// MARK: Mise au propre

#[test]
fn lm_studio_request_has_the_expected_shape() {
    let world = FakeWorld::new();
    world.route_fixed("/v1/chat/completions", 200, &reply("Tu peux me résumer ça ?"));
    let c = cleaner(&world, no_keys());
    let out = c.cleanup("euh tu peux me résumer ça", &lm_studio(), &["Claude Code".to_string()]).unwrap();
    assert_eq!(out, "Tu peux me résumer ça ?");
    let requests = world.requests();
    let r = &requests[0];
    assert_eq!(r.url, "http://127.0.0.1:1234/v1/chat/completions");
    assert_eq!(header(r, "Content-Type"), Some("application/json"));
    assert_eq!(header(r, "Authorization"), None);
    assert_eq!(header(r, "X-Title"), None);
    let b = body(r);
    assert_eq!(b["model"], "google/gemma-4-e4b");
    assert_eq!(b["temperature"], 0);
    assert_eq!(b["max_tokens"], 200, "au moins 200 jetons");
    assert_eq!(b["reasoning_effort"], "none");
    assert_eq!(b["ttl"], 1800);
    let messages = b["messages"].as_array().unwrap();
    assert_eq!(messages[0]["role"], "system");
    assert!(messages[0]["content"].as_str().unwrap().contains("Claude Code"));
    assert!(messages.last().unwrap()["content"].as_str().unwrap().starts_with("<dictée>\neuh tu peux me résumer ça\n</dictée>"));
}

#[test]
fn max_tokens_follows_a_long_dictation() {
    let world = FakeWorld::new();
    let long = "mot ".repeat(400);
    world.route("/v1/chat/completions", move |_| Ok(HttpResponse { status: 200, body: reply("Mot.").into_bytes() }));
    let c = cleaner(&world, no_keys());
    let _ = c.rewrite(&long, &lm_studio(), &[]);
    assert_eq!(body(&world.requests()[0])["max_tokens"], long.chars().count());
}

#[test]
fn openrouter_needs_a_key_and_sends_it() {
    let world = FakeWorld::new();
    world.route_fixed("/v1/chat/completions", 200, &reply("Bonjour tout le monde."));
    let settings = AppSettings::default();
    let without = cleaner(&world, no_keys());
    assert_eq!(without.cleanup("bonjour tout le monde", &settings, &[]).unwrap_err().message, "Clé OpenRouter manquante");
    let with = cleaner(&world, Keys { deepgram: None, openrouter: Some("sk-or") });
    assert_eq!(with.cleanup("bonjour tout le monde", &settings, &[]).unwrap(), "Bonjour tout le monde.");
    let r = &world.requests()[0];
    assert_eq!(r.url, "https://openrouter.ai/api/v1/chat/completions");
    assert_eq!(header(r, "Authorization"), Some("Bearer sk-or"));
    assert_eq!(header(r, "X-Title"), Some("Scriibe"));
    assert_eq!(body(r)["provider"]["sort"], "latency");
    assert_eq!(body(r)["model"], "anthropic/claude-haiku-4.5");
}

#[test]
fn engine_none_and_literal_mode_never_call_a_model() {
    let world = FakeWorld::new();
    let c = cleaner(&world, no_keys());
    let none = AppSettings { cleanup_engine: CleanupEngine::None, ..AppSettings::default() };
    assert!(c.rewrite("x", &none, &[]).unwrap_err().message.starts_with("Aucun moteur de mise au propre"));
    let literal = AppSettings { rewrite_mode: RewriteMode::Literal, ..lm_studio() };
    assert_eq!(c.cleanup("tel quel", &literal, &[]).unwrap(), "tel quel");
    assert!(world.requests().is_empty());
}

#[test]
fn an_answer_is_rejected_by_the_guard() {
    let world = FakeWorld::new();
    world.route_fixed("/v1/chat/completions", 200, &reply("La capitale de l'Australie est Canberra."));
    let c = cleaner(&world, no_keys());
    let error = c.cleanup("Quelle est la capitale de l'Australie ?", &lm_studio(), &[]).unwrap_err();
    assert_eq!(error.message, "Reformulation écartée");
    assert_eq!(world.requests().len(), 1, "Simple ne réessaie pas");
}

#[test]
fn a_rejected_style_mode_is_retried_in_simple() {
    let world = FakeWorld::new();
    let calls = Arc::new(Mutex::new(0));
    let counter = calls.clone();
    world.route("/v1/chat/completions", move |_| {
        let mut n = counter.lock().unwrap();
        *n += 1;
        let content = if *n == 1 { "La capitale de l'Australie est Canberra." } else { "Quelle est la capitale de l'Australie ?" };
        Ok(HttpResponse { status: 200, body: reply(content).into_bytes() })
    });
    let c = cleaner(&world, no_keys());
    let settings = AppSettings { rewrite_mode: RewriteMode::Formal, ..lm_studio() };
    let out = c.cleanup("Quelle est la capitale de l'Australie ?", &settings, &[]).unwrap();
    assert_eq!(out, "Quelle est la capitale de l'Australie ?");
    let requests = world.requests();
    assert_eq!(requests.len(), 2);
    // Le second essai est en Simple : sa consigne n'a pas de phrase de style.
    let system = |i: usize| body(&requests[i])["messages"][0]["content"].as_str().unwrap().to_string();
    assert!(system(0).contains("- Style :") && !system(1).contains("- Style :"));
}

#[test]
fn playful_mode_is_cleaned_in_simple_for_now() {
    let world = FakeWorld::new();
    world.route_fixed("/v1/chat/completions", 200, &reply("Bonjour tout le monde."));
    let c = cleaner(&world, no_keys());
    let settings = AppSettings { rewrite_mode: RewriteMode::Playful, ..lm_studio() };
    assert_eq!(c.cleanup("bonjour tout le monde", &settings, &[]).unwrap(), "Bonjour tout le monde.");
    assert!(!body(&world.requests()[0])["messages"][0]["content"].as_str().unwrap().contains("- Style :"));
}

#[test]
fn a_truncated_or_unreadable_reply_is_an_error() {
    let world = FakeWorld::new();
    let c = cleaner(&world, no_keys());
    world.route_fixed(
        "/v1/chat/completions",
        200,
        &json!({"choices": [{"message": {"content": "Bonj"}, "finish_reason": "length"}]}).to_string(),
    );
    assert_eq!(
        c.rewrite("x", &lm_studio(), &[]).unwrap_err().message,
        "Réponse LM Studio coupée (dictée trop longue, ou modèle qui boucle)"
    );
    let world = FakeWorld::new();
    let c = cleaner(&world, no_keys());
    world.route_fixed("/v1/chat/completions", 200, "{}");
    assert_eq!(c.rewrite("x", &lm_studio(), &[]).unwrap_err().message, "Réponse LM Studio illisible");
}

#[test]
fn service_errors_are_explained_in_french() {
    let world = FakeWorld::new();
    let c = cleaner(&world, no_keys());
    assert_eq!(c.rewrite("x", &lm_studio(), &[]).unwrap_err().message, "LM Studio ne répond pas — lance-le sur cet ordinateur");
    let ollama = AppSettings { cleanup_engine: CleanupEngine::Ollama, ollama_model: "qwen".into(), ..AppSettings::default() };
    assert_eq!(c.rewrite("x", &ollama, &[]).unwrap_err().message, "Ollama ne répond pas — lance-le sur cet ordinateur");
    let no_model = AppSettings { cleanup_engine: CleanupEngine::Ollama, ..AppSettings::default() };
    assert_eq!(c.rewrite("x", &no_model, &[]).unwrap_err().message, "Choisis un modèle Ollama dans Réglages");
    world.route_fixed("/v1/chat/completions", 401, "");
    assert_eq!(c.rewrite("x", &lm_studio(), &[]).unwrap_err().message, "Clé LM Studio refusée");
}

#[test]
fn llama_cpp_starts_its_server_and_sends_the_file_name() {
    let world = FakeWorld::new();
    world.route_fixed("/v1/chat/completions", 200, &reply("Bonjour tout le monde."));
    let c = cleaner(&world, no_keys());
    let settings = AppSettings { cleanup_engine: CleanupEngine::LlamaCpp, ..AppSettings::default() };
    // Aucun modèle choisi : le premier repéré sert.
    assert_eq!(c.cleanup("bonjour tout le monde", &settings, &[]).unwrap(), "Bonjour tout le monde.");
    assert_eq!(world.spawns().len(), 1);
    let request = world.requests().into_iter().find(|r| r.url.ends_with("/v1/chat/completions")).unwrap();
    assert_eq!(request.url, "http://127.0.0.1:8179/v1/chat/completions");
    assert_eq!(body(&request)["model"], "gemma-4-E4B-it-Q4_0.gguf");
    assert_eq!(body(&request)["reasoning_effort"], "none");
    assert_eq!(body(&request).get("ttl"), None);
}

#[test]
fn translation_goes_through_the_guard() {
    let world = FakeWorld::new();
    let c = cleaner(&world, no_keys());
    world.route_fixed("/v1/chat/completions", 200, &reply("Hello everyone, I'll be there tomorrow."));
    let ok = c.translate("bonjour à tous je serai là demain", TranslationLanguage::En, &lm_studio(), &[]).unwrap();
    assert_eq!(ok, "Hello everyone, I'll be there tomorrow.");
    let sent = body(&world.requests()[0]);
    assert!(sent["messages"][0]["content"].as_str().unwrap().contains("Tu es un traducteur de dictée vocale"));
    world.route_fixed("/v1/chat/completions", 200, &reply("Sure, I'll remind you on Monday."));
    let error = c.translate("rappelle-moi d'appeler le docteur Martin lundi", TranslationLanguage::En, &lm_studio(), &[]).unwrap_err();
    assert_eq!(error.message, "Traduction écartée");
}

#[test]
fn warm_up_sends_the_real_prompt_with_one_token() {
    let world = FakeWorld::new();
    world.route_fixed("/v1/chat/completions", 200, &reply("."));
    let c = cleaner(&world, no_keys());
    c.warm_up(&lm_studio(), false, &["Scribe".to_string()], None);
    let b = body(&world.requests()[0]);
    assert_eq!(b["max_tokens"], 1);
    assert!(b["messages"][0]["content"].as_str().unwrap().contains("Scribe"));
    assert!(b["messages"].as_array().unwrap().last().unwrap()["content"].as_str().unwrap().contains("Bonjour."));
}

#[test]
fn warm_up_is_skipped_when_no_model_is_needed_and_is_a_head_for_the_cloud() {
    let world = FakeWorld::new();
    let c = cleaner(&world, no_keys());
    let literal = AppSettings { rewrite_mode: RewriteMode::Literal, ..lm_studio() };
    c.warm_up(&literal, false, &[], None);
    assert!(world.requests().is_empty());
    c.warm_up(&literal, true, &[], None);
    assert_eq!(world.requests().len(), 1, "une traduction force le préchauffage");

    let world = FakeWorld::new();
    let cloud = cleaner(&world, Keys { deepgram: None, openrouter: Some("k") });
    cloud.warm_up(&AppSettings::default(), false, &[], None);
    let r = &world.requests()[0];
    assert_eq!((r.method.clone(), r.url.as_str()), (Method::Head, "https://openrouter.ai/api/v1/models"));
}

#[test]
fn local_models_are_listed_without_embeddings_and_sorted() {
    let world = FakeWorld::new();
    world.route_fixed("/v1/models", 200, &json!({"data": [{"id": "zeta"}, {"id": "text-embedding-nomic"}, {"id": "alpha"}]}).to_string());
    let c = cleaner(&world, no_keys());
    assert_eq!(c.local_models(CleanupEngine::LmStudio, &AppSettings::default()).unwrap(), vec!["alpha", "zeta"]);
    assert_eq!(c.local_models(CleanupEngine::LlamaCpp, &AppSettings::default()).unwrap(), vec!["/m/gemma-4-E4B-it-Q4_0.gguf"]);
    assert!(c.local_models(CleanupEngine::OpenRouter, &AppSettings::default()).unwrap().is_empty());
    let down = FakeWorld::new();
    assert!(cleaner(&down, no_keys()).local_models(CleanupEngine::Ollama, &AppSettings::default()).is_err());
}

#[test]
fn network_error_is_not_confused_with_an_http_error() {
    let world = FakeWorld::new();
    world.route("/v1/chat/completions", |_| Err(NetworkError("coupé".into())));
    let c = cleaner(&world, no_keys());
    let _ = NoLog;
    assert_eq!(c.rewrite("x", &lm_studio(), &[]).unwrap_err().status, None);
}
