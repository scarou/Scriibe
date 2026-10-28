//! Icône de la zone de notification : une pastille dont la couleur dit l'état (repos, écoute, traitement, collé, message), une
//! infobulle et un menu. L'icône vit sur le fil principal (qui pompe les messages de Windows) ; l'indicateur envoyé aux autres fils
//! lui poste des ordres par un canal.

use crate::indicator::{Indicator, IndicatorState};
use crate::tray_menu::{MenuView, TrayMenuSource};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};
use windows_sys::Win32::UI::WindowsAndMessaging::{DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE};

enum Command {
    State(IndicatorState),
    Translation(Option<String>),
}

/// Ce que les autres fils utilisent pour montrer l'état.
pub struct TrayIndicator {
    tx: Mutex<Sender<Command>>,
    generation: Arc<AtomicU64>,
}

impl Indicator for TrayIndicator {
    fn show(&self, state: IndicatorState) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        let _ = self.tx.lock().map(|tx| tx.send(Command::State(state)));
    }

    fn set_translation(&self, language: Option<String>) {
        let _ = self.tx.lock().map(|tx| tx.send(Command::Translation(language)));
    }

    fn set_level(&self, _: f32) {}

    fn flash(&self, state: IndicatorState, seconds: f64) {
        self.show(state);
        let generation = Arc::clone(&self.generation);
        let seen = generation.load(Ordering::SeqCst);
        let tx = self.tx.lock().map(|tx| tx.clone());
        if let Ok(tx) = tx {
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_secs_f64(seconds.max(0.1)));
                // Une autre dictée a commencé entre-temps : on ne l'efface pas.
                if generation.load(Ordering::SeqCst) == seen {
                    let _ = tx.send(Command::State(IndicatorState::Hidden));
                }
            });
        }
    }
}

pub struct TrayHost {
    tray: TrayIcon,
    rx: Receiver<Command>,
    quit: MenuItem,
    open_data: MenuItem,
    open_log: MenuItem,
    open_page: MenuItem,
    dictation_hint: MenuItem,
    translate_hint: MenuItem,
    copy_last: MenuItem,
    mode_menu: Submenu,
    version_item: MenuItem,
    /// Les modes du sous-menu (identifiant du mode, case) ; refait quand la liste change.
    mode_items: Vec<(String, CheckMenuItem)>,
    mode_ids: HashMap<MenuId, String>,
    shown: Option<MenuView>,
    source: Arc<Mutex<Option<Arc<dyn TrayMenuSource>>>>,
    /// Adresse de la page d'essais, connue une fois son serveur démarré.
    page_url: Arc<Mutex<Option<String>>>,
    data_folder: PathBuf,
    log_file: PathBuf,
    idle_tip: String,
    translation: Option<String>,
}

/// Transparence de la plume du logo (32×32, un octet par point), tracée depuis `logo/scriibe-symbole.svg` par `tools/faire_icones.*`.
const FEATHER: &[u8; 32 * 32] = include_bytes!("plume32.alpha");

/// L'icône de la zone de notification : un disque de la couleur de l'état, avec la plume du logo en blanc.
fn circle(color: [u8; 3]) -> Icon {
    const SIZE: u32 = 32;
    let mut rgba = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let (dx, dy) = (x as f32 + 0.5 - 16.0, y as f32 + 0.5 - 16.0);
            let distance = (dx * dx + dy * dy).sqrt();
            // Bord adouci sur un pixel.
            let alpha = (14.5 - distance).clamp(0.0, 1.0);
            let feather = f32::from(FEATHER[(y * SIZE + x) as usize]) / 255.0;
            let mix = |c: u8| (f32::from(c) * (1.0 - feather) + 255.0 * feather) as u8;
            rgba.extend_from_slice(&[mix(color[0]), mix(color[1]), mix(color[2]), (alpha * 255.0) as u8]);
        }
    }
    Icon::from_rgba(rgba, SIZE, SIZE).expect("32×32 RGBA est un format valide")
}

fn color_of(state: &IndicatorState) -> [u8; 3] {
    match state {
        IndicatorState::Hidden => [120, 124, 134],
        IndicatorState::Listening => [224, 56, 56],
        IndicatorState::Working => [240, 160, 32],
        IndicatorState::Done => [64, 180, 96],
        IndicatorState::Message(_) => [64, 120, 224],
    }
}

impl TrayHost {
    /// À appeler sur le fil principal. `idle_tip` : l'infobulle au repos (rappelle la touche).
    pub fn create(
        idle_tip: &str,
        data_folder: PathBuf,
        log_file: PathBuf,
        page_url: Arc<Mutex<Option<String>>>,
        source: Arc<Mutex<Option<Arc<dyn TrayMenuSource>>>>,
    ) -> Result<(Self, TrayIndicator), String> {
        let menu = Menu::new();
        let open_page = MenuItem::new("Ouvrir Scriibe", true, None);
        let dictation_hint = MenuItem::new("", false, None);
        let translate_hint = MenuItem::new("", false, None);
        let copy_last = MenuItem::new("Copier la dernière dictée", true, None);
        let mode_menu = Submenu::new("Mode", true);
        let version_item = MenuItem::new("", false, None);
        let open_data = MenuItem::new("Ouvrir le dossier des données", true, None);
        let open_log = MenuItem::new("Ouvrir le journal", true, None);
        let quit = MenuItem::new("Quitter Scriibe", true, None);
        let separator = || PredefinedMenuItem::separator();
        let error = |e| format!("menu de l'icône : {e}");
        menu.append(&open_page).map_err(error)?;
        menu.append(&separator()).map_err(error)?;
        menu.append(&dictation_hint).map_err(error)?;
        menu.append(&translate_hint).map_err(error)?;
        menu.append(&copy_last).map_err(error)?;
        menu.append(&separator()).map_err(error)?;
        menu.append(&mode_menu).map_err(error)?;
        menu.append(&separator()).map_err(error)?;
        menu.append(&version_item).map_err(error)?;
        menu.append(&open_data).map_err(error)?;
        menu.append(&open_log).map_err(error)?;
        menu.append(&separator()).map_err(error)?;
        menu.append(&quit).map_err(error)?;
        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip(idle_tip)
            .with_icon(circle(color_of(&IndicatorState::Hidden)))
            .build()
            .map_err(|e| format!("icône de la zone de notification : {e}"))?;
        let (tx, rx) = mpsc::channel();
        let indicator = TrayIndicator { tx: Mutex::new(tx), generation: Arc::new(AtomicU64::new(0)) };
        let host = Self {
            tray,
            rx,
            quit,
            open_data,
            open_log,
            open_page,
            dictation_hint,
            translate_hint,
            copy_last,
            mode_menu,
            version_item,
            mode_items: Vec::new(),
            mode_ids: HashMap::new(),
            shown: None,
            source,
            page_url,
            data_folder,
            log_file,
            idle_tip: idle_tip.to_string(),
            translation: None,
        };
        Ok((host, indicator))
    }

    fn apply(&mut self, command: Command) {
        match command {
            Command::Translation(language) => self.translation = language,
            Command::State(state) => {
                let tip = match &state {
                    IndicatorState::Hidden => self.idle_tip.clone(),
                    IndicatorState::Listening => match &self.translation {
                        Some(language) => format!("Scriibe — à l'écoute (traduction : {language})"),
                        None => "Scriibe — à l'écoute…".to_string(),
                    },
                    other => format!("Scriibe — {}", crate::indicator::describe(other)),
                };
                if state == IndicatorState::Hidden || state == IndicatorState::Done {
                    self.translation = None;
                }
                let _ = self.tray.set_icon(Some(circle(color_of(&state))));
                // L'infobulle de Windows est limitée à 127 caractères.
                let _ = self.tray.set_tooltip(Some(tip.chars().take(120).collect::<String>()));
            }
        }
    }

    /// Remet le menu d'accord avec les réglages (texte des touches, mode coché, liste des modes) : seulement ce qui a changé.
    fn refresh_menu(&mut self) {
        let Some(source) = self.source.lock().ok().and_then(|s| s.clone()) else { return };
        let view = source.view();
        if self.shown.as_ref() == Some(&view) {
            return;
        }
        let before = self.shown.as_ref();
        let same_modes = before.is_some_and(|b| {
            b.groups.iter().flatten().map(|m| (&m.id, &m.label)).eq(view.groups.iter().flatten().map(|m| (&m.id, &m.label)))
        });
        self.dictation_hint.set_text(&view.dictation_hint);
        self.translate_hint.set_text(&view.translate_hint);
        self.version_item.set_text(&view.version);
        self.mode_menu.set_text(&view.mode_title);
        self.mode_menu.set_enabled(view.modes_enabled);
        if !same_modes {
            while self.mode_menu.remove_at(0).is_some() {}
            self.mode_items.clear();
            self.mode_ids.clear();
            for (index, group) in view.groups.iter().enumerate() {
                if index > 0 {
                    let _ = self.mode_menu.append(&PredefinedMenuItem::separator());
                }
                for mode in group {
                    let item = CheckMenuItem::new(&mode.label, view.modes_enabled, mode.checked, None);
                    let _ = self.mode_menu.append(&item);
                    self.mode_ids.insert(item.id().clone(), mode.id.clone());
                    self.mode_items.push((mode.id.clone(), item));
                }
            }
        } else {
            for (group, shown) in view.groups.iter().flatten().zip(&self.mode_items) {
                shown.1.set_checked(group.checked);
                shown.1.set_enabled(view.modes_enabled);
            }
        }
        self.shown = Some(view);
    }

    /// Boucle du fil principal : messages de Windows, ordres de l'indicateur, menu. Rend la main quand on choisit « Quitter ».
    pub fn run(mut self) {
        let mut last_refresh = std::time::Instant::now() - Duration::from_secs(10);
        loop {
            if last_refresh.elapsed() >= Duration::from_millis(500) {
                last_refresh = std::time::Instant::now();
                self.refresh_menu();
            }
            // SAFETY : pompe les messages de ce fil (obligatoire pour que l'icône et son menu répondent).
            unsafe {
                let mut message: MSG = std::mem::zeroed();
                while PeekMessageW(&mut message, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
            while let Ok(command) = self.rx.try_recv() {
                self.apply(command);
            }
            while let Ok(event) = MenuEvent::receiver().try_recv() {
                if event.id == *self.quit.id() {
                    return;
                }
                if event.id == *self.open_page.id() {
                    let url = self.page_url.lock().ok().and_then(|u| u.clone());
                    if let Some(url) = url {
                        crate::open::open(&url);
                    }
                } else if event.id == *self.copy_last.id() {
                    if let Some(source) = self.source.lock().ok().and_then(|s| s.clone()) {
                        source.copy_last_dictation();
                    }
                } else if let Some(mode) = self.mode_ids.get(&event.id).cloned() {
                    if let Some(source) = self.source.lock().ok().and_then(|s| s.clone()) {
                        source.choose_mode(&mode);
                    }
                    // Le clic a coché la case : le menu se remet tout de suite d'accord avec les réglages.
                    self.shown = None;
                    last_refresh = std::time::Instant::now() - Duration::from_secs(10);
                } else if event.id == *self.open_data.id() {
                    crate::open::open(&self.data_folder.display().to_string());
                } else if event.id == *self.open_log.id() {
                    crate::open::open(&self.log_file.display().to_string());
                }
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}
