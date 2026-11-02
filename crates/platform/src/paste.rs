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

pub trait Paster: Send + Sync {
    /// Met `text` dans le presse-papiers, attend que les touches de modification soient relâchées, envoie le collage.
    /// `restore_clipboard` : remet ensuite l'ancien contenu (en arrière-plan), s'il n'a pas changé entre-temps.
    fn paste(&self, text: &str, restore_clipboard: bool) -> PasteOutcome;
    /// Un mot ou un groupe de mots est-il sélectionné dans l'appli active ? Le texte collé le remplacera. Sans réponse du système (appli qui
    /// n'expose pas sa sélection, système sans cette fonction) : `active` est faux. À appeler une fois, juste avant `paste`.
    fn selection(&self) -> Selection {
        Selection::default()
    }
    /// Met `text` dans le presse-papiers, sans coller (« Copier la dernière dictée »). Faux si le système n'a pas pu.
    fn copy(&self, _text: &str) -> bool {
        false
    }
}
