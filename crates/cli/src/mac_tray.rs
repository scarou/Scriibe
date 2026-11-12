//! macOS et Linux (AppIndicator, barre du haut de GNOME) : l'icône de la barre des menus et son menu, par l'API d'icône de Tauri (NSStatusItem exige le fil principal, que Tauri garde).
//! Même contenu que sous Windows (`scriibe_platform::windows::tray`) : un disque dont la couleur dit l'état, une infobulle, le menu décrit
//! par `tray_source` (touches, mode en un clic, copier la dernière dictée, ouvrir la fenêtre, quitter).
//!
//! Les éléments du menu de Tauri se modifient depuis n'importe quel fil : un fil relit `TrayMenuSource::view` deux fois par seconde et ne
//! change que ce qui a changé. Les clics arrivent au gestionnaire d'application (`on_menu_event`), sur le fil principal : ils lancent leur
//! travail dans un fil à part.

use scriibe_platform::indicator::{Indicator, IndicatorState};
use scriibe_platform::tray_menu::{self, MenuView, TrayMenuSource};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::{AppHandle, Wry};

const MODE_PREFIX: &str = "tray:mode:";
const OPEN_PAGE: &str = "tray:open";
const UPDATE: &str = "tray:update";
const COPY_LAST: &str = "tray:copy";
const OPEN_DATA: &str = "tray:data";
const OPEN_LOG: &str = "tray:log";
const QUIT: &str = "tray:quit";
const APP_QUIT: &str = "app:quit";

/// L'état partagé entre l'indicateur (appelé par n'importe quel fil) et l'icône (créée plus tard, par Tauri, sur le fil principal).
pub struct TrayState {
    icon: Mutex<Option<TrayIcon<Wry>>>,
    generation: AtomicU64,
    translation: Mutex<Option<String>>,
    english: AtomicBool,
    idle_tip: Mutex<String>,
}

impl TrayState {
    fn apply(&self, state: &IndicatorState) {
        let translation = self.translation.lock().ok().and_then(|t| t.clone());
        let idle = self.idle_tip.lock().map(|t| t.clone()).unwrap_or_default();
        let tip = tray_menu::state_tooltip(state, &idle, translation.as_deref(), self.english.load(Ordering::SeqCst));
        if matches!(state, IndicatorState::Hidden | IndicatorState::Done) {
            if let Ok(mut t) = self.translation.lock() {
                *t = None;
            }
        }
        let image = Image::new_owned(tray_menu::status_icon_rgba(state), tray_menu::ICON_SIZE, tray_menu::ICON_SIZE);
        if let Some(icon) = self.icon.lock().ok().and_then(|i| i.clone()) {
            let _ = icon.set_icon(Some(image));
            let _ = icon.set_tooltip(Some(tip));
        }
    }
}

/// Ce que les autres fils utilisent pour montrer l'état.
pub struct TrayIndicator {
    state: Arc<TrayState>,
}

/// L'indicateur et l'état qu'il partage avec l'icône. `idle_tip` : l'infobulle au repos (rappelle la touche).
pub fn indicator(idle_tip: &str) -> (Arc<TrayState>, TrayIndicator) {
    let state = Arc::new(TrayState {
        icon: Mutex::new(None),
        generation: AtomicU64::new(0),
        translation: Mutex::new(None),
        english: AtomicBool::new(false),
        idle_tip: Mutex::new(idle_tip.to_string()),
    });
    (state.clone(), TrayIndicator { state })
}

impl Indicator for TrayIndicator {
    fn show(&self, state: IndicatorState) {
        self.state.generation.fetch_add(1, Ordering::SeqCst);
        self.state.apply(&state);
    }

    fn set_translation(&self, language: Option<String>) {
        if let Ok(mut t) = self.state.translation.lock() {
            *t = language;
        }
    }

    fn set_level(&self, _: f32) {}

    fn flash(&self, state: IndicatorState, seconds: f64) {
        self.show(state);
        let shared = self.state.clone();
        let seen = shared.generation.load(Ordering::SeqCst);
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs_f64(seconds.max(0.1)));
            // Une autre dictée a commencé entre-temps : on ne l'efface pas.
            if shared.generation.load(Ordering::SeqCst) == seen {
                shared.apply(&IndicatorState::Hidden);
            }
        });
    }
}

/// Ce que le menu montre et fait, connu en partie seulement une fois l'appli construite (d'où les cases à remplir).
pub struct Shell {
    pub state: Arc<TrayState>,
    pub source: Arc<Mutex<Option<Arc<dyn TrayMenuSource>>>>,
    /// Adresse de l'interface, connue une fois son serveur démarré.
    pub page_url: Arc<Mutex<Option<String>>>,
    pub data_folder: PathBuf,
    pub log_file: PathBuf,
    /// Arrête les moteurs puis le programme.
    pub quit: Arc<dyn Fn() + Send + Sync>,
}

static SHELL: OnceLock<Shell> = OnceLock::new();

#[cfg(target_os = "macos")]
/// Le menu de l'appli (invisible, l'appli n'a pas de Dock) : il sert aux raccourcis ⌘C, ⌘V, ⌘A, ⌘Z, ⌘W et ⌘Q dans la fenêtre.
pub fn app_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let quit = MenuItem::with_id(app, APP_QUIT, "Quitter Scriibe", true, Some("CmdOrCtrl+Q"))?;
    let scriibe =
        Submenu::with_items(app, "Scriibe", true, &[&PredefinedMenuItem::hide(app, None)?, &PredefinedMenuItem::separator(app)?, &quit])?;
    let edit = Submenu::with_items(
        app,
        "Édition",
        true,
        &[
            &PredefinedMenuItem::undo(app, None)?,
            &PredefinedMenuItem::redo(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
        ],
    )?;
    let window = Submenu::with_items(
        app,
        "Fenêtre",
        true,
        &[&PredefinedMenuItem::minimize(app, None)?, &PredefinedMenuItem::close_window(app, None)?],
    )?;
    Menu::with_items(app, &[&scriibe, &edit, &window])
}

/// Un clic sur le menu de l'icône ou sur celui de l'appli.
pub fn on_menu_event(id: &str) {
    let Some(shell) = SHELL.get() else { return };
    let source = || shell.source.lock().ok().and_then(|s| s.clone());
    match id {
        QUIT | APP_QUIT => (shell.quit)(),
        OPEN_PAGE | UPDATE => {
            if let Some(url) = shell.page_url.lock().ok().and_then(|u| u.clone()) {
                std::thread::spawn(move || scriibe_platform::open::open(&url));
            }
        }
        COPY_LAST => {
            if let Some(source) = source() {
                std::thread::spawn(move || source.copy_last_dictation());
            }
        }
        OPEN_DATA => scriibe_platform::open::open(&shell.data_folder.display().to_string()),
        OPEN_LOG => scriibe_platform::open::open(&shell.log_file.display().to_string()),
        other => {
            if let (Some(mode), Some(source)) = (other.strip_prefix(MODE_PREFIX), source()) {
                let mode = mode.to_string();
                std::thread::spawn(move || {
                    source.choose_mode(&mode);
                    MODE_CLICKED.store(true, Ordering::SeqCst);
                });
            }
        }
    }
}

/// Faut-il remettre le menu d'accord avec la vue ? Quand elle a changé depuis la dernière fois.
/// `forced` : un clic sur un mode vient d'avoir lieu. Le menu natif a déjà coché ou décoché la case tout seul (muda) : même si les
/// réglages n'ont pas changé (mode déjà coché, mode refusé), il faut la remettre d'accord avec eux. Windows fait de même.
fn needs_refresh(shown: Option<&MenuView>, view: &MenuView, forced: bool) -> bool {
    forced || shown != Some(view)
}

/// Levé après chaque clic sur un mode ; le fil du menu le lit et le baisse.
static MODE_CLICKED: AtomicBool = AtomicBool::new(false);

struct Items {
    menu: Menu<Wry>,
    /// « Nouvelle version disponible » : en tête du menu seulement quand une version plus récente existe.
    update: MenuItem<Wry>,
    update_shown: bool,
    open_page: MenuItem<Wry>,
    dictation_hint: MenuItem<Wry>,
    translate_hint: MenuItem<Wry>,
    copy_last: MenuItem<Wry>,
    mode_menu: Submenu<Wry>,
    version: MenuItem<Wry>,
    open_data: MenuItem<Wry>,
    open_log: MenuItem<Wry>,
    quit: MenuItem<Wry>,
    /// Les modes du sous-menu (identifiant, case) ; refait quand la liste change.
    modes: Vec<(String, CheckMenuItem<Wry>)>,
}

impl Items {
    /// Remet le menu d'accord avec les réglages : seulement ce qui a changé.
    fn refresh(&mut self, app: &AppHandle, view: &MenuView, before: Option<&MenuView>) -> tauri::Result<()> {
        let same_modes = before.is_some_and(|b| {
            b.groups.iter().flatten().map(|m| (&m.id, &m.label)).eq(view.groups.iter().flatten().map(|m| (&m.id, &m.label)))
        });
        self.open_page.set_text(&view.labels.open_page)?;
        self.copy_last.set_text(&view.labels.copy_last)?;
        self.open_data.set_text(&view.labels.open_data)?;
        self.open_log.set_text(&view.labels.open_log)?;
        self.quit.set_text(&view.labels.quit)?;
        self.dictation_hint.set_text(&view.dictation_hint)?;
        self.translate_hint.set_text(&view.translate_hint)?;
        self.version.set_text(&view.version)?;
        match (&view.update, self.update_shown) {
            (Some(text), shown) => {
                self.update.set_text(text)?;
                if !shown {
                    self.menu.insert(&self.update, 0)?;
                    self.update_shown = true;
                }
            }
            (None, true) => {
                self.menu.remove(&self.update)?;
                self.update_shown = false;
            }
            (None, false) => {}
        }
        self.mode_menu.set_text(&view.mode_title)?;
        self.mode_menu.set_enabled(view.modes_enabled)?;
        if same_modes {
            for (group, shown) in view.groups.iter().flatten().zip(&self.modes) {
                shown.1.set_checked(group.checked)?;
                shown.1.set_enabled(view.modes_enabled)?;
            }
            return Ok(());
        }
        while let Ok(Some(_)) = self.mode_menu.remove_at(0) {}
        self.modes.clear();
        for (index, group) in view.groups.iter().enumerate() {
            if index > 0 {
                self.mode_menu.append(&PredefinedMenuItem::separator(app)?)?;
            }
            for mode in group {
                let item = CheckMenuItem::with_id(
                    app,
                    format!("{MODE_PREFIX}{}", mode.id),
                    &mode.label,
                    view.modes_enabled,
                    mode.checked,
                    None::<&str>,
                )?;
                self.mode_menu.append(&item)?;
                self.modes.push((mode.id.clone(), item));
            }
        }
        Ok(())
    }
}

/// Crée l'icône et son menu (sur le fil principal, depuis le `setup` de Tauri), puis lance le fil qui garde le menu à jour.
pub fn install(app: &AppHandle, shell: Shell) -> Result<(), String> {
    let error = |e: tauri::Error| format!("icône de la barre des menus : {e}");
    let item = |id: &str, text: &str, enabled: bool| MenuItem::with_id(app, id, text, enabled, None::<&str>);
    let update = item(UPDATE, "", true).map_err(error)?;
    let open_page = item(OPEN_PAGE, "Ouvrir Scriibe", true).map_err(error)?;
    let dictation_hint = item("tray:hint1", "", false).map_err(error)?;
    let translate_hint = item("tray:hint2", "", false).map_err(error)?;
    let copy_last = item(COPY_LAST, "Copier la dernière dictée", true).map_err(error)?;
    let mode_menu = Submenu::with_id(app, "tray:modes", "Mode", true).map_err(error)?;
    let version = item("tray:version", "", false).map_err(error)?;
    let open_data = item(OPEN_DATA, "Ouvrir le dossier des données", true).map_err(error)?;
    let open_log = item(OPEN_LOG, "Ouvrir le journal", true).map_err(error)?;
    let quit = item(QUIT, "Quitter Scriibe", true).map_err(error)?;
    let separator = || PredefinedMenuItem::separator(app);
    let menu = Menu::with_items(
        app,
        &[
            &open_page,
            &separator().map_err(error)?,
            &dictation_hint,
            &translate_hint,
            &copy_last,
            &separator().map_err(error)?,
            &mode_menu,
            &separator().map_err(error)?,
            &version,
            &open_data,
            &open_log,
            &separator().map_err(error)?,
            &quit,
        ],
    )
    .map_err(error)?;
    let idle_tip = shell.state.idle_tip.lock().map(|t| t.clone()).unwrap_or_default();
    let tray = TrayIconBuilder::with_id("scriibe")
        .menu(&menu)
        .tooltip(idle_tip)
        .icon(Image::new_owned(tray_menu::status_icon_rgba(&IndicatorState::Hidden), tray_menu::ICON_SIZE, tray_menu::ICON_SIZE))
        .build(app)
        .map_err(error)?;
    if let Ok(mut slot) = shell.state.icon.lock() {
        *slot = Some(tray);
    }
    let (state, source) = (shell.state.clone(), shell.source.clone());
    let _ = SHELL.set(shell);

    let app = app.clone();
    let mut items = Items {
        menu,
        update,
        update_shown: false,
        open_page,
        dictation_hint,
        translate_hint,
        copy_last,
        mode_menu,
        version,
        open_data,
        open_log,
        quit,
        modes: Vec::new(),
    };
    std::thread::Builder::new()
        .name("scriibe-menu".into())
        .spawn(move || {
            let mut shown: Option<MenuView> = None;
            loop {
                let forced = MODE_CLICKED.swap(false, Ordering::SeqCst);
                if let Some(view) = source.lock().ok().and_then(|s| s.clone()).map(|s| s.view()) {
                    if needs_refresh(shown.as_ref(), &view, forced) {
                        state.english.store(view.english, Ordering::SeqCst);
                        if items.refresh(&app, &view, shown.as_ref()).is_ok() {
                            shown = Some(view);
                        }
                    }
                }
                std::thread::sleep(Duration::from_millis(500));
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_click_on_a_mode_refreshes_the_menu_even_when_the_settings_did_not_change() {
        // Le menu natif a coché ou décoché la case tout seul (muda) : il faut la remettre d'accord avec les réglages.
        let view = MenuView::default();
        assert!(!needs_refresh(Some(&view), &view, false), "rien n'a changé, rien à faire");
        assert!(needs_refresh(Some(&view), &view, true), "un clic sur un mode force la remise d'accord");
        assert!(needs_refresh(None, &view, false), "premier passage");
    }
}
