//! Le menu de l'icône : ce qu'il montre (touches, mode en cours) et ce qu'il fait (changer de mode, copier la dernière dictée). L'icône
//! (`windows::tray`, et celle de la barre des menus du Mac, dans `crates/cli`) relit `view` régulièrement : le menu reste juste quand les
//! réglages changent. Le dessin de l'icône et le texte de l'infobulle, communs aux deux, sont ici.

use crate::indicator::IndicatorState;

/// Côté de l'icône, en points.
pub const ICON_SIZE: u32 = 32;

/// Transparence de la plume du logo (32×32, un octet par point), tracée depuis `logo/scriibe-symbole.svg` par `tools/faire_icones.*`.
const FEATHER: &[u8; (ICON_SIZE * ICON_SIZE) as usize] = include_bytes!("plume32.alpha");

fn color_of(state: &IndicatorState) -> [u8; 3] {
    match state {
        IndicatorState::Hidden => [120, 124, 134],
        IndicatorState::Listening => [224, 56, 56],
        IndicatorState::Working => [240, 160, 32],
        IndicatorState::Done => [64, 180, 96],
        IndicatorState::Message(_) => [64, 120, 224],
    }
}

/// L'icône pour cet état : un disque de la couleur de l'état, avec la plume du logo en blanc (RGBA, `ICON_SIZE` × `ICON_SIZE`).
pub fn status_icon_rgba(state: &IndicatorState) -> Vec<u8> {
    let color = color_of(state);
    let half = ICON_SIZE as f32 / 2.0;
    let mut rgba = Vec::with_capacity((ICON_SIZE * ICON_SIZE * 4) as usize);
    for y in 0..ICON_SIZE {
        for x in 0..ICON_SIZE {
            let (dx, dy) = (x as f32 + 0.5 - half, y as f32 + 0.5 - half);
            let distance = (dx * dx + dy * dy).sqrt();
            // Bord adouci sur un pixel.
            let alpha = (half - 1.5 - distance).clamp(0.0, 1.0);
            let feather = f32::from(FEATHER[(y * ICON_SIZE + x) as usize]) / 255.0;
            let mix = |c: u8| (f32::from(c) * (1.0 - feather) + 255.0 * feather) as u8;
            rgba.extend_from_slice(&[mix(color[0]), mix(color[1]), mix(color[2]), (alpha * 255.0) as u8]);
        }
    }
    rgba
}

/// L'infobulle de l'icône : au repos `idle_tip` (rappelle la touche), sinon l'état, avec la langue de traduction pendant l'écoute.
/// Au plus 120 signes (la limite de Windows est 127).
pub fn state_tooltip(state: &IndicatorState, idle_tip: &str, translation: Option<&str>, english: bool) -> String {
    let tip = match state {
        IndicatorState::Hidden => idle_tip.to_string(),
        IndicatorState::Listening => match (translation, english) {
            (Some(language), false) => format!("Scriibe — à l'écoute (traduction : {language})"),
            (Some(language), true) => format!("Scriibe — listening (translation: {language})"),
            (None, false) => "Scriibe — à l'écoute…".to_string(),
            (None, true) => "Scriibe — listening…".to_string(),
        },
        other => format!("Scriibe — {}", crate::indicator::describe_in(other, english)),
    };
    tip.chars().take(120).collect()
}

/// Un mode du sous-menu « Mode ».
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModeEntry {
    /// Valeur d'un mode livré ou identifiant d'un mode personnel (ce que `choose_mode` reçoit).
    pub id: String,
    pub label: String,
    pub summary: String,
    pub checked: bool,
}

/// Les textes fixes du menu, dans la langue de l'interface.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MenuLabels {
    pub open_page: String,
    pub copy_last: String,
    pub mode: String,
    pub open_data: String,
    pub open_log: String,
    pub quit: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MenuView {
    /// L'interface est en anglais (pour l'infobulle de l'icône).
    pub english: bool,
    pub labels: MenuLabels,
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
    /// « Nouvelle version disponible (abc1234) » quand la vérification en a trouvé une : entrée en tête du menu, qui ouvre la fenêtre.
    pub update: Option<String>,
}

pub trait TrayMenuSource: Send + Sync {
    fn view(&self) -> MenuView;
    fn choose_mode(&self, id: &str);
    fn copy_last_dictation(&self);
}
