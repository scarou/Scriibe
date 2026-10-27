//! Clients des services : Deepgram (transcription), OpenRouter, LM Studio, Ollama et llama.cpp (mise au propre, même protocole
//! « chat completions »). Port de `Services.swift` (Scribe), sans la logique de texte, qui est dans `scriibe-core`.

use crate::http::{send, Http, HttpRequest, ServiceError};
use crate::llama::{self, LlamaLauncher};
use crate::runtime::{KeyKind, Log, Secrets};
use crate::server::{EngineServer, Launcher};
use crate::whisper::{self, Whisper};
use scriibe_core::cleaner::{self, Message};
use scriibe_core::lang::LanguageGuesser;
use scriibe_core::modes::RewriteMode;
use scriibe_core::settings::{AppSettings, CleanupEngine, DictationLanguage, TranscriptionEngine};
use scriibe_core::translation::TranslationLanguage;
use serde_json::{json, Map, Value};
use std::sync::Arc;
use std::time::Duration;

// MARK: Deepgram (transcription)

/// Encode un morceau d'adresse (tout sauf les caractères non réservés).
fn percent_encode(text: &str) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => out.push(byte as char),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

pub mod deepgram {
    use super::*;

    /// Transcrit avec Deepgram ; si le modèle ou la langue n'est pas pris en charge (400), retombe sur `nova-2`, qui gère le français
    /// depuis longtemps.
    pub fn transcribe(
        http: &dyn Http,
        wav: &[u8],
        key: &str,
        model: &str,
        language: DictationLanguage,
        keyterms: &[String],
    ) -> Result<String, ServiceError> {
        match request(http, wav, key, model, language, keyterms) {
            Err(e) if e.status == Some(400) && model != "nova-2" => request(http, wav, key, "nova-2", language, keyterms),
            other => other,
        }
    }

    pub fn url(model: &str, language: DictationLanguage, keyterms: &[String]) -> String {
        let mut items = vec![
            ("model".to_string(), model.to_string()),
            ("language".to_string(), language.raw().to_string()),
            ("smart_format".to_string(), "true".to_string()),
            ("punctuate".to_string(), "true".to_string()),
        ];
        let terms = keyterms.iter().map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).take(100);
        if model.starts_with("nova-3") {
            items.extend(terms.map(|t| ("keyterm".to_string(), t)));
        } else {
            items.extend(terms.map(|t| ("keywords".to_string(), format!("{t}:2"))));
        }
        let query: Vec<String> = items.iter().map(|(k, v)| format!("{k}={}", percent_encode(v))).collect();
        format!("https://api.deepgram.com/v1/listen?{}", query.join("&"))
    }

    fn request(
        http: &dyn Http,
        wav: &[u8],
        key: &str,
        model: &str,
        language: DictationLanguage,
        keyterms: &[String],
    ) -> Result<String, ServiceError> {
        let request = HttpRequest::post(url(model, language, keyterms), wav.to_vec(), Duration::from_secs(30))
            .header("Authorization", format!("Token {key}"))
            .header("Content-Type", "audio/wav");
        let data = send(http, request, "Deepgram", false)?;
        let json: Value = serde_json::from_slice(&data).map_err(|_| ServiceError::new("Réponse Deepgram illisible"))?;
        json["results"]["channels"][0]["alternatives"][0]["transcript"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| ServiceError::new("Réponse Deepgram illisible"))
    }

    /// Vérifie une clé (bouton « Tester » des Réglages).
    pub fn test(http: &dyn Http, key: &str) -> Result<(), ServiceError> {
        let request = HttpRequest::get("https://api.deepgram.com/v1/projects", Duration::from_secs(15))
            .header("Authorization", format!("Token {key}"));
        send(http, request, "Deepgram", false).map(|_| ())
    }
}

pub mod openrouter {
    use super::*;

    /// Vérifie une clé (bouton « Tester » des Réglages).
    pub fn test(http: &dyn Http, key: &str) -> Result<(), ServiceError> {
        let request =
            HttpRequest::get("https://openrouter.ai/api/v1/key", Duration::from_secs(15)).header("Authorization", format!("Bearer {key}"));
        send(http, request, "OpenRouter", false).map(|_| ())
    }
}

// MARK: Transcription (Deepgram ou Whisper local)

pub struct Transcriber {
    pub http: Arc<dyn Http>,
    pub secrets: Arc<dyn Secrets>,
    pub whisper: Arc<Whisper>,
    /// Modèles Whisper repérés sur l'ordinateur (le meilleur d'abord), pour le choix par défaut.
    pub detected_whisper_models: Arc<dyn Fn() -> Vec<String> + Send + Sync>,
}

impl Transcriber {
    pub fn transcribe(&self, wav: &[u8], settings: &AppSettings, vocabulary: &[String]) -> Result<String, ServiceError> {
        match settings.transcription_engine {
            TranscriptionEngine::Deepgram => {
                let key = self.secrets.get(KeyKind::Deepgram).ok_or_else(|| ServiceError::new("Clé Deepgram manquante — voir Réglages"))?;
                deepgram::transcribe(self.http.as_ref(), wav, &key, &settings.transcription_model, settings.language, vocabulary)
            }
            TranscriptionEngine::Whisper => {
                let model = whisper::resolved_model(&settings.whisper_model_path, &(self.detected_whisper_models)());
                self.whisper.transcribe(wav, &model, settings.language, vocabulary, settings.whisper_idle_minutes)
            }
        }
    }
}

// MARK: Mise au propre

struct Endpoint {
    base: String,
    key: Option<String>,
    service: &'static str,
    local: bool,
    extra: Map<String, Value>,
}

fn object(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap_or_default()
}

pub struct Cleaner {
    pub http: Arc<dyn Http>,
    pub secrets: Arc<dyn Secrets>,
    pub log: Arc<dyn Log>,
    pub guesser: Arc<dyn LanguageGuesser + Send + Sync>,
    /// Le serveur llama.cpp, si ce moteur est disponible.
    pub llama: Option<Arc<EngineServer<LlamaLauncher>>>,
    /// Les fichiers GGUF repérés (le meilleur d'abord).
    pub llama_models: Arc<dyn Fn() -> Vec<String> + Send + Sync>,
    pub fs: Arc<dyn crate::fs::FileSystem>,
    /// Préchauffages en cours : la dictée les attend (voir `WarmUps`).
    pub warm_ups: WarmUps,
}

/// Compte les préchauffages en cours. Sur un processeur lent, lire le prompt prend ≈ 30 s : une dictée envoyée pendant ce temps
/// le relirait elle-même en même temps (trois lectures pour deux, et l'appel dépasse son délai). Elle attend donc la fin des
/// préchauffages, puis trouve le prompt déjà lu. Sur une machine rapide, le préchauffage est déjà fini : aucune attente.
#[derive(Default)]
pub struct WarmUps {
    running: std::sync::Mutex<usize>,
    done: std::sync::Condvar,
}

/// Plus longtemps que le délai du préchauffage (60 s) : on n'attend jamais indéfiniment.
const WARM_UP_WAIT: Duration = Duration::from_secs(65);

impl WarmUps {
    fn begin(&self) -> WarmUpGuard<'_> {
        *self.running.lock().unwrap_or_else(|e| e.into_inner()) += 1;
        WarmUpGuard(self)
    }

    /// Rend le temps passé à attendre (zéro s'il n'y avait rien en cours).
    fn wait(&self) -> Duration {
        let started = std::time::Instant::now();
        let guard = self.running.lock().unwrap_or_else(|e| e.into_inner());
        if *guard > 0 {
            let _ = self.done.wait_timeout_while(guard, WARM_UP_WAIT, |running| *running > 0);
            return started.elapsed();
        }
        Duration::ZERO
    }
}

struct WarmUpGuard<'a>(&'a WarmUps);

impl Drop for WarmUpGuard<'_> {
    fn drop(&mut self) {
        let mut running = self.0.running.lock().unwrap_or_else(|e| e.into_inner());
        *running -= 1;
        self.0.done.notify_all();
    }
}

impl Cleaner {
    fn endpoint(&self, settings: &AppSettings) -> Result<Endpoint, ServiceError> {
        Ok(match settings.cleanup_engine {
            CleanupEngine::OpenRouter => {
                let key = self.secrets.get(KeyKind::OpenRouter).ok_or_else(|| ServiceError::new("Clé OpenRouter manquante"))?;
                // Laisse OpenRouter choisir le fournisseur le plus réactif pour ce modèle.
                Endpoint {
                    base: "https://openrouter.ai/api".into(),
                    key: Some(key),
                    service: "OpenRouter",
                    local: false,
                    extra: object(json!({"provider": {"sort": "latency"}})),
                }
            }
            // reasoning_effort none : sans ça, les modèles « qui réfléchissent » (Gemma 4, Qwen 3) mettent 10 s et renvoient du vide.
            // ttl : LM Studio garde le modèle chargé 30 min après la dernière dictée.
            CleanupEngine::LmStudio => Endpoint {
                base: settings.lm_studio_url.clone(),
                key: None,
                service: "LM Studio",
                local: true,
                extra: object(json!({"reasoning_effort": "none", "ttl": 1800})),
            },
            CleanupEngine::Ollama => Endpoint {
                base: settings.ollama_url.clone(),
                key: None,
                service: "Ollama",
                local: true,
                extra: object(json!({"reasoning_effort": "none", "keep_alive": "30m"})),
            },
            CleanupEngine::None => return Err(ServiceError::new("Aucun moteur de mise au propre : choisis-en un dans Réglages → Moteurs")),
            // Le serveur est lancé par l'appli : seul le port change.
            CleanupEngine::LlamaCpp => {
                let port = self.llama.as_ref().map_or(llama::PORT, |s| s.launcher().config().port);
                Endpoint {
                    base: format!("http://127.0.0.1:{port}"),
                    key: None,
                    service: "llama.cpp",
                    local: true,
                    extra: object(json!({"reasoning_effort": "none"})),
                }
            }
        })
    }

    /// Le modèle llama.cpp à lancer : le fichier choisi, ou à défaut le premier repéré.
    pub fn resolved_llama_model(&self, settings: &AppSettings) -> String {
        llama::resolved_model(self.fs.as_ref(), &settings.llama_cpp_model, &(self.llama_models)())
    }

    /// Le modèle tel qu'on l'envoie et l'affiche (llama.cpp : le nom du fichier, pas son chemin).
    pub fn model_label(&self, settings: &AppSettings) -> String {
        if settings.cleanup_engine == CleanupEngine::LlamaCpp {
            settings.cleanup_engine.display_model(&self.resolved_llama_model(settings))
        } else {
            settings.active_cleanup_model().to_string()
        }
    }

    /// llama.cpp : le serveur est démarré (ou redémarré après une longue pause) avant l'appel ; les autres moteurs tournent déjà.
    /// `within` : une dictée n'attend pas plus longtemps un serveur qui ne démarre pas ; le préchauffage attend sans limite.
    fn prepare(&self, settings: &AppSettings, within: Option<Duration>) -> Result<(), ServiceError> {
        if settings.cleanup_engine != CleanupEngine::LlamaCpp {
            return Ok(());
        }
        let server = self.llama.as_ref().ok_or_else(|| ServiceError::new("llama.cpp n'est pas disponible sur ce système"))?;
        let path = self.resolved_llama_model(settings);
        match within {
            Some(limit) => llama::ensure_running_within(server, &path, limit),
            None => server.ensure_running(&path),
        }
    }

    fn post(&self, ep: &Endpoint, path: &str, body: Value, timeout: Duration) -> Result<Vec<u8>, ServiceError> {
        let mut request = HttpRequest::post(format!("{}{path}", ep.base), body.to_string().into_bytes(), timeout)
            .header("Content-Type", "application/json");
        if let Some(key) = &ep.key {
            request = request.header("Authorization", format!("Bearer {key}"));
        }
        if !ep.local {
            request = request.header("X-Title", crate::APP_NAME);
        }
        send(self.http.as_ref(), request, ep.service, ep.local)
    }

    fn complete(&self, messages: &[Message], length: usize, settings: &AppSettings) -> Result<String, ServiceError> {
        let ep = self.endpoint(settings)?;
        if settings.active_cleanup_model().is_empty() && settings.cleanup_engine != CleanupEngine::LlamaCpp {
            return Err(ServiceError::new(format!("Choisis un modèle {} dans Réglages", ep.service)));
        }
        let waited = self.warm_ups.wait();
        if waited >= Duration::from_secs(1) {
            self.log.write(&format!("mise au propre: attente du préchauffage {:.1} s", waited.as_secs_f64()));
        }
        self.prepare(settings, Some(Duration::from_secs(15)))?;
        let mut body = json!({
            "model": self.model_label(settings),
            "temperature": 0,
            "max_tokens": length.max(200),
            "messages": messages.iter().map(|m| json!({"role": m.role, "content": m.content})).collect::<Vec<_>>(),
        });
        if let Some(o) = body.as_object_mut() {
            o.extend(ep.extra.clone());
        }
        let data = self.post(&ep, "/v1/chat/completions", body, Duration::from_secs(if ep.local { 40 } else { 20 }))?;
        let unreadable = || ServiceError::new(format!("Réponse {} illisible", ep.service));
        let json: Value = serde_json::from_slice(&data).map_err(|_| unreadable())?;
        let choice = json["choices"].get(0).ok_or_else(unreadable)?;
        let content = choice["message"]["content"].as_str().ok_or_else(unreadable)?;
        // Sortie coupée par la limite de jetons : un texte amputé de sa fin ne doit pas passer pour une mise au propre.
        if choice["finish_reason"].as_str() == Some("length") {
            return Err(ServiceError::new(format!("Réponse {} coupée (dictée trop longue, ou modèle qui boucle)", ep.service)));
        }
        Ok(cleaner::strip(content))
    }

    /// Réponse du modèle, sans garde-fou (utilisée telle quelle pour mesurer).
    pub fn rewrite(&self, text: &str, settings: &AppSettings, vocabulary: &[String]) -> Result<String, ServiceError> {
        let messages = cleaner::messages(text, vocabulary, &settings.active_profile());
        self.complete(&messages, text.chars().count(), settings)
    }

    /// Dictée mise au propre. Littéral : aucun modèle. Un mode de style écarté par le garde-fou est retenté en Simple, plus fidèle,
    /// avant de coller le texte brut.
    ///
    /// Amusant : la mise au propre se fait en Simple ; le moteur schtroumpf n'est pas encore porté (voir `docs/ECARTS.md`).
    pub fn cleanup(&self, text: &str, settings: &AppSettings, vocabulary: &[String]) -> Result<String, ServiceError> {
        if settings.rewrite_mode == RewriteMode::Literal {
            return Ok(text.to_string());
        }
        let mut settings = settings.clone();
        if settings.rewrite_mode == RewriteMode::Playful {
            settings.rewrite_mode = RewriteMode::Simple;
        }
        let cleaned = self.rewrite(text, &settings, vocabulary)?;
        let rewrites = settings.active_profile().rewrites;
        if cleaner::accepts(text, &cleaned, rewrites) {
            return Ok(cleaned);
        }
        if rewrites {
            let mut simple = settings.clone();
            simple.rewrite_mode = RewriteMode::Simple;
            simple.custom_mode_id = None;
            let retry = self.rewrite(text, &simple, vocabulary)?;
            if cleaner::accepts_for_mode(text, &retry, RewriteMode::Simple) {
                return Ok(retry);
            }
        }
        Err(ServiceError::new(cleaner::REWRITE_REJECTED))
    }

    /// Réponse du modèle à la demande de traduction, sans garde-fou.
    pub fn translation(
        &self,
        text: &str,
        language: TranslationLanguage,
        settings: &AppSettings,
        vocabulary: &[String],
    ) -> Result<String, ServiceError> {
        let messages = cleaner::translation_messages(text, language, vocabulary);
        self.complete(&messages, text.chars().count(), settings)
    }

    /// Dictée mise au propre et traduite en un seul appel, puis passée au garde-fou de la traduction. Le mode de reformulation ne
    /// s'applique pas : la traduction se fait toujours dans l'esprit du mode Simple.
    pub fn translate(
        &self,
        text: &str,
        language: TranslationLanguage,
        settings: &AppSettings,
        vocabulary: &[String],
    ) -> Result<String, ServiceError> {
        let out = self.translation(text, language, settings, vocabulary)?;
        if cleaner::accepts_translation(text, &out, language, self.guesser.as_ref()) {
            Ok(out)
        } else {
            Err(ServiceError::new(cleaner::TRANSLATION_REJECTED))
        }
    }

    /// Prépare le moteur pendant que la personne parle : charge le modèle local, ou ouvre la connexion au cloud. Le préchauffage envoie
    /// le vrai prompt (consigne, exemples, mots du dictionnaire, une dictée factice) : le modèle le lit à ce moment-là, et la vraie
    /// dictée n'a plus que sa propre phrase à lire. `force` : une traduction appelle le modèle même en mode Littéral.
    pub fn warm_up(&self, settings: &AppSettings, force: bool, vocabulary: &[String], translating: Option<TranslationLanguage>) {
        if !(settings.uses_cleanup_model() || force) {
            return;
        }
        let Ok(ep) = self.endpoint(settings) else { return };
        if ep.local {
            let _running = self.warm_ups.begin();
            if let Err(e) = self.prepare(settings, None) {
                self.log.write(&format!("préchauffage: {} pas prêt ({e})", ep.service));
                return;
            }
            let messages = cleaner::warm_up_messages(settings, vocabulary, translating);
            let mut body = json!({
                "model": self.model_label(settings),
                "max_tokens": 1,
                "messages": messages.iter().map(|m| json!({"role": m.role, "content": m.content})).collect::<Vec<_>>(),
            });
            if let Some(o) = body.as_object_mut() {
                o.extend(ep.extra.clone());
            }
            if let Err(e) = self.post(&ep, "/v1/chat/completions", body, Duration::from_secs(60)) {
                self.log.write(&format!("préchauffage: {} a refusé ou n'a pas répondu ({e})", ep.service));
            }
        } else {
            let mut request = HttpRequest::get(format!("{}/v1/models", ep.base), Duration::from_secs(5));
            request.method = crate::http::Method::Head;
            let _ = self.http.send(request);
        }
    }

    /// Modèles disponibles dans LM Studio ou Ollama (hors modèles d'« embedding »), ou fichiers GGUF pour llama.cpp.
    pub fn local_models(&self, engine: CleanupEngine, settings: &AppSettings) -> Result<Vec<String>, ServiceError> {
        let base = match engine {
            CleanupEngine::LlamaCpp => return Ok((self.llama_models)()),
            CleanupEngine::LmStudio => &settings.lm_studio_url,
            CleanupEngine::Ollama => &settings.ollama_url,
            _ => return Ok(Vec::new()),
        };
        let request = HttpRequest::get(format!("{base}/v1/models"), Duration::from_secs(3));
        let data = send(self.http.as_ref(), request, engine.name(), true)?;
        let json: Value = serde_json::from_slice(&data).unwrap_or(Value::Null);
        let mut ids: Vec<String> = json["data"]
            .as_array()
            .map(|list| {
                list.iter().filter_map(|m| m["id"].as_str().map(str::to_string)).filter(|id| !id.to_lowercase().contains("embed")).collect()
            })
            .unwrap_or_default();
        ids.sort();
        Ok(ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn a_dictation_waits_for_the_warm_up_in_progress_and_not_otherwise() {
        let warm_ups = Arc::new(WarmUps::default());
        assert_eq!(warm_ups.wait(), Duration::ZERO, "rien en cours : aucune attente");
        let background = Arc::clone(&warm_ups);
        let (started, ready) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            let _running = background.begin();
            started.send(()).unwrap();
            // Large : sous charge, le fil de test peut reprendre bien après ce signal, et il doit encore trouver le préchauffage en cours.
            std::thread::sleep(Duration::from_millis(700));
        });
        ready.recv().unwrap();
        let waited = warm_ups.wait();
        worker.join().unwrap();
        assert!(waited >= Duration::from_millis(100), "attend la fin du préchauffage ({waited:?})");
        assert_eq!(warm_ups.wait(), Duration::ZERO, "fini : plus d'attente");
    }
}
