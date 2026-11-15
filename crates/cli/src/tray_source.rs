// Seul le menu de l'icône de Windows l'utilise pour l'instant (macOS et Linux viendront à l'étape 7) ; les tests tournent partout.
#![cfg_attr(not(windows), allow(dead_code))]

//! Ce que le menu de l'icône montre et fait (port du menu de la barre des menus de Scribe, `AppDelegate.swift`) : les touches, le mode en
//! cours (changé en un clic), « Copier la dernière dictée ».

use scriibe_core::modes::RewriteMode;
use scriibe_core::settings::{AppSettings, UiLang};
use scriibe_platform::paste::Paster;
use scriibe_platform::profiles::ProfileInfo;
use scriibe_platform::storage::{FileConfig, FileHistory};
use scriibe_platform::tray_menu::{MenuLabels, MenuView, ModeEntry, ProfileEntry, TrayMenuSource};
use std::sync::Arc;

/// Le menu pour ces réglages, dans la langue de l'interface (le choix, ou celle du système).
pub fn menu_view(settings: &AppSettings, version: &str) -> MenuView {
    let lang = settings.ui_language.resolve(&scriibe_platform::locale::system_language());
    let mut view = menu_view_in(settings, version, lang);
    view.update = crate::web::update::available().map(|v| update_text(&v.version, lang));
    view
}

/// L'entrée du menu qui annonce une nouvelle version.
fn update_text(version: &str, lang: UiLang) -> String {
    if lang == UiLang::En {
        format!("New version available ({version})")
    } else {
        format!("Nouvelle version disponible ({version})")
    }
}

/// Le menu pour ces réglages et cette langue. Hors Windows, la touche par défaut est celle du système (voir `docs/ECARTS.md`).
pub fn menu_view_in(settings: &AppSettings, version: &str, lang: UiLang) -> MenuView {
    let english = lang == UiLang::En;
    let name = |label: String| if english { scriibe_platform::keys::english_label(&label) } else { label };
    let dictation = name(if settings.dictation_keys.is_empty() {
        scriibe_platform::keys::default_dictation_label().to_string()
    } else {
        scriibe_platform::keys::label(&settings.dictation_keys)
    });
    let translate = name(if settings.translate_keys.is_empty() {
        scriibe_platform::keys::default_translate_label().to_string()
    } else {
        scriibe_platform::keys::label(&settings.translate_keys)
    });
    let has_engine = settings.has_cleanup_engine();
    // Reformulation coupée : aucun mode coché ; en choisir un la rallume.
    // Le mode Amusant n'est plus proposé : la page le montre comme Simple, le menu aussi.
    let current = settings.rewriting_on().then(|| settings.mode_choice()).map(|c| {
        if c == RewriteMode::Playful.raw() {
            RewriteMode::Simple.raw().to_string()
        } else {
            c
        }
    });
    let entry = |id: String, label: String, summary: String| {
        let checked = current.as_deref() == Some(id.as_str());
        ModeEntry { id, label, summary, checked }
    };
    let mut groups: Vec<Vec<ModeEntry>> = RewriteMode::MENU_GROUPS
        .iter()
        .map(|group| {
            group
                .iter()
                .filter(|mode| **mode != RewriteMode::Playful && !settings.is_hidden_in_menu(mode.raw()))
                .map(|mode| {
                    entry(
                        mode.raw().to_string(),
                        (if english { mode.label_en() } else { mode.label() }).to_string(),
                        mode.summary().to_string(),
                    )
                })
                .collect::<Vec<_>>()
        })
        .filter(|group| !group.is_empty())
        .collect();
    // Les modes décochés dans les réglages n'y figurent pas, même le mode actif ; un groupe vide disparaît.
    let custom: Vec<ModeEntry> = settings
        .custom_modes
        .iter()
        .filter(|m| !settings.is_hidden_in_menu(&m.id))
        .map(|m| entry(m.id.clone(), m.label(), (if english { "Custom mode" } else { "Mode personnel" }).to_string()))
        .collect();
    if !custom.is_empty() {
        groups.push(custom);
    }
    let mode_label = RewriteMode::from_raw(&settings.mode_choice())
        .filter(|_| english)
        .map_or_else(|| settings.active_mode_label(), |m| m.label_en().to_string());
    let (dictation_hint, translate_hint, mode_title, labels) = if english {
        (
            format!("Hold {dictation} to dictate"),
            if has_engine {
                format!("Hold {dictation} + {translate} to translate into {}", settings.translation_language.name_en())
            } else {
                "Translation unavailable: no cleanup engine".to_string()
            },
            if !has_engine {
                "Mode: no engine (literal transcription)".to_string()
            } else if settings.rewriting_on() {
                format!("Mode: {mode_label}")
            } else {
                "Mode: rewriting off".to_string()
            },
            MenuLabels {
                open_page: "Open Scriibe".into(),
                copy_last: "Copy the last dictation".into(),
                mode: "Mode".into(),
                user: "User".into(),
                open_data: "Open the data folder".into(),
                open_log: "Open the log".into(),
                quit: "Quit Scriibe".into(),
            },
        )
    } else {
        (
            format!("Maintiens {dictation} pour dicter"),
            if has_engine {
                format!("Maintiens {dictation} + {translate} pour traduire en {}", settings.translation_language.name())
            } else {
                "Traduction indisponible : aucun moteur de mise au propre".to_string()
            },
            if !has_engine {
                "Mode : aucun moteur (transcription littérale)".to_string()
            } else if settings.rewriting_on() {
                format!("Mode : {}", settings.active_mode_label())
            } else {
                "Mode : reformulation coupée".to_string()
            },
            MenuLabels {
                open_page: "Ouvrir Scriibe".into(),
                copy_last: "Copier la dernière dictée".into(),
                mode: "Mode".into(),
                user: "Utilisateur".into(),
                open_data: "Ouvrir le dossier des données".into(),
                open_log: "Ouvrir le journal".into(),
                quit: "Quitter Scriibe".into(),
            },
        )
    };
    MenuView {
        english,
        labels,
        dictation_hint,
        translate_hint,
        mode_title,
        modes_enabled: has_engine,
        groups,
        version: format!("Scriibe {version}"),
        profile_id: String::new(),
        profile_title: String::new(),
        profiles: Vec::new(),
        update: None,
    }
}

/// Ajoute à la vue le sous-menu des utilisateurs (étape 4c, décision 3) : l'utilisateur actif coché, **seulement s'il y en a au moins deux**
/// (une personne seule ne voit aucun changement). `active` : l'identifiant de l'utilisateur actif ; `None` : rien n'est rangé par utilisateur.
pub fn with_profiles(view: &mut MenuView, active: Option<&str>, users: &[ProfileInfo]) {
    view.profile_id = active.unwrap_or_default().to_string();
    if users.len() < 2 {
        return;
    }
    let is_active = |p: &ProfileInfo| active.is_some_and(|a| a.eq_ignore_ascii_case(&p.id));
    view.profiles = users.iter().map(|p| ProfileEntry { id: p.id.clone(), label: p.name.clone(), checked: is_active(p) }).collect();
    let name = users.iter().find(|p| is_active(p)).map_or("?", |p| p.name.as_str());
    let separator = if view.english { ": " } else { " : " };
    view.profile_title = format!("{}{separator}{name}", view.labels.user);
}

pub struct TraySource {
    pub config: Arc<FileConfig>,
    pub history: Arc<FileHistory>,
    pub paster: Arc<dyn Paster>,
    pub version: String,
    /// Une dictée est en cours ou en traitement : on ne change pas d'utilisateur à ce moment (elle serait enregistrée chez l'autre).
    pub busy: Arc<dyn Fn() -> bool + Send + Sync>,
    /// Le journal (le passage à un autre utilisateur y est noté, comme celui demandé par la page).
    pub log: Option<Arc<dyn scriibe_engines::runtime::Log>>,
}

impl TrayMenuSource for TraySource {
    fn view(&self) -> MenuView {
        // Lu d'un seul tenant : un changement d'utilisateur ne peut pas s'intercaler entre les réglages (les modes) et l'identifiant, sinon le menu
        // montrerait les modes d'un utilisateur avec l'identifiant de l'autre et le clic suivant s'appliquerait chez celui-ci (relecture de la tâche 12).
        let _guard = self.config.lock_edit();
        let mut view = menu_view(&self.config.settings(), &self.version);
        with_profiles(&mut view, self.config.active_profile_id().as_deref(), &self.config.cached_profiles());
        view
    }

    fn choose_mode(&self, profile: &str, id: &str) {
        // Le menu montrait cet utilisateur : si ce n'est plus lui, le mode ne s'applique pas à l'autre (verrou et contrôle ensemble).
        let Ok(_guard) = self.config.lock_for(Some(profile).filter(|p| !p.is_empty())) else { return };
        let mut settings = self.config.settings();
        // Un mode personnel supprimé entre l'ouverture du menu et le clic : rien ne change.
        if !settings.has_cleanup_engine() {
            return;
        }
        if settings.pick_mode(id) {
            let _ = self.config.save_settings(&settings);
        }
    }

    fn choose_profile(&self, id: &str) {
        match crate::web::switch_profile(&self.config, &self.history, (self.busy)(), id) {
            Ok(user) => {
                if let Some(log) = &self.log {
                    log.write(&format!("utilisateur: passage à « {} » (demandé par le menu de l'icône)", user.name));
                }
            }
            Err(e) => {
                if let Some(log) = &self.log {
                    log.write(&format!("utilisateur: changement refusé ({e:?})"));
                }
            }
        }
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
    use scriibe_core::modes::CustomMode;
    use scriibe_core::settings::CleanupEngine;

    fn settings(engine: CleanupEngine) -> AppSettings {
        AppSettings { cleanup_engine: engine, cleanup_enabled: true, ..AppSettings::default() }
    }

    #[test]
    fn the_menu_lists_the_eight_modes_in_groups_and_checks_the_current_one() {
        let view = menu_view_in(&settings(CleanupEngine::LlamaCpp), "0.1.0", UiLang::Fr);
        let ids: Vec<Vec<&str>> = view.groups.iter().map(|g| g.iter().map(|m| m.id.as_str()).collect()).collect();
        assert_eq!(ids, vec![vec!["literal", "simple", "formal", "calm"], vec!["wellness", "tech", "letter", "sales"]]);
        let checked: Vec<&str> = view.groups.iter().flatten().filter(|m| m.checked).map(|m| m.id.as_str()).collect();
        assert_eq!(checked, vec!["simple"]);
        assert!(view.modes_enabled);
        assert_eq!(view.mode_title, "Mode : Simple");
        let (key, translate) = (scriibe_platform::keys::default_dictation_label(), scriibe_platform::keys::default_translate_label());
        assert_eq!(view.dictation_hint, format!("Maintiens {key} pour dicter"));
        assert_eq!(view.translate_hint, format!("Maintiens {key} + {translate} pour traduire en anglais"));
    }

    #[test]
    fn unchecked_modes_leave_the_menu_even_the_current_one_and_empty_groups_vanish() {
        let mut s = settings(CleanupEngine::LlamaCpp);
        s.hidden_modes = vec!["simple".into(), "wellness".into(), "tech".into(), "letter".into(), "sales".into()];
        let view = menu_view_in(&s, "0.1.0", UiLang::Fr);
        let ids: Vec<Vec<&str>> = view.groups.iter().map(|g| g.iter().map(|m| m.id.as_str()).collect()).collect();
        assert_eq!(ids, vec![vec!["literal", "formal", "calm"]]);
        assert!(view.groups.iter().flatten().all(|m| !m.checked), "le mode actif (Simple) est masqué : aucune coche");
        assert_eq!(view.mode_title, "Mode : Simple", "le mode reste actif");
    }

    #[test]
    fn unchecked_custom_modes_leave_their_group() {
        let mut s = settings(CleanupEngine::LlamaCpp);
        s.custom_modes = vec![CustomMode::from_value(
            &serde_json::json!({"id": "4F6D1B7E-0C1A-4E55-9C0B-2A7F3C9D8E11", "name": "SMS", "style": "Court."}),
        )
        .unwrap()];
        assert_eq!(menu_view_in(&s, "0.1.0", UiLang::Fr).groups.len(), 3);
        s.hidden_modes = vec!["4F6D1B7E-0C1A-4E55-9C0B-2A7F3C9D8E11".into()];
        assert_eq!(menu_view_in(&s, "0.1.0", UiLang::Fr).groups.len(), 2);
    }

    fn user(id: &str, name: &str) -> ProfileInfo {
        ProfileInfo { id: id.into(), name: name.into(), folder: std::path::PathBuf::from(id) }
    }

    #[test]
    fn a_single_user_sees_no_user_submenu_but_the_menu_still_knows_who_is_active() {
        let mut view = menu_view_in(&settings(CleanupEngine::LlamaCpp), "0.1.0", UiLang::Fr);
        with_profiles(&mut view, Some("AAAA"), &[user("AAAA", "Anne")]);
        assert!(view.profiles.is_empty() && view.profile_title.is_empty(), "une personne seule ne voit aucun changement");
        assert_eq!(view.profile_id, "AAAA", "l'identifiant part quand même avec un clic sur un mode");
        let mut bare = menu_view_in(&settings(CleanupEngine::LlamaCpp), "0.1.0", UiLang::Fr);
        with_profiles(&mut bare, None, &[]);
        assert_eq!((bare.profile_id.as_str(), bare.profiles.len()), ("", 0), "aucun utilisateur rangé : rien");
    }

    #[test]
    fn from_two_users_the_submenu_checks_the_active_one_and_is_titled_in_the_interface_language() {
        let users = [user("AAAA", "Anne"), user("BBBB", "Bruno")];
        let mut view = menu_view_in(&settings(CleanupEngine::LlamaCpp), "0.1.0", UiLang::Fr);
        with_profiles(&mut view, Some("bbbb"), &users);
        assert_eq!(view.profile_title, "Utilisateur : Bruno");
        let checked: Vec<(&str, bool)> = view.profiles.iter().map(|p| (p.id.as_str(), p.checked)).collect();
        assert_eq!(checked, vec![("AAAA", false), ("BBBB", true)], "les majuscules de l'identifiant ne comptent pas");
        let mut english = menu_view_in(&settings(CleanupEngine::LlamaCpp), "0.1.0", UiLang::En);
        with_profiles(&mut english, Some("AAAA"), &users);
        assert_eq!(english.profile_title, "User: Anne");
        // Deux vues qui ne diffèrent que par l'utilisateur coché sont différentes : le menu se met à jour.
        let mut other = menu_view_in(&settings(CleanupEngine::LlamaCpp), "0.1.0", UiLang::En);
        with_profiles(&mut other, Some("BBBB"), &users);
        assert_ne!(english, other);
    }

    #[test]
    fn the_update_entry_names_the_new_commit_in_the_interface_language() {
        assert_eq!(update_text("0.2.0", UiLang::Fr), "Nouvelle version disponible (0.2.0)");
        assert_eq!(update_text("0.2.0", UiLang::En), "New version available (0.2.0)");
        assert_eq!(menu_view_in(&settings(CleanupEngine::LlamaCpp), "0.1.0", UiLang::Fr).update, None);
    }

    #[test]
    fn without_an_engine_the_modes_are_off_and_translation_is_unavailable() {
        let view = menu_view_in(&settings(CleanupEngine::None), "0.1.0", UiLang::Fr);
        assert!(!view.modes_enabled);
        assert_eq!(view.mode_title, "Mode : aucun moteur (transcription littérale)");
        assert_eq!(view.translate_hint, "Traduction indisponible : aucun moteur de mise au propre");
        assert!(view.groups.iter().flatten().all(|m| !m.checked));
    }

    #[test]
    fn a_switched_off_rewrite_checks_no_mode() {
        let mut s = settings(CleanupEngine::OpenRouter);
        s.cleanup_enabled = false;
        let view = menu_view_in(&s, "0.1.0", UiLang::Fr);
        assert_eq!(view.mode_title, "Mode : reformulation coupée");
        assert!(view.groups.iter().flatten().all(|m| !m.checked));
    }

    #[test]
    fn the_chosen_keys_are_named_in_the_hints() {
        let mut s = settings(CleanupEngine::OpenRouter);
        s.dictation_keys = vec!["left_control".into(), "f9".into()];
        s.translate_keys = vec!["left_shift".into()];
        let view = menu_view_in(&s, "0.1.0", UiLang::Fr);
        assert_eq!(view.dictation_hint, "Maintiens Ctrl gauche + F9 pour dicter");
        assert_eq!(view.translate_hint, "Maintiens Ctrl gauche + F9 + Maj gauche pour traduire en anglais");
    }

    #[test]
    fn the_menu_speaks_english_when_the_interface_does() {
        let mut s = settings(CleanupEngine::LlamaCpp);
        let view = menu_view_in(&s, "0.1.0", UiLang::En);
        assert!(view.english);
        assert_eq!(view.mode_title, "Mode: Simple");
        let (key, translate) = if cfg!(target_os = "macos") { ("Right Option", "⌘") } else { ("Right Ctrl", "Shift") };
        assert_eq!(view.dictation_hint, format!("Hold {key} to dictate"));
        assert_eq!(view.translate_hint, format!("Hold {key} + {translate} to translate into English"));
        assert_eq!((view.labels.open_page.as_str(), view.labels.quit.as_str()), ("Open Scriibe", "Quit Scriibe"));
        let labels: Vec<&str> = view.groups.iter().flatten().map(|m| m.label.as_str()).collect();
        assert_eq!(labels[..3], ["Literal", "Simple", "Formal"]);
        s.cleanup_engine = CleanupEngine::None;
        assert_eq!(menu_view_in(&s, "0.1.0", UiLang::En).mode_title, "Mode: no engine (literal transcription)");
        let french = menu_view_in(&settings(CleanupEngine::LlamaCpp), "0.1.0", UiLang::Fr);
        assert!(!french.english);
        assert_eq!(french.labels.quit, "Quitter Scriibe");
    }

    #[test]
    fn automatic_follows_the_system_language_and_defaults_to_english() {
        use scriibe_core::settings::UiLanguage;
        assert_eq!(UiLanguage::Auto.resolve("fr-FR"), UiLang::Fr);
        assert_eq!(UiLanguage::Auto.resolve("fr_FR.UTF-8"), UiLang::Fr);
        assert_eq!(UiLanguage::Auto.resolve("en-US"), UiLang::En);
        assert_eq!(UiLanguage::Auto.resolve("de-DE"), UiLang::En);
        assert_eq!(UiLanguage::Auto.resolve(""), UiLang::En);
        assert_eq!(UiLanguage::Fr.resolve("en-US"), UiLang::Fr);
        assert_eq!(UiLanguage::En.resolve("fr-FR"), UiLang::En);
    }
}
