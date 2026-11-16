//! « Nouveau mail » (chantier 6, étape 3) : sous la touche de commande, « nouveau mail » ouvre un message vierge dans la messagerie par
//! défaut, et « nouveau mail réunion de lundi » préremplit l'**objet**. Rien n'est envoyé ; pas de destinataire (Scriibe ne lit pas les
//! contacts : « nouveau mail à Marie » met « à Marie » en objet).
//!
//! Une seule fiche, avec un complément **facultatif** ([`Argument::OptionalRest`]). L'adresse est toujours `mailto:` ou
//! `mailto:?subject=<objet encodé>` : l'objet est encodé octet par octet (RFC 3986), donc un `&to=` ou un `&body=` dicté reste du texte.
//! Le journal n'a que l'identifiant ; l'objet n'y est jamais, ni dans la pastille, ni dans l'historique.

use crate::commands::{normalize, Argument, CommandDef, Needs, Scope};
use crate::web_search::percent_encode;

/// Identifiant de la commande (le journal ne garde que lui).
pub const ID: &str = "mail.nouveau";
/// Longueur maximale (en caractères) de l'objet, avant encodage.
pub const MAX_SUBJECT_CHARS: usize = 200;

const PHRASES_FR: [&str; 5] = ["nouveau mail", "nouveau courriel", "nouvel email", "nouveau email", "nouveau e-mail"];
const PHRASES_EN: [&str; 3] = ["new email", "new mail", "new e-mail"];

/// Les mots qui, en tête du reste, annoncent l'objet (« nouveau mail objet réunion ») et sont ôtés.
const SUBJECT_LEADS: [&str; 3] = ["objet", "sujet", "subject"];

/// La fiche de la commande : sous la touche seulement, sans sélection, complément facultatif.
pub fn defs() -> Vec<CommandDef> {
    let list = |l: &[&str]| l.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();
    vec![CommandDef {
        id: ID.into(),
        scope: Scope::UnderKey,
        needs: Needs::Nothing,
        argument: Argument::OptionalRest,
        phrases_fr: list(&PHRASES_FR),
        phrases_en: list(&PHRASES_EN),
    }]
}

/// Les phrases livrées (normalisées, deux langues) : celles qu'un extrait, un site ou une application ne peuvent pas reprendre.
pub(crate) fn phrases() -> Vec<String> {
    PHRASES_FR.iter().chain(&PHRASES_EN).map(|p| normalize(p)).collect()
}

/// Cet identifiant est-il celui de la commande « nouveau mail » ?
pub fn is_mail(id: &str) -> bool {
    id == ID
}

/// L'objet tel qu'il ira dans l'adresse (avant encodage) : blancs et contrôles ramenés à une espace, mot d'annonce (« objet », « sujet »)
/// ôté en tête, ponctuation de bord retirée, coupé à [`MAX_SUBJECT_CHARS`]. `None` : rien à mettre en objet.
pub fn clean_subject(argument: Option<&str>) -> Option<String> {
    let flat: String = argument?.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    let mut subject = flat.split_whitespace().collect::<Vec<_>>().join(" ");
    let first: String = subject.chars().take_while(|c| c.is_alphanumeric()).collect();
    if SUBJECT_LEADS.iter().any(|lead| first.eq_ignore_ascii_case(lead)) {
        subject = subject.chars().skip(first.chars().count()).collect();
    }
    let subject = subject.trim_matches(|c: char| !c.is_alphanumeric());
    let subject: String = subject.chars().take(MAX_SUBJECT_CHARS).collect();
    let subject = subject.trim_end().to_string();
    (!subject.is_empty()).then_some(subject)
}

/// L'adresse à ouvrir : `mailto:` seul, ou `mailto:?subject=` suivi de l'objet encodé. Jamais de destinataire ni de corps.
pub fn url(argument: Option<&str>) -> String {
    match clean_subject(argument) {
        Some(subject) => format!("mailto:?subject={}", percent_encode(&subject)),
        None => "mailto:".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{recognize_for, Context};
    use crate::settings::{AppSettings, DictationLanguage};

    fn settings() -> AppSettings {
        AppSettings { language: DictationLanguage::Multi, ..AppSettings::default() }
    }

    /// Chantier 6, étape 3 : l'objet est nettoyé (mot d'annonce ôté, blancs réduits, 200 signes au plus).
    #[test]
    fn commands_531_the_subject_is_cleaned() {
        assert_eq!(clean_subject(None), None);
        assert_eq!(clean_subject(Some("   ")), None);
        assert_eq!(clean_subject(Some("réunion de lundi")).as_deref(), Some("réunion de lundi"));
        assert_eq!(clean_subject(Some("objet réunion de lundi")).as_deref(), Some("réunion de lundi"));
        assert_eq!(clean_subject(Some("Sujet : le budget")).as_deref(), Some("le budget"));
        assert_eq!(clean_subject(Some("subject quarterly report")).as_deref(), Some("quarterly report"));
        assert_eq!(clean_subject(Some("objet")), None, "le mot d'annonce seul ne fait pas un objet");
        assert_eq!(clean_subject(Some("objets perdus")).as_deref(), Some("objets perdus"), "pas un mot d'annonce");
        assert_eq!(clean_subject(Some("à   Marie\n\tbonjour")).as_deref(), Some("à Marie bonjour"));
        assert_eq!(clean_subject(Some(&"a".repeat(500))).map(|s| s.chars().count()), Some(MAX_SUBJECT_CHARS));
    }

    /// L'objet est encodé : un `&to=` ou un `&body=` dicté reste du texte ; jamais de destinataire ni de corps ajoutés.
    #[test]
    fn commands_532_the_subject_is_encoded() {
        assert_eq!(url(None), "mailto:");
        assert_eq!(url(Some("  ")), "mailto:");
        assert_eq!(url(Some("réunion de lundi")), "mailto:?subject=r%C3%A9union%20de%20lundi");
        let hostile = url(Some("x&to=a@b.fr&body=SECRET#? fin"));
        assert_eq!(hostile.matches('&').count(), 0, "{hostile}");
        assert!(hostile.starts_with("mailto:?subject=x%26to%3Da%40b.fr%26body%3DSECRET%23%3F%20fin"), "{hostile}");
        assert!(crate::web_search::is_openable(&hostile));
        assert!(crate::web_search::is_openable(&url(None)));
        assert!(crate::web_search::is_openable(&url(Some("réunion"))));
    }

    /// La fiche : complément facultatif, sous la touche seulement ; « nouveau mail » seul et suivi d'un objet sont la même commande.
    #[test]
    fn commands_533_new_mail_is_recognized_with_or_without_a_subject_under_the_key_only() {
        let settings = settings();
        for (spoken, argument) in [
            ("nouveau mail", None),
            ("Nouveau courriel.", None),
            ("new email", None),
            ("nouveau mail réunion de lundi", Some("réunion de lundi")),
            ("nouveau mail objet réunion de lundi", Some("objet réunion de lundi")),
            ("new email about the budget", Some("about the budget")),
        ] {
            let found = recognize_for(spoken, &settings, Context::UnderKey).unwrap_or_else(|| panic!("{spoken}"));
            assert_eq!(found.id, ID, "{spoken}");
            assert_eq!(found.argument.as_deref(), argument, "{spoken}");
            assert_eq!(found.needs, Needs::Nothing);
        }
        assert!(recognize_for("nouveau", &settings, Context::UnderKey).is_none());
        assert!(recognize_for("mail", &settings, Context::UnderKey).is_none());
        // Sans la touche, c'est du texte (non-régression), même quand les commandes vocales sont permises.
        assert!(settings.voice_commands_enabled);
        for spoken in ["nouveau mail", "nouveau mail réunion de lundi", "new email"] {
            assert!(recognize_for(spoken, &settings, Context::NoKey).is_none(), "{spoken}");
        }
    }
}
