//! Collage dans l'appli active : presse-papiers + Ctrl+V (⌘V sur Mac), puis ancien presse-papiers remis.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PasteOutcome {
    Pasted,
    /// Le texte est dans le presse-papiers mais n'a pas pu être collé tout seul (raison courte, pour le message).
    CopiedOnly(String),
}

/// La sélection de l'appli active, avec en quelques mots pour le journal comment on l'a su (« active », « aucune », « inconnue (…) » ; vide
/// si le système ne sait pas la lire). Jamais le texte sélectionné.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Selection {
    pub active: bool,
    pub report: String,
}

/// Le texte sélectionné, lu pour une commande vocale (`docs/COMMANDES.md`). Il peut être confidentiel : ni `Debug` ni `Display` n'en
/// montrent le contenu (un `{:?}` perdu dans un message ou un journal ne le laisse pas passer), il n'est ni cloné ni sérialisé, et sa
/// mémoire est écrasée quand il disparaît. Seul `expose` le donne, à l'endroit précis où une commande en a besoin.
pub struct SelectedText(String);

impl SelectedText {
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    /// Le nombre de caractères : la seule chose qu'on peut dire au journal.
    pub fn char_count(&self) -> usize {
        self.0.chars().count()
    }

    /// Le texte lui-même, pour la commande qui en a besoin (ne jamais l'écrire dans un journal, un message ou l'historique).
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for SelectedText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SelectedText(<{} signes>)", self.char_count())
    }
}

impl Drop for SelectedText {
    fn drop(&mut self) {
        let mut bytes = std::mem::take(&mut self.0).into_bytes();
        bytes.fill(0);
        std::hint::black_box(&bytes);
    }
}

/// Pourquoi la sélection n'a pas pu être lue (raisons à texte fixe : elles peuvent aller dans le journal et la pastille).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unreadable {
    /// macOS : l'autorisation « Accessibilité » manque.
    NotTrusted,
    /// Le presse-papiers contient une image ou des fichiers : le ⌘C d'essai les écraserait pour de bon.
    ClipboardNotText,
    /// Le système ne sait pas lire la sélection (ou la lecture a échoué).
    Failed,
}

impl Unreadable {
    pub fn reason(self) -> &'static str {
        match self {
            Self::NotTrusted => "autorisation Accessibilité manquante",
            Self::ClipboardNotText => "le presse-papiers contient autre chose que du texte",
            Self::Failed => "sélection illisible",
        }
    }
}

/// Le résultat de `Paster::selected_text`.
#[derive(Debug)]
pub enum SelectedRead {
    Text(SelectedText),
    /// Aucun texte n'est sélectionné.
    NoSelection,
    Unreadable(Unreadable),
}

/// Raisons à texte fixe d'un raccourci non envoyé (`Paster::press_shortcut`).
pub const SHORTCUT_UNSUPPORTED: &str = "non pris en charge sur ce système";
pub const SHORTCUT_NOT_TRUSTED: &str = "autorisation Accessibilité manquante";
pub const SHORTCUT_KEY_UNKNOWN: &str = "touche non prise en charge";

pub trait Paster: Send + Sync {
    /// Met `text` dans le presse-papiers, attend que les touches de modification soient relâchées, envoie le collage.
    /// `restore_clipboard` : remet ensuite l'ancien contenu (en arrière-plan), s'il n'a pas changé entre-temps.
    fn paste(&self, text: &str, restore_clipboard: bool) -> PasteOutcome;
    /// Un mot ou un groupe de mots est-il sélectionné dans l'appli active ? Le texte collé le remplacera. Sans réponse du système (appli qui
    /// n'expose pas sa sélection, système sans cette fonction) : `active` est faux. À appeler une fois, juste avant `paste`.
    fn selection(&self) -> Selection {
        Selection::default()
    }
    /// Lit le texte sélectionné dans l'appli active, pour une commande vocale. À n'appeler qu'une fois la commande reconnue (la lecture peut
    /// passer par un ⌘C d'essai). Le presse-papiers est remis tel qu'il était. Par défaut : le système ne sait pas.
    fn selected_text(&self) -> SelectedRead {
        SelectedRead::Unreadable(Unreadable::Failed)
    }
    /// Appuie un raccourci de l'appli active (gras, italique, souligné) sur la sélection, qui reste telle quelle. Renvoie la raison, à texte fixe
    /// (elle peut aller dans le journal et la pastille), si le raccourci n'a pas pu être envoyé. Par défaut : le système ne sait pas.
    fn press_shortcut(&self, _shortcut: &scriibe_core::shortcut::Shortcut) -> Result<(), &'static str> {
        Err(SHORTCUT_UNSUPPORTED)
    }
    /// Met `text` dans le presse-papiers, sans coller (« Copier la dernière dictée »). Faux si le système n'a pas pu.
    fn copy(&self, _text: &str) -> bool {
        false
    }
}
