//! Réglages de l'application (port de `AppSettings` et des énumérations de `Models.swift`, Scribe).
//!
//! Le format du fichier est celui de Scribe : mêmes clés, mêmes valeurs. La lecture est tolérante : un réglage absent ou illisible
//! garde sa valeur par défaut, pour qu'un fichier ancien, abîmé ou écrit par une version future ne fasse jamais perdre les réglages.

use crate::modes::{self, CustomMode, ModeProfile, RewriteMode};
use crate::retention;
use crate::spoken_signs::ListBullet;
use crate::translation::TranslationLanguage;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// Touche à maintenir pour dicter. Les valeurs sont celles de Scribe (macOS), plus Ctrl droit pour Windows et Linux.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Hotkey {
    #[default]
    #[serde(rename = "rightOption")]
    RightOption,
    #[serde(rename = "leftOption")]
    LeftOption,
    #[serde(rename = "rightCommand")]
    RightCommand,
    #[serde(rename = "fn")]
    Fn,
    /// Ctrl droit (Windows et Linux).
    #[serde(rename = "rightControl")]
    RightControl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DictationLanguage {
    #[default]
    Fr,
    En,
    Multi,
}

impl DictationLanguage {
    /// Valeur écrite dans les réglages et envoyée aux services (« fr », « en », « multi »).
    pub fn raw(self) -> &'static str {
        match self {
            Self::Fr => "fr",
            Self::En => "en",
            Self::Multi => "multi",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TranscriptionEngine {
    #[default]
    Deepgram,
    Whisper,
}

impl TranscriptionEngine {
    pub fn is_local(self) -> bool {
        self == Self::Whisper
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CleanupEngine {
    #[default]
    #[serde(rename = "openRouter")]
    OpenRouter,
    #[serde(rename = "lmStudio")]
    LmStudio,
    #[serde(rename = "llamaCpp")]
    LlamaCpp,
    #[serde(rename = "ollama")]
    Ollama,
    /// Aucun moteur : le texte est transcrit tel quel (mode Littéral), les modes et la traduction sont indisponibles.
    #[serde(rename = "none")]
    None,
}

impl CleanupEngine {
    pub const ALL: [CleanupEngine; 5] = [Self::OpenRouter, Self::LmStudio, Self::LlamaCpp, Self::Ollama, Self::None];

    pub fn name(self) -> &'static str {
        match self {
            Self::OpenRouter => "OpenRouter",
            Self::LmStudio => "LM Studio",
            Self::LlamaCpp => "llama.cpp",
            Self::Ollama => "Ollama",
            Self::None => "Aucun",
        }
    }

    pub fn is_local(self) -> bool {
        self != Self::OpenRouter
    }

    /// Les moteurs qu'on peut comparer ou essayer (tous sauf « Aucun »).
    pub fn with_model() -> Vec<CleanupEngine> {
        Self::ALL.into_iter().filter(|e| *e != Self::None).collect()
    }

    pub fn label(self) -> String {
        match self {
            Self::None => "Aucun (transcription littérale)".to_string(),
            e if e.is_local() => format!("{} (sur cet ordinateur)", e.name()),
            e => format!("{} (cloud)", e.name()),
        }
    }

    /// Le modèle tel qu'on l'affiche : pour llama.cpp, l'identifiant est le chemin du fichier GGUF, on n'en montre que le nom.
    pub fn display_model(self, id: &str) -> String {
        if self == Self::LlamaCpp {
            id.rsplit(['/', '\\']).next().unwrap_or(id).to_string()
        } else {
            id.to_string()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CaptureStyle {
    #[default]
    Pill,
    Dot,
    Meter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CapturePosition {
    Left,
    #[default]
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CaptureColor {
    #[default]
    Scribe,
    Gray,
    Red,
    Blue,
    Black,
}

/// L'apparence de la fenêtre : claire, sombre, ou celle du système (`Auto`, décidée par la page).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Auto,
    Light,
    Dark,
}

/// La langue des textes de l'interface (pas celle de la dictée ni de la traduction). `Auto` suit celle du système, décidée par la page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UiLanguage {
    #[default]
    Auto,
    Fr,
    En,
}

impl UiLanguage {
    /// La langue réellement affichée : le choix, ou (en automatique) celle du système si c'est le français, sinon l'anglais.
    /// `system` : le nom de la langue du système (« fr-FR », « en_US.UTF-8 »…).
    pub fn resolve(self, system: &str) -> UiLang {
        match self {
            Self::Fr => UiLang::Fr,
            Self::En => UiLang::En,
            Self::Auto if system.trim().to_lowercase().starts_with("fr") => UiLang::Fr,
            Self::Auto => UiLang::En,
        }
    }
}

/// La langue des textes de l'interface, une fois le choix « automatique » résolu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiLang {
    Fr,
    En,
}

/// Un modèle à comparer dans le banc d'essai de la mise au propre.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrialCandidate {
    pub id: String,
    pub engine: CleanupEngine,
    pub model: String,
}

impl TrialCandidate {
    fn new(engine: CleanupEngine, model: &str) -> Self {
        Self { id: modes::new_id(), engine, model: model.into() }
    }

    fn from_value(v: &Value) -> Option<Self> {
        let o = v.as_object()?;
        Some(Self {
            id: modes::parse_id(o.get("id")).unwrap_or_else(modes::new_id),
            engine: serde_json::from_value(o.get("engine")?.clone()).ok()?,
            model: o.get("model").and_then(Value::as_str).unwrap_or("").to_string(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AppSettings {
    pub hotkey: Hotkey,
    pub language: DictationLanguage,
    #[serde(rename = "transcriptionEngine")]
    pub transcription_engine: TranscriptionEngine,
    #[serde(rename = "transcriptionModel")]
    pub transcription_model: String,
    #[serde(rename = "whisperModelPath")]
    pub whisper_model_path: String,
    #[serde(rename = "startSoundName")]
    pub start_sound_name: String,
    #[serde(rename = "endSoundName")]
    pub end_sound_name: String,
    #[serde(rename = "soundVolume")]
    pub sound_volume: f64,
    #[serde(rename = "restoreClipboard")]
    pub restore_clipboard: bool,
    /// Met en pause le média qui joue (YouTube, Musique…) pendant la dictée et le relance ensuite.
    #[serde(rename = "pauseMedia")]
    pub pause_media: bool,
    #[serde(rename = "trialText")]
    pub trial_text: String,
    #[serde(rename = "trialCandidates")]
    pub trial_candidates: Vec<TrialCandidate>,
    #[serde(rename = "cleanupEnabled")]
    pub cleanup_enabled: bool,
    #[serde(rename = "rewriteMode")]
    pub rewrite_mode: RewriteMode,
    /// Phrase de style modifiée, par mode livré (vide : celle d'origine).
    #[serde(rename = "styleOverrides")]
    pub style_overrides: BTreeMap<String, String>,
    /// Modes créés dans Réglages → Les modes.
    #[serde(rename = "customModes")]
    pub custom_modes: Vec<CustomMode>,
    /// Mode personnel choisi (`None` : le mode livré `rewrite_mode`).
    #[serde(rename = "customModeID", skip_serializing_if = "Option::is_none")]
    pub custom_mode_id: Option<String>,
    /// Simple et Amusant : texte de Whisper déjà propre collé sans modèle.
    #[serde(rename = "skipWhenClean")]
    pub skip_when_clean: bool,
    /// Micro choisi (vide : celui du système).
    #[serde(rename = "microphoneUID")]
    pub microphone_uid: String,
    /// Son nom, pour l'afficher quand il est débranché.
    #[serde(rename = "microphoneName")]
    pub microphone_name: String,
    #[serde(rename = "translationLanguage")]
    pub translation_language: TranslationLanguage,
    #[serde(rename = "captureStyle")]
    pub capture_style: CaptureStyle,
    #[serde(rename = "capturePosition")]
    pub capture_position: CapturePosition,
    #[serde(rename = "captureColor")]
    pub capture_color: CaptureColor,
    /// De 0,3 à 1.
    #[serde(rename = "captureOpacity")]
    pub capture_opacity: f64,
    #[serde(rename = "cleanupEngine")]
    pub cleanup_engine: CleanupEngine,
    /// Modèle OpenRouter.
    #[serde(rename = "cleanupModel")]
    pub cleanup_model: String,
    #[serde(rename = "lmStudioModel")]
    pub lm_studio_model: String,
    #[serde(rename = "ollamaModel")]
    pub ollama_model: String,
    /// Chemin du fichier GGUF du moteur llama.cpp intégré (vide : le premier modèle repéré).
    #[serde(rename = "llamaCppModel")]
    pub llama_cpp_model: String,
    #[serde(rename = "lmStudioURL")]
    pub lm_studio_url: String,
    #[serde(rename = "ollamaURL")]
    pub ollama_url: String,
    #[serde(rename = "typingWPM")]
    pub typing_wpm: i64,
    /// Garder le texte des dictées dans l'historique ; faux : seule la fiche chiffrée est gardée (confidentialité).
    #[serde(rename = "keepDictationText")]
    pub keep_dictation_text: bool,
    /// Minutes sans dictée avant que `whisper-server` s'arrête ; 0 : jamais. Il revient à l'appui de la touche.
    #[serde(rename = "whisperIdleMinutes")]
    pub whisper_idle_minutes: i64,
    /// Caractère mis devant un élément de liste dicté (« tiret du lait »).
    #[serde(rename = "listBullet")]
    pub list_bullet: ListBullet,
    /// Nombre de jours d'historique gardés (aujourd'hui compris) ; `None` : tout garder.
    #[serde(rename = "historyKeepDays", skip_serializing_if = "Option::is_none")]
    pub history_keep_days: Option<i64>,
    /// Appareil imposé à `llama-server` (« Vulkan1 », « none ») ; vide : automatique. Propre à Scriibe (Scribe ignore ce champ).
    #[serde(rename = "llamaDevice")]
    pub llama_device: String,
    /// Minutes sans dictée avant de libérer `llama-server` ; 0 : tant que le programme tourne. Propre à Scriibe.
    #[serde(rename = "llamaIdleMinutes")]
    pub llama_idle_minutes: i64,
    /// L'assistant de premier lancement a été passé (appliqué ou écarté). Propre à Scriibe.
    #[serde(rename = "setupDone")]
    pub setup_done: bool,
    /// Afficher la pastille d'écoute à l'écran (en plus de l'icône de la zone de notification). Propre à Scriibe ; active par défaut depuis S05
    /// (essayée sur le PC de test : trois styles, traduction, focus conservé).
    #[serde(rename = "overlayEnabled")]
    pub overlay_enabled: bool,
    /// Touche ou combinaison de dictée choisie (identifiants de `scriibe_platform::keys`, par exemple `["left_control", "f9"]`) ; vide : la
    /// touche par défaut du système (`hotkey`). Propre à Scriibe.
    #[serde(rename = "dictationKeys")]
    pub dictation_keys: Vec<String>,
    /// Touche tenue en plus pour traduire (une seule, ou « Maj, l'une ou l'autre ») ; vide : celle par défaut (Maj). Propre à Scriibe.
    #[serde(rename = "translateKeys")]
    pub translate_keys: Vec<String>,
    /// Langue de l'interface (S08) : automatique (celle du système), français ou anglais. Propre à Scriibe.
    #[serde(rename = "uiLanguage")]
    pub ui_language: UiLanguage,
    /// Apparence de la fenêtre : celle du système (par défaut), claire ou sombre. Propre à Scriibe.
    pub theme: Theme,
    /// Chercher toutes les 6 heures s'il existe une nouvelle version (S12) ; faux (par défaut, décision de Christophe en S13) : seulement le bouton de l'onglet « Mise à jour ». Propre à Scriibe.
    #[serde(rename = "checkUpdates")]
    pub check_updates: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            hotkey: Hotkey::default(),
            language: DictationLanguage::default(),
            transcription_engine: TranscriptionEngine::default(),
            transcription_model: "nova-3".into(),
            whisper_model_path: String::new(),
            start_sound_name: "Tink".into(),
            end_sound_name: "Pop".into(),
            sound_volume: 0.5,
            restore_clipboard: true,
            pause_media: false,
            trial_text: "euh donc je voulais te dire que euh le rendez-vous avec le client c'est à 5h non pardon à 6h et faudrait que tu prépares le devis".into(),
            trial_candidates: vec![
                TrialCandidate::new(CleanupEngine::LmStudio, "google/gemma-4-e4b"),
                TrialCandidate::new(CleanupEngine::OpenRouter, "anthropic/claude-haiku-4.5"),
            ],
            cleanup_enabled: true,
            rewrite_mode: RewriteMode::Simple,
            style_overrides: BTreeMap::new(),
            custom_modes: Vec::new(),
            custom_mode_id: None,
            skip_when_clean: false,
            microphone_uid: String::new(),
            microphone_name: String::new(),
            translation_language: TranslationLanguage::En,
            capture_style: CaptureStyle::default(),
            capture_position: CapturePosition::default(),
            capture_color: CaptureColor::default(),
            capture_opacity: 1.0,
            cleanup_engine: CleanupEngine::OpenRouter,
            cleanup_model: "anthropic/claude-haiku-4.5".into(),
            lm_studio_model: "google/gemma-4-e4b".into(),
            ollama_model: String::new(),
            llama_cpp_model: String::new(),
            lm_studio_url: "http://127.0.0.1:1234".into(),
            ollama_url: "http://127.0.0.1:11434".into(),
            typing_wpm: 40,
            keep_dictation_text: true,
            whisper_idle_minutes: 10,
            list_bullet: ListBullet::Dash,
            history_keep_days: None,
            llama_device: String::new(),
            llama_idle_minutes: LLAMA_IDLE_DEFAULT,
            setup_done: false,
            overlay_enabled: true,
            dictation_keys: Vec::new(),
            translate_keys: Vec::new(),
            ui_language: UiLanguage::default(),
            theme: Theme::default(),
            check_updates: false,
        }
    }
}

/// 30 minutes, comme Scribe.
pub const LLAMA_IDLE_DEFAULT: i64 = 30;

/// Un nom d'appareil de llama.cpp (« Vulkan1 », « none ») : lettres, chiffres, « _ » ; rien d'autre ne part en option au moteur.
pub fn is_device_name(name: &str) -> bool {
    name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Durées proposées pour libérer Whisper au repos (0 : jamais).
pub const WHISPER_IDLE_CHOICES: [i64; 5] = [0, 5, 10, 30, 60];

fn get<T: DeserializeOwned>(o: &Map<String, Value>, key: &str) -> Option<T> {
    o.get(key).and_then(|v| serde_json::from_value(v.clone()).ok())
}

impl AppSettings {
    /// Lit des réglages écrits en JSON. Échoue seulement si le texte n'est pas un objet JSON.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let value: Value = serde_json::from_str(json)?;
        Self::from_value(&value).ok_or_else(|| serde::de::Error::custom("les réglages ne sont pas un objet JSON"))
    }

    /// Lecture tolérante : un réglage absent ou illisible garde sa valeur par défaut. `None` si ce n'est pas un objet.
    pub fn from_value(value: &Value) -> Option<Self> {
        let o = value.as_object()?;
        let d = Self::default();
        // Anciennes versions : de simples interrupteurs « son au début / à la fin ».
        let legacy_start: Option<bool> = get(o, "startSound");
        let legacy_end: Option<bool> = get(o, "endSound");
        let none = NO_SOUND.to_string();
        let custom_modes: Vec<CustomMode> = o
            .get("customModes")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(CustomMode::from_value).collect())
            .unwrap_or_default();
        // Un mode personnel supprimé ou illisible : retour au mode livré. Choisi, il s'appuie sur Simple.
        let custom_mode_id = modes::parse_id(o.get("customModeID")).filter(|id| custom_modes.iter().any(|m| &m.id == id));
        let rewrite_mode = if custom_mode_id.is_some() { RewriteMode::Simple } else { get(o, "rewriteMode").unwrap_or(d.rewrite_mode) };
        Some(Self {
            hotkey: get(o, "hotkey").unwrap_or(d.hotkey),
            language: get(o, "language").unwrap_or(d.language),
            transcription_engine: get(o, "transcriptionEngine").unwrap_or(d.transcription_engine),
            transcription_model: get(o, "transcriptionModel").unwrap_or(d.transcription_model),
            whisper_model_path: get(o, "whisperModelPath").unwrap_or(d.whisper_model_path),
            start_sound_name: get(o, "startSoundName").unwrap_or(if legacy_start == Some(false) {
                none.clone()
            } else {
                d.start_sound_name
            }),
            end_sound_name: get(o, "endSoundName").unwrap_or(if legacy_end == Some(false) { none } else { d.end_sound_name }),
            sound_volume: get(o, "soundVolume").unwrap_or(d.sound_volume),
            restore_clipboard: get(o, "restoreClipboard").unwrap_or(d.restore_clipboard),
            pause_media: get(o, "pauseMedia").unwrap_or(d.pause_media),
            trial_text: get(o, "trialText").unwrap_or(d.trial_text),
            trial_candidates: o
                .get("trialCandidates")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(TrialCandidate::from_value).collect())
                .unwrap_or(d.trial_candidates),
            cleanup_enabled: get(o, "cleanupEnabled").unwrap_or(d.cleanup_enabled),
            rewrite_mode,
            style_overrides: get(o, "styleOverrides").unwrap_or(d.style_overrides),
            custom_modes,
            custom_mode_id,
            skip_when_clean: get(o, "skipWhenClean").unwrap_or(d.skip_when_clean),
            microphone_uid: get(o, "microphoneUID").unwrap_or(d.microphone_uid),
            microphone_name: get(o, "microphoneName").unwrap_or(d.microphone_name),
            translation_language: get(o, "translationLanguage").unwrap_or(d.translation_language),
            capture_style: get(o, "captureStyle").unwrap_or(d.capture_style),
            capture_position: get(o, "capturePosition").unwrap_or(d.capture_position),
            capture_color: get(o, "captureColor").unwrap_or(d.capture_color),
            capture_opacity: get::<f64>(o, "captureOpacity").unwrap_or(d.capture_opacity).clamp(0.3, 1.0),
            cleanup_engine: get(o, "cleanupEngine").unwrap_or(d.cleanup_engine),
            cleanup_model: get(o, "cleanupModel").unwrap_or(d.cleanup_model),
            lm_studio_model: get(o, "lmStudioModel").unwrap_or(d.lm_studio_model),
            ollama_model: get(o, "ollamaModel").unwrap_or(d.ollama_model),
            llama_cpp_model: get(o, "llamaCppModel").unwrap_or(d.llama_cpp_model),
            lm_studio_url: get(o, "lmStudioURL").unwrap_or(d.lm_studio_url),
            ollama_url: get(o, "ollamaURL").unwrap_or(d.ollama_url),
            typing_wpm: get(o, "typingWPM").unwrap_or(d.typing_wpm),
            keep_dictation_text: get(o, "keepDictationText").unwrap_or(d.keep_dictation_text),
            // Une valeur écrite à la main (15, -5) devient la durée proposée la plus proche : le menu la connaît toujours.
            whisper_idle_minutes: get::<i64>(o, "whisperIdleMinutes")
                .and_then(|v| WHISPER_IDLE_CHOICES.into_iter().min_by_key(|c| (c - v).abs()))
                .unwrap_or(d.whisper_idle_minutes),
            list_bullet: get(o, "listBullet").unwrap_or(d.list_bullet),
            history_keep_days: get::<i64>(o, "historyKeepDays").map(retention::clamped),
            llama_device: get::<String>(o, "llamaDevice").filter(|d| is_device_name(d)).unwrap_or(d.llama_device),
            llama_idle_minutes: get::<i64>(o, "llamaIdleMinutes").map_or(d.llama_idle_minutes, |m| m.clamp(0, 24 * 60)),
            setup_done: get(o, "setupDone").unwrap_or(d.setup_done),
            overlay_enabled: get(o, "overlayEnabled").unwrap_or(d.overlay_enabled),
            dictation_keys: get(o, "dictationKeys").unwrap_or(d.dictation_keys),
            translate_keys: get(o, "translateKeys").unwrap_or(d.translate_keys),
            ui_language: get(o, "uiLanguage").unwrap_or(d.ui_language),
            theme: get(o, "theme").unwrap_or(d.theme),
            check_updates: get(o, "checkUpdates").unwrap_or(d.check_updates),
        })
    }

    /// Le modèle de mise au propre du moteur choisi.
    pub fn active_cleanup_model(&self) -> &str {
        match self.cleanup_engine {
            CleanupEngine::OpenRouter => &self.cleanup_model,
            CleanupEngine::LmStudio => &self.lm_studio_model,
            CleanupEngine::Ollama => &self.ollama_model,
            CleanupEngine::LlamaCpp => &self.llama_cpp_model,
            CleanupEngine::None => "",
        }
    }

    pub fn set_active_cleanup_model(&mut self, model: &str) {
        match self.cleanup_engine {
            CleanupEngine::OpenRouter => self.cleanup_model = model.into(),
            CleanupEngine::LmStudio => self.lm_studio_model = model.into(),
            CleanupEngine::Ollama => self.ollama_model = model.into(),
            CleanupEngine::LlamaCpp => self.llama_cpp_model = model.into(),
            CleanupEngine::None => {}
        }
    }

    /// Les listes dictées (« tiret », « point 2 »…) sont du français : pas en dictée anglaise (« Point 2 is… »).
    pub fn lists_on(&self) -> bool {
        self.language != DictationLanguage::En
    }

    /// Un moteur de mise au propre est choisi (sinon : transcription littérale, modes et traduction indisponibles).
    pub fn has_cleanup_engine(&self) -> bool {
        self.cleanup_engine != CleanupEngine::None
    }

    /// La reformulation est en marche : activée par l'interrupteur « Reformuler le texte » et un moteur choisi.
    pub fn rewriting_on(&self) -> bool {
        self.cleanup_enabled && self.has_cleanup_engine()
    }

    /// Un modèle de mise au propre est appelé : reformulation activée et mode autre que Littéral.
    pub fn uses_cleanup_model(&self) -> bool {
        self.rewriting_on() && self.rewrite_mode != RewriteMode::Literal
    }

    /// L'option « Sauter si déjà propre » a un effet avec ces réglages : modèle appelé, mode Simple ou Amusant, Whisper.
    pub fn skip_applies(&self) -> bool {
        self.uses_cleanup_model() && self.active_profile().is_plain_simple() && self.transcription_engine == TranscriptionEngine::Whisper
    }

    pub fn everything_local(&self) -> bool {
        self.transcription_engine == TranscriptionEngine::Whisper && (!self.uses_cleanup_model() || self.cleanup_engine.is_local())
    }

    // MARK: Modes (extension de `Modes.swift`)

    /// Le mode personnel choisi, s'il existe encore.
    pub fn active_custom_mode(&self) -> Option<&CustomMode> {
        let id = self.custom_mode_id.as_ref()?;
        self.custom_modes.iter().find(|m| &m.id == id)
    }

    /// La phrase de style d'un mode livré telle qu'envoyée : modifiée (une ligne, finie par un point), ou d'origine si elle est vide.
    pub fn style_for(&self, mode: RewriteMode) -> Option<String> {
        if !mode.has_style() {
            return None;
        }
        self.style_overrides.get(mode.raw()).and_then(|s| ModeProfile::sentence(s)).or_else(|| mode.style().map(str::to_string))
    }

    /// La phrase de style telle que tapée (pour l'éditer sans perdre une espace ou un retour à la ligne en cours de frappe).
    pub fn typed_style(&self, mode: RewriteMode) -> String {
        self.style_overrides.get(mode.raw()).cloned().unwrap_or_else(|| mode.style().unwrap_or("").to_string())
    }

    pub fn is_style_modified(&self, mode: RewriteMode) -> bool {
        self.style_for(mode).as_deref() != mode.style()
    }

    /// Un mode livré avec sa phrase de style telle que réglée.
    pub fn profile_for(&self, mode: RewriteMode) -> ModeProfile {
        let mut p = mode.profile();
        p.style = self.style_for(mode);
        p
    }

    /// Ce qu'envoie la mise au propre : le mode personnel choisi, sinon le mode livré.
    pub fn active_profile(&self) -> ModeProfile {
        self.active_custom_mode().map(CustomMode::profile).unwrap_or_else(|| self.profile_for(self.rewrite_mode))
    }

    /// Nom du mode en cours, « (consigne modifiée) » pour un mode livré dont la phrase de style a changé.
    pub fn active_mode_label(&self) -> String {
        if let Some(custom) = self.active_custom_mode() {
            return custom.label();
        }
        if self.is_style_modified(self.rewrite_mode) {
            format!("{} (consigne modifiée)", self.rewrite_mode.label())
        } else {
            self.rewrite_mode.label().to_string()
        }
    }

    /// Le choix du mode dans un menu : valeur d'un mode livré, ou identifiant d'un mode personnel.
    pub fn mode_choice(&self) -> String {
        self.active_custom_mode().map(|m| m.id.clone()).unwrap_or_else(|| self.rewrite_mode.raw().to_string())
    }

    /// Change le mode. Un mode personnel s'appuie sur Simple : ni Littéral ni Amusant ne s'appliquent alors.
    /// Un identifiant inconnu est ignoré.
    pub fn set_mode_choice(&mut self, choice: &str) {
        if let Some(mode) = RewriteMode::from_raw(choice) {
            self.rewrite_mode = mode;
            self.custom_mode_id = None;
        } else if let Some(id) = modes::parse_id(Some(&Value::String(choice.to_string()))) {
            if self.custom_modes.iter().any(|m| m.id == id) {
                self.rewrite_mode = RewriteMode::Simple;
                self.custom_mode_id = Some(id);
            }
        }
    }
}

/// Valeur du son « aucun » dans les réglages.
pub const NO_SOUND: &str = "Aucun";
