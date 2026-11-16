//! Applications ouvertes à la voix (chantier 6, étape 3) : sous la touche de commande, une phrase choisie (« ouvre le navigateur ») lance une
//! application de la **liste autorisée** de la personne. Couche pure : on ne dit ici que **quelle** application est demandée ; l'énumération
//! des applications installées et le lancement vivent dans `crates/platform` (`installed_apps`), l'aiguillage dans `crates/pipeline`.
//!
//! **Sécurité** (décision 3 de `docs/CADRAGE_COMMANDES.md`) : aucune commande système, aucun script, aucun argument. Le réglage ne garde
//! qu'un **identifiant** d'application (identifiant de bundle sur Mac, nom du fichier `.desktop` sous Linux, chemin relatif du raccourci du
//! menu Démarrer sous Windows). À **chaque** lancement, `crates/platform` retrouve cet identifiant dans une énumération fraîche des
//! applications installées et ne lance que ce que **cette liste** désigne : un `voiceApps` écrit à la main ne permet donc rien de plus.
//!
//! Champ **d'ordinateur** (jamais exporté) : les identifiants ne valent que sur ce système. Le journal n'a que `appli.<n>` (ni le nom de
//! l'application, ni les phrases) ; `Debug` masque le nom, l'identifiant et les phrases.

use serde::{Deserialize, Serialize};

use crate::commands::{normalize, Argument, CommandDef, Needs, Scope};
use crate::import_commands::{MAX_PHRASE, MAX_PHRASES};
use crate::settings::AppSettings;

/// Préfixe des identifiants des applications : `appli.<n>` (n ≥ 1). Le journal ne garde que cet identifiant.
pub const PREFIX: &str = "appli.";
/// Identifiant que la page donne à une application pas encore créée, pour contrôler sa première phrase (`commands::validate_phrase`).
pub const NEW: &str = "appli.nouveau";
/// Nombre maximal d'applications.
pub const MAX_APPS: usize = 30;
/// Longueur maximale (en caractères) du nom affiché.
pub const MAX_NAME: usize = 100;
/// Longueur maximale (en caractères) de l'identifiant d'application.
pub const MAX_APP_ID: usize = 300;

/// Une application de la liste autorisée : un nom (pour s'y retrouver dans la page), l'identifiant système choisi dans la liste des
/// applications installées, des phrases (« ouvre le navigateur »). Les phrases valent pour toutes les langues de dictée.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceApp {
    pub id: String,
    pub name: String,
    #[serde(rename = "appId")]
    pub app_id: String,
    #[serde(default)]
    pub phrases: Vec<String>,
}

/// Le nom, l'identifiant d'application et les phrases n'ont rien à faire dans un `{:?}` perdu (journal, message d'erreur).
impl std::fmt::Debug for VoiceApp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VoiceApp")
            .field("id", &self.id)
            .field("name", &"<nom>")
            .field("app_id", &"<identifiant>")
            .field("phrases", &self.phrases.len())
            .finish()
    }
}

/// Cet identifiant est-il celui d'une application ?
pub fn is_app(id: &str) -> bool {
    number(id).is_some()
}

/// Le numéro d'un identifiant d'application (`appli.12` → 12), au moins 1.
fn number(id: &str) -> Option<u32> {
    let digits = id.strip_prefix(PREFIX)?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse::<u32>().ok().filter(|n| *n >= 1)
}

/// L'identifiant système de l'application `id`. `None` : application disparue des réglages.
pub fn app_id_of<'a>(id: &str, settings: &'a AppSettings) -> Option<&'a str> {
    settings.voice_apps.iter().find(|a| a.id == id).map(|a| a.app_id.as_str())
}

/// Les fiches de toutes les applications en vigueur (sous la touche seulement, sans complément : la phrase est toute la transcription).
pub fn defs(settings: &AppSettings) -> Vec<CommandDef> {
    settings
        .voice_apps
        .iter()
        .map(|a| CommandDef {
            scope: Scope::UnderKey,
            id: a.id.clone(),
            needs: Needs::Nothing,
            argument: Argument::None,
            phrases_fr: a.phrases.clone(),
            phrases_en: a.phrases.clone(),
        })
        .collect()
}

/// Un identifiant d'application lu dans un fichier de réglages : non vide, au plus [`MAX_APP_ID`] caractères, sans caractère de contrôle.
/// Ce n'est qu'un contrôle de forme : ce qui se lance vient de la liste fraîche des applications installées, jamais de cette chaîne.
pub fn is_plausible_app_id(app_id: &str) -> bool {
    !app_id.is_empty() && app_id.chars().count() <= MAX_APP_ID && !app_id.chars().any(char::is_control) && app_id == app_id.trim()
}

impl VoiceApp {
    /// Le prochain identifiant libre.
    pub fn next_id(existing: &[VoiceApp]) -> String {
        let n = existing.iter().filter_map(|a| number(&a.id)).max().map_or(1, |m| m + 1);
        format!("{PREFIX}{n}")
    }

    /// Les applications lues dans un fichier de réglages écrit à la main : nom, identifiant d'application plausible et au moins une phrase
    /// exigés ; identifiants `appli.<n>` uniques ; au plus [`MAX_APPS`]. Une application mal écrite est écartée seule. Une phrase vide, trop
    /// longue, déjà prise par une commande du programme ou par une application précédente est écartée (un mot seul est permis : la phrase
    /// doit être toute la transcription).
    pub fn cleaned(list: Vec<VoiceApp>) -> Vec<VoiceApp> {
        let taken = crate::commands::program_phrases();
        let mut seen_phrases: Vec<String> = Vec::new();
        let mut out: Vec<VoiceApp> = Vec::new();
        for mut app in list {
            if out.len() >= MAX_APPS {
                break;
            }
            app.name = app.name.split_whitespace().collect::<Vec<_>>().join(" ");
            let mut mine: Vec<String> = Vec::new();
            app.phrases = app
                .phrases
                .into_iter()
                .map(|p| p.split_whitespace().collect::<Vec<_>>().join(" "))
                .filter(|p| {
                    let n = normalize(p);
                    let keep = !n.is_empty()
                        && p.chars().count() <= MAX_PHRASE
                        && !taken.contains(&n)
                        && !seen_phrases.contains(&n)
                        && !mine.contains(&n);
                    if keep {
                        mine.push(n);
                    }
                    keep
                })
                .take(MAX_PHRASES)
                .collect();
            if is_app(&app.id)
                && !app.name.is_empty()
                && app.name.chars().count() <= MAX_NAME
                && is_plausible_app_id(&app.app_id)
                && !app.phrases.is_empty()
                && !out.iter().any(|o| o.id == app.id)
            {
                seen_phrases.extend(mine);
                out.push(app);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{recognize_for, route_under_key, validate_phrase, Context, Lang, PhraseError, Route};
    use crate::settings::DictationLanguage;

    fn app(id: &str, name: &str, app_id: &str, phrases: &[&str]) -> VoiceApp {
        VoiceApp { id: id.into(), name: name.into(), app_id: app_id.into(), phrases: phrases.iter().map(|p| (*p).to_string()).collect() }
    }

    fn with(list: Vec<VoiceApp>) -> AppSettings {
        AppSettings { voice_apps: VoiceApp::cleaned(list), language: DictationLanguage::Multi, ..AppSettings::default() }
    }

    /// Une application se lit comme une commande sous la touche seulement ; la phrase est toute la transcription ; un mot seul est permis.
    #[test]
    fn commands_541_an_app_phrase_is_a_command_under_the_key_only() {
        let settings = with(vec![
            app("appli.1", "Safari", "com.apple.Safari", &["ouvre le navigateur", "navigateur"]),
            app("appli.2", "Notes", "com.apple.Notes", &["ouvre mes notes"]),
        ]);
        for (spoken, id) in [
            ("ouvre le navigateur", "appli.1"),
            ("Ouvre le navigateur.", "appli.1"),
            ("Navigateur", "appli.1"),
            ("ouvre mes notes", "appli.2"),
        ] {
            let found = recognize_for(spoken, &settings, Context::UnderKey).unwrap_or_else(|| panic!("{spoken}"));
            assert_eq!(found.id, id, "{spoken}");
            assert_eq!((found.needs, found.argument), (Needs::Nothing, None), "{spoken}");
        }
        // La phrase doit être toute la transcription.
        assert!(recognize_for("ouvre le navigateur web", &settings, Context::UnderKey).is_none());
        assert!(recognize_for("je dis navigateur", &settings, Context::UnderKey).is_none());
        // Sans la touche : jamais (non-régression : c'est du texte).
        assert!(settings.voice_commands_enabled);
        for spoken in ["ouvre le navigateur", "navigateur", "ouvre mes notes"] {
            assert!(recognize_for(spoken, &settings, Context::NoKey).is_none(), "{spoken}");
        }
        assert!(matches!(route_under_key("navigateur", &settings), Route::Command(c) if c.id == "appli.1"));
        assert_eq!(app_id_of("appli.2", &settings), Some("com.apple.Notes"));
        assert_eq!(app_id_of("appli.9", &settings), None);
    }

    /// Une application mal écrite est écartée seule ; les phrases prises, doublons et excès aussi ; `Debug` ne montre rien.
    #[test]
    fn commands_542_apps_are_cleaned_when_the_settings_are_read() {
        let kept = VoiceApp::cleaned(vec![
            app("appli.1", "  Safari  ", "com.apple.Safari", &["  ouvre   le navigateur ", "OUVRE LE NAVIGATEUR", ""]),
            app("appli.1", "Doublon", "com.x", &["autre phrase"]),
            app("perso.1", "Mauvais préfixe", "com.x", &["phrase un"]),
            app("appli.0", "Zéro", "com.x", &["phrase zéro"]),
            app("appli.2", "", "com.x", &["sans nom"]),
            app("appli.3", "Sans identifiant", "", &["sans identifiant"]),
            app("appli.4", "Identifiant à contrôle", "com.x\n-a calc", &["identifiant piégé"]),
            app("appli.5", "Sans phrase", "com.x", &[]),
            app("appli.6", "Phrase du programme", "com.x", &["mettre en gras", "nouvelle recherche google", "nouveau mail"]),
            app("appli.7", "Phrase déjà prise", "com.x", &["ouvre le navigateur"]),
            app("appli.8", "n".repeat(MAX_NAME + 1).as_str(), "com.x", &["nom trop long"]),
            app("appli.9", "Bonne", "com.bonne", &["ouvre bonne"]),
        ]);
        let ids: Vec<&str> = kept.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(ids, ["appli.1", "appli.9"]);
        assert_eq!(kept[0].name, "Safari");
        assert_eq!(kept[0].phrases, ["ouvre le navigateur"]);
        let many: Vec<VoiceApp> = (1..=40).map(|n| app(&format!("appli.{n}"), "A", "com.a", &[&format!("phrase numéro {n}")])).collect();
        assert_eq!(VoiceApp::cleaned(many).len(), MAX_APPS);
        let shown = format!("{:?}", app("appli.1", "SECRET-NOM", "SECRET-ID", &["SECRET-PHRASE"]));
        assert!(!shown.contains("SECRET"), "{shown}");
    }

    /// Identifiants : seulement `appli.<n>`, n ≥ 1 ; le prochain est libre.
    #[test]
    fn commands_543_ids_and_next_id() {
        assert!(is_app("appli.1") && is_app("appli.42"));
        for no in ["appli.", "appli.0", "appli.-1", "appli.1a", "appli.nouveau", "extrait.1", "recherche.1", "appli", ""] {
            assert!(!is_app(no), "{no}");
        }
        let list = vec![app("appli.1", "A", "a", &["a b"]), app("appli.4", "B", "b", &["c d"])];
        assert_eq!(VoiceApp::next_id(&list), "appli.5");
        assert_eq!(VoiceApp::next_id(&[]), "appli.1");
    }

    /// Une phrase d'application ne peut pas être celle d'une autre commande : `validate_phrase` la refuse ; une application pas encore créée
    /// (`appli.nouveau`) se contrôle comme les autres, un mot seul permis.
    #[test]
    fn commands_544_a_phrase_belongs_to_one_command_only() {
        let settings = with(vec![app("appli.1", "Safari", "com.apple.Safari", &["ouvre le navigateur"])]);
        let defs = crate::commands::all_defs(&settings);
        let none = crate::commands::VoicePhrases::default();
        assert_eq!(validate_phrase("ouvre le navigateur", NEW, Lang::Fr, &defs, &none), Err(PhraseError::Taken("appli.1".into())));
        assert_eq!(validate_phrase("nouveau mail", NEW, Lang::Fr, &defs, &none), Err(PhraseError::Taken(crate::mail::ID.into())));
        assert_eq!(validate_phrase("mettre en gras", NEW, Lang::Fr, &defs, &none), Err(PhraseError::Taken("format.gras".into())));
        assert!(validate_phrase("navigateur", NEW, Lang::Fr, &defs, &none).is_ok());
        assert!(validate_phrase("ouvre le navigateur", "appli.1", Lang::Fr, &defs, &none).is_ok(), "sa propre phrase");
        assert_eq!(validate_phrase("x", "appli.9", Lang::Fr, &defs, &none), Err(PhraseError::UnknownCommand));
    }

    /// `voiceApps` est un champ **d'ordinateur** : rangé côté ordinateur, jamais dans le fichier de l'utilisateur, ni dans ce qu'une
    /// sauvegarde d'un autre système restaure.
    #[test]
    fn commands_545_voice_apps_belong_to_the_computer_and_never_travel() {
        use crate::settings::{owner_of, Owner};
        assert_eq!(owner_of("voiceApps"), Some(Owner::Computer));
        assert!(crate::backup::COMPUTER_FIELDS.contains(&"voiceApps"));
        let settings = with(vec![app("appli.1", "Safari", "com.apple.Safari", &["ouvre le navigateur"])]);
        let (computer, user) = settings.split();
        assert!(computer.contains_key("voiceApps") && !user.contains_key("voiceApps"));
        // Relu depuis un fichier d'ordinateur, il est contrôlé comme les autres.
        let reread = AppSettings::from_value(&serde_json::Value::Object(computer)).unwrap();
        assert_eq!(reread.voice_apps.len(), 1);
        // Une sauvegarde d'un autre système ne remplace pas les applications de cet ordinateur.
        let backup =
            serde_json::json!({"voiceApps": [{"id": "appli.1", "name": "X", "appId": "x.y", "phrases": ["a b"]}], "language": "fr"});
        let current = serde_json::json!({"voiceApps": []});
        let restored = crate::backup::restored_settings(&backup, Some(&current), false);
        assert_eq!(restored["voiceApps"], serde_json::json!([]));
        assert_eq!(restored["language"], "fr");
    }
}
