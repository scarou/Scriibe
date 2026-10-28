//! Le menu de l'icône : ce qu'il montre (touches, mode en cours) et ce qu'il fait (changer de mode, copier la dernière dictée). L'icône
//! (`windows::tray`) relit `view` régulièrement : le menu reste juste quand les réglages changent.

/// Un mode du sous-menu « Mode ».
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModeEntry {
    /// Valeur d'un mode livré ou identifiant d'un mode personnel (ce que `choose_mode` reçoit).
    pub id: String,
    pub label: String,
    pub summary: String,
    pub checked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MenuView {
    /// « Maintiens Ctrl droit pour dicter ».
    pub dictation_hint: String,
    pub translate_hint: String,
    /// « Mode : Simple », « Mode : reformulation coupée »…
    pub mode_title: String,
    /// Aucun moteur de mise au propre : les modes ne sont pas choisissables.
    pub modes_enabled: bool,
    /// Les modes livrés par groupes (séparés par un trait), puis les modes personnels.
    pub groups: Vec<Vec<ModeEntry>>,
    pub version: String,
}

pub trait TrayMenuSource: Send + Sync {
    fn view(&self) -> MenuView;
    fn choose_mode(&self, id: &str);
    fn copy_last_dictation(&self);
}
