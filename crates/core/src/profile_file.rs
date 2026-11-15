//! Le fichier d'un utilisateur entier (`kind: "profile"`, étape 5 de `docs/CADRAGE_PROFILS.md`, décisions 1, 4 et 5) : l'export, et l'import
//! qui crée toujours un **nouvel** utilisateur.
//!
//! **Ce qui part** : le nom affiché et le prénom (case décochée par défaut), les réglages de personne (langues, genre, mode choisi et
//! fidélité de Simple, modes masqués du menu, puce de liste, commandes vocales et leurs phrases), le dictionnaire, les modes
//! personnels (avec ou sans exemples), les consignes modifiées des modes livrés, les commandes perso, les sites de recherche vocale (avec
//! leur adresse, **décochés à l'import** : le fichier d'un autre ne choisit pas où s'ouvre une recherche à la voix). **Jamais** : l'historique,
//! les statistiques (ni leurs trois réglages `keepDictationText`, `historyKeepDays`, `typingWPM`), les clés d'API, le banc d'essai, ni aucun
//! champ propre à l'ordinateur (touches, micro, moteurs, apparence…).
//!
//! L'export est construit **champ par champ, d'après une liste blanche** ([`PREFERENCE_KEYS`] et les clés écrites plus bas) : un futur champ
//! de réglage ne part jamais par défaut. L'import relit le fichier avec les lecteurs des étapes 2 et 3 (mêmes limites, mêmes contrôles) et
//! les règles de collision de `import`, `import_commands` et `import_dictionary` appliquées à un utilisateur vide : identifiants toujours
//! neufs (`customModeID` et `hiddenModes` suivent la table des nouveaux identifiants), titres numérotés, raccourcis contrôlés.

use crate::commands::{CustomCommand, VoicePhrases};
use crate::dictionary::UserDictionary;
use crate::export;
use crate::import::{self, one_line, MAX_BYTES, MAX_TEXT};
use crate::import_commands::{self, MAX_PHRASE, MAX_PHRASES};
use crate::import_dictionary;
use crate::modes::{self, CustomMode, RewriteMode};
use crate::settings::AppSettings;
use crate::web_search::{self, WebSearch};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashMap};

/// Les réglages de personne exportés avec la case « Réglages de personne » (clés des fichiers de réglages). `firstName` n'y est pas : il a
/// sa propre case (identité) ; `customModeID` et `customModes` suivent la case des modes ; `hiddenModes` en fait partie, sans les identifiants
/// de modes personnels quand les modes ne partent pas.
pub const PREFERENCE_KEYS: [&str; 8] = [
    "language",
    "translationLanguage",
    "speakerGender",
    "rewriteMode",
    "simpleFidelity",
    "listBullet",
    "voiceCommandsEnabled",
    "voiceCommands",
];

/// Longueur maximale (en caractères) du nom affiché et du prénom.
pub const MAX_NAME: usize = 40;
/// Nombre maximal de modes masqués, d'entrées de consignes modifiées et de commandes vocales d'une langue dans un fichier.
pub const MAX_LISTED: usize = 100;
/// Longueur maximale (en caractères) d'un identifiant de mode (masqué ou modifié).
pub const MAX_ID: usize = 100;

/// Le nom d'un utilisateur importé qui n'en a pas (et que la page ne nomme pas).
pub const UNNAMED: &str = "Utilisateur importé";

/// Les cases de l'aperçu, à l'export comme à l'import.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Categories {
    /// Le nom affiché et le prénom.
    pub identity: bool,
    /// Les réglages de personne ([`PREFERENCE_KEYS`] et les modes masqués).
    pub preferences: bool,
    pub words: bool,
    pub corrections: bool,
    /// Les modes personnels (et le mode choisi s'il en fait partie).
    pub modes: bool,
    /// Les exemples des modes personnels (sans effet si `modes` est décoché).
    pub examples: bool,
    /// Les consignes modifiées des modes livrés.
    pub styles: bool,
    /// Les commandes perso.
    pub commands: bool,
    /// Les sites de recherche vocale de la personne (adresse comprise). Cochés à l'export, **décochés à l'import** (chantier 6, décision 5).
    pub web_searches: bool,
}

impl Categories {
    /// L'export : tout part, **sauf le nom affiché et le prénom** (décision 5, relecture C6).
    pub fn export_default() -> Self {
        Self {
            identity: false,
            preferences: true,
            words: true,
            corrections: true,
            modes: true,
            examples: true,
            styles: true,
            commands: true,
            web_searches: true,
        }
    }

    /// L'import : tout ce que le fichier contient (le fichier a déjà été limité par la personne qui l'a écrit), **sauf les sites de
    /// recherche** : la page montre l'hôte de chacun et la personne les coche (un site importé s'ouvrirait sur une phrase qu'elle dira).
    pub fn import_default() -> Self {
        Self { identity: true, web_searches: false, ..Self::export_default() }
    }

    /// Lit les cases d'une requête de la page ; une case absente prend la valeur par défaut donnée.
    pub fn from_value(input: &Value, default: Self) -> Self {
        let get = |key: &str, d: bool| input.get(key).and_then(Value::as_bool).unwrap_or(d);
        Self {
            identity: get("identity", default.identity),
            preferences: get("preferences", default.preferences),
            words: get("words", default.words),
            corrections: get("corrections", default.corrections),
            modes: get("modes", default.modes),
            examples: get("examples", default.examples),
            styles: get("styles", default.styles),
            commands: get("commands", default.commands),
            web_searches: get("web_searches", default.web_searches),
        }
    }
}

/// Le nom de fichier proposé : `scriibe-utilisateur-<jour>.scriibe.json` (jamais le nom de l'utilisateur).
pub fn file_name(day: &str) -> String {
    format!("scriibe-utilisateur-{day}{}", export::EXTENSION)
}

/// Ce qu'un utilisateur contient, par catégorie, pour l'aperçu de l'export (rien n'est écrit).
#[derive(Debug, Clone, PartialEq)]
pub struct Contents {
    pub name: String,
    pub first_name: String,
    pub preferences: Vec<String>,
    pub words: Vec<String>,
    pub corrections: Vec<(String, String)>,
    pub modes: Vec<String>,
    pub examples: usize,
    pub styles: Vec<String>,
    pub commands: Vec<String>,
    /// Les sites de recherche vocale : le nom et l'hôte que fixe l'adresse (vide si l'adresse est refusée).
    pub web_searches: Vec<(String, String)>,
}

/// Le nom lisible d'un réglage de personne, pour l'aperçu.
fn preference_label(key: &str) -> &'static str {
    match key {
        "language" => "langue dictée",
        "translationLanguage" => "langue de traduction",
        "speakerGender" => "genre",
        "rewriteMode" => "mode choisi",
        "simpleFidelity" => "fidélité du mode Simple",
        "listBullet" => "puce des listes",
        "voiceCommandsEnabled" => "commandes vocales (interrupteur)",
        "voiceCommands" => "phrases des commandes vocales",
        _ => "réglage",
    }
}

/// Les réglages de personne qui s'exporteraient (leur nom lisible) : ceux de la liste blanche qui ont une valeur.
fn preference_entries(settings: &AppSettings) -> Map<String, Value> {
    let all = serde_json::to_value(settings).ok().and_then(|v| v.as_object().cloned()).unwrap_or_default();
    let mut out = Map::new();
    for key in PREFERENCE_KEYS {
        if let Some(value) = all.get(key) {
            if key == "voiceCommands" && *value == json!(VoicePhrases::default()) {
                continue;
            }
            out.insert(key.to_string(), value.clone());
        }
    }
    out
}

/// L'inventaire d'un utilisateur : ce que l'aperçu de l'export compte et liste.
pub fn contents(name: &str, settings: &AppSettings, dictionary: &UserDictionary) -> Contents {
    let mut preferences: Vec<String> = preference_entries(settings).keys().map(|k| preference_label(k).to_string()).collect();
    if !settings.hidden_modes.is_empty() {
        preferences.push("modes masqués du menu".into());
    }
    Contents {
        name: name.to_string(),
        first_name: settings.first_name.clone(),
        preferences,
        words: dictionary.words.clone(),
        corrections: dictionary.corrections.iter().map(|c| (c.heard.clone(), c.written.clone())).collect(),
        modes: settings.custom_modes.iter().map(CustomMode::label).collect(),
        examples: settings.custom_modes.iter().map(|m| m.examples.len()).sum(),
        styles: settings.style_overrides.iter().filter(|(_, s)| !s.trim().is_empty()).map(|(id, _)| id.clone()).collect(),
        commands: settings.custom_commands.iter().map(|c| c.name.clone()).collect(),
        web_searches: settings
            .web_searches
            .iter()
            .map(|w| (w.name.clone(), web_search::template_host(&w.url).unwrap_or_default()))
            .collect(),
    }
}

/// Le fichier d'export d'un utilisateur. Seules les catégories cochées partent ; une catégorie vide n'a pas de clé. Le contenu est recopié
/// champ par champ (liste blanche) : `settings` n'est jamais écrit tel quel.
pub fn export_profile(
    name: &str,
    settings: &AppSettings,
    dictionary: &UserDictionary,
    take: Categories,
    app_version: &str,
    day: &str,
) -> Value {
    let mut content = Map::new();
    if take.identity {
        if !name.trim().is_empty() {
            content.insert("name".into(), json!(name.trim()));
        }
        if !settings.first_name.trim().is_empty() {
            content.insert("firstName".into(), json!(settings.first_name.trim()));
        }
    }
    if take.preferences {
        content.extend(preference_entries(settings));
        // Un identifiant de mode personnel masqué n'a de sens que si le mode part avec : sinon il est retiré.
        let custom: Vec<&str> = settings.custom_modes.iter().map(|m| m.id.as_str()).collect();
        let hidden: Vec<&String> = settings.hidden_modes.iter().filter(|h| take.modes || !custom.contains(&h.as_str())).collect();
        if !hidden.is_empty() {
            content.insert("hiddenModes".into(), json!(hidden));
        }
    }
    let mut dict = Map::new();
    if take.words && !dictionary.words.is_empty() {
        dict.insert("words".into(), json!(dictionary.words));
    }
    if take.corrections && !dictionary.corrections.is_empty() {
        let list: Vec<Value> = dictionary.corrections.iter().map(|c| json!({"heard": c.heard, "written": c.written})).collect();
        dict.insert("corrections".into(), Value::Array(list));
    }
    if !dict.is_empty() {
        content.insert("dictionary".into(), Value::Object(dict));
    }
    if take.modes && !settings.custom_modes.is_empty() {
        let list: Vec<Value> = settings.custom_modes.iter().map(|m| export::mode_entry(m, take.examples)).collect();
        content.insert("customModes".into(), Value::Array(list));
        if let Some(id) = &settings.custom_mode_id {
            content.insert("customModeID".into(), json!(id));
        }
    }
    if take.styles {
        let styles: BTreeMap<&String, &String> = settings.style_overrides.iter().filter(|(_, s)| !s.trim().is_empty()).collect();
        if !styles.is_empty() {
            content.insert("styleOverrides".into(), json!(styles));
        }
    }
    if take.commands && !settings.custom_commands.is_empty() {
        let list = export::commands_export(&settings.custom_commands, app_version, day);
        content.insert("customCommands".into(), list["content"]["customCommands"].clone());
    }
    if take.web_searches && !settings.web_searches.is_empty() {
        // Sans identifiant : l'import en donne de neufs. Le nom, l'adresse et les phrases suffisent.
        let list: Vec<Value> = settings.web_searches.iter().map(|w| json!({"name": w.name, "url": w.url, "phrases": w.phrases})).collect();
        content.insert("webSearches".into(), Value::Array(list));
    }
    export::envelope("profile", app_version, day, Value::Object(content))
}

/// Ce que contient un fichier d'utilisateur, lu et contrôlé (rien n'est encore appliqué).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Incoming {
    pub name: Option<String>,
    pub first_name: Option<String>,
    /// Les réglages de personne du fichier : clés de [`PREFERENCE_KEYS`] seulement, valeurs telles qu'elles sont dans le fichier (relues
    /// ensuite par la lecture tolérante des réglages).
    pub preferences: Map<String, Value>,
    pub hidden_modes: Vec<String>,
    pub dictionary: import_dictionary::Incoming,
    pub modes: Vec<CustomMode>,
    /// Le mode personnel choisi (son identifiant **dans le fichier**).
    pub custom_mode_id: Option<String>,
    pub style_overrides: BTreeMap<String, String>,
    pub commands: Vec<CustomCommand>,
    /// Les sites de recherche du fichier (identifiant vide : il est donné à l'import). Pas encore contrôlés : l'aperçu montre ce qui sera
    /// refusé, l'import écarte les sites refusés un par un.
    pub web_searches: Vec<WebSearch>,
}

fn is_empty(incoming: &Incoming) -> bool {
    incoming.name.is_none()
        && incoming.first_name.is_none()
        && incoming.preferences.is_empty()
        && incoming.hidden_modes.is_empty()
        && incoming.dictionary.words.is_empty()
        && incoming.dictionary.corrections.is_empty()
        && incoming.modes.is_empty()
        && incoming.style_overrides.is_empty()
        && incoming.commands.is_empty()
        && incoming.web_searches.is_empty()
}

fn too_long(text: &str, max: usize) -> bool {
    text.chars().count() > max
}

/// Un texte court (nom, prénom) : une seule ligne ; `None` s'il est absent ou vide.
fn short_text(content: &Map<String, Value>, key: &str) -> Result<Option<String>, String> {
    match content.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) if too_long(s, MAX_NAME * 4) => Err("un nom du fichier est trop long".into()),
        Some(Value::String(s)) => {
            let line = one_line(s);
            if line.chars().count() > MAX_NAME {
                return Err("un nom du fichier est trop long : 40 caractères au maximum".into());
            }
            Ok((!line.is_empty()).then_some(line))
        }
        Some(_) => Err("le fichier est mal formé (un nom n'est pas un texte)".into()),
    }
}

/// Les phrases des commandes vocales : deux langues, des commandes, des phrases courtes, en nombre limité.
fn check_voice_commands(value: &Value) -> Result<(), String> {
    let bad = || "les phrases des commandes vocales du fichier sont mal formées".to_string();
    let langs = value.as_object().ok_or_else(bad)?;
    for (_, list) in langs {
        let commands = list.as_object().ok_or_else(bad)?;
        if commands.len() > MAX_LISTED {
            return Err("trop de commandes vocales dans ce fichier : 100 au maximum par langue".into());
        }
        for (id, phrases) in commands {
            if too_long(id, MAX_ID) {
                return Err(bad());
            }
            let phrases = phrases.as_array().ok_or_else(bad)?;
            if phrases.len() > MAX_PHRASES {
                return Err("une commande vocale a trop de phrases : 20 au maximum".into());
            }
            for phrase in phrases {
                let phrase = phrase.as_str().ok_or_else(bad)?;
                if too_long(phrase, MAX_PHRASE) {
                    return Err(format!("une phrase de commande vocale est trop longue : {MAX_PHRASE} caractères au maximum"));
                }
            }
        }
    }
    Ok(())
}

/// Les sites de recherche vocale d'un fichier : au plus [`web_search::MAX_SITES`], chacun avec un nom, une adresse et des phrases courtes en
/// nombre limité. Les adresses ne sont pas encore jugées ici (l'aperçu montre l'hôte, l'import écarte les adresses refusées).
fn read_web_searches(value: &Value) -> Result<Vec<WebSearch>, String> {
    let bad = || "la liste des sites de recherche du fichier est mal formée".to_string();
    let list = value.as_array().ok_or_else(bad)?;
    if list.len() > web_search::MAX_SITES {
        return Err(format!("trop de sites de recherche dans ce fichier : {} au maximum", web_search::MAX_SITES));
    }
    let mut out = Vec::new();
    for item in list {
        let site = item.as_object().ok_or_else(bad)?;
        let text = |key: &str| site.get(key).and_then(Value::as_str).ok_or_else(bad);
        let (name, url) = (text("name")?, text("url")?);
        if too_long(name, web_search::MAX_NAME * 4) || too_long(url, web_search::MAX_TEMPLATE * 4) {
            return Err("un site de recherche du fichier a un nom ou une adresse trop long".into());
        }
        let phrases = site.get("phrases").and_then(Value::as_array).ok_or_else(bad)?;
        if phrases.len() > MAX_PHRASES {
            return Err(format!("un site de recherche a trop de phrases : {MAX_PHRASES} au maximum"));
        }
        let phrases = phrases
            .iter()
            .map(|p| match p.as_str() {
                Some(p) if !too_long(p, MAX_PHRASE) => Ok(p.to_string()),
                Some(_) => Err(format!("une phrase de site de recherche est trop longue : {MAX_PHRASE} caractères au maximum")),
                None => Err(bad()),
            })
            .collect::<Result<Vec<_>, _>>()?;
        out.push(WebSearch { id: String::new(), name: name.to_string(), url: url.to_string(), phrases });
    }
    Ok(out)
}

fn id_list(value: &Value, what: &str) -> Result<Vec<String>, String> {
    let bad = || format!("{what} du fichier est mal formée");
    let list = value.as_array().ok_or_else(bad)?;
    if list.len() > MAX_LISTED {
        return Err(format!("trop d'éléments dans {what} du fichier : 100 au maximum"));
    }
    list.iter()
        .map(|v| match v.as_str() {
            Some(s) if !too_long(s, MAX_ID) => Ok(s.to_string()),
            _ => Err(bad()),
        })
        .collect()
}

/// Lit un fichier d'export d'utilisateur. `Err` : un message clair pour la page (rien n'a été modifié). Les clés inconnues sont ignorées,
/// jamais copiées ; seule la liste blanche est lue, avec les limites des étapes 2 et 3.
pub fn parse_profile(text: &str) -> Result<Incoming, String> {
    let root = export::open_envelope(
        text,
        "profile",
        MAX_BYTES,
        "fichier trop gros : 1 Mo au maximum",
        "ce fichier ne contient pas d'utilisateur (autre type d'export)",
    )?;
    let content = root["content"].as_object().ok_or("ce fichier ne contient pas d'utilisateur")?;
    let mut incoming = Incoming { name: short_text(content, "name")?, first_name: short_text(content, "firstName")?, ..Default::default() };
    for key in PREFERENCE_KEYS {
        match content.get(key) {
            None | Some(Value::Null) => {}
            Some(value) => {
                if key == "voiceCommands" {
                    check_voice_commands(value)?;
                }
                incoming.preferences.insert(key.to_string(), value.clone());
            }
        }
    }
    if let Some(hidden) = content.get("hiddenModes").filter(|v| !v.is_null()) {
        incoming.hidden_modes = id_list(hidden, "la liste des modes masqués")?;
    }
    if let Some(dictionary) = content.get("dictionary").filter(|v| !v.is_null()) {
        incoming.dictionary = import_dictionary::read_content(dictionary)?;
    }
    if let Some(list) = content.get("customModes").filter(|v| !v.is_null()) {
        incoming.modes = import::read_modes(list.as_array().ok_or("la liste des modes du fichier est mal formée")?)?;
        incoming.custom_mode_id = modes::parse_id(content.get("customModeID"));
    }
    if let Some(styles) = content.get("styleOverrides").filter(|v| !v.is_null()) {
        let styles = styles.as_object().ok_or("les consignes modifiées du fichier sont mal formées")?;
        if styles.len() > MAX_LISTED {
            return Err("trop de consignes modifiées dans ce fichier : 100 au maximum".into());
        }
        for (id, style) in styles {
            let style = style.as_str().ok_or("les consignes modifiées du fichier sont mal formées")?;
            if too_long(id, MAX_ID) || too_long(style, MAX_TEXT) {
                return Err("une consigne modifiée est trop longue : 20 000 caractères au maximum".into());
            }
            incoming.style_overrides.insert(id.clone(), style.to_string());
        }
    }
    if let Some(list) = content.get("customCommands").filter(|v| !v.is_null()) {
        incoming.commands = import_commands::read_commands(list.as_array().ok_or("la liste des commandes du fichier est mal formée")?)?;
    }
    if let Some(list) = content.get("webSearches").filter(|v| !v.is_null()) {
        incoming.web_searches = read_web_searches(list)?;
    }
    if is_empty(&incoming) {
        return Err("aucun contenu dans ce fichier".into());
    }
    Ok(incoming)
}

/// L'aperçu d'un fichier d'utilisateur : ce qu'il donnerait, par catégorie.
#[derive(Debug, Clone, PartialEq)]
pub struct Preview {
    pub name: Option<String>,
    pub first_name: Option<String>,
    pub preferences: Vec<String>,
    pub hidden_modes: usize,
    pub words: usize,
    pub corrections: usize,
    pub modes: Vec<import::PreviewItem>,
    pub styles: Vec<String>,
    pub commands: Vec<import_commands::Candidate>,
    pub web_searches: Vec<SitePreview>,
}

/// Un site de recherche du fichier, pour l'aperçu : son nom, l'hôte que fixe son adresse (`None` : adresse refusée, le site ne sera pas
/// importé) et ses phrases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SitePreview {
    pub name: String,
    pub host: Option<String>,
    pub phrases: Vec<String>,
}

/// L'aperçu avant import : les modes et commandes sont évalués face à un utilisateur vide (titres numérotés entre eux, raccourcis contrôlés).
pub fn preview(incoming: &Incoming) -> Preview {
    let blank = AppSettings::default();
    Preview {
        name: incoming.name.clone(),
        first_name: incoming.first_name.clone(),
        preferences: incoming.preferences.keys().map(|k| preference_label(k).to_string()).collect(),
        hidden_modes: incoming.hidden_modes.len(),
        words: incoming.dictionary.words.len(),
        corrections: incoming.dictionary.corrections.len(),
        modes: import::preview(&[], &incoming.modes),
        styles: usable_styles(incoming).into_keys().collect(),
        commands: import_commands::preview(&blank, &incoming.commands),
        web_searches: incoming
            .web_searches
            .iter()
            .map(|w| SitePreview { name: w.name.clone(), host: web_search::template_host(w.url.trim()), phrases: w.phrases.clone() })
            .collect(),
    }
}

/// Les consignes modifiées qui visent un mode livré (une clé qui n'en est pas un est écartée).
fn usable_styles(incoming: &Incoming) -> BTreeMap<String, String> {
    incoming
        .style_overrides
        .iter()
        .filter(|(id, style)| RewriteMode::from_raw(id).is_some() && !style.trim().is_empty())
        .map(|(id, style)| (id.clone(), style.clone()))
        .collect()
}

/// Ce que l'import a construit : le contenu du nouvel utilisateur, prêt à être écrit.
#[derive(Debug, Clone, PartialEq)]
pub struct Built {
    /// Les clés du `profile.json` du nouvel utilisateur, **sans** le nom affiché (donné à part) : seulement celles que l'import renseigne,
    /// les autres prennent leur valeur par défaut à la lecture.
    pub profile: Map<String, Value>,
    pub dictionary: UserDictionary,
    pub outcome: Outcome,
}

/// Ce qui a été importé.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub words: usize,
    pub corrections: usize,
    pub modes: Vec<String>,
    pub examples: usize,
    pub styles: usize,
    /// Les titres sous lesquels des commandes ont été ajoutées.
    pub commands: Vec<String>,
    /// Les commandes écartées (raccourci illisible, plus de phrase utilisable).
    pub commands_skipped: usize,
    /// Les noms des sites de recherche ajoutés, et le nombre de ceux qui sont écartés (adresse refusée, plus de phrase utilisable).
    pub web_searches: Vec<String>,
    pub web_searches_skipped: usize,
    pub preferences: usize,
    pub identity: bool,
}

/// Construit le contenu du nouvel utilisateur à partir des catégories cochées. Les identifiants des modes, de leurs exemples et des
/// commandes sont **toujours neufs** ; `customModeID` et `hiddenModes` suivent la table (ancien → nouvel identifiant). Les valeurs passent
/// par la lecture tolérante des réglages : ce qui est écrit est ce que l'application lirait.
pub fn build(incoming: &Incoming, take: Categories) -> Built {
    let mut settings = AppSettings::default();
    let mut keys: Vec<&str> = Vec::new();
    let mut outcome = Outcome::default();
    if take.identity {
        if let Some(first) = &incoming.first_name {
            settings.first_name.clone_from(first);
            keys.push("firstName");
            outcome.identity = true;
        }
    }
    // Les modes : identifiants neufs (table ancien → nouveau), puis les titres numérotés entre eux comme à l'import de modes.
    let mut table: HashMap<String, String> = HashMap::new();
    if take.modes && !incoming.modes.is_empty() {
        let mut renewed = incoming.modes.clone();
        for mode in &mut renewed {
            let fresh = modes::new_id();
            table.insert(mode.id.clone(), fresh.clone());
            mode.id = fresh;
            for example in &mut mode.examples {
                example.id = modes::new_id();
            }
            if !take.examples {
                mode.examples.clear();
            }
        }
        let applied = import::apply(&mut settings.custom_modes, renewed, &[]);
        outcome.modes = applied.added;
        outcome.examples = settings.custom_modes.iter().map(|m| m.examples.len()).sum();
        keys.push("customModes");
        if let Some(new_id) = incoming.custom_mode_id.as_ref().and_then(|old| table.get(old)) {
            settings.custom_mode_id = Some(new_id.clone());
            keys.push("customModeID");
        }
    }
    if take.preferences {
        let parsed = AppSettings::from_value(&Value::Object(incoming.preferences.clone())).unwrap_or_default();
        for key in incoming.preferences.keys() {
            match key.as_str() {
                "language" => settings.language = parsed.language,
                "translationLanguage" => settings.translation_language = parsed.translation_language,
                "speakerGender" => settings.speaker_gender = parsed.speaker_gender,
                "rewriteMode" => settings.rewrite_mode = parsed.rewrite_mode,
                "simpleFidelity" => settings.simple_fidelity = parsed.simple_fidelity,
                "listBullet" => settings.list_bullet = parsed.list_bullet,
                "voiceCommandsEnabled" => settings.voice_commands_enabled = parsed.voice_commands_enabled,
                "voiceCommands" => settings.voice_commands = parsed.voice_commands.clone(),
                _ => continue,
            }
            keys.push(key.as_str());
            outcome.preferences += 1;
        }
        if !incoming.hidden_modes.is_empty() {
            settings.hidden_modes = incoming.hidden_modes.iter().map(|h| table.get(h).cloned().unwrap_or_else(|| h.clone())).collect();
            keys.push("hiddenModes");
            outcome.preferences += 1;
        }
    }
    if take.styles {
        let styles = usable_styles(incoming);
        if !styles.is_empty() {
            outcome.styles = styles.len();
            settings.style_overrides = styles;
            keys.push("styleOverrides");
        }
    }
    if take.commands && !incoming.commands.is_empty() {
        let applied = import_commands::apply(&mut settings, &incoming.commands, &[]);
        outcome.commands = applied.added;
        outcome.commands_skipped = applied.skipped;
        keys.push("customCommands");
    }
    if take.web_searches && !incoming.web_searches.is_empty() {
        // Identifiants toujours neufs (`recherche.10`, `.11`…), puis le même contrôle que la lecture des réglages : un site refusé est écarté seul.
        let numbered: Vec<WebSearch> = incoming
            .web_searches
            .iter()
            .enumerate()
            .map(|(i, w)| WebSearch { id: format!("{}{}", web_search::PREFIX, web_search::FIRST_CUSTOM as usize + i), ..w.clone() })
            .collect();
        let kept = WebSearch::cleaned(numbered);
        outcome.web_searches_skipped = incoming.web_searches.len() - kept.len();
        outcome.web_searches = kept.iter().map(|w| w.name.clone()).collect();
        if !kept.is_empty() {
            settings.web_searches = kept;
            keys.push("webSearches");
        }
    }
    let mut dictionary = UserDictionary::default();
    let (words, corrections) = (take.words, take.corrections);
    let kept = import_dictionary::Incoming {
        words: if words { incoming.dictionary.words.clone() } else { Vec::new() },
        corrections: if corrections { incoming.dictionary.corrections.clone() } else { Vec::new() },
    };
    let done = import_dictionary::apply(&mut dictionary, &kept, &[], &[]);
    outcome.words = done.words_added;
    outcome.corrections = done.corrections_added;

    // Ce qui est écrit est ce que l'application lirait : une dernière lecture tolérante (modes masqués inconnus, mode choisi disparu…).
    let normalized = serde_json::to_value(&settings).ok().and_then(|v| AppSettings::from_value(&v)).unwrap_or(settings);
    let (_, user) = normalized.split();
    let profile: Map<String, Value> = user.into_iter().filter(|(k, _)| keys.contains(&k.as_str())).collect();
    Built { profile, dictionary, outcome }
}

#[cfg(test)]
#[path = "profile_file_tests.rs"]
mod tests;
