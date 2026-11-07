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
    /// Langue du catalogue dont la phrase est venue (les guillemets « » en français, " " en anglais en dépendent).
    pub lang: Lang,
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

/// Préfixe des identifiants des commandes gras, italique et souligné (tâche 8) : `format.gras`, `format.italique`, `format.souligne`.
pub const FORMAT_PREFIX: &str = "format.";

/// Les raccourcis de l'appli que les commandes gras, italique et souligné appuient (réglables ; `Mod` = ⌘ sur Mac, Ctrl ailleurs).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormatShortcuts {
    #[serde(default = "FormatShortcuts::default_bold")]
    pub bold: String,
    #[serde(default = "FormatShortcuts::default_italic")]
    pub italic: String,
    #[serde(default = "FormatShortcuts::default_underline")]
    pub underline: String,
    #[serde(default = "FormatShortcuts::default_strikethrough")]
    pub strikethrough: String,
}

impl FormatShortcuts {
    fn default_bold() -> String {
        "Mod+B".into()
    }
    fn default_italic() -> String {
        "Mod+I".into()
    }
    fn default_underline() -> String {
        "Mod+U".into()
    }
    /// Le barré n'a pas de raccourci commun à toutes les applis ; ⌘⇧X est celui de Notes, de Google Docs et de Word pour Mac.
    fn default_strikethrough() -> String {
        "Mod+Shift+X".into()
    }

    /// Les raccourcis lus dans un fichier écrit à la main : un raccourci illisible (ou sans modificateur) redevient celui par défaut.
    pub fn cleaned(self) -> Self {
        let good =
            |own: String, default: String| if crate::shortcut::Shortcut::parse(&own).is_some() { own.trim().to_string() } else { default };
        Self {
            bold: good(self.bold, Self::default_bold()),
            italic: good(self.italic, Self::default_italic()),
            underline: good(self.underline, Self::default_underline()),
            strikethrough: good(self.strikethrough, Self::default_strikethrough()),
        }
    }

    /// Le raccourci (texte) de la commande `id` ; `None` : pas une commande de mise en forme.
    pub fn for_command(&self, id: &str) -> Option<&str> {
        match id.strip_prefix(FORMAT_PREFIX)? {
            "gras" => Some(&self.bold),
            "italique" => Some(&self.italic),
            "souligne" => Some(&self.underline),
            "barre" => Some(&self.strikethrough),
            _ => None,
        }
    }
}

impl Default for FormatShortcuts {
    fn default() -> Self {
        Self {
            bold: Self::default_bold(),
            italic: Self::default_italic(),
            underline: Self::default_underline(),
            strikethrough: Self::default_strikethrough(),
        }
    }
}

/// Préfixe des identifiants des commandes « caractères autour » (tâche 7) : `entourer.parentheses`, `entourer.guillemets`, `entourer.crochets`,
/// `entourer.accolades`.
pub const SURROUND_PREFIX: &str = "entourer.";

/// Les caractères mis autour d'une sélection par la commande `id` (ouvrant, fermant), selon la langue de la phrase dite :
/// guillemets français « » (espaces insécables) ou anglais " ". `None` : identifiant qui n'est pas une commande « autour ».
fn pair(id: &str, lang: Lang) -> Option<(&'static str, &'static str)> {
    match (id.strip_prefix(SURROUND_PREFIX)?, lang) {
        ("parentheses", _) => Some(("(", ")")),
        ("crochets", _) => Some(("[", "]")),
        ("accolades", _) => Some(("{", "}")),
        ("guillemets", Lang::Fr) => Some(("«\u{00A0}", "\u{00A0}»")),
        ("guillemets", Lang::En) => Some(("\"", "\"")),
        _ => None,
    }
}

/// Le texte sélectionné entouré par la commande `id`. Les blancs et sauts de ligne de bord restent dehors (« mot␠ » → « (mot)␠ »), pour que le
/// signe se colle au texte. `None` : pas une commande « autour », ou rien d'autre que des blancs à entourer.
pub fn surround(id: &str, lang: Lang, selected: &str) -> Option<String> {
    let (open, close) = pair(id, lang)?;
    let core = selected.trim();
    if core.is_empty() {
        return None;
    }
    let start = selected.len() - selected.trim_start().len();
    let end = selected.trim_end().len();
    Some(format!("{}{open}{core}{close}{}", &selected[..start], &selected[end..]))
}

/// Les commandes connues : « ajouter au dictionnaire » (tâche 3), « caractères autour » (tâche 7). Les suivantes (gras…) s'ajoutent ici.
pub fn builtin() -> Vec<CommandDef> {
    let v = |l: &[&str]| l.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();
    // « Entre parenthèses », « mets entre parenthèses »… : une commande sur sélection, sans complément.
    let around = |id: &str, fr: &[&str], en: &[&str]| CommandDef {
        id: format!("{SURROUND_PREFIX}{id}"),
        needs: Needs::Selection,
        argument: Argument::None,
        phrases_fr: v(fr),
        phrases_en: v(en),
    };
    let format = |id: &str, fr: &[&str], en: &[&str]| CommandDef {
        id: format!("{FORMAT_PREFIX}{id}"),
        needs: Needs::Selection,
        argument: Argument::None,
        phrases_fr: v(fr),
        phrases_en: v(en),
    };
    vec![
        format(
            "gras",
            &["mettre en gras", "mets en gras", "met en gras", "mais en gras", "mettez en gras", "passe en gras", "en gras"],
            &["make it bold", "make bold", "put in bold", "in bold"],
        ),
        format(
            "italique",
            &[
                "mettre en italique",
                "mets en italique",
                "met en italique",
                "mais en italique",
                "mettez en italique",
                "passe en italique",
                "en italique",
            ],
            &["make it italic", "make italic", "put in italics", "in italics"],
        ),
        format(
            "souligne",
            &[
                "mettre en souligné",
                "mets en souligné",
                "met en souligné",
                "mais en souligné",
                "mettez en souligné",
                "souligne le texte",
                "souligne ça",
            ],
            &["make it underlined", "underline it", "underline this", "put in underline"],
        ),
        format(
            "barre",
            &["mettre en barré", "mets en barré", "met en barré", "mais en barré", "mettez en barré", "barre le texte", "barrer le texte"],
            &["strike it through", "strike through", "make it strikethrough", "put in strikethrough"],
        ),
        around(
            "parentheses",
            &[
                "mettre entre parenthèses",
                "mets entre parenthèses",
                "entre parenthèses",
                "mettez entre parenthèses",
                "entoure de parenthèses",
                "entourer de parenthèses",
            ],
            &["in parentheses", "put in parentheses", "put it in parentheses", "wrap in parentheses", "put between parentheses"],
        ),
        around(
            "guillemets",
            &[
                "mettre entre guillemets",
                "mets entre guillemets",
                "entre guillemets",
                "mettez entre guillemets",
                "entoure de guillemets",
                "entourer de guillemets",
            ],
            &["in quotes", "put in quotes", "put it in quotes", "wrap in quotes", "in quotation marks", "put in quotation marks"],
        ),
        around(
            "crochets",
            &[
                "mettre entre crochets",
                "mets entre crochets",
                "entre crochets",
                "mettez entre crochets",
                "entoure de crochets",
                "entourer de crochets",
            ],
            &["in brackets", "put in brackets", "put it in brackets", "in square brackets", "put in square brackets", "wrap in brackets"],
        ),
        around(
            "accolades",
            &[
                "mettre entre accolades",
                "mets entre accolades",
                "entre accolades",
                "mettez entre accolades",
                "entoure d'accolades",
                "entourer d'accolades",
            ],
            &["in curly brackets", "put in curly brackets", "put it in curly brackets", "in braces", "put in braces", "wrap in braces"],
        ),
        CommandDef {
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
        },
    ]
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
    let mut best: Option<(usize, &CommandDef, Option<usize>, Lang)> = None;
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
                if starts && fits && best.as_ref().is_none_or(|(len, _, _, _)| n > *len) {
                    best = Some((n, def, Some(spoken[n - 1].1), *lang));
                }
            }
        }
    }
    let (_, def, cut, lang) = best?;
    let argument = match def.argument {
        Argument::None => None,
        Argument::Rest => {
            let rest = raw[cut?..].trim_matches(|c: char| !c.is_alphanumeric());
            Some(rest.to_string())
        }
    };
    Some(Recognized { id: def.id.clone(), lang, needs: def.needs, argument })
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
        let both = vec![builtin().into_iter().find(|d| d.id == ADD).unwrap(), other];
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

    // ---- Tâche 7 : caractères autour ----

    fn id_of(raw: &str, lang: DictationLanguage) -> Option<(String, Lang)> {
        rec(raw, lang).map(|r| (r.id, r.lang))
    }

    #[test]
    fn commands_201_each_surround_phrase_is_recognized_with_its_language() {
        for (raw, id, l) in [
            ("Entre parenthèses.", "entourer.parentheses", Lang::Fr),
            ("Mets entre guillemets", "entourer.guillemets", Lang::Fr),
            ("entre crochets", "entourer.crochets", Lang::Fr),
            ("Entoure d'accolades !", "entourer.accolades", Lang::Fr),
        ] {
            assert_eq!(id_of(raw, DictationLanguage::Fr), Some((id.to_string(), l)), "{raw}");
        }
        for (raw, id) in [
            ("In parentheses.", "entourer.parentheses"),
            ("Put it in quotes", "entourer.guillemets"),
            ("in square brackets", "entourer.crochets"),
            ("Wrap in braces", "entourer.accolades"),
        ] {
            assert_eq!(id_of(raw, DictationLanguage::En), Some((id.to_string(), Lang::En)), "{raw}");
        }
        // Les deux langues ensemble : la langue de la phrase dite est rendue.
        assert_eq!(id_of("entre guillemets", DictationLanguage::Multi).map(|r| r.1), Some(Lang::Fr));
        assert_eq!(id_of("in quotes", DictationLanguage::Multi).map(|r| r.1), Some(Lang::En));
        // Pas dans la mauvaise langue.
        assert!(id_of("in quotes", DictationLanguage::Fr).is_none());
        assert!(id_of("entre guillemets", DictationLanguage::En).is_none());
    }

    #[test]
    fn commands_202_surround_needs_a_selection_and_is_not_a_longer_sentence() {
        assert!(builtin()
            .iter()
            .filter(|d| d.id.starts_with(SURROUND_PREFIX))
            .all(|d| d.needs == Needs::Selection && d.argument == Argument::None));
        for raw in [
            "Mets le tout entre parenthèses",
            "entre parenthèses et crochets",
            "j'ai mis entre parenthèses",
            "parenthèses",
            "put the word in quotes",
        ] {
            assert!(id_of(raw, DictationLanguage::Multi).is_none(), "{raw}");
        }
    }

    #[test]
    fn commands_203_no_phrase_is_shared_between_two_commands() {
        let defs = builtin();
        for lang in [Lang::Fr, Lang::En] {
            let mut seen: Vec<(String, String)> = Vec::new();
            for d in &defs {
                for p in effective_phrases(d, &VoicePhrases::default(), lang) {
                    assert!(word_count(&normalize(&p)) >= MIN_WORDS, "{p}");
                    let n = normalize(&p);
                    assert!(!seen.iter().any(|(_, s)| *s == n), "phrase en double : {p}");
                    seen.push((d.id.clone(), n));
                }
            }
        }
    }

    #[test]
    fn commands_204_surround_writes_the_pair_for_each_command() {
        let s = |id: &str, lang, text: &str| surround(&format!("entourer.{id}"), lang, text);
        assert_eq!(s("parentheses", Lang::Fr, "mot"), Some("(mot)".into()));
        assert_eq!(s("parentheses", Lang::En, "word"), Some("(word)".into()));
        assert_eq!(s("crochets", Lang::Fr, "mot"), Some("[mot]".into()));
        assert_eq!(s("accolades", Lang::En, "word"), Some("{word}".into()));
        assert_eq!(s("guillemets", Lang::Fr, "mot"), Some("«\u{a0}mot\u{a0}»".into()));
        assert_eq!(s("guillemets", Lang::En, "word"), Some("\"word\"".into()));
    }

    #[test]
    fn commands_205_surround_keeps_edge_blanks_outside_and_refuses_nothing_to_wrap() {
        assert_eq!(surround("entourer.parentheses", Lang::Fr, " un mot \n"), Some(" (un mot) \n".into()));
        assert_eq!(surround("entourer.parentheses", Lang::Fr, "a\nb"), Some("(a\nb)".into()));
        assert_eq!(surround("entourer.parentheses", Lang::Fr, " \n\t"), None);
        assert_eq!(surround("entourer.parentheses", Lang::Fr, ""), None);
        assert_eq!(surround("dictionnaire.ajouter", Lang::Fr, "mot"), None);
        assert_eq!(surround("entourer.inconnu", Lang::Fr, "mot"), None);
    }

    #[test]
    fn commands_206_a_chosen_phrase_cannot_take_one_of_another_surround_command() {
        let defs = builtin();
        let custom = VoicePhrases::default();
        assert_eq!(
            validate_phrase("entre crochets", "entourer.parentheses", Lang::Fr, &defs, &custom),
            Err(PhraseError::Taken("entourer.crochets".into()))
        );
        assert_eq!(validate_phrase("parenthèse autour", "entourer.parentheses", Lang::Fr, &defs, &custom), Ok("parenthèse autour".into()));
    }

    // ---- Tâche 8 : gras, italique, souligné ----

    #[test]
    fn commands_301_format_phrases_are_recognized_in_both_languages() {
        for (raw, id) in [
            ("Mets en gras.", "format.gras"),
            ("Passe en italique", "format.italique"),
            ("Mets en souligné !", "format.souligne"),
            ("En gras", "format.gras"),
            // Whisper entend souvent « mets » comme « mais » (essai réel du 07/11/2026).
            ("Mais en gras.", "format.gras"),
            ("mais en italique", "format.italique"),
            ("Mais en souligné", "format.souligne"),
            ("Mettre en barré.", "format.barre"),
            ("mets en barre", "format.barre"),
        ] {
            assert_eq!(id_of(raw, DictationLanguage::Fr), Some((id.to_string(), Lang::Fr)), "{raw}");
        }
        for (raw, id) in [
            ("Make it bold.", "format.gras"),
            ("make italic", "format.italique"),
            ("Underline it", "format.souligne"),
            ("Strike it through", "format.barre"),
            ("In bold", "format.gras"),
        ] {
            assert_eq!(id_of(raw, DictationLanguage::En), Some((id.to_string(), Lang::En)), "{raw}");
        }
        for raw in ["Mets le titre en gras", "gras", "bold", "make the title bold"] {
            assert!(id_of(raw, DictationLanguage::Multi).is_none(), "{raw}");
        }
    }

    #[test]
    fn commands_302_format_commands_need_a_selection_and_have_no_argument() {
        let defs = builtin();
        let format: Vec<_> = defs.iter().filter(|d| d.id.starts_with(FORMAT_PREFIX)).collect();
        assert_eq!(format.len(), 4);
        assert!(format.iter().all(|d| d.needs == Needs::Selection && d.argument == Argument::None));
    }

    #[test]
    fn commands_303_format_shortcuts_default_to_mod_b_i_u_and_unreadable_ones_are_corrected() {
        let d = FormatShortcuts::default();
        assert_eq!(
            (d.for_command("format.gras"), d.for_command("format.italique"), d.for_command("format.souligne")),
            (Some("Mod+B"), Some("Mod+I"), Some("Mod+U"))
        );
        assert_eq!(d.for_command("entourer.parentheses"), None);
        let fixed =
            FormatShortcuts { bold: "G".into(), italic: " Ctrl+Alt+I ".into(), underline: "".into(), strikethrough: "x".into() }.cleaned();
        assert_eq!((fixed.bold.as_str(), fixed.italic.as_str(), fixed.underline.as_str()), ("Mod+B", "Ctrl+Alt+I", "Mod+U"));
        assert_eq!(fixed.strikethrough, "Mod+Shift+X");
    }

    #[test]
    fn commands_304_no_phrase_is_shared_across_all_commands_including_format_ones() {
        // Reprend la règle de commands_203 avec la liste complète (la liste a grandi).
        let defs = builtin();
        for lang in [Lang::Fr, Lang::En] {
            let mut seen = std::collections::BTreeSet::new();
            for d in &defs {
                for p in effective_phrases(d, &VoicePhrases::default(), lang) {
                    assert!(seen.insert(normalize(&p)), "phrase en double : {p}");
                }
            }
        }
    }
}
