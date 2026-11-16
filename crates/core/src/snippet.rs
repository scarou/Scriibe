//! Extraits vocaux (chantier 6, étape 2) : une phrase dite **sous la touche de commande** colle un texte que la personne a écrit
//! (« insérer mon adresse email » → l'adresse). Couche pure : on ne dit ici que **quel** extrait a été demandé ; le collage vit dans
//! `crates/platform` (`Paster::paste_transient`) et l'aiguillage dans `crates/pipeline`.
//!
//! **Confidentialité** (décision 5 de `docs/CADRAGE_COMMANDES.md`) : un extrait contient souvent une adresse, un IBAN ou une signature. Il
//! est collé **tel quel** (sans modèle, sans dictionnaire, sans typographie, sans espace ajoutée), sans passer par le chemin d'une dictée
//! (donc ni dans `rawText`, ni dans `finalText`, ni dans l'historique), et le presse-papiers d'avant est toujours remis. Le journal n'a que
//! l'identifiant (`extrait.<n>`) et la longueur ; le nom et le texte n'y sont jamais. `Debug` ne montre pas le texte.
//!
//! Seulement sous la touche : dite sans elle, la phrase d'un extrait reste du texte comme un autre (portée `UnderKey`).

use serde::{Deserialize, Serialize};

use crate::commands::{normalize, Argument, CommandDef, Needs, Scope};
use crate::import_commands::{MAX_PHRASE, MAX_PHRASES};
use crate::settings::AppSettings;

/// Préfixe des identifiants des extraits : `extrait.<n>` (n ≥ 1). Le journal ne garde que cet identifiant.
pub const PREFIX: &str = "extrait.";
/// Identifiant que la page donne à un extrait pas encore créé, pour contrôler sa première phrase (`commands::validate_phrase`).
pub const NEW: &str = "extrait.nouveau";
/// Nombre maximal d'extraits.
pub const MAX_SNIPPETS: usize = 50;
/// Longueur maximale (en caractères) du nom d'un extrait.
pub const MAX_NAME: usize = 40;
/// Longueur maximale (en caractères) du texte d'un extrait.
pub const MAX_TEXT: usize = 5_000;

/// Un extrait : un nom (pour s'y retrouver dans la page), des phrases (« insérer mon adresse email »), le texte à coller (plusieurs
/// lignes permises). Les phrases valent pour toutes les langues de dictée (c'est la personne qui les écrit).
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snippet {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub phrases: Vec<String>,
    pub text: String,
}

/// Le texte d'un extrait est confidentiel : un `{:?}` perdu (réglages, message d'erreur, journal) n'en montre que la longueur.
impl std::fmt::Debug for Snippet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Snippet")
            .field("id", &self.id)
            .field("name", &"<nom>")
            .field("phrases", &self.phrases.len())
            .field("text", &format_args!("<{} signes>", self.text.chars().count()))
            .finish()
    }
}

/// Cet identifiant est-il celui d'un extrait ?
pub fn is_snippet(id: &str) -> bool {
    number(id).is_some()
}

/// Le numéro d'un identifiant d'extrait (`extrait.12` → 12), au moins 1.
fn number(id: &str) -> Option<u32> {
    let digits = id.strip_prefix(PREFIX)?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse::<u32>().ok().filter(|n| *n >= 1)
}

/// Le texte de l'extrait `id`, tel qu'il sera collé. `None` : extrait disparu ou vide.
pub fn text_of<'a>(id: &str, settings: &'a AppSettings) -> Option<&'a str> {
    settings.snippets.iter().find(|s| s.id == id).map(|s| s.text.as_str()).filter(|t| !t.trim().is_empty())
}

/// Les fiches de tous les extraits en vigueur (sous la touche seulement, sans complément : la phrase est toute la transcription).
pub fn defs(settings: &AppSettings) -> Vec<CommandDef> {
    settings
        .snippets
        .iter()
        .map(|s| CommandDef {
            scope: Scope::UnderKey,
            id: s.id.clone(),
            needs: Needs::Nothing,
            argument: Argument::None,
            phrases_fr: s.phrases.clone(),
            phrases_en: s.phrases.clone(),
        })
        .collect()
}

/// Le texte d'un extrait lu ou reçu : fins de ligne ramenées à `\n`, sans caractère de contrôle (hors saut de ligne et tabulation), et
/// sans espace ni saut de ligne en trop aux bords. `None` : vide, ou plus long que [`MAX_TEXT`].
pub fn clean_text(text: &str) -> Option<String> {
    let unified = text.replace("\r\n", "\n").replace('\r', "\n");
    let kept: String = unified.chars().filter(|c| !c.is_control() || matches!(c, '\n' | '\t')).collect();
    let trimmed = kept.trim_matches(|c: char| c.is_whitespace());
    (!trimmed.is_empty() && trimmed.chars().count() <= MAX_TEXT).then(|| trimmed.to_string())
}

impl Snippet {
    /// Le prochain identifiant libre.
    pub fn next_id(existing: &[Snippet]) -> String {
        let n = existing.iter().filter_map(|s| number(&s.id)).max().map_or(1, |m| m + 1);
        format!("{PREFIX}{n}")
    }

    /// Les extraits lus dans un fichier de réglages écrit à la main : nom, au moins une phrase et un texte non vide (au plus [`MAX_TEXT`]
    /// caractères) exigés ; identifiants `extrait.<n>` uniques ; au plus [`MAX_SNIPPETS`]. Un extrait mal écrit est écarté seul. Une phrase
    /// vide, trop longue, déjà prise par une commande du programme ou par un extrait précédent est écartée (une phrase d'un seul mot est
    /// permise : la phrase doit être toute la transcription).
    pub fn cleaned(list: Vec<Snippet>) -> Vec<Snippet> {
        let taken = crate::commands::program_phrases();
        let mut seen_phrases: Vec<String> = Vec::new();
        let mut out: Vec<Snippet> = Vec::new();
        for mut snippet in list {
            if out.len() >= MAX_SNIPPETS {
                break;
            }
            snippet.name = snippet.name.split_whitespace().collect::<Vec<_>>().join(" ");
            let mut mine: Vec<String> = Vec::new();
            snippet.phrases = snippet
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
            let text = clean_text(&snippet.text);
            if is_snippet(&snippet.id)
                && !snippet.name.is_empty()
                && snippet.name.chars().count() <= MAX_NAME
                && !snippet.phrases.is_empty()
                && text.is_some()
                && !out.iter().any(|o| o.id == snippet.id)
            {
                snippet.text = text.unwrap_or_default();
                seen_phrases.extend(mine);
                out.push(snippet);
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

    fn snippet(id: &str, name: &str, phrases: &[&str], text: &str) -> Snippet {
        Snippet { id: id.into(), name: name.into(), phrases: phrases.iter().map(|p| (*p).to_string()).collect(), text: text.into() }
    }

    fn with(list: Vec<Snippet>) -> AppSettings {
        AppSettings { snippets: Snippet::cleaned(list), language: DictationLanguage::Multi, ..AppSettings::default() }
    }

    /// Un extrait se lit comme une commande sous la touche : la phrase est toute la transcription ; un mot seul est permis.
    #[test]
    fn commands_521_a_snippet_phrase_is_a_command_under_the_key_only() {
        let settings = with(vec![
            snippet("extrait.1", "Adresse", &["insérer mon adresse email", "mon adresse email"], "moi@example.com"),
            snippet("extrait.2", "Signature", &["signature"], "Cordialement,\nL'auteur"),
        ]);
        for (spoken, id) in [
            ("insérer mon adresse email", "extrait.1"),
            ("Insérer mon adresse email.", "extrait.1"),
            ("Mon adresse email", "extrait.1"),
            ("Signature", "extrait.2"),
        ] {
            let found = recognize_for(spoken, &settings, Context::UnderKey).unwrap_or_else(|| panic!("{spoken}"));
            assert_eq!(found.id, id, "{spoken}");
            assert_eq!((found.needs, found.argument), (Needs::Nothing, None), "pas de sélection, pas de complément : {spoken}");
        }
        // Une phrase plus longue (avant ou après) n'est pas la phrase de l'extrait.
        assert!(recognize_for("je dis signature", &settings, Context::UnderKey).is_none());
        assert!(recognize_for("signature de la lettre", &settings, Context::UnderKey).is_none());
        // Sans la touche : jamais, même quand les commandes vocales sont permises (non-régression : c'est du texte).
        assert!(settings.voice_commands_enabled);
        for spoken in ["insérer mon adresse email", "signature"] {
            assert!(recognize_for(spoken, &settings, Context::NoKey).is_none(), "{spoken}");
        }
        assert!(matches!(route_under_key("signature", &settings), Route::Command(c) if c.id == "extrait.2"));
    }

    /// Le texte d'un extrait est nettoyé à la lecture, un extrait mal écrit est écarté seul, les phrases prises le sont aussi.
    #[test]
    fn commands_522_snippets_are_cleaned_when_the_settings_are_read() {
        let long = "x".repeat(MAX_TEXT + 1);
        let exact = "y".repeat(MAX_TEXT);
        let kept = Snippet::cleaned(vec![
            snippet("extrait.1", "  Adresse   perso ", &["  mon   adresse ", "MON ADRESSE", "mon-adresse", ""], "  a@b.fr\r\nrue 1\u{7}  "),
            snippet("extrait.1", "Doublon d'identifiant", &["autre phrase"], "texte"),
            snippet("perso.1", "Mauvais préfixe", &["phrase"], "texte"),
            snippet("extrait.0", "Numéro zéro", &["phrase zéro"], "texte"),
            snippet("extrait.x", "Numéro absent", &["phrase x"], "texte"),
            snippet("extrait.2", "", &["sans nom"], "texte"),
            snippet("extrait.3", "Sans phrase", &[], "texte"),
            snippet("extrait.4", "Texte vide", &["texte vide"], "  \n "),
            snippet("extrait.5", "Texte trop long", &["texte long"], &long),
            snippet("extrait.6", "Texte à la limite", &["limite"], &exact),
            snippet(
                "extrait.7",
                "Phrase du programme",
                &["mettre en gras", "ajouter au dictionnaire", "nouvelle recherche google"],
                "texte",
            ),
            snippet("extrait.8", "Phrase déjà prise", &["mon adresse"], "texte"),
            snippet(&format!("extrait.{}", 9), &"n".repeat(MAX_NAME + 1), &["nom trop long"], "texte"),
        ]);
        let ids: Vec<&str> = kept.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, ["extrait.1", "extrait.6"], "un extrait mal écrit est écarté seul, les autres sont gardés");
        assert_eq!(kept[0].name, "Adresse perso");
        assert_eq!(kept[0].phrases, ["mon adresse"], "espaces ramenés à un, variantes de casse et de ponctuation et phrase vide écartées");
        assert_eq!(kept[0].text, "a@b.fr\nrue 1", "fins de ligne unifiées, contrôle retiré, bords nettoyés");
        assert_eq!(kept[1].text.chars().count(), MAX_TEXT);
        // Au plus 50 extraits.
        let many: Vec<Snippet> = (1..=60).map(|n| snippet(&format!("extrait.{n}"), "N", &[&format!("phrase numéro {n}")], "t")).collect();
        assert_eq!(Snippet::cleaned(many).len(), MAX_SNIPPETS);
        // Le texte ne se lit pas dans un `{:?}`.
        let shown = format!("{:?}", snippet("extrait.1", "SECRET-NOM", &["SECRET-PHRASE"], "FR76 SECRET-IBAN"));
        assert!(!shown.contains("SECRET"), "{shown}");
    }

    /// Identifiants : seulement `extrait.<n>`, n ≥ 1 ; le prochain est libre ; le texte se lit par l'identifiant.
    #[test]
    fn commands_523_ids_next_id_and_text() {
        assert!(is_snippet("extrait.1") && is_snippet("extrait.42"));
        for no in ["extrait.", "extrait.0", "extrait.-1", "extrait.1a", "extrait.nouveau", "recherche.1", "perso.1", "extrait", ""] {
            assert!(!is_snippet(no), "{no}");
        }
        let list = vec![snippet("extrait.1", "A", &["a b"], "t"), snippet("extrait.4", "B", &["c d"], "u")];
        assert_eq!(Snippet::next_id(&list), "extrait.5");
        assert_eq!(Snippet::next_id(&[]), "extrait.1");
        let settings = with(list);
        assert_eq!(text_of("extrait.4", &settings), Some("u"));
        assert_eq!(text_of("extrait.9", &settings), None);
    }

    /// Une phrase d'extrait ne peut pas être celle d'une autre commande (programme, site, commande perso) : `validate_phrase` la refuse, et
    /// un extrait pas encore créé (`extrait.nouveau`) se contrôle comme les autres.
    #[test]
    fn commands_524_a_phrase_belongs_to_one_command_only() {
        let settings = with(vec![snippet("extrait.1", "Adresse", &["mon adresse"], "t")]);
        let defs = crate::commands::all_defs(&settings);
        let none = crate::commands::VoicePhrases::default();
        assert_eq!(
            validate_phrase("mon adresse", NEW, Lang::Fr, &defs, &none),
            Err(PhraseError::Taken("extrait.1".into())),
            "déjà à un extrait"
        );
        assert_eq!(
            validate_phrase("mettre en gras", NEW, Lang::Fr, &defs, &none),
            Err(PhraseError::Taken("format.gras".into())),
            "à une commande du programme"
        );
        assert_eq!(
            validate_phrase("nouvelle recherche youtube", NEW, Lang::Fr, &defs, &none),
            Err(PhraseError::Taken("recherche.2".into()))
        );
        assert!(validate_phrase("signature", NEW, Lang::Fr, &defs, &none).is_ok(), "un mot seul est permis pour un extrait");
        assert_eq!(validate_phrase("x", "extrait.9", Lang::Fr, &defs, &none), Err(PhraseError::UnknownCommand));
        assert!(validate_phrase("mon adresse", "extrait.1", Lang::Fr, &defs, &none).is_ok(), "sa propre phrase");
    }
}
