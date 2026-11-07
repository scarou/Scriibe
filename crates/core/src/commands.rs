//! Commandes vocales « verbe + objet » : reconnaissance d'une phrase de commande dans la transcription brute.
//!
//! Couche pure (aucune entrée-sortie, aucun système) décrite dans `docs/COMMANDES.md` (Conception) : la phrase doit être **toute** la
//! transcription (option A), comparée sans ponctuation, majuscules ni accents. La lecture de la sélection et l'exécution vivent ailleurs
//! (`crates/platform`, `crates/pipeline`) : ici on ne dit que **quelle** commande a été dite, et son complément éventuel.
//!
//! Extensible : les extraits (ROADMAP n°4) et la recherche web (n°7) ajoutent leurs propres [`CommandDef`] à la liste de [`builtin`].

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::settings::{AppSettings, DictationLanguage};

/// Une phrase de commande compte au moins deux mots : un mot seul (« gras ») est trop souvent du texte à coller.
pub const MIN_WORDS: usize = 2;

/// Ce qu'une commande exige pour s'exécuter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Needs {
    /// Un texte sélectionné ; sans lui, la phrase reste du texte à coller.
    Selection,
    /// Rien de particulier (extraits, recherche web).
    Nothing,
}

/// Ce qui suit la phrase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Argument {
    /// Rien : la phrase est toute la transcription.
    None,
    /// Le reste de la transcription, non vide (« recherche YouTube » + des mots).
    Rest,
}

/// Fiche d'une commande : identifiant « verbe.objet », exigences et phrases par défaut.
#[derive(Debug, Clone)]
pub struct CommandDef {
    pub id: String,
    pub needs: Needs,
    pub argument: Argument,
    pub phrases_fr: Vec<String>,
    pub phrases_en: Vec<String>,
}

/// Phrases choisies par la personne, par langue puis par identifiant de commande ; elles **remplacent** celles par défaut.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoicePhrases {
    #[serde(default)]
    pub fr: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub en: BTreeMap<String, Vec<String>>,
}

impl VoicePhrases {
    /// Phrases lues dans un fichier de réglages écrit à la main : espaces retirés, vides et doublons écartés.
    pub fn cleaned(self) -> Self {
        fn clean(m: BTreeMap<String, Vec<String>>) -> BTreeMap<String, Vec<String>> {
            m.into_iter()
                .filter_map(|(id, list)| {
                    let mut seen = Vec::new();
                    let mut out = Vec::new();
                    for p in list {
                        let p = p.split_whitespace().collect::<Vec<_>>().join(" ");
                        let n = normalize(&p);
                        if !n.is_empty() && !seen.contains(&n) {
                            seen.push(n);
                            out.push(p);
                        }
                    }
                    (!out.is_empty()).then_some((id, out))
                })
                .collect()
        }
        Self { fr: clean(self.fr), en: clean(self.en) }
    }

    fn for_language(&self, lang: Lang) -> &BTreeMap<String, Vec<String>> {
        match lang {
            Lang::Fr => &self.fr,
            Lang::En => &self.en,
        }
    }
}

/// Langue d'un catalogue de phrases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Fr,
    En,
}

/// La commande reconnue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recognized {
    pub id: String,
    pub needs: Needs,
    /// Le reste de la transcription pour une commande à complément (texte tel que transcrit, ponctuation de bord retirée).
    pub argument: Option<String>,
}

/// Pourquoi une phrase écrite par la personne est refusée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhraseError {
    /// Moins de [`MIN_WORDS`] mots.
    TooShort,
    /// Déjà utilisée par une autre commande (son identifiant).
    Taken(String),
    /// Commande inconnue.
    UnknownCommand,
}

/// Les commandes connues. Aujourd'hui une seule (tâche 3) ; les suivantes (caractères autour, gras…) s'ajoutent ici.
pub fn builtin() -> Vec<CommandDef> {
    let v = |l: &[&str]| l.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();
    vec![CommandDef {
        id: "dictionnaire.ajouter".into(),
        needs: Needs::Selection,
        argument: Argument::None,
        phrases_fr: v(&[
            "ajouter au dictionnaire",
            "ajoute au dictionnaire",
            "ajoutez au dictionnaire",
            "ajouter ça au dictionnaire",
            "ajoute ça au dictionnaire",
            "ajouter ce mot au dictionnaire",
            "ajoute ce mot au dictionnaire",
        ]),
        phrases_en: v(&[
            "add to dictionary",
            "add to the dictionary",
            "add this to dictionary",
            "add this to the dictionary",
            "add that to the dictionary",
            "add this word to the dictionary",
        ]),
    }]
}

/// Les mots d'un texte sous forme normalisée, avec la fin (en octets) de chacun dans le texte d'origine.
fn words(text: &str) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut end = 0;
    for (i, c) in text.char_indices() {
        if c.is_alphanumeric() {
            fold_char(c, &mut cur);
            end = i + c.len_utf8();
        } else if !cur.is_empty() {
            out.push((std::mem::take(&mut cur), end));
        }
    }
    if !cur.is_empty() {
        out.push((cur, end));
    }
    out
}

/// Minuscule sans accent (œ → oe, æ → ae, ß → ss) ; les autres caractères restent tels quels, en minuscules.
fn fold_char(c: char, out: &mut String) {
    for l in c.to_lowercase() {
        match l {
            'à' | 'â' | 'ä' | 'á' | 'ã' | 'å' => out.push('a'),
            'ç' => out.push('c'),
            'é' | 'è' | 'ê' | 'ë' => out.push('e'),
            'î' | 'ï' | 'í' | 'ì' => out.push('i'),
            'ñ' => out.push('n'),
            'ô' | 'ö' | 'ó' | 'ò' | 'õ' => out.push('o'),
            'ù' | 'û' | 'ü' | 'ú' => out.push('u'),
            'ÿ' | 'ý' => out.push('y'),
            'œ' => out.push_str("oe"),
            'æ' => out.push_str("ae"),
            'ß' => out.push_str("ss"),
            other => out.push(other),
        }
    }
}

/// Forme comparée : minuscules, accents retirés, ponctuation, apostrophes, tirets et espaces ramenés à un seul espace.
pub fn normalize(text: &str) -> String {
    words(text).into_iter().map(|(w, _)| w).collect::<Vec<_>>().join(" ")
}

fn word_count(normalized: &str) -> usize {
    normalized.split(' ').filter(|w| !w.is_empty()).count()
}

/// Les phrases en vigueur d'une commande : celles de la personne si elle en a donné, sinon celles par défaut.
pub fn effective_phrases(def: &CommandDef, custom: &VoicePhrases, lang: Lang) -> Vec<String> {
    match custom.for_language(lang).get(&def.id) {
        Some(list) if !list.is_empty() => list.clone(),
        _ => match lang {
            Lang::Fr => def.phrases_fr.clone(),
            Lang::En => def.phrases_en.clone(),
        },
    }
}

/// Les catalogues à tester pour cette langue de dictée : la sienne, ou les deux pour « plusieurs langues ».
fn catalogues(language: DictationLanguage) -> &'static [Lang] {
    match language {
        DictationLanguage::Fr => &[Lang::Fr],
        DictationLanguage::En => &[Lang::En],
        DictationLanguage::Multi => &[Lang::Fr, Lang::En],
    }
}

/// Reconnaît une commande dans la transcription brute `raw`. `None` : c'est du texte à traiter comme d'habitude.
///
/// La phrase doit être toute la transcription ; pour une commande à complément, la phrase puis un reste non vide. À égalité de
/// début, la phrase la plus longue l'emporte ; si deux commandes ont la même phrase, la première de `defs` l'emporte.
/// Les phrases de moins de [`MIN_WORDS`] mots (écrites à la main dans un fichier) sont ignorées.
pub fn recognize(raw: &str, defs: &[CommandDef], custom: &VoicePhrases, language: DictationLanguage) -> Option<Recognized> {
    let spoken = words(raw);
    if spoken.len() < MIN_WORDS {
        return None;
    }
    let mut best: Option<(usize, &CommandDef, Option<usize>)> = None;
    for def in defs {
        for lang in catalogues(language) {
            for phrase in effective_phrases(def, custom, *lang) {
                let normalized = normalize(&phrase);
                let n = word_count(&normalized);
                if n < MIN_WORDS || n > spoken.len() {
                    continue;
                }
                let starts = spoken.iter().take(n).map(|(w, _)| w.as_str()).eq(normalized.split(' '));
                let fits = match def.argument {
                    Argument::None => spoken.len() == n,
                    Argument::Rest => spoken.len() > n,
                };
                if starts && fits && best.as_ref().is_none_or(|(len, _, _)| n > *len) {
                    best = Some((n, def, Some(spoken[n - 1].1)));
                }
            }
        }
    }
    let (_, def, cut) = best?;
    let argument = match def.argument {
        Argument::None => None,
        Argument::Rest => {
            let rest = raw[cut?..].trim_matches(|c: char| !c.is_alphanumeric());
            Some(rest.to_string())
        }
    };
    Some(Recognized { id: def.id.clone(), needs: def.needs, argument })
}

/// Reconnaissance avec les réglages : interrupteur général, langue de dictée, phrases de la personne.
pub fn recognize_for(raw: &str, settings: &AppSettings) -> Option<Recognized> {
    if !settings.voice_commands_enabled {
        return None;
    }
    recognize(raw, &builtin(), &settings.voice_commands, settings.language)
}

/// Contrôle d'une phrase que la personne veut donner à la commande `id` (dans la langue `lang`) : au moins deux mots, et pas déjà
/// celle d'une autre commande (comparée sous forme normalisée, phrases par défaut ou choisies). Rend la phrase nettoyée.
pub fn validate_phrase(phrase: &str, id: &str, lang: Lang, defs: &[CommandDef], custom: &VoicePhrases) -> Result<String, PhraseError> {
    if !defs.iter().any(|d| d.id == id) {
        return Err(PhraseError::UnknownCommand);
    }
    let normalized = normalize(phrase);
    if word_count(&normalized) < MIN_WORDS {
        return Err(PhraseError::TooShort);
    }
    for def in defs.iter().filter(|d| d.id != id) {
        if effective_phrases(def, custom, lang).iter().any(|p| normalize(p) == normalized) {
            return Err(PhraseError::Taken(def.id.clone()));
        }
    }
    Ok(phrase.split_whitespace().collect::<Vec<_>>().join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ADD: &str = "dictionnaire.ajouter";

    fn rec(raw: &str, lang: DictationLanguage) -> Option<Recognized> {
        recognize(raw, &builtin(), &VoicePhrases::default(), lang)
    }

    fn only(raw: &str) -> bool {
        rec(raw, DictationLanguage::Fr).is_some_and(|r| r.id == ADD && r.argument.is_none() && r.needs == Needs::Selection)
    }

    /// Cas 1 : la phrase exacte est reconnue.
    #[test]
    fn commands_001_exact_phrase() {
        assert!(only("ajouter au dictionnaire"));
    }

    /// Cas 2 : ponctuation, majuscules, accents et espaces sont ignorés.
    #[test]
    fn commands_002_punctuation_case_and_accents_are_ignored() {
        assert!(only("Ajoute au *Dictionnaire* !"));
        assert!(only("Ajouter au dictionnaire."));
        assert!(only(", AJOUTER  AU   DICTIONNAIRE…"));
        assert!(only("Ajoute ca au dictionnaire")); // « ça » sans cédille
        assert!(only("Ajoute ça au dictionnaire"));
        assert_eq!(normalize("L’œuf — Été-Ça"), "l oeuf ete ca");
    }

    /// Cas 3 : une phrase plus longue (avant ou après) n'est pas une commande.
    #[test]
    fn commands_003_a_longer_sentence_is_not_a_command() {
        assert!(rec("ajouter au dictionnaire le mot cri be", DictationLanguage::Fr).is_none());
        assert!(rec("je voudrais ajouter au dictionnaire", DictationLanguage::Fr).is_none());
        assert!(rec("ajouter", DictationLanguage::Fr).is_none());
        assert!(rec("", DictationLanguage::Fr).is_none());
    }

    /// Cas 4 : la reconnaissance ne dépend pas de la sélection ; c'est l'appelant qui, sans sélection, colle la phrase. Ici : la fiche
    /// dit qu'elle exige une sélection, et une fiche sans exigence se reconnaît de la même façon.
    #[test]
    fn commands_004_the_definition_says_whether_a_selection_is_needed() {
        let r = rec("ajouter au dictionnaire", DictationLanguage::Fr).unwrap();
        assert_eq!(r.needs, Needs::Selection);
        let free = CommandDef {
            id: "extrait.insérer".into(),
            needs: Needs::Nothing,
            argument: Argument::None,
            phrases_fr: vec!["insérer mon adresse".into()],
            phrases_en: vec![],
        };
        let r = recognize("Insérer mon adresse.", &[free], &VoicePhrases::default(), DictationLanguage::Fr).unwrap();
        assert_eq!((r.id.as_str(), r.needs), ("extrait.insérer", Needs::Nothing));
    }

    /// Cas 5 : l'anglais ; « plusieurs langues » teste les deux catalogues ; le français seul ne reconnaît pas l'anglais.
    #[test]
    fn commands_005_english_and_both_languages() {
        assert!(rec("Add to dictionary.", DictationLanguage::En).is_some());
        assert!(rec("add this to the dictionary", DictationLanguage::En).is_some());
        assert!(rec("add to dictionary", DictationLanguage::Fr).is_none());
        assert!(rec("ajouter au dictionnaire", DictationLanguage::En).is_none());
        assert!(rec("add to dictionary", DictationLanguage::Multi).is_some());
        assert!(rec("ajouter au dictionnaire", DictationLanguage::Multi).is_some());
    }

    /// Cas 6 : une phrase choisie remplace celles par défaut ; refus d'une phrase prise ou d'un seul mot.
    #[test]
    fn commands_006_custom_phrase_replaces_the_defaults_and_is_checked() {
        let mut custom = VoicePhrases::default();
        custom.fr.insert(ADD.into(), vec!["mémorise ce mot".into()]);
        assert!(recognize("Mémorise ce mot !", &builtin(), &custom, DictationLanguage::Fr).is_some());
        assert!(recognize("ajouter au dictionnaire", &builtin(), &custom, DictationLanguage::Fr).is_none());
        // L'anglais garde ses phrases par défaut.
        assert!(recognize("add to dictionary", &builtin(), &custom, DictationLanguage::En).is_some());
        // Un seul mot, ou pas de mot : refusé.
        let defs = builtin();
        assert_eq!(validate_phrase("mémorise", ADD, Lang::Fr, &defs, &custom), Err(PhraseError::TooShort));
        assert_eq!(validate_phrase(" … ", ADD, Lang::Fr, &defs, &custom), Err(PhraseError::TooShort));
        assert_eq!(validate_phrase("x", "inconnue", Lang::Fr, &defs, &custom), Err(PhraseError::UnknownCommand));
        assert_eq!(validate_phrase("  Mémorise   ce  mot ", ADD, Lang::Fr, &defs, &custom).as_deref(), Ok("Mémorise ce mot"));
        // Déjà prise par une autre commande (comparée sous forme normalisée).
        let other = CommandDef {
            id: "autre.chose".into(),
            needs: Needs::Selection,
            argument: Argument::None,
            phrases_fr: vec!["mets en gras".into()],
            phrases_en: vec![],
        };
        let both = vec![builtin().remove(0), other];
        assert_eq!(validate_phrase("Mets en GRAS.", ADD, Lang::Fr, &both, &custom), Err(PhraseError::Taken("autre.chose".into())));
    }

    /// Une phrase d'un seul mot écrite à la main dans le fichier de réglages est ignorée, sans bloquer les autres.
    #[test]
    fn commands_006_one_word_phrases_from_a_file_are_ignored() {
        let mut custom = VoicePhrases::default();
        custom.fr.insert(ADD.into(), vec!["mémorise".into(), "retiens ça".into()]);
        assert!(recognize("mémorise", &builtin(), &custom, DictationLanguage::Fr).is_none());
        assert!(recognize("retiens ça", &builtin(), &custom, DictationLanguage::Fr).is_some());
    }

    /// Commande à complément : la phrase puis un reste non vide, rendu tel que transcrit.
    #[test]
    fn commands_extra_argument_is_the_rest_of_the_sentence() {
        let search = CommandDef {
            id: "recherche.youtube".into(),
            needs: Needs::Nothing,
            argument: Argument::Rest,
            phrases_fr: vec!["recherche YouTube".into()],
            phrases_en: vec![],
        };
        let defs = [search];
        let none = VoicePhrases::default();
        let r = recognize("Recherche YouTube, Chats rigolos !", &defs, &none, DictationLanguage::Fr).unwrap();
        assert_eq!(r.argument.as_deref(), Some("Chats rigolos"));
        assert!(recognize("Recherche YouTube.", &defs, &none, DictationLanguage::Fr).is_none());
    }

    /// Entre deux phrases qui commencent pareil, la plus longue l'emporte.
    #[test]
    fn commands_extra_the_longest_phrase_wins() {
        let a = CommandDef {
            id: "a".into(),
            needs: Needs::Nothing,
            argument: Argument::Rest,
            phrases_fr: vec!["recherche sur le web".into()],
            phrases_en: vec![],
        };
        let b = CommandDef {
            id: "b".into(),
            needs: Needs::Nothing,
            argument: Argument::Rest,
            phrases_fr: vec!["recherche sur le web images".into()],
            phrases_en: vec![],
        };
        let r = recognize("recherche sur le web images chats", &[a, b], &VoicePhrases::default(), DictationLanguage::Fr).unwrap();
        assert_eq!((r.id.as_str(), r.argument.as_deref()), ("b", Some("chats")));
    }

    /// Interrupteur général et réglages lus d'un fichier.
    #[test]
    fn commands_extra_settings_switch_and_file() {
        let on = AppSettings::from_json("{}").unwrap();
        assert!(on.voice_commands_enabled);
        assert!(recognize_for("Ajouter au dictionnaire.", &on).is_some());
        let off = AppSettings::from_json(r#"{"voiceCommandsEnabled": false}"#).unwrap();
        assert!(recognize_for("Ajouter au dictionnaire.", &off).is_none());
        let custom =
            AppSettings::from_json(r#"{"voiceCommands": {"fr": {"dictionnaire.ajouter": ["  mémorise   ça ", "", "Mémorise ça"]}}}"#)
                .unwrap();
        assert_eq!(custom.voice_commands.fr[ADD], vec!["mémorise ça".to_string()]);
        assert!(recognize_for("Mémorise ça.", &custom).is_some());
        // Valeur incorrecte : réglage par défaut.
        let bad = AppSettings::from_json(r#"{"voiceCommands": 3, "voiceCommandsEnabled": "oui"}"#).unwrap();
        assert!(bad.voice_commands_enabled && bad.voice_commands == VoicePhrases::default());
    }
}
