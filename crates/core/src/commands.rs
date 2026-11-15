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

/// Où une commande vaut : partout, ou seulement quand la touche de commande est tenue (chantier 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Sans la touche comme sous la touche : les commandes d'avant le chantier 6 et celles de la personne.
    Always,
    /// Sous la touche seulement : toutes les nouvelles commandes (recherches, extraits, applications, mail, reformulation). Dite sans
    /// la touche, leur phrase est du texte comme un autre.
    UnderKey,
}

/// Dans quelle situation une phrase est lue : sans la touche de commande (une dictée ordinaire) ou sous la touche.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Context {
    NoKey,
    UnderKey,
}

impl Scope {
    /// Une phrase de commande a-t-elle une portée qui la rend lisible dans ce contexte ?
    fn applies_in(self, context: Context) -> bool {
        self == Scope::Always || context == Context::UnderKey
    }

    /// Mots au moins dans une phrase de cette portée : un mot seul (« gras ») est trop souvent du texte à coller sans la touche, alors
    /// que sous la touche une phrase d'un mot est permise.
    pub fn min_words(self) -> usize {
        match self {
            Scope::Always => MIN_WORDS,
            Scope::UnderKey => 1,
        }
    }
}

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
    pub scope: Scope,
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

    /// Les raccourcis lus dans un fichier écrit à la main : un raccourci illisible (ou sans modificateur) redevient celui par défaut.
    pub fn cleaned(self) -> Self {
        let good =
            |own: String, default: String| if crate::shortcut::Shortcut::parse(&own).is_some() { own.trim().to_string() } else { default };
        Self {
            bold: good(self.bold, Self::default_bold()),
            italic: good(self.italic, Self::default_italic()),
            underline: good(self.underline, Self::default_underline()),
        }
    }

    /// Le raccourci (texte) de la commande `id` ; `None` : pas une commande de mise en forme.
    pub fn for_command(&self, id: &str) -> Option<&str> {
        match id.strip_prefix(FORMAT_PREFIX)? {
            "gras" => Some(&self.bold),
            "italique" => Some(&self.italic),
            "souligne" => Some(&self.underline),
            _ => None,
        }
    }
}

impl Default for FormatShortcuts {
    fn default() -> Self {
        Self { bold: Self::default_bold(), italic: Self::default_italic(), underline: Self::default_underline() }
    }
}

/// Préfixe des identifiants des commandes de la personne : `perso.<numéro>`.
pub const CUSTOM_PREFIX: &str = "perso.";

/// Une commande de la personne : une phrase (ou plusieurs) qui appuie un raccourci de l'appli active, avec le titre qu'elle lui donne.
/// Les phrases valent pour toutes les langues de dictée (c'est la personne qui les écrit).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomCommand {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub phrases: Vec<String>,
    /// Raccourci lu par [`crate::shortcut::Shortcut::parse`] (« Mod+Shift+X »).
    pub shortcut: String,
    /// `true` (par défaut) : la commande n'agit que si du texte est sélectionné ; sinon elle appuie le raccourci dans tous les cas.
    #[serde(default = "CustomCommand::default_needs_selection", rename = "needsSelection")]
    pub needs_selection: bool,
}

impl CustomCommand {
    fn default_needs_selection() -> bool {
        true
    }

    /// Les commandes lues dans un fichier de réglages : titre, phrases (au moins deux mots) et raccourci lisible exigés, identifiants
    /// uniques et préfixés ; une commande incomplète est écartée.
    pub fn cleaned(list: Vec<CustomCommand>) -> Vec<CustomCommand> {
        let mut out: Vec<CustomCommand> = Vec::new();
        for mut c in list {
            c.name = c.name.split_whitespace().collect::<Vec<_>>().join(" ");
            c.shortcut = c.shortcut.trim().to_string();
            let mut seen = Vec::new();
            c.phrases = c
                .phrases
                .into_iter()
                .map(|p| p.split_whitespace().collect::<Vec<_>>().join(" "))
                .filter(|p| {
                    let n = normalize(p);
                    let keep = word_count(&n) >= MIN_WORDS && !seen.contains(&n);
                    seen.push(n);
                    keep
                })
                .collect();
            if c.id.starts_with(CUSTOM_PREFIX)
                && !c.name.is_empty()
                && !c.phrases.is_empty()
                && crate::shortcut::Shortcut::parse(&c.shortcut).is_some()
                && !out.iter().any(|o| o.id == c.id)
            {
                out.push(c);
            }
        }
        out
    }

    fn def(&self) -> CommandDef {
        CommandDef {
            scope: Scope::Always,
            id: self.id.clone(),
            needs: if self.needs_selection { Needs::Selection } else { Needs::Nothing },
            argument: Argument::None,
            phrases_fr: self.phrases.clone(),
            phrases_en: self.phrases.clone(),
        }
    }
}

/// Toutes les commandes en vigueur : celles du programme, puis celles de la personne (à phrase égale, celles du programme l'emportent).
pub fn all_defs(settings: &AppSettings) -> Vec<CommandDef> {
    let mut defs = builtin();
    defs.extend(settings.custom_commands.iter().map(CustomCommand::def));
    defs
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
        scope: Scope::Always,
        id: format!("{SURROUND_PREFIX}{id}"),
        needs: Needs::Selection,
        argument: Argument::None,
        phrases_fr: v(fr),
        phrases_en: v(en),
    };
    let format = |id: &str, fr: &[&str], en: &[&str]| CommandDef {
        scope: Scope::Always,
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
                "mettre en souligner",
                "mets en souligner",
                "mais en souligner",
                "souligne le texte",
                "souligne ça",
            ],
            &["make it underlined", "underline it", "underline this", "put in underline"],
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
            scope: Scope::Always,
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
///
/// `context` : sans la touche de commande, seules les fiches de portée « partout » comptent et leurs phrases font au moins [`MIN_WORDS`]
/// mots ; sous la touche, toutes les fiches comptent et une phrase d'un mot est permise pour celles qui sont « sous la touche seulement ».
pub fn recognize(
    raw: &str,
    defs: &[CommandDef],
    custom: &VoicePhrases,
    language: DictationLanguage,
    context: Context,
) -> Option<Recognized> {
    let spoken = words(raw);
    let shortest = match context {
        Context::NoKey => MIN_WORDS,
        Context::UnderKey => 1,
    };
    if spoken.len() < shortest {
        return None;
    }
    let mut best: Option<(usize, &CommandDef, Option<usize>, Lang)> = None;
    for def in defs.iter().filter(|d| d.scope.applies_in(context)) {
        for lang in catalogues(language) {
            for phrase in effective_phrases(def, custom, *lang) {
                let normalized = normalize(&phrase);
                let n = word_count(&normalized);
                if n < def.scope.min_words() || n > spoken.len() {
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

/// Reconnaissance avec les réglages : langue de dictée, phrases de la personne, et, **sans la touche**, l'interrupteur général
/// `voiceCommandsEnabled` (il garde les commandes d'avant le chantier 6 contre un déclenchement par erreur dans une dictée ordinaire).
/// Sous la touche, la personne a demandé une commande : l'interrupteur ne s'applique pas.
pub fn recognize_for(raw: &str, settings: &AppSettings, context: Context) -> Option<Recognized> {
    if context == Context::NoKey && !settings.voice_commands_enabled {
        return None;
    }
    recognize(raw, &all_defs(settings), &settings.voice_commands, settings.language, context)
}

/// Sous la touche, une phrase qui n'est ni une commande ni proche d'une commande peut être une consigne de reformulation (étape 4) :
/// au plus ce nombre de mots. Au-delà, c'est une dictée prise par mégarde.
pub const INSTRUCTION_MAX_WORDS: usize = 25;

/// Ce que devient une phrase dite sous la touche de commande (cadrage, décision 1, « Aiguillage sous la touche »).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    /// Une commande du catalogue (celles d'avant le chantier 6 comprises).
    Command(Recognized),
    /// Pas une commande, mais **proche** d'une : « Commande non reconnue », le modèle n'est pas appelé.
    Close,
    /// Ni l'un ni l'autre, et au plus [`INSTRUCTION_MAX_WORDS`] mots : une consigne possible (la reformulation de l'étape 4).
    Instruction,
    /// Plus de [`INSTRUCTION_MAX_WORDS`] mots : une dictée prise par mégarde sous la touche.
    TooLong,
}

/// Mots pivots d'une commande sur sélection (formes normalisées) : une phrase qui en contient un est prise pour une commande mal dite.
fn pivot_words(lang: Lang) -> &'static [&'static str] {
    match lang {
        Lang::Fr => &[
            "gras",
            "italique",
            "souligne",
            "parenthese",
            "parentheses",
            "guillemet",
            "guillemets",
            "crochet",
            "crochets",
            "accolade",
            "accolades",
            "dictionnaire",
        ],
        Lang::En => &[
            "bold",
            "italic",
            "italics",
            "underline",
            "underlined",
            "parenthesis",
            "parentheses",
            "quote",
            "quotes",
            "bracket",
            "brackets",
            "brace",
            "braces",
            "dictionary",
        ],
    }
}

/// Les deux suites de mots diffèrent d'**un seul** mot : un mot changé, ajouté ou retiré (pas zéro, pas deux).
fn one_word_apart(a: &[String], b: &[String]) -> bool {
    match a.len().cmp(&b.len()) {
        std::cmp::Ordering::Equal => a.iter().zip(b).filter(|(x, y)| x != y).count() == 1,
        std::cmp::Ordering::Greater | std::cmp::Ordering::Less => {
            let (long, short) = if a.len() > b.len() { (a, b) } else { (b, a) };
            long.len() - short.len() == 1
                && (0..long.len()).any(|skip| long.iter().enumerate().filter(|(i, _)| *i != skip).map(|(_, w)| w).eq(short.iter()))
        }
    }
}

/// La phrase est-elle **proche** d'une commande connue (B1) : elle contient un mot pivot d'une commande sur sélection, ou elle n'est qu'à un
/// mot d'une phrase du catalogue (celles en vigueur, de la personne comprises).
fn is_close(raw: &str, settings: &AppSettings) -> bool {
    let spoken: Vec<String> = words(raw).into_iter().map(|(w, _)| w).collect();
    if spoken.is_empty() {
        return false;
    }
    let langs = catalogues(settings.language);
    if langs.iter().any(|lang| pivot_words(*lang).iter().any(|pivot| spoken.iter().any(|w| w == pivot))) {
        return true;
    }
    all_defs(settings).iter().any(|def| {
        langs.iter().any(|lang| {
            effective_phrases(def, &settings.voice_commands, *lang).iter().any(|phrase| {
                let phrase: Vec<String> = normalize(phrase).split(' ').filter(|w| !w.is_empty()).map(str::to_string).collect();
                one_word_apart(&spoken, &phrase)
            })
        })
    })
}

/// L'aiguillage d'une phrase dite sous la touche de commande : une commande reconnue, une phrase proche d'une commande, une consigne
/// possible ou une dictée prise par mégarde. Sous la touche, `voiceCommandsEnabled` ne s'applique pas (voir [`recognize_for`]).
pub fn route_under_key(raw: &str, settings: &AppSettings) -> Route {
    if let Some(command) = recognize_for(raw, settings, Context::UnderKey) {
        return Route::Command(command);
    }
    if is_close(raw, settings) {
        return Route::Close;
    }
    if words(raw).len() <= INSTRUCTION_MAX_WORDS {
        Route::Instruction
    } else {
        Route::TooLong
    }
}

/// Contrôle d'une phrase que la personne veut donner à la commande `id` (dans la langue `lang`) : au moins deux mots, et pas déjà
/// celle d'une autre commande (comparée sous forme normalisée, phrases par défaut ou choisies). Rend la phrase nettoyée.
pub fn validate_phrase(phrase: &str, id: &str, lang: Lang, defs: &[CommandDef], custom: &VoicePhrases) -> Result<String, PhraseError> {
    // Une commande de la personne pas encore créée (« perso.nouveau ») se contrôle comme les autres.
    if !defs.iter().any(|d| d.id == id) && !id.starts_with(CUSTOM_PREFIX) {
        return Err(PhraseError::UnknownCommand);
    }
    // La longueur minimale est celle de la portée de la commande : deux mots pour une commande qui vaut aussi sans la touche (et pour une
    // commande de la personne, pas encore créée), un seul pour une commande « sous la touche seulement ».
    let min = defs.iter().find(|d| d.id == id).map_or(MIN_WORDS, |d| d.scope.min_words());
    let normalized = normalize(phrase);
    if word_count(&normalized) < min {
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
        recognize(raw, &builtin(), &VoicePhrases::default(), lang, Context::NoKey)
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
            scope: Scope::Always,
            id: "extrait.insérer".into(),
            needs: Needs::Nothing,
            argument: Argument::None,
            phrases_fr: vec!["insérer mon adresse".into()],
            phrases_en: vec![],
        };
        let r = recognize("Insérer mon adresse.", &[free], &VoicePhrases::default(), DictationLanguage::Fr, Context::NoKey).unwrap();
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
        assert!(recognize("Mémorise ce mot !", &builtin(), &custom, DictationLanguage::Fr, Context::NoKey).is_some());
        assert!(recognize("ajouter au dictionnaire", &builtin(), &custom, DictationLanguage::Fr, Context::NoKey).is_none());
        // L'anglais garde ses phrases par défaut.
        assert!(recognize("add to dictionary", &builtin(), &custom, DictationLanguage::En, Context::NoKey).is_some());
        // Un seul mot, ou pas de mot : refusé.
        let defs = builtin();
        assert_eq!(validate_phrase("mémorise", ADD, Lang::Fr, &defs, &custom), Err(PhraseError::TooShort));
        assert_eq!(validate_phrase(" … ", ADD, Lang::Fr, &defs, &custom), Err(PhraseError::TooShort));
        assert_eq!(validate_phrase("x", "inconnue", Lang::Fr, &defs, &custom), Err(PhraseError::UnknownCommand));
        assert_eq!(validate_phrase("  Mémorise   ce  mot ", ADD, Lang::Fr, &defs, &custom).as_deref(), Ok("Mémorise ce mot"));
        // Déjà prise par une autre commande (comparée sous forme normalisée).
        let other = CommandDef {
            scope: Scope::Always,
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
        assert!(recognize("mémorise", &builtin(), &custom, DictationLanguage::Fr, Context::NoKey).is_none());
        assert!(recognize("retiens ça", &builtin(), &custom, DictationLanguage::Fr, Context::NoKey).is_some());
    }

    /// Commande à complément : la phrase puis un reste non vide, rendu tel que transcrit.
    #[test]
    fn commands_extra_argument_is_the_rest_of_the_sentence() {
        let search = CommandDef {
            scope: Scope::Always,
            id: "recherche.youtube".into(),
            needs: Needs::Nothing,
            argument: Argument::Rest,
            phrases_fr: vec!["recherche YouTube".into()],
            phrases_en: vec![],
        };
        let defs = [search];
        let none = VoicePhrases::default();
        let r = recognize("Recherche YouTube, Chats rigolos !", &defs, &none, DictationLanguage::Fr, Context::NoKey).unwrap();
        assert_eq!(r.argument.as_deref(), Some("Chats rigolos"));
        assert!(recognize("Recherche YouTube.", &defs, &none, DictationLanguage::Fr, Context::NoKey).is_none());
    }

    /// Entre deux phrases qui commencent pareil, la plus longue l'emporte.
    #[test]
    fn commands_extra_the_longest_phrase_wins() {
        let a = CommandDef {
            scope: Scope::Always,
            id: "a".into(),
            needs: Needs::Nothing,
            argument: Argument::Rest,
            phrases_fr: vec!["recherche sur le web".into()],
            phrases_en: vec![],
        };
        let b = CommandDef {
            scope: Scope::Always,
            id: "b".into(),
            needs: Needs::Nothing,
            argument: Argument::Rest,
            phrases_fr: vec!["recherche sur le web images".into()],
            phrases_en: vec![],
        };
        let r = recognize("recherche sur le web images chats", &[a, b], &VoicePhrases::default(), DictationLanguage::Fr, Context::NoKey)
            .unwrap();
        assert_eq!((r.id.as_str(), r.argument.as_deref()), ("b", Some("chats")));
    }

    /// Interrupteur général et réglages lus d'un fichier.
    fn under_key_search() -> CommandDef {
        CommandDef {
            scope: Scope::UnderKey,
            id: "recherche.1".into(),
            needs: Needs::Nothing,
            argument: Argument::Rest,
            phrases_fr: vec!["nouvelle recherche youtube".into()],
            phrases_en: vec!["new search youtube".into()],
        }
    }

    /// Chantier 6, étape 1a : une commande « sous la touche seulement » n'existe pas sans la touche (sa phrase reste du texte).
    #[test]
    fn commands_401_an_under_key_command_is_not_recognized_without_the_key() {
        let defs = [under_key_search()];
        let none = VoicePhrases::default();
        let said = "Nouvelle recherche YouTube chats rigolos";
        assert!(recognize(said, &defs, &none, DictationLanguage::Fr, Context::NoKey).is_none());
        let found = recognize(said, &defs, &none, DictationLanguage::Fr, Context::UnderKey).unwrap();
        assert_eq!((found.id.as_str(), found.argument.as_deref()), ("recherche.1", Some("chats rigolos")));
    }

    /// Une phrase d'un mot est permise pour une commande « sous la touche seulement », sous la touche ; jamais pour une commande qui vaut
    /// aussi sans la touche (même écrite à la main dans un fichier).
    #[test]
    fn commands_402_a_one_word_phrase_is_allowed_only_for_an_under_key_command_under_the_key() {
        let one = CommandDef {
            scope: Scope::UnderKey,
            id: "extrait.1".into(),
            needs: Needs::Nothing,
            argument: Argument::None,
            phrases_fr: vec!["signature".into()],
            phrases_en: vec!["signature".into()],
        };
        let none = VoicePhrases::default();
        assert!(recognize("Signature.", std::slice::from_ref(&one), &none, DictationLanguage::Fr, Context::UnderKey).is_some());
        assert!(recognize("Signature.", std::slice::from_ref(&one), &none, DictationLanguage::Fr, Context::NoKey).is_none());
        let always = CommandDef { scope: Scope::Always, id: "perso.9".into(), ..one.clone() };
        assert!(recognize("Signature.", &[always], &none, DictationLanguage::Fr, Context::UnderKey).is_none());
        // Le contrôle d'une phrase suit la portée de la commande : un mot suffit pour l'une, pas pour une commande qui vaut aussi sans touche.
        let defs = [one, builtin().remove(0)];
        let add = defs[1].id.clone();
        assert!(validate_phrase("paraphe", "extrait.1", Lang::Fr, &defs, &none).is_ok());
        assert_eq!(validate_phrase("paraphe", &add, Lang::Fr, &defs, &none), Err(PhraseError::TooShort));
        // Une phrase reste à une seule commande, quelle que soit la portée.
        assert_eq!(validate_phrase("mets en gras", "extrait.1", Lang::Fr, &defs, &none), Err(PhraseError::Taken(add)));
    }

    /// L'interrupteur `voiceCommandsEnabled` garde les commandes sans la touche ; sous la touche, il ne s'applique pas.
    #[test]
    fn commands_403_the_switch_guards_only_the_no_key_case() {
        let off = AppSettings { voice_commands_enabled: false, ..AppSettings::default() };
        assert!(recognize_for("Ajouter au dictionnaire.", &off, Context::NoKey).is_none());
        assert_eq!(recognize_for("Ajouter au dictionnaire.", &off, Context::UnderKey).map(|r| r.id).as_deref(), Some(ADD));
    }

    /// Aiguillage sous la touche : commande, phrase proche, consigne possible (au plus 25 mots), dictée prise par mégarde.
    #[test]
    fn commands_404_under_the_key_a_phrase_is_a_command_a_near_miss_an_instruction_or_too_long() {
        let settings = AppSettings::default();
        let route = |raw: &str| route_under_key(raw, &settings);
        assert!(matches!(route("mets en gras"), Route::Command(c) if c.id == "format.gras"));
        // Mot pivot d'une commande sur sélection : proche, même si la phrase est longue.
        assert_eq!(route("mets ça en rouge et en gras s'il te plaît"), Route::Close);
        assert_eq!(route("entoure ça de crochets"), Route::Close);
        // À un mot d'une phrase du catalogue (un mot de plus, de moins ou changé).
        assert_eq!(route("ajouter au dictionnaire ici"), Route::Close);
        assert_eq!(route("ajouter dictionnaire"), Route::Close);
        // « mets en rouge » est à un mot de « mets en gras » : proche, donc jamais prise pour une consigne (choix du cadrage, B1).
        assert_eq!(route("mets en rouge"), Route::Close);
        assert_eq!(route("rends ça plus amical"), Route::Instruction);
        // Une consigne libre : ni pivot ni proche.
        assert_eq!(route("cherche les fautes"), Route::Instruction);
        assert_eq!(route("raccourcis"), Route::Instruction);
        let long = vec!["mot"; INSTRUCTION_MAX_WORDS].join(" ");
        assert_eq!(route(&long), Route::Instruction);
        assert_eq!(route(&format!("{long} mot")), Route::TooLong);
    }

    #[test]
    fn commands_405_one_word_apart_means_exactly_one_word() {
        let v = |t: &str| t.split(' ').map(str::to_string).collect::<Vec<_>>();
        assert!(one_word_apart(&v("a b c"), &v("a x c")));
        assert!(one_word_apart(&v("a b c"), &v("a b")));
        assert!(one_word_apart(&v("a b"), &v("a b c")));
        assert!(!one_word_apart(&v("a b c"), &v("a b c")));
        assert!(!one_word_apart(&v("a b c"), &v("x y c")));
        assert!(!one_word_apart(&v("a b c d"), &v("a b")));
    }

    #[test]
    fn commands_extra_settings_switch_and_file() {
        let on = AppSettings::from_json("{}").unwrap();
        assert!(on.voice_commands_enabled);
        assert!(recognize_for("Ajouter au dictionnaire.", &on, Context::NoKey).is_some());
        let off = AppSettings::from_json(r#"{"voiceCommandsEnabled": false}"#).unwrap();
        assert!(recognize_for("Ajouter au dictionnaire.", &off, Context::NoKey).is_none());
        let custom =
            AppSettings::from_json(r#"{"voiceCommands": {"fr": {"dictionnaire.ajouter": ["  mémorise   ça ", "", "Mémorise ça"]}}}"#)
                .unwrap();
        assert_eq!(custom.voice_commands.fr[ADD], vec!["mémorise ça".to_string()]);
        assert!(recognize_for("Mémorise ça.", &custom, Context::NoKey).is_some());
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
            // Whisper écrit aussi « souligner » (essai réel du 07/11/2026, Notes).
            ("Mettre en souligner.", "format.souligne"),
        ] {
            assert_eq!(id_of(raw, DictationLanguage::Fr), Some((id.to_string(), Lang::Fr)), "{raw}");
        }
        for (raw, id) in [
            ("Make it bold.", "format.gras"),
            ("make italic", "format.italique"),
            ("Underline it", "format.souligne"),
            ("In bold", "format.gras"),
        ] {
            assert_eq!(id_of(raw, DictationLanguage::En), Some((id.to_string(), Lang::En)), "{raw}");
        }
        for raw in ["Mets le titre en gras", "gras", "bold", "make the title bold"] {
            assert!(id_of(raw, DictationLanguage::Multi).is_none(), "{raw}");
        }
    }

    fn mine(id: &str, name: &str, phrases: &[&str], shortcut: &str) -> CustomCommand {
        CustomCommand {
            id: id.into(),
            name: name.into(),
            phrases: phrases.iter().map(|p| (*p).to_string()).collect(),
            shortcut: shortcut.into(),
            needs_selection: true,
        }
    }

    #[test]
    fn commands_310_custom_commands_are_recognized_in_every_dictation_language_and_cleaned() {
        let settings = AppSettings {
            custom_commands: CustomCommand::cleaned(vec![
                mine("perso.1", "Titre 1", &["  appliquer   le titre ", "titre"], "Mod+Alt+1"),
                mine("perso.1", "Doublon", &["autre phrase ici"], "Mod+Alt+2"),
                mine("autre.3", "Mauvais préfixe", &["phrase préfixe"], "Mod+Alt+3"),
                mine("perso.4", "", &["sans titre ici"], "Mod+Alt+4"),
                mine("perso.5", "Sans raccourci lisible", &["raccourci illisible"], "B"),
            ]),
            ..AppSettings::default()
        };
        assert_eq!(settings.custom_commands.len(), 1);
        assert_eq!(settings.custom_commands[0].phrases, vec!["appliquer le titre"], "espaces ramenés à un, mot seul écarté");
        for language in [DictationLanguage::Fr, DictationLanguage::En, DictationLanguage::Multi] {
            let s = AppSettings { language, ..settings.clone() };
            assert_eq!(recognize_for("Appliquer le titre.", &s, Context::NoKey).map(|r| r.id), Some("perso.1".to_string()));
        }
    }

    #[test]
    fn commands_311_a_custom_phrase_cannot_take_a_phrase_of_another_command_and_a_new_one_is_checked() {
        let settings =
            AppSettings { custom_commands: vec![mine("perso.1", "T", &["appliquer le titre"], "Mod+Alt+1")], ..AppSettings::default() };
        let defs = all_defs(&settings);
        let custom = VoicePhrases::default();
        assert!(
            matches!(validate_phrase("mettre en gras", "perso.nouveau", Lang::Fr, &defs, &custom), Err(PhraseError::Taken(id)) if id == "format.gras")
        );
        assert!(
            matches!(validate_phrase("Appliquer le titre", "perso.nouveau", Lang::Fr, &defs, &custom), Err(PhraseError::Taken(id)) if id == "perso.1")
        );
        assert_eq!(validate_phrase("  faire   ceci ", "perso.nouveau", Lang::Fr, &defs, &custom), Ok("faire ceci".to_string()));
        assert_eq!(validate_phrase("faire ceci", "inconnue.x", Lang::Fr, &defs, &custom), Err(PhraseError::UnknownCommand));
    }

    #[test]
    fn commands_302_format_commands_need_a_selection_and_have_no_argument() {
        let defs = builtin();
        let format: Vec<_> = defs.iter().filter(|d| d.id.starts_with(FORMAT_PREFIX)).collect();
        assert_eq!(format.len(), 3);
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
        let fixed = FormatShortcuts { bold: "G".into(), italic: " Ctrl+Alt+I ".into(), underline: "".into() }.cleaned();
        assert_eq!((fixed.bold.as_str(), fixed.italic.as_str(), fixed.underline.as_str()), ("Mod+B", "Ctrl+Alt+I", "Mod+U"));
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
