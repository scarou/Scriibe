//! Wayland (GNOME 48 et suivants) : ce qu'une appli ordinaire peut obtenir passe par les **portails** du bureau (D-Bus), pas par le
//! serveur graphique.
//!
//! - **Touche globale** : portail `GlobalShortcuts`. Scriibe propose un raccourci, GNOME le fait accepter ou changer dans sa propre
//!   fenêtre (une seule fois : il s'en souvient) ; les signaux `Activated` et `Deactivated` disent appui et relâchement. GNOME ne permet
//!   pas une touche de modification seule (Ctrl droit) : c'est donc une combinaison, choisie dans la fenêtre de GNOME. L'appui et le
//!   relâchement sont rejoués comme ceux des touches de dictée en vigueur, pour que la machine à états du cœur ne change pas.
//! - **Collage** : le texte va dans le presse-papiers (`arboard`, par XWayland, que GNOME relaie aux applis Wayland), puis Ctrl+V est
//!   envoyé par le portail `RemoteDesktop` (`NotifyKeyboardKeysym` : la touche V de la disposition en vigueur). GNOME demande une fois
//!   l'autorisation de « contrôler l'ordinateur » ; le jeton qu'il rend est gardé (`portail-wayland.json`) pour ne plus la redemander.
//!   Sans cette autorisation : le texte reste dans le presse-papiers (« copie seule »).
//!
//! Tout ce qui parle aux portails vit dans un seul fil (une boucle `tokio` sur une connexion D-Bus), que les autres modules appellent par
//! des messages. L'appli s'y déclare d'abord (`register_host_app`, identifiant `APP_ID`) : sans cela, GNOME ne sait pas à qui rattacher
//! le raccourci et l'autorisation.

use crate::keys::{translate_key, KeyEvent, Subscription};
use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
use ashpd::desktop::remote_desktop::{DeviceType, KeyState, RemoteDesktop, SelectDevicesOptions};
use ashpd::desktop::{PersistMode, Session};
use futures_util::StreamExt;
use scriibe_core::hotkey::{Binding, Key};
use scriibe_core::settings::Hotkey;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};

/// L'identifiant de l'appli auprès du bureau (et le nom de son fichier `.desktop`).
pub const APP_ID: &str = "fr.scriibe.Scriibe";
/// Le raccourci de dictée proposé à GNOME (spécification « shortcuts » de XDG) ; la personne peut en choisir un autre dans sa fenêtre.
pub const PREFERRED_TRIGGER: &str = "CTRL+ALT+space";
const SHORTCUT_DICTATION: &str = "dictation";
const SHORTCUT_TRANSLATE: &str = "translate";
/// Proposé par défaut pour traduire : en face de Ctrl + ← (choisi pour dicter sur le PC d’essai) ; GNOME laisse en choisir un autre.
pub const PREFERRED_TRANSLATE_TRIGGER: &str = "CTRL+Right";
const KEYSYM_CONTROL_L: i32 = 0xFFE3;
const KEYSYM_V: i32 = 0x76;
/// La fenêtre d'autorisation de GNOME attend la personne : au-delà, le collage abandonne (le texte reste copié).
const PERMISSION_TIMEOUT: Duration = Duration::from_secs(60);

/// Vrai dans une session Wayland (sauf `SCRIIBE_FORCE_X11`, pour essayer la couche X11 sous XWayland).
pub fn is_wayland() -> bool {
    std::env::var("XDG_SESSION_TYPE").is_ok_and(|t| t.eq_ignore_ascii_case("wayland")) && std::env::var_os("SCRIIBE_FORCE_X11").is_none()
}

enum Command {
    Listen { sink: Sender<KeyEvent>, done: oneshot::Sender<Result<Triggers, String>> },
    StopListening,
    PasteKeys { done: oneshot::Sender<Result<(), String>> },
}

struct Portal {
    tx: mpsc::UnboundedSender<Command>,
}

/// Quand la fenêtre de Scriibe a le focus, GNOME envoie l'appui du raccourci mais jamais son relâchement (vu le 04/11 : dictée tenue
/// 5 min). La page, elle, voit le relâchement des touches : elle le signale ici (`release_hint`).
static RELEASE_HINT: std::sync::Mutex<Option<mpsc::UnboundedSender<()>>> = std::sync::Mutex::new(None);

/// Une touche relâchée dans la fenêtre de Scriibe : si un raccourci est tenu, il est relâché.
pub fn release_hint() {
    if let Some(tx) = RELEASE_HINT.lock().ok().and_then(|t| t.clone()) {
        let _ = tx.send(());
    }
}

static PORTAL: OnceLock<Result<Portal, String>> = OnceLock::new();

fn portal() -> Result<&'static Portal, String> {
    PORTAL.get_or_init(start_portal_thread).as_ref().map_err(Clone::clone)
}

fn start_portal_thread() -> Result<Portal, String> {
    let (tx, rx) = mpsc::unbounded_channel();
    std::thread::Builder::new()
        .name("scriibe-portails".into())
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("portails: boucle impossible ({e})");
                    return;
                }
            };
            let local = tokio::task::LocalSet::new();
            local.block_on(&runtime, run(rx));
        })
        .map_err(|e| format!("fil des portails : {e}"))?;
    Ok(Portal { tx })
}

/// Écrit `~/.local/share/applications/fr.scriibe.Scriibe.desktop` s'il manque : le bureau rattache l'identifiant de l'appli à ce fichier
/// (nom affiché dans ses fenêtres d'autorisation). Il sert aussi de lanceur dans la liste des applications.
fn ensure_desktop_file() {
    let Some(base) = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
    else {
        return;
    };
    // L'icône du dock et de la liste des applis, par son chemin complet (par son nom, la liste de GNOME ne la trouvait pas). GNOME relie la fenêtre (classe X11 « Scriibe », par XWayland) à ce fichier par `StartupWMClass`, et en prend `Icon`.
    let icon = base.join("icons/hicolor/scalable/apps").join(format!("{APP_ID}.svg"));
    if std::fs::read(&icon).ok().as_deref() != Some(ICON_SVG) {
        if let Some(dir) = icon.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(&icon, ICON_SVG);
    }
    let path = base.join("applications").join(format!("{APP_ID}.desktop"));
    let Ok(exe) = std::env::current_exe() else { return };
    let content = format!(
        "[Desktop Entry]\nType=Application\nName=Scriibe\nComment=Dictée vocale\nExec=\"{}\" lancer\nIcon={}\nStartupWMClass=Scriibe\nTerminal=false\nCategories=Utility;Accessibility;\n",
        exe.display(),
        icon.display()
    );
    // Réécrit seulement s'il manque ou a changé (programme déplacé, ancienne version sans icône).
    if std::fs::read_to_string(&path).is_ok_and(|old| old == content) {
        return;
    }
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(&path, content);
}

/// L'icône de Scriibe pour Linux (violette, `logo/`).
const ICON_SVG: &[u8] = include_bytes!("../../../../logo/scriibe-icone-linux.svg");

fn token_path() -> PathBuf {
    crate::storage::data_folder().join("portail-wayland.json")
}

fn load_restore_token() -> Option<String> {
    let text = std::fs::read_to_string(token_path()).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    value.get("remote_desktop").and_then(|v| v.as_str()).map(str::to_string)
}

fn save_restore_token(token: &str) {
    let value = serde_json::json!({ "remote_desktop": token });
    let _ = std::fs::write(token_path(), value.to_string());
}

/// La session de contrôle du clavier (portail `RemoteDesktop`), ouverte au premier collage et gardée.
struct Remote {
    proxy: RemoteDesktop,
    session: Session<RemoteDesktop>,
}

async fn open_remote(conn: &ashpd::zbus::Connection) -> Result<Remote, String> {
    let proxy = RemoteDesktop::with_connection(conn.clone()).await.map_err(|e| format!("portail RemoteDesktop absent ({e})"))?;
    let session = proxy.create_session(Default::default()).await.map_err(|e| format!("session RemoteDesktop refusée ({e})"))?;
    let token = load_restore_token();
    proxy
        .select_devices(
            &session,
            SelectDevicesOptions::default()
                .set_devices(ashpd::enumflags2::BitFlags::from(DeviceType::Keyboard))
                .set_persist_mode(PersistMode::ExplicitlyRevoked)
                .set_restore_token(token.as_deref()),
        )
        .await
        .map_err(|e| format!("clavier virtuel refusé ({e})"))?;
    let started = tokio::time::timeout(PERMISSION_TIMEOUT, async {
        proxy.start(&session, None, Default::default()).await.and_then(|request| request.response())
    })
    .await
    .map_err(|_| "autorisation de GNOME sans réponse".to_string())?
    .map_err(|e| format!("autorisation de contrôle refusée ({e})"))?;
    if let Some(token) = started.restore_token() {
        save_restore_token(token);
    }
    Ok(Remote { proxy, session })
}

async fn send_ctrl_v(remote: &Remote) -> Result<(), String> {
    for (keysym, state) in [
        (KEYSYM_CONTROL_L, KeyState::Pressed),
        (KEYSYM_V, KeyState::Pressed),
        (KEYSYM_V, KeyState::Released),
        (KEYSYM_CONTROL_L, KeyState::Released),
    ] {
        remote
            .proxy
            .notify_keyboard_keysym(&remote.session, keysym, state, Default::default())
            .await
            .map_err(|e| format!("Ctrl+V refusé par GNOME ({e})"))?;
    }
    Ok(())
}

/// Les touches de dictée en vigueur (rejouées à l'appui du raccourci) ; Ctrl droit si aucune n'est connue (capture en cours).
fn dictation_keys() -> Vec<Key> {
    let keys = crate::keys::current_hook_rules().binding.map(|b| b.dictation).unwrap_or_default();
    if keys.is_empty() {
        Binding::from_hotkey(Hotkey::RightControl).dictation
    } else {
        keys
    }
}

/// Traduire = dicter en tenant aussi une touche de traduction (Maj par défaut) : le raccourci de traduction rejoue les deux.
fn translate_keys() -> Vec<Key> {
    let binding = crate::keys::current_hook_rules().binding.unwrap_or_else(|| Binding::from_hotkey(Hotkey::RightControl));
    let mut keys = dictation_keys();
    if let Some(extra) = binding.translate.iter().find(|k| !keys.contains(k)) {
        keys.push(*extra);
    }
    keys
}

/// Les deux raccourcis acceptés par GNOME (descriptions de GNOME), dictée puis traduction.
type Triggers = (String, Option<String>);

/// Ouvre la session de raccourcis, fait accepter le raccourci à GNOME et relaie appui et relâchement jusqu'à `stop`.
async fn listen(
    conn: ashpd::zbus::Connection,
    sink: Sender<KeyEvent>,
    done: oneshot::Sender<Result<Triggers, String>>,
    mut stop: oneshot::Receiver<()>,
) {
    let setup = async {
        let proxy = GlobalShortcuts::with_connection(conn).await.map_err(|e| format!("portail GlobalShortcuts absent ({e})"))?;
        let session = proxy.create_session(Default::default()).await.map_err(|e| format!("session de raccourcis refusée ({e})"))?;
        let shortcuts = [
            NewShortcut::new(SHORTCUT_DICTATION, "Dicter (maintenir, parler, relâcher)").preferred_trigger(PREFERRED_TRIGGER),
            NewShortcut::new(SHORTCUT_TRANSLATE, "Dicter et traduire (maintenir, parler, relâcher)")
                .preferred_trigger(PREFERRED_TRANSLATE_TRIGGER),
        ];
        let bound = proxy
            .bind_shortcuts(&session, &shortcuts, None, Default::default())
            .await
            .and_then(|request| request.response())
            .map_err(|e| format!("raccourci refusé par GNOME ({e})"))?;
        let trigger = bound
            .shortcuts()
            .iter()
            .find(|s| s.id() == SHORTCUT_DICTATION)
            .map(|s| s.trigger_description().to_string())
            .ok_or("raccourci de dictée non retenu par GNOME")?;
        let translate = bound
            .shortcuts()
            .iter()
            .find(|s| s.id() == SHORTCUT_TRANSLATE)
            .map(|s| s.trigger_description().to_string())
            .filter(|t| !t.trim().is_empty());
        let trigger = (trigger, translate);
        let activated = proxy.receive_activated().await.map_err(|e| e.to_string())?;
        let deactivated = proxy.receive_deactivated().await.map_err(|e| e.to_string())?;
        Ok::<_, String>((proxy, session, trigger, activated, deactivated))
    };
    let (_proxy, session, trigger, activated, deactivated) = match setup.await {
        Ok(ready) => ready,
        Err(e) => {
            let _ = done.send(Err(e));
            return;
        }
    };
    let _ = done.send(Ok(trigger));
    let mut activated = std::pin::pin!(activated);
    let mut deactivated = std::pin::pin!(deactivated);
    let mut held: Vec<Key> = Vec::new();
    let mut pressed: Vec<Key> = Vec::new();
    let mut active = String::new();
    let (hint_tx, mut hints) = mpsc::unbounded_channel();
    if let Ok(mut slot) = RELEASE_HINT.lock() {
        *slot = Some(hint_tx);
    }
    loop {
        tokio::select! {
            _ = &mut stop => break,
            Some(()) = hints.recv() => {
                if !pressed.is_empty() {
                    eprintln!("portails: raccourci « {active} » relâché dans la fenêtre de Scriibe (GNOME ne l'a pas signalé)");
                    active.clear();
                    for key in pressed.drain(..).rev() {
                        if let Some(event) = translate_key(&mut held, key, false) {
                            let _ = sink.send(event);
                        }
                    }
                }
            }
            Some(signal) = activated.next() => {
                let id = signal.shortcut_id();
                if pressed.is_empty() && (id == SHORTCUT_DICTATION || id == SHORTCUT_TRANSLATE) {
                    active = id.to_string();
                    pressed = if id == SHORTCUT_TRANSLATE { translate_keys() } else { dictation_keys() };
                    eprintln!("portails: raccourci « {id} » appuyé");
                    for key in &pressed {
                        if let Some(event) = translate_key(&mut held, *key, true) {
                            let _ = sink.send(event);
                        }
                    }
                }
            }
            Some(signal) = deactivated.next() => {
                if signal.shortcut_id() == active {
                    eprintln!("portails: raccourci « {active} » relâché");
                    active.clear();
                    for key in pressed.drain(..).rev() {
                        if let Some(event) = translate_key(&mut held, key, false) {
                            let _ = sink.send(event);
                        }
                    }
                }
            }
            else => break,
        }
    }
    let _ = session.close().await;
}

async fn run(mut rx: mpsc::UnboundedReceiver<Command>) {
    ensure_desktop_file();
    let conn = match ashpd::zbus::Connection::session().await {
        Ok(c) => Some(c),
        Err(e) => {
            eprintln!("portails: bus de session introuvable ({e})");
            None
        }
    };
    if let Some(conn) = &conn {
        let registered = match ashpd::AppID::try_from(APP_ID) {
            Ok(id) => ashpd::register_host_app_with_connection(conn.clone(), id).await.map_err(|e| e.to_string()),
            Err(e) => Err(e.to_string()),
        };
        if let Err(e) = &registered {
            // Pas bloquant : un bureau sans ce registre (portail ancien) donne quand même les portails, sous un identifiant vide.
            eprintln!("portails: déclaration de l'appli refusée ({e})");
        }
    }
    let mut remote: Option<Remote> = None;
    let mut stop_listening: Option<oneshot::Sender<()>> = None;
    while let Some(command) = rx.recv().await {
        let Some(conn) = conn.clone() else {
            match command {
                Command::Listen { done, .. } => {
                    let _ = done.send(Err("bus de session introuvable".into()));
                }
                Command::PasteKeys { done } => {
                    let _ = done.send(Err("bus de session introuvable".into()));
                }
                Command::StopListening => {}
            }
            continue;
        };
        match command {
            Command::Listen { sink, done } => {
                if let Some(stop) = stop_listening.take() {
                    let _ = stop.send(());
                }
                let (stop_tx, stop_rx) = oneshot::channel();
                stop_listening = Some(stop_tx);
                tokio::task::spawn_local(listen(conn, sink, done, stop_rx));
            }
            Command::StopListening => {
                if let Some(stop) = stop_listening.take() {
                    let _ = stop.send(());
                }
            }
            Command::PasteKeys { done } => {
                // Une session gardée peut avoir été fermée par GNOME (veille, verrouillage, nuit : « Invalid session ») : on la rouvre
                // aussitôt, une fois (le jeton évite de redemander l'autorisation).
                let mut result = Err(String::new());
                for attempt in 0..2 {
                    if remote.is_none() {
                        match open_remote(&conn).await {
                            Ok(r) => remote = Some(r),
                            Err(e) => {
                                result = Err(e);
                                break;
                            }
                        }
                    }
                    result = match &remote {
                        Some(r) => send_ctrl_v(r).await,
                        None => Err("pas de session RemoteDesktop".into()),
                    };
                    if result.is_ok() {
                        break;
                    }
                    remote = None;
                    if attempt == 0 {
                        eprintln!("portails: session RemoteDesktop perdue, rouverte ({})", result.as_ref().err().map_or("", |e| e));
                    }
                }
                let _ = done.send(result);
            }
        }
    }
}

/// Touche globale par le portail `GlobalShortcuts`.
#[derive(Default)]
pub struct WaylandKeys;

struct Listening;

impl Subscription for Listening {}

impl Drop for Listening {
    fn drop(&mut self) {
        if let Ok(portal) = portal() {
            let _ = portal.tx.send(Command::StopListening);
        }
    }
}

/// Le dernier raccourci accepté par GNOME (« Ctrl+Alt+Espace »), pour le journal et la page.
static TRIGGER: std::sync::Mutex<Option<Triggers>> = std::sync::Mutex::new(None);

pub fn current_trigger() -> Option<String> {
    TRIGGER.lock().ok().and_then(|t| t.clone()).map(|(t, _)| pretty_trigger(&t))
}

/// Le raccourci de traduction accepté par GNOME (« Ctrl + Alt + T »), s'il y en a un.
pub fn current_translate_trigger() -> Option<String> {
    TRIGGER.lock().ok().and_then(|t| t.clone()).and_then(|(_, t)| t).map(|t| pretty_trigger(&t))
}

/// « Appuyer sur <Control>Left » (texte de GNOME, dans sa langue) → « Ctrl + ← », dans les noms de touches de Scriibe.
pub fn pretty_trigger(description: &str) -> String {
    let Some(start) = description.find('<').or_else(|| description.rfind(' ').map(|i| i + 1)) else {
        return description.to_string();
    };
    let mut rest = &description[start..];
    let mut parts: Vec<String> = Vec::new();
    while let Some(after) = rest.strip_prefix('<') {
        let Some(end) = after.find('>') else { break };
        parts.push(
            match &after[..end] {
                "Control" | "Primary" => "Ctrl",
                "Alt" => "Alt",
                "Shift" => "Maj",
                "Super" | "Meta" => "Super",
                other => other,
            }
            .to_string(),
        );
        rest = &after[end + 1..];
    }
    let key = match rest {
        "Left" => "←",
        "Right" => "→",
        "Up" => "↑",
        "Down" => "↓",
        "space" => "Espace",
        "Return" => "Entrée",
        other => other,
    };
    if !key.is_empty() {
        parts.push(if key.chars().count() == 1 { key.to_uppercase() } else { key.to_string() });
    }
    parts.join(" + ")
}

impl crate::keys::KeyListener for WaylandKeys {
    fn start(&self, sink: Sender<KeyEvent>) -> Result<Box<dyn Subscription>, String> {
        let portal = portal()?;
        let (done, wait) = oneshot::channel();
        portal.tx.send(Command::Listen { sink, done }).map_err(|_| "fil des portails arrêté".to_string())?;
        // La première fois, GNOME montre sa fenêtre et attend la personne.
        let trigger = wait.blocking_recv().map_err(|_| "fil des portails arrêté".to_string())??;
        eprintln!("portails: raccourci de dictée accepté par GNOME : {}", trigger.0);
        match &trigger.1 {
            Some(t) => eprintln!("portails: raccourci de traduction accepté par GNOME : {t}"),
            None => eprintln!("portails: pas de raccourci de traduction"),
        }
        if let Ok(mut slot) = TRIGGER.lock() {
            *slot = Some(trigger);
        }
        Ok(Box::new(Listening))
    }
}

/// Envoie Ctrl+V à l'appli active par le portail `RemoteDesktop` (ouvre la session, et demande l'autorisation, au premier appel).
pub fn send_ctrl_v_blocking() -> Result<(), String> {
    let portal = portal()?;
    let (done, wait) = oneshot::channel();
    portal.tx.send(Command::PasteKeys { done }).map_err(|_| "fil des portails arrêté".to_string())?;
    wait.blocking_recv().map_err(|_| "fil des portails arrêté".to_string())?
}

#[cfg(test)]
mod tests {
    use super::pretty_trigger;

    #[test]
    fn the_gnome_trigger_is_shown_with_scriibe_key_names() {
        assert_eq!(pretty_trigger("Appuyer sur <Control>Left"), "Ctrl + ←");
        assert_eq!(pretty_trigger("Press <Control><Alt>space"), "Ctrl + Alt + Espace");
        assert_eq!(pretty_trigger("<Shift><Super>d"), "Maj + Super + D");
        assert_eq!(pretty_trigger("F9"), "F9");
    }
}
