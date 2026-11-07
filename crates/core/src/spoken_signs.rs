//! Signes dictés : « ouvrez la parenthèse » → « ( ». Whisper écrit ces commandes de bien des façons
//! (« Ouvre la parenthèse, », « fermez parenthèse. »…) : chaque signe accepte ces variantes et recolle les espaces.
//! Port de `SpokenSigns.swift` (Scribe).

use crate::re::{cached, is_match, matches_with_group, regex};
use serde::{Deserialize, Serialize};

/// Ouvrant : collé au mot qui suit. Fermant : collé au mot qui précède. Ligne : coupe la ligne.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Opening,
    Closing,
    Line,
    /// Collé des deux côtés : « jean arobase gmail » → « jean@gmail ».
    Glued,
    /// Espace de chaque côté : « tom esperluette jerry » → « tom & jerry ».
    Spaced,
    /// Collé au mot qui précède : « gmail point com » → « gmail.com ».
    Suffix,
}

/// Un signe de la liste intégrée.
#[derive(Debug, Clone)]
pub struct Sign {
    pub said: &'static str,
    pub pattern: &'static str,
    pub written: &'static str,
    pub kind: Kind,
    /// Reconnu seulement quand il est dit seul, entre deux ponctuations (voir [`lone_line`]) : `pattern` n'est pas utilisé.
    pub isolated: bool,
    /// Langues de dictée où le signe est actif (S18 : les signes anglais ne s'appliquent pas à une dictée française).
    pub langs: Langs,
}

/// Langue d'un signe : français, anglais, ou les deux (symboles que les deux langues disent pareil : « underscore », « tilde »).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Langs {
    Fr,
    En,
    Both,
}

/// Quelles langues de signes sont actives. `Neutral` : texte traduit ou langue inconnue, seuls les signes `Both` s'appliquent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Active {
    pub fr: bool,
    pub en: bool,
}

impl Active {
    pub const NEUTRAL: Active = Active { fr: false, en: false };
    pub const FR: Active = Active { fr: true, en: false };
    pub const EN: Active = Active { fr: false, en: true };
    pub const BOTH: Active = Active { fr: true, en: true };

    pub fn of(language: crate::settings::DictationLanguage) -> Active {
        use crate::settings::DictationLanguage as D;
        match language {
            D::Fr => Active::FR,
            D::En => Active::EN,
            D::Multi => Active::BOTH,
        }
    }

    fn allows(self, langs: Langs) -> bool {
        match langs {
            Langs::Both => true,
            Langs::Fr => self.fr,
            Langs::En => self.en,
        }
    }
}

// Impératifs seulement : « fermer la parenthèse des vacances » parle d'une parenthèse, ce n'est pas une commande.
pub const ALL: &[Sign] = &[
    Sign {
        said: "ouvrez la parenthèse",
        pattern: r"(?:ouvre|ouvrez)(?:\s+la)?\s+parenth[eè]ses?",
        written: "(",
        kind: Kind::Opening,
        isolated: false,
        langs: Langs::Both,
    },
    Sign {
        said: "fermez la parenthèse",
        pattern: r"(?:ferme|fermez)(?:\s+la)?\s+parenth[eè]ses?",
        written: ")",
        kind: Kind::Closing,
        isolated: false,
        langs: Langs::Both,
    },
    Sign {
        said: "ouvrez les guillemets",
        pattern: r"(?:ouvre|ouvrez)(?:\s+les)?\s+guillemets?",
        written: "«\u{00A0}",
        kind: Kind::Opening,
        isolated: false,
        langs: Langs::Both,
    },
    Sign {
        said: "fermez les guillemets",
        pattern: r"(?:ferme|fermez)(?:\s+les)?\s+guillemets?",
        written: "\u{00A0}»",
        kind: Kind::Closing,
        isolated: false,
        langs: Langs::Both,
    },
    Sign {
        said: "ouvrez le crochet",
        pattern: r"(?:ouvre|ouvrez)(?:\s+le)?\s+crochets?",
        written: "[",
        kind: Kind::Opening,
        isolated: false,
        langs: Langs::Both,
    },
    Sign {
        said: "fermez le crochet",
        pattern: r"(?:ferme|fermez)(?:\s+le)?\s+crochets?",
        written: "]",
        kind: Kind::Closing,
        isolated: false,
        langs: Langs::Both,
    },
    Sign {
        said: "points de suspension",
        pattern: r"points\s+de\s+suspension",
        written: "…",
        kind: Kind::Closing,
        isolated: false,
        langs: Langs::Both,
    },
    // Whisper avale parfois le « à » : « point la ligne ».
    Sign {
        said: "point à la ligne",
        pattern: r"point(?:\s+[àa])?\s+la\s+ligne",
        written: ".\n",
        kind: Kind::Line,
        isolated: false,
        langs: Langs::Both,
    },
    // « passe la ligne » sans « à » se dit au sport (« le coureur passe la ligne ») : le « à » reste obligatoire pour lui.
    Sign {
        said: "retour à la ligne",
        pattern: r"(?:retour(?:\s+[àa])?|passe\s+[àa])\s+la\s+ligne",
        written: "\n",
        kind: Kind::Line,
        isolated: false,
        langs: Langs::Both,
    },
    Sign {
        said: "nouveau paragraphe",
        pattern: r"nouveau\s+paragraphe",
        written: "\n\n",
        kind: Kind::Line,
        isolated: false,
        langs: Langs::Both,
    },
    // « à la ligne » dit seul : « aller à la ligne 5 » ou « arrivé à la ligne d'arrivée » se disent aussi, donc pas de remplacement partout.
    Sign { said: "à la ligne (dit seul)", pattern: "", written: "\n", kind: Kind::Line, isolated: true, langs: Langs::Both },
    // S18 (tâche 6) : signes ajoutés (docs/COMMANDES.md § 4, docs/ECARTS.md). Jamais un nom nu courant (« point », « plus », « virgule »,
    // « étoile »…) : des mots composés peu ambigus, ou un impératif. L'ordre compte : « barre oblique inverse » avant « barre oblique ».
    // Singulier seulement : « trois points d'exclamation » (pluriel) est la correction du dictionnaire (« !!! »), pas un signe.
    // A. Ponctuation (fermants : la typographie française ajoute l'espace insécable devant « ! ? : ; »).
    Sign {
        said: "point d'exclamation",
        pattern: r"point\s+d['’]exclamation",
        written: "!",
        kind: Kind::Closing,
        isolated: false,
        langs: Langs::Fr,
    },
    Sign {
        said: "exclamation mark",
        pattern: r"exclamation\s+(?:mark|point)",
        written: "!",
        kind: Kind::Closing,
        isolated: false,
        langs: Langs::En,
    },
    Sign {
        said: "point d'interrogation",
        pattern: r"point\s+d['’]interrogation",
        written: "?",
        kind: Kind::Closing,
        isolated: false,
        langs: Langs::Fr,
    },
    Sign { said: "question mark", pattern: r"question\s+mark", written: "?", kind: Kind::Closing, isolated: false, langs: Langs::En },
    Sign { said: "point-virgule", pattern: r"point[\s-]+virgule", written: ";", kind: Kind::Closing, isolated: false, langs: Langs::Fr },
    Sign { said: "semicolon", pattern: r"semi-?\s?colon", written: ";", kind: Kind::Closing, isolated: false, langs: Langs::En },
    // « deux points » en deux mots est un nombre (« j'ai marqué deux points ») : le trait d'union, ou un impératif, est obligatoire.
    Sign {
        said: "deux-points",
        pattern: r"(?:(?:[ée]cris|[ée]crivez|tape|tapez|mets|mettez)\s+deux\s+points|deux-points)",
        written: ":",
        kind: Kind::Closing,
        isolated: false,
        langs: Langs::Fr,
    },
    // « colon cancer » parle du côlon : exclu.
    Sign {
        said: "colon",
        pattern: r"(?<!semi-)(?<!semi )colon(?!\s+(?:cancer|polyps?|surgery|screening|cleanse))",
        written: ":",
        kind: Kind::Closing,
        isolated: false,
        langs: Langs::En,
    },
    // B. Symboles de l'écriture d'adresses, de chemins et de code.
    Sign { said: "arobase", pattern: r"arr?obase", written: "@", kind: Kind::Glued, isolated: false, langs: Langs::Fr },
    Sign { said: "at sign", pattern: r"at\s+(?:sign|symbol)", written: "@", kind: Kind::Glued, isolated: false, langs: Langs::En },
    Sign { said: "point com", pattern: r"point\s+com", written: ".com", kind: Kind::Suffix, isolated: false, langs: Langs::Fr },
    Sign { said: "point fr", pattern: r"point\s+fr", written: ".fr", kind: Kind::Suffix, isolated: false, langs: Langs::Fr },
    Sign { said: "point org", pattern: r"point\s+org", written: ".org", kind: Kind::Suffix, isolated: false, langs: Langs::Fr },
    Sign { said: "point net", pattern: r"point\s+net", written: ".net", kind: Kind::Suffix, isolated: false, langs: Langs::Fr },
    Sign { said: "point io", pattern: r"point\s+io", written: ".io", kind: Kind::Suffix, isolated: false, langs: Langs::Fr },
    Sign { said: "point eu", pattern: r"point\s+eu", written: ".eu", kind: Kind::Suffix, isolated: false, langs: Langs::Fr },
    Sign { said: "point be", pattern: r"point\s+be", written: ".be", kind: Kind::Suffix, isolated: false, langs: Langs::Fr },
    Sign { said: "point ch", pattern: r"point\s+ch", written: ".ch", kind: Kind::Suffix, isolated: false, langs: Langs::Fr },
    Sign { said: "dot com", pattern: r"dot\s+com", written: ".com", kind: Kind::Suffix, isolated: false, langs: Langs::En },
    Sign { said: "dot org", pattern: r"dot\s+org", written: ".org", kind: Kind::Suffix, isolated: false, langs: Langs::En },
    Sign { said: "dot net", pattern: r"dot\s+net", written: ".net", kind: Kind::Suffix, isolated: false, langs: Langs::En },
    Sign { said: "dot io", pattern: r"dot\s+io", written: ".io", kind: Kind::Suffix, isolated: false, langs: Langs::En },
    Sign { said: "dot fr", pattern: r"dot\s+fr", written: ".fr", kind: Kind::Suffix, isolated: false, langs: Langs::En },
    Sign { said: "dot eu", pattern: r"dot\s+eu", written: ".eu", kind: Kind::Suffix, isolated: false, langs: Langs::En },
    Sign {
        said: "antislash",
        pattern: r"anti-?slash|back-?slash|bar(?:re)?[\s-]*oblique\s+inverse",
        written: "\\",
        kind: Kind::Glued,
        isolated: false,
        langs: Langs::Both,
    },
    Sign { said: "barre oblique", pattern: r"bar(?:re)?[\s-]*oblique", written: "/", kind: Kind::Glued, isolated: false, langs: Langs::Fr },
    Sign { said: "forward slash", pattern: r"forward\s+slash", written: "/", kind: Kind::Glued, isolated: false, langs: Langs::En },
    Sign {
        said: "underscore",
        pattern: r"underscore|tiret\s+bas|tiret\s+du\s+(?:8|huit)",
        written: "_",
        kind: Kind::Glued,
        isolated: false,
        langs: Langs::Both,
    },
    Sign { said: "tilde", pattern: r"tilde", written: "~", kind: Kind::Glued, isolated: false, langs: Langs::Both },
    Sign { said: "esperluette", pattern: r"esperluette|ampersand", written: "&", kind: Kind::Spaced, isolated: false, langs: Langs::Both },
    Sign { said: "astérisque", pattern: r"ast[ée]risque|asterisk", written: "*", kind: Kind::Glued, isolated: false, langs: Langs::Both },
    // Collé au mot qui suit : « hashtag scriibe » → « #scriibe ».
    Sign {
        said: "hashtag",
        pattern: r"hashtag|croisillon|(?:hash|pound|number)\s+sign",
        written: "#",
        kind: Kind::Opening,
        isolated: false,
        langs: Langs::Both,
    },
    Sign {
        said: "ouvrez l'accolade",
        pattern: r"(?:ouvre|ouvrez)(?:\s+l['’]|\s+la\s+|\s+les\s+|\s+)accolades?",
        written: "{",
        kind: Kind::Opening,
        isolated: false,
        langs: Langs::Fr,
    },
    Sign {
        said: "fermez l'accolade",
        pattern: r"(?:ferme|fermez)(?:\s+l['’]|\s+la\s+|\s+les\s+|\s+)accolades?",
        written: "}",
        kind: Kind::Closing,
        isolated: false,
        langs: Langs::Fr,
    },
    Sign {
        said: "open curly bracket",
        pattern: r"open\s+(?:a\s+)?(?:curly\s+(?:bracket|brace)|brace)s?",
        written: "{",
        kind: Kind::Opening,
        isolated: false,
        langs: Langs::En,
    },
    Sign {
        said: "close curly bracket",
        pattern: r"close\s+(?:a\s+)?(?:curly\s+(?:bracket|brace)|brace)s?",
        written: "}",
        kind: Kind::Closing,
        isolated: false,
        langs: Langs::En,
    },
    // C. Anglais : équivalents des signes français déjà présents.
    Sign {
        said: "open parenthesis",
        pattern: r"open\s+(?:a\s+)?(?:parenthesis|parentheses|paren)",
        written: "(",
        kind: Kind::Opening,
        isolated: false,
        langs: Langs::En,
    },
    Sign {
        said: "close parenthesis",
        pattern: r"close\s+(?:a\s+)?(?:parenthesis|parentheses|paren)",
        written: ")",
        kind: Kind::Closing,
        isolated: false,
        langs: Langs::En,
    },
    Sign {
        said: "open bracket",
        pattern: r"open\s+(?:a\s+)?(?:square\s+)?brackets?",
        written: "[",
        kind: Kind::Opening,
        isolated: false,
        langs: Langs::En,
    },
    Sign {
        said: "close bracket",
        pattern: r"close\s+(?:a\s+)?(?:square\s+)?brackets?",
        written: "]",
        kind: Kind::Closing,
        isolated: false,
        langs: Langs::En,
    },
    Sign { said: "open quote", pattern: r"open\s+(?:a\s+)?quotes?", written: "\"", kind: Kind::Opening, isolated: false, langs: Langs::En },
    Sign {
        said: "close quote",
        pattern: r"close\s+(?:a\s+)?quotes?",
        written: "\"",
        kind: Kind::Closing,
        isolated: false,
        langs: Langs::En,
    },
    Sign { said: "ellipsis", pattern: r"ellipsis|dot\s+dot\s+dot", written: "…", kind: Kind::Closing, isolated: false, langs: Langs::En },
    Sign { said: "new paragraph", pattern: r"new\s+paragraph", written: "\n\n", kind: Kind::Line, isolated: false, langs: Langs::En },
    Sign { said: "new line", pattern: r"new\s+line", written: "\n", kind: Kind::Line, isolated: false, langs: Langs::En },
];

/// Le caractère mis devant un élément de liste dicté (« tiret du lait » → « - du lait »).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ListBullet {
    #[default]
    Dash,
    Dot,
    Star,
}

impl ListBullet {
    pub fn symbol(self) -> &'static str {
        match self {
            ListBullet::Dash => "-",
            ListBullet::Dot => "•",
            ListBullet::Star => "*",
        }
    }
}

const BLANK: &str = r"[ \t\x{00A0}\x{202F}]";

/// Nombres en lettres reconnus après « point » (du plus long au plus court : l'ordre n'a pas d'effet sur le résultat, il rend le motif stable).
const NUMBER_WORDS: [(&str, u32); 13] = [
    ("quatre", 4),
    ("douze", 12),
    ("trois", 3),
    ("onze", 11),
    ("deux", 2),
    ("cinq", 5),
    ("sept", 7),
    ("huit", 8),
    ("neuf", 9),
    ("une", 1),
    ("six", 6),
    ("dix", 10),
    ("un", 1),
];

fn number_words_alternation() -> String {
    NUMBER_WORDS.iter().map(|(w, _)| *w).collect::<Vec<_>>().join("|")
}

fn number_of_word(word: &str) -> Option<u32> {
    NUMBER_WORDS.iter().find(|(w, _)| *w == word).map(|(_, n)| *n)
}

// Début de dictée, fin de phrase ou saut de ligne devant (ou une ponctuation déjà absorbée par le « à la ligne » d'avant : deux de suite
// font deux coupures) ; ponctuation (ou fin) derrière. La virgule de Whisper devant saute, et les sauts de ligne qu'il a mis lui-même
// autour (un par segment, deux après une longue pause) sont absorbés : « à la ligne » donne une seule coupure.
// (Le moteur n'accepte pas une assertion arrière de longueur variable : « {0,2} » blancs est écrit en trois assertions de longueur fixe.)
regex!(
    LONE_LINE,
    format!(
        r"(?i)(^|[,.;:!?…]|\n|(?:(?<=[,.;:!?…\n])|(?<=[,.;:!?…\n][ \t])|(?<=[,.;:!?…\n][ \t]{{2}})))(?>(?:{b}|\n)*)[àa]{b}+la{b}+ligne(?![\p{{L}}\p{{N}}])(?={b}*(?:[,.;:…]|\n|$)){b}*(?:\.{{1,3}}|…|[,;:])?(?:{b}|\n)*",
        b = BLANK
    )
);

// Whisper n'écrit parfois aucune ponctuation (« Ma liste à la ligne tirée du lait à la ligne tirée du pain »), constaté avec la voix de
// Christophe : « à la ligne » devant un début d'élément de liste (« tiret », « puce », « numéro suivant », « point 2 ») est alors une
// commande aussi. Devant un autre mot, rien ne le distingue de « aller à la ligne 5 » : il reste du texte (« retour à la ligne » marche partout).
fn list_start() -> String {
    let b = BLANK;
    format!(
        r"(?:(?:tir(?:et|er|ez|ée|é)|puce|num[ée]ro{b}+suivant)(?![\p{{L}}\p{{N}}'’-])|point{b}+(?:\d{{1,2}}|(?:{words})(?={b}*[,.:;]))(?![\p{{L}}\p{{N}}]|[.,]\d|{b}+\d))(?={b}*[,.:;–—]?{b}*[\p{{L}}\p{{N}}])",
        words = number_words_alternation()
    )
}
regex!(
    LONE_LINE_BEFORE_LIST,
    format!(
        r"(?i)(?>(?:{b}|\n)*),?(?>(?:{b}|\n)*)(?<![\p{{L}}\p{{N}}'’])à{b}+la{b}+ligne(?![\p{{L}}\p{{N}}])(?:{b}|\n)*(?={ls})",
        b = BLANK,
        ls = list_start()
    )
);

/// Remplace les signes dictés de la liste intégrée, puis les débuts d'éléments de liste (« tiret », « point un »…).
pub fn apply(text: &str, bullet: ListBullet, lists: bool) -> String {
    apply_for(text, bullet, lists, Active::FR)
}

/// Comme [`apply`], avec les signes des langues `active` (français seul pour `apply`, qui garde le comportement de Scribe).
pub fn apply_for(text: &str, bullet: ListBullet, lists: bool, active: Active) -> String {
    let mut t = text.to_string();
    let mut changed = false;
    for sign in ALL.iter().filter(|s| !s.isolated && active.allows(s.langs)) {
        let (out, n) = replace(sign.pattern, sign.kind, sign.written, &t);
        t = out;
        changed = changed || n > 0;
    }
    if active.fr || active.en {
        let restored = restore_dropped_at(&t);
        if restored != t {
            t = restored;
            changed = true;
        }
    }
    let alone = lone_line(&t);
    if alone != t {
        t = alone;
        changed = true;
    }
    if lists {
        let listed = list_items(&t, bullet);
        if listed != t {
            t = listed;
            changed = true;
        }
    }
    if changed {
        tidy(&t)
    } else {
        t
    }
}

// Whisper efface souvent « arobase » et écrit « jean.gmail.com » (essai réel du 07/11/2026). Un site « jean.gmail.com » n'existe pas : une adresse
// d'un fournisseur de courriel courant, précédée d'un seul nom collé par un point, retrouve son « @ ». Hors « www », « mail », « accounts »…
regex!(
    DROPPED_AT,
    r"(?i)(?<![\p{L}\p{N}_.@%+-])(?!(?:www|mail|accounts|support|help)\.(?:gmail|googlemail|hotmail|icloud)\.)([\p{L}\p{N}_-]+(?:\.[\p{L}\p{N}_-]+)*)\.(gmail|googlemail|hotmail|icloud)\.(com|fr|net)(?![\p{L}\p{N}])"
);

fn restore_dropped_at(text: &str) -> String {
    if !text.contains('.') {
        return text.to_string();
    }
    DROPPED_AT.replace_all(text, "${1}@${2}.${3}").into_owned()
}

/// « à la ligne » dit seul (« Les courses, à la ligne, du lait ») coupe la ligne comme « retour à la ligne » ;
/// dans une phrase (« aller à la ligne 5 », « passer à la ligne suivante », « arrivé à la ligne d'arrivée ») il reste du texte.
pub fn lone_line(text: &str) -> String {
    if !text.to_lowercase().contains("ligne") {
        return text.to_string();
    }
    // D'abord sans ponctuation devant un début de liste, pour que le premier motif ne les voie pas.
    let mut out = text.to_string();
    for (whole, _) in matches_with_group(&LONE_LINE_BEFORE_LIST, text, 0).into_iter().rev() {
        out.replace_range(whole.0..whole.1, "\n");
    }
    if out != text {
        return lone_line(&out);
    }
    for (whole, before) in matches_with_group(&LONE_LINE, text, 1).into_iter().rev() {
        // Le signe qui précède reste s'il termine une phrase (« Bonjour. À la ligne » → « Bonjour. ⏎ ») ; la virgule saute.
        let before = &text[before.0..before.1];
        let keep = if before.is_empty() || ",\n".contains(before) { "" } else { before };
        out.replace_range(whole.0..whole.1, &format!("{keep}\n"));
    }
    out
}

// MARK: Listes à puces et numérotées

fn item_tail() -> String {
    format!(r"{b}*[,.:;–—]?{b}*(\S.*)$", b = BLANK)
}
regex!(BULLET_ITEM, format!(r"(?i)^{b}*(?:tiret|puce)(?![\p{{L}}\p{{N}}'’-]){t}", b = BLANK, t = item_tail()));
// Whisper entend parfois « tiret » comme « tirer », « tirez », « tiré » ou « tirée » : ces verbes ordinaires ne comptent
// qu'après un saut de ligne, jamais en tête de dictée (« Tirez la porte. »).
// Scriibe (S12) : Whisper écrit aussi « puce » en « + » ou « pus » (« retour à la ligne + du pain », « retour à la ligne, Pus du pain ») ; même règle, après un saut de ligne seulement
// (« +33 6… » et « 1 + 2 » ne sont jamais en tête de ligne suivie d'un blanc et d'un mot).
regex!(MISHEARD_BULLET, format!(r"(?i)^{b}*(?:tirer|tirez|tirée|tiré|pus|\+)(?![\p{{L}}\p{{N}}'’-]){t}", b = BLANK, t = item_tail()));
// « point 1 » mais pas « point 1,5 » (un nombre à virgule) ; en lettres, « point un, » demande sa ponctuation (« point un peu plus » reste du texte).
regex!(
    NUMBERED_ITEM,
    format!(
        r"(?i)^{b}*point{b}+(\d{{1,2}}|(?:{w})(?={b}*[,.:;]))(?![\p{{L}}\p{{N}}]|[.,]\d|{b}+\d){t}",
        b = BLANK,
        w = number_words_alternation(),
        t = item_tail()
    )
);
regex!(NEXT_ITEM, format!(r"(?i)^{b}*num[ée]ro{b}+suivant(?![\p{{L}}\p{{N}}]){t}", b = BLANK, t = item_tail()));
regex!(ANY_LIST_WORD, r"(?i)tir(?:et|er|ez|é)|puce|pus|point|num[ée]ro|\+");
regex!(NUMBERED_LINE, r"^[ \t]*(\d{1,2})\.[ \t]");

/// Au début d'une ligne seulement : « tiret » (ou « tirer », « tirez », « tiré », « tirée », « pus » ou « + » mal entendus, après un saut de ligne)
/// ou « puce » → le caractère de puce ; « point un » ou « point 2 » → « 1. », « 2. » ; « numéro suivant » → le numéro d'après
/// (1 si la ligne précédente n'est pas numérotée). Une commande sans rien derrière (« Tiret. ») ou au milieu d'une phrase
/// (« il a mis un tiret ») reste du texte. La majuscule n'est pas forcée.
pub fn list_items(text: &str, bullet: ListBullet) -> String {
    if !is_match(&ANY_LIST_WORD, text) {
        return text.to_string();
    }
    let mut previous_number: Option<u32> = None;
    let mut lines = Vec::new();
    for (index, line) in text.split('\n').enumerate() {
        // Un reste sans lettre ni chiffre (« Tiret. ») n'est pas un élément.
        let rest = |content: &str| content.chars().any(|c| c.is_alphanumeric()).then(|| content.to_string());
        // Une ligne vide (« nouveau paragraphe ») ne coupe pas la numérotation.
        if line.chars().all(char::is_whitespace) {
            lines.push(line.to_string());
            continue;
        }
        let group = |re: &fancy_regex::Regex, n: usize| -> Option<(String, Option<String>)> {
            let caps = re.captures(line).ok().flatten()?;
            let content = caps.get(n)?.as_str().to_string();
            let first = if n == 2 { caps.get(1).map(|m| m.as_str().to_string()) } else { None };
            Some((content, first))
        };
        let bulleted = group(&BULLET_ITEM, 1).or_else(|| if index > 0 { group(&MISHEARD_BULLET, 1) } else { None });
        if let Some(content) = bulleted.and_then(|(c, _)| rest(&c)) {
            previous_number = None;
            lines.push(format!("{} {}", bullet.symbol(), content));
            continue;
        }
        let mut number: Option<u32> = None;
        let mut content: Option<String> = None;
        if let Some((c, said)) = group(&NUMBERED_ITEM, 2) {
            if let Some(c) = rest(&c) {
                let said = said.unwrap_or_default().to_lowercase();
                number = said.parse().ok().or_else(|| number_of_word(&said));
                content = Some(c);
            }
        }
        if content.is_none() {
            if let Some((c, _)) = group(&NEXT_ITEM, 1) {
                if let Some(c) = rest(&c) {
                    number = Some(previous_number.unwrap_or(0) + 1);
                    content = Some(c);
                }
            }
        }
        if let (Some(number), Some(content)) = (number, content) {
            previous_number = Some(number);
            lines.push(format!("{number}. {content}"));
            continue;
        }
        // Une ligne déjà numérotée (« 3. café ») continue la suite ; toute autre ligne l'interrompt.
        previous_number = NUMBERED_LINE.captures(line).ok().flatten().and_then(|c| c.get(1)).and_then(|m| m.as_str().parse().ok());
        lines.push(line.to_string());
    }
    lines.join("\n")
}

/// Une correction du dictionnaire qui n'écrit que de la ponctuation (« trois points d'exclamation » → « !!! »)
/// suit les mêmes règles d'espacement ; renvoie `None` pour une correction ordinaire (« pour cent » → « % » garde ses espaces).
pub fn kind_of(written: &str) -> Option<Kind> {
    let signs: Vec<char> = written.chars().filter(|c| *c != ' ' && *c != '\u{00A0}').collect();
    if signs.is_empty() {
        return None;
    }
    if signs.iter().all(|c| *c == '\n') {
        return Some(Kind::Line);
    }
    if signs.iter().all(|c| "([{«“".contains(*c)) {
        return Some(Kind::Opening);
    }
    if signs.iter().all(|c| ".,;:!?…)]}»”".contains(*c)) {
        return Some(Kind::Closing);
    }
    None
}

/// Mots qui, juste avant la commande, montrent qu'on parle d'un signe au lieu de le dicter :
/// « je ferme la parenthèse », « un nouveau paragraphe », « il faut passer… ».
const DESCRIBING_WORDS: [&str; 72] = [
    "je", "tu", "il", "elle", "on", "nous", "vous", "ils", "elles", "un", "une", "le", "la", "les", "ce", "cet", "cette", "du", "de",
    "des", "au", "aux", "à", "mon", "ton", "son", "notre", "votre", "leur", "pour", "sans", "quel", "quelle", "tel", "telle", "faut",
    "dois", "doit", "peux", "peut", "veux", "veut", "vais", "vas", "va",
    // Anglais (S18) : « the colon is part of… », « I open a bracket ».
    "i", "you", "he", "she", "we", "they", "the", "a", "an", "my", "your", "his", "her", "our", "their", "to", "will", "can", "must",
    "should", "need", "want", "have", "has", "this", "that", "of",
];

/// Remplace `core` (motif, sans tenir compte des majuscules) en absorbant la ponctuation que Whisper met autour.
/// Renvoie le nouveau texte et le nombre de remplacements.
pub fn replace(core: &str, kind: Kind, written: &str, text: &str) -> (String, usize) {
    let (before, after) = match kind {
        Kind::Opening => (r"[ \t]*,?[ \t]*".to_string(), r"[ \t]*[,.;:]?[ \t]*".to_string()),
        Kind::Closing => {
            // « génial, trois points d'exclamation ! » → « génial!!! » : un point ou le même signe ajouté par Whisper ferait double emploi.
            // « fermez la parenthèse, est arrivé » : la virgule de Whisper après une parenthèse, un crochet ou des guillemets saute,
            // sauf devant un mot qui ouvre une nouvelle proposition (« fermez les guillemets, puis il est parti »).
            let last = written.chars().last().unwrap_or(' ');
            let keeps = r"(?:mais|puis|et|donc|car|alors|ou|qui|que|qu'|quand|si|parce|lorsque)";
            let after = if "!?.…".contains(last) {
                format!(r"(?:[ \t]*[{}])?", fancy_regex::escape(&format!(".{last}")))
            } else if ")]»}".contains(last) {
                format!(r"(?:[ \t]*,(?=[ \t]+(?!{keeps}(?![\p{{L}}]))))?")
            } else {
                String::new()
            };
            (r"[ \t]*[,.;]?[ \t]*".to_string(), after)
        }
        // Collé : les blancs autour sont absorbés, la ponctuation de Whisper reste (la fin de phrase ne disparaît pas).
        Kind::Glued | Kind::Spaced => (r"[ \t]*".to_string(), r"[ \t]*".to_string()),
        Kind::Suffix => (r"[ \t]*".to_string(), String::new()),
        Kind::Line => {
            // Les sauts de ligne que Whisper a déjà mis autour de la commande (un par segment, deux après une pause) sont absorbés.
            // Atomiques : sans cela, une longue suite d'espaces et de sauts de ligne ralentit la recherche de façon cubique.
            let before = if written.starts_with('.') { r"(?>[ \t\n]*)[,.]?(?>[ \t\n]*)" } else { r"(?>[ \t\n]*),?(?>[ \t\n]*)" };
            (before.to_string(), r"(?>[ \t\n]*)[,.;:]?(?>[ \t\n]*)".to_string())
        }
    };
    // Pas d'apostrophe juste avant : « j'ouvre une parenthèse » se raconte, ne se dicte pas.
    let pattern = format!(r"(?i){before}(?<![\p{{L}}\p{{N}}'’])({core})(?![\p{{L}}\p{{N}}]){after}");
    let Some(regex) = cached(&pattern) else { return (text.to_string(), 0) };
    let mut out = text.to_string();
    let mut count = 0;
    // De la fin vers le début, pour que les positions restent justes pendant les remplacements.
    for (whole, group) in matches_with_group(&regex, text, 1).into_iter().rev() {
        if describes(&text[..group.0]) {
            continue;
        }
        let preceding = text[..whole.0].chars().last();
        let replacement = match kind {
            Kind::Opening => format!(" {written}"),
            Kind::Glued | Kind::Suffix => written.to_string(),
            // Espace après seulement devant un mot : « tom & jerry », mais « tom & » en fin de phrase ou devant une ponctuation.
            Kind::Spaced => match text[whole.1..].chars().next() {
                Some(c) if c.is_alphanumeric() => format!(" {written} "),
                _ => format!(" {written}"),
            },
            // « Tu viens ? Point à la ligne » : le point d'interrogation suffit.
            Kind::Line if written.starts_with('.') && preceding.is_some_and(|p| "?!….".contains(p)) => written[1..].to_string(),
            _ => written.to_string(),
        };
        out.replace_range(whole.0..whole.1, &replacement);
        count += 1;
    }
    (out, count)
}

/// Vrai si le texte qui précède la commande se termine par un mot de `DESCRIBING_WORDS` (sans ponctuation entre les deux).
fn describes(before: &str) -> bool {
    let Some(last) = before.chars().last() else { return false };
    if last != ' ' && last != '\t' {
        return false;
    }
    let trimmed = before.trim_matches(|c: char| c.is_whitespace() && c != '\n' && c != '\r');
    if !trimmed.chars().last().is_some_and(char::is_alphabetic) {
        return false;
    }
    // Le dernier mot entier : « rendez-vous » n'est pas « vous », « l'heure » n'est pas « heure ».
    let word = trimmed.split_whitespace().last().map(str::to_lowercase).unwrap_or_default();
    DESCRIBING_WORDS.contains(&word.as_str())
}

regex!(LEADING_BLANKS, r"(?m)^[ \t]+");
regex!(TRAILING_BLANKS, r"[ \t]+(?=\n)");
regex!(DOUBLE_SPACES, r"(?<=\S) {2,}");

/// Après remplacement : pas d'espace en début de ligne, pas d'espace double, majuscule en début de ligne.
pub fn tidy(text: &str) -> String {
    let t = LEADING_BLANKS.replace_all(text, "");
    let t = TRAILING_BLANKS.replace_all(&t, "");
    let t = DOUBLE_SPACES.replace_all(&t, " ");
    let mut out = String::with_capacity(t.len());
    let mut capitalize_next = false;
    for ch in t.chars() {
        if ch == '\n' {
            capitalize_next = true;
            out.push(ch);
            continue;
        }
        if capitalize_next && ch.is_alphabetic() {
            out.extend(ch.to_uppercase());
            capitalize_next = false;
            continue;
        }
        if !ch.is_whitespace() {
            capitalize_next = false;
        }
        out.push(ch);
    }
    out
}
