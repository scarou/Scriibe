//! L'appli au premier plan (pour l'historique et le tableau de bord).

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppInfo {
    /// Nom affiché (« Inconnue » si on ne sait pas).
    pub name: String,
    /// Identifiant stable : nom de l'exécutable sous Windows et Linux, identifiant de bundle sous macOS.
    pub id: Option<String>,
}

impl AppInfo {
    pub fn unknown() -> Self {
        Self { name: "Inconnue".into(), id: None }
    }
}

pub trait ForegroundApp: Send + Sync {
    fn current(&self) -> AppInfo;
}
