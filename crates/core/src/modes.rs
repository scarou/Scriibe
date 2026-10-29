//! Modes de reformulation : les huit modes livrés, les modes personnels et ce que le modèle reçoit pour chacun.
//! Port de `Modes.swift` et des parties de `Models.swift` qui concernent les modes (Scribe).
//!
//! La consigne de base (ne jamais répondre, garder les personnes…) et le garde-fou restent fixes ; seule la phrase de style change,
//! ou s'ajoute dans un mode créé par la personne.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Le ton de la mise au propre. Littéral : aucun modèle, seulement signes dictés, dictionnaire et typographie.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RewriteMode {
    Literal,
    #[default]
    Simple,
    Formal,
    Wellness,
    Tech,
    Letter,
    Sales,
    Playful,
}

impl RewriteMode {
    /// Dans l'ordre de Scribe (`CaseIterable`) : l'empreinte des prompts en dépend.
    pub const ALL: [RewriteMode; 8] =
        [Self::Literal, Self::Simple, Self::Formal, Self::Wellness, Self::Tech, Self::Letter, Self::Sales, Self::Playful];

    /// Groupes du menu (séparés par un trait).
    pub const MENU_GROUPS: [&'static [RewriteMode]; 3] =
        [&[Self::Literal, Self::Simple, Self::Formal], &[Self::Wellness, Self::Tech, Self::Letter, Self::Sales], &[Self::Playful]];

    /// Valeur écrite dans les fichiers de réglages.
    pub fn raw(self) -> &'static str {
        match self {
            Self::Literal => "literal",
            Self::Simple => "simple",
            Self::Formal => "formal",
            Self::Wellness => "wellness",
            Self::Tech => "tech",
            Self::Letter => "letter",
            Self::Sales => "sales",
            Self::Playful => "playful",
        }
    }

    pub fn from_raw(raw: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|m| m.raw() == raw)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Literal => "Littéral",
            Self::Simple => "Simple",
            Self::Formal => "Soutenu",
            Self::Wellness => "Pro · Sophrologie et bien-être",
            Self::Tech => "Pro · Informatique et IA",
            Self::Letter => "Pro · Courrier administratif",
            Self::Sales => "Pro · Commercial",
            Self::Playful => "Amusant (schtroumpf)",
        }
    }

    /// Le nom du mode pour l'interface en anglais (le menu de l'icône ; la page a sa propre traduction, `ui/src/lib/locales/en-server.ts`).
    pub fn label_en(self) -> &'static str {
        match self {
            Self::Literal => "Literal",
            Self::Simple => "Simple",
            Self::Formal => "Formal",
            Self::Wellness => "Pro · Sophrology and wellbeing",
            Self::Tech => "Pro · IT and AI",
            Self::Letter => "Pro · Official letters",
            Self::Sales => "Pro · Sales",
            Self::Playful => "Playful (Smurf)",
        }
    }

    pub fn summary(self) -> &'static str {
        match self {
            Self::Literal => "Aucune reformulation : signes dictés, dictionnaire et typographie seulement",
            Self::Simple => "Retire les « euh », met la ponctuation, garde tes mots",
            Self::Formal => "Français soutenu, tournures familières reformulées",
            Self::Wellness => "Ton posé et bienveillant, vocabulaire de la sophrologie",
            Self::Tech => "Précis et concis, termes techniques gardés en anglais",
            Self::Letter => "Neutre et poli, destinataire vouvoyé",
            Self::Sales => "Engageant et clair, orienté client",
            Self::Playful => "Pour rire : le texte mis au propre (Simple), puis passé en langage schtroumpf sur cet ordinateur",
        }
    }

    /// Les modes de style réorganisent les phrases : le garde-fou compte alors les mots en commun avec plus de souplesse.
    /// Amusant n'en est pas : le modèle fait une mise au propre Simple, puis le moteur schtroumpf transforme le texte.
    pub fn rewrites_freely(self) -> bool {
        !matches!(self, Self::Literal | Self::Simple | Self::Playful)
    }

    /// Consigne de style ajoutée au prompt ; `None` pour Simple (la consigne de base suffit) et Littéral (pas de modèle).
    pub fn style(self) -> Option<&'static str> {
        match self {
            Self::Literal | Self::Simple | Self::Playful => None,
            Self::Formal => Some("Écris dans un français soutenu et élégant : reformule les tournures familières, choisis des mots plus recherchés."),
            Self::Wellness => Some("Écris en professionnel de la sophrologie et du bien-être : ton posé, chaleureux et bienveillant, vocabulaire juste du métier (séance, protocole, respiration, relaxation dynamique, visualisation, verbalisation)."),
            Self::Tech => Some("Écris en professionnel de l'informatique et de l'IA : ton précis et concis ; garde en anglais les termes techniques d'usage (prompt, build, merge, commit, pipeline, API)."),
            Self::Letter => Some("Écris en style de courrier administratif : ton neutre, poli et clair, phrases complètes ; vouvoie le destinataire."),
            Self::Sales => Some("Écris en style commercial : ton engageant, positif et clair, orienté vers le client, phrases courtes."),
        }
    }

    /// Les modes dont la phrase de style se modifie (Simple n'en a pas ; Littéral et Amusant n'en envoient pas).
    pub fn has_style(self) -> bool {
        self.style().is_some()
    }

    /// Le mode tel qu'il est livré, avec sa phrase de style d'origine.
    pub fn profile(self) -> ModeProfile {
        ModeProfile {
            rewrites: self.rewrites_freely(),
            style: self.style().map(str::to_string),
            vouvoie: self == Self::Letter,
            examples: if !self.rewrites_freely() {
                Examples::Faithful
            } else if self == Self::Letter {
                Examples::None
            } else {
                Examples::Style
            },
        }
    }
}

/// Un exemple donné au modèle : une dictée, puis le texte attendu dans ce mode.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ModeExample {
    pub id: String,
    pub dictation: String,
    pub expected: String,
}

impl ModeExample {
    pub fn new(dictation: &str, expected: &str) -> Self {
        Self { id: new_id(), dictation: dictation.into(), expected: expected.into() }
    }

    /// Les deux côtés remplis : sinon l'exemple n'est pas envoyé.
    pub fn is_complete(&self) -> bool {
        !self.dictation.trim().is_empty() && !self.expected.trim().is_empty()
    }

    /// Lecture tolérante : un champ illisible garde une valeur sûre ; `None` si ce n'est pas un objet.
    pub fn from_value(v: &Value) -> Option<Self> {
        let o = v.as_object()?;
        Some(Self { id: id_of(o.get("id")), dictation: string_of(o.get("dictation")), expected: string_of(o.get("expected")) })
    }
}

/// Ce que le modèle reçoit pour un mode : sa phrase de style et sa façon de réécrire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModeProfile {
    /// Reformule les phrases (modes métiers) : garde-fou plus souple, exemples de style, nouvel essai en Simple si écarté.
    pub rewrites: bool,
    /// Phrase de style ; `None` pour Simple.
    pub style: Option<String>,
    /// Courrier : le destinataire est vouvoyé, la personne à qui l'on parle n'est donc pas gardée telle quelle.
    pub vouvoie: bool,
    pub examples: Examples,
}

/// Exemples donnés au modèle avant la dictée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Examples {
    Faithful,
    Style,
    None,
    Own(Vec<ModeExample>),
}

impl ModeProfile {
    /// Simple tel quel : ni style, ni consigne, ni exemples à soi. Seul ce mode peut sauter le modèle (« Sauter si déjà propre »,
    /// dictées de trois mots ou moins) ; une consigne en plus doit toujours passer par le modèle.
    pub fn is_plain_simple(&self) -> bool {
        !self.rewrites && self.style.is_none() && self.examples == Examples::Faithful
    }

    /// Consigne tapée par la personne, prête pour la liste à tirets du prompt : une seule ligne, finie par un point.
    /// Les phrases livrées finissent déjà par un point et ne changent pas.
    pub fn sentence(text: &str) -> Option<String> {
        let line = text
            .split(['\n', '\r', '\u{0B}', '\u{0C}', '\u{85}', '\u{2028}', '\u{2029}'])
            .map(|l| l.trim_matches(|c: char| c == ' ' || c == '\t' || c == '\u{00A0}'))
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        if line.is_empty() {
            return None;
        }
        Some(if line.ends_with(['.', '!', '?', '…']) { line } else { line + "." })
    }
}

/// Un mode créé dans Réglages → Les modes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomMode {
    pub id: String,
    pub name: String,
    pub style: String,
    /// Vrai : reformule comme les modes métiers ; faux : garde les mots, comme Simple, avec la consigne en plus.
    pub rewrites: bool,
    /// Comme Courrier : le destinataire est vouvoyé (et, sans exemples à soi, aucun exemple : ceux de Soutenu tutoient).
    pub vouvoie: bool,
    /// Ses propres exemples ; sans eux, ceux de Simple (fidèle) ou de Soutenu (reformule).
    pub examples: Vec<ModeExample>,
}

impl CustomMode {
    pub fn new(name: &str, style: &str, rewrites: bool) -> Self {
        Self { id: new_id(), name: name.into(), style: style.into(), rewrites, vouvoie: false, examples: Vec::new() }
    }

    /// Le nom affiché : jamais vide.
    pub fn label(&self) -> String {
        let n = self.name.trim();
        if n.is_empty() {
            "Mode sans nom".to_string()
        } else {
            n.to_string()
        }
    }

    pub fn profile(&self) -> ModeProfile {
        let own: Vec<ModeExample> = self.examples.iter().filter(|e| e.is_complete()).cloned().collect();
        // Le vouvoiement ne vaut que pour un mode qui reformule (un mode fidèle garde celui de la dictée).
        let shots = if !own.is_empty() {
            Examples::Own(own)
        } else if !self.rewrites {
            Examples::Faithful
        } else if self.vouvoie {
            Examples::None
        } else {
            Examples::Style
        };
        ModeProfile {
            rewrites: self.rewrites,
            style: ModeProfile::sentence(&self.style),
            vouvoie: self.rewrites && self.vouvoie,
            examples: shots,
        }
    }

    /// Lecture tolérante : un champ illisible garde une valeur sûre (mode fidèle) ; `None` si ce n'est pas un objet.
    pub fn from_value(v: &Value) -> Option<Self> {
        let o = v.as_object()?;
        Some(Self {
            id: id_of(o.get("id")),
            name: string_of(o.get("name")),
            style: string_of(o.get("style")),
            rewrites: bool_of(o.get("rewrites")),
            vouvoie: bool_of(o.get("vouvoie")),
            examples: o
                .get("examples")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(ModeExample::from_value).collect())
                .unwrap_or_default(),
        })
    }
}

/// Un nouvel identifiant, écrit comme `UUID.uuidString` de Swift (majuscules).
pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string().to_uppercase()
}

/// Un identifiant lu dans un fichier : celui du fichier s'il est valide, sinon un nouveau.
pub(crate) fn id_of(v: Option<&Value>) -> String {
    parse_id(v).unwrap_or_else(new_id)
}

/// Un identifiant valide, en majuscules, ou `None`.
pub fn parse_id(v: Option<&Value>) -> Option<String> {
    v.and_then(Value::as_str).and_then(|s| uuid::Uuid::parse_str(s).ok()).map(|u| u.to_string().to_uppercase())
}

fn string_of(v: Option<&Value>) -> String {
    v.and_then(Value::as_str).unwrap_or("").to_string()
}

fn bool_of(v: Option<&Value>) -> bool {
    v.and_then(Value::as_bool).unwrap_or(false)
}
