// Seul le menu de l'icône de Windows l'utilise pour l'instant (macOS et Linux viendront à l'étape 7) ; les tests tournent partout.
#![cfg_attr(not(windows), allow(dead_code))]

//! Ce que le menu de l'icône montre et fait (port du menu de la barre des menus de Scribe, `AppDelegate.swift`) : les touches, le mode en
//! cours (changé en un clic), « Copier la dernière dictée ».

use scriibe_core::modes::RewriteMode;
use scriibe_core::settings::AppSettings;
use scriibe_platform::paste::Paster;
use scriibe_platform::storage::{FileConfig, FileHistory};
use scriibe_platform::tray_menu::{MenuView, ModeEntry, TrayMenuSource};
use std::sync::Arc;

/// Le menu pour ces réglages. Hors Windows, la touche par défaut est celle du système (voir `docs/ECARTS.md`).
pub fn menu_view(settings: &AppSettings, version: &str) -> MenuView {
    let dictation =
        if settings.dictation_keys.is_empty() { "Ctrl droit".to_string() } else { scriibe_platform::keys::label(&settings.dictation_keys) };
    let translate =
        if settings.translate_keys.is_empty() { "Maj".to_string() } else { scriibe_platform::keys::label(&settings.translate_keys) };
    let has_engine = settings.has_cleanup_engine();
    // Reformulation coupée : aucun mode coché ; en choisir un la rallume.
    let current = settings.rewriting_on().then(|| settings.mode_choice());
    let entry = |id: String, label: String, summary: String| {
        let checked = current.as_deref() == Some(id.as_str());
        ModeEntry { id, label, summary, checked }
    };
    let mut groups: Vec<Vec<ModeEntry>> = RewriteMode::MENU_GROUPS
        .iter()
        .map(|group| {
            group
                .iter()
                .filter(|mode| **mode != RewriteMode::Playful)
                .map(|mode| entry(mode.raw().to_string(), mode.label().to_string(), mode.summary().to_string()))
                .collect::<Vec<_>>()
        })
        .filter(|group| !group.is_empty())
        .collect();
    if !settings.custom_modes.is_empty() {
        groups.push(settings.custom_modes.iter().map(|m| entry(m.id.clone(), m.label(), "Mode personnel".to_string())).collect());
    }
    MenuView {
        dictation_hint: format!("Maintiens {dictation} pour dicter"),
        translate_hint: if has_engine {
            format!("Maintiens {dictation} + {translate} pour traduire en {}", settings.translation_language.name())
        } else {
            "Traduction indisponible : aucun moteur de mise au propre".to_string()
        },
        mode_title: if !has_engine {
            "Mode : aucun moteur (transcription littérale)".to_string()
        } else if settings.rewriting_on() {
            format!("Mode : {}", settings.active_mode_label())
        } else {
            "Mode : reformulation coupée".to_string()
        },
        modes_enabled: has_engine,
        groups,
        version: format!("Scriibe {version}"),
    }
}

pub struct TraySource {
    pub config: Arc<FileConfig>,
    pub history: Arc<FileHistory>,
    pub paster: Arc<dyn Paster>,
    pub version: String,
}

impl TrayMenuSource for TraySource {
    fn view(&self) -> MenuView {
        menu_view(&self.config.settings(), &self.version)
    }

    fn choose_mode(&self, id: &str) {
        let mut settings = self.config.settings();
        // Un mode personnel supprimé entre l'ouverture du menu et le clic : rien ne change.
        if !settings.has_cleanup_engine() {
            return;
        }
        settings.set_mode_choice(id);
        if settings.mode_choice() != id {
            return;
        }
        settings.cleanup_enabled = true;
        let _ = self.config.save_settings(&settings);
    }

    fn copy_last_dictation(&self) {
        let text = self
            .history
            .read(|store, _, _| store.last_dictation_text.clone().or_else(|| store.history.first().map(|d| d.final_text.clone())));
        if let Some(text) = text.filter(|t| !t.is_empty()) {
            self.paster.copy(&text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scriibe_core::settings::CleanupEngine;

    fn settings(engine: CleanupEngine) -> AppSettings {
        AppSettings { cleanup_engine: engine, cleanup_enabled: true, ..AppSettings::default() }
    }

    #[test]
    fn the_menu_lists_the_seven_modes_in_groups_and_checks_the_current_one() {
        let view = menu_view(&settings(CleanupEngine::LlamaCpp), "0.1.0");
        let ids: Vec<Vec<&str>> = view.groups.iter().map(|g| g.iter().map(|m| m.id.as_str()).collect()).collect();
        assert_eq!(ids, vec![vec!["literal", "simple", "formal"], vec!["wellness", "tech", "letter", "sales"]]);
        let checked: Vec<&str> = view.groups.iter().flatten().filter(|m| m.checked).map(|m| m.id.as_str()).collect();
        assert_eq!(checked, vec!["simple"]);
        assert!(view.modes_enabled);
        assert_eq!(view.mode_title, "Mode : Simple");
        assert_eq!(view.dictation_hint, "Maintiens Ctrl droit pour dicter");
        assert_eq!(view.translate_hint, "Maintiens Ctrl droit + Maj pour traduire en anglais");
    }

    #[test]
    fn without_an_engine_the_modes_are_off_and_translation_is_unavailable() {
        let view = menu_view(&settings(CleanupEngine::None), "0.1.0");
        assert!(!view.modes_enabled);
        assert_eq!(view.mode_title, "Mode : aucun moteur (transcription littérale)");
        assert_eq!(view.translate_hint, "Traduction indisponible : aucun moteur de mise au propre");
        assert!(view.groups.iter().flatten().all(|m| !m.checked));
    }

    #[test]
    fn a_switched_off_rewrite_checks_no_mode() {
        let mut s = settings(CleanupEngine::OpenRouter);
        s.cleanup_enabled = false;
        let view = menu_view(&s, "0.1.0");
        assert_eq!(view.mode_title, "Mode : reformulation coupée");
        assert!(view.groups.iter().flatten().all(|m| !m.checked));
    }

    #[test]
    fn the_chosen_keys_are_named_in_the_hints() {
        let mut s = settings(CleanupEngine::OpenRouter);
        s.dictation_keys = vec!["left_control".into(), "f9".into()];
        s.translate_keys = vec!["left_shift".into()];
        let view = menu_view(&s, "0.1.0");
        assert_eq!(view.dictation_hint, "Maintiens Ctrl gauche + F9 pour dicter");
        assert_eq!(view.translate_hint, "Maintiens Ctrl gauche + F9 + Maj gauche pour traduire en anglais");
    }
}
