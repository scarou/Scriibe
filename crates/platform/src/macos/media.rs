//! Pause des médias : quels processus sortent du son ou captent le micro (CoreAudio, liste des processus : macOS 14.2 et après), et la
//! touche Lecture/Pause du clavier (événement système `NX_KEYTYPE_PLAY`, comme dans `MediaPause.swift` de Scribe). La mécanique
//! (délais, vérification, relance) est dans `crate::media::MediaPauser`.

use super::ffi::{self, Ref};
use crate::media::AudioSource;
use objc2::encode::{Encode, Encoding};
use objc2::runtime::AnyObject;
use objc2::{class, msg_send};
use scriibe_core::media_pause::AudioProcess;
use std::collections::HashMap;
use std::ffi::{c_char, c_void};
use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Une valeur de CoreAudio écrite en quatre caractères (`'prs#'`).
const fn code(text: &[u8; 4]) -> u32 {
    u32::from_be_bytes(*text)
}

const SYSTEM_OBJECT: u32 = 1;
const SCOPE_GLOBAL: u32 = code(b"glob");
const ELEMENT_MAIN: u32 = 0;
const PROCESS_OBJECT_LIST: u32 = code(b"prs#");
const PROCESS_PID: u32 = code(b"ppid");
const PROCESS_BUNDLE_ID: u32 = code(b"pbid");
const PROCESS_RUNNING_INPUT: u32 = code(b"piri");
const PROCESS_RUNNING_OUTPUT: u32 = code(b"piro");
const UTF8: u32 = 0x0800_0100;

#[repr(C)]
struct PropertyAddress {
    selector: u32,
    scope: u32,
    element: u32,
}

#[link(name = "CoreAudio", kind = "framework")]
extern "C" {
    fn AudioObjectGetPropertyDataSize(
        object: u32,
        address: *const PropertyAddress,
        qualifier_size: u32,
        qualifier: *const c_void,
        size: *mut u32,
    ) -> i32;
    fn AudioObjectGetPropertyData(
        object: u32,
        address: *const PropertyAddress,
        qualifier_size: u32,
        qualifier: *const c_void,
        size: *mut u32,
        data: *mut c_void,
    ) -> i32;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFStringGetCString(string: Ref, buffer: *mut c_char, size: isize, encoding: u32) -> bool;
}

// Charge AppKit : la classe `NSEvent` doit exister pour fabriquer l'événement de la touche Lecture/Pause.
#[link(name = "AppKit", kind = "framework")]
extern "C" {}

extern "C" {
    /// Le chemin de l'exécutable d'un processus (libproc, dans la bibliothèque système) ; 0 si le processus n'existe plus.
    fn proc_pidpath(pid: i32, buffer: *mut c_void, size: u32) -> i32;
}

/// Le son de début et de fin de Scriibe est joué par `afplay` : il sort du son sans être un média à interrompre (sinon la touche
/// Lecture/Pause relancerait une vidéo déjà arrêtée).
fn is_own_sound(pid: i64) -> bool {
    let Ok(pid) = i32::try_from(pid) else { return false };
    let mut buffer = [0u8; 1024];
    // SAFETY : le tampon a la taille annoncée ; `proc_pidpath` rend le nombre d'octets écrits.
    let written = unsafe { proc_pidpath(pid, buffer.as_mut_ptr().cast(), buffer.len() as u32) };
    usize::try_from(written).ok().filter(|n| *n > 0).is_some_and(|n| buffer[..n].ends_with(b"/afplay"))
}

/// Les lecteurs qui disent eux-mêmes s'ils jouent : (identifiant de bundle, script AppleScript qui rend `true` ou `false`). VLC (et d'autres)
/// gardent leur flux audio ouvert en pause : CoreAudio les voit « jouer » à tort, et la touche Lecture/Pause relancerait un lecteur arrêté.
const SELF_REPORTING_PLAYERS: &[(&str, &str)] = &[
    ("org.videolan.vlc", "tell application id \"org.videolan.vlc\" to return playing"),
    ("com.apple.Music", "tell application id \"com.apple.Music\" to return (player state is playing)"),
    ("com.spotify.client", "tell application id \"com.spotify.client\" to return (player state is playing)"),
];

/// Les navigateurs fondés sur Chromium (identifiant de l'appli). Leurs processus audio ont un identifiant en `….helper…`. Un navigateur
/// garde son flux audio ouvert une dizaine de secondes après la pause d'une vidéo (mesuré avec Chrome, S15) : CoreAudio les voit « jouer »
/// à tort, et la touche Lecture/Pause relancerait la vidéo. On interroge donc les pages (JavaScript par Apple Events : Chrome, menu
/// Présentation → Développeur → « Autoriser JavaScript provenant d'Apple Events »).
const BROWSERS: &[&str] =
    &["com.google.Chrome", "com.brave.Browser", "com.microsoft.edgemac", "com.vivaldi.Vivaldi", "org.chromium.Chromium"];

/// L'appli navigateur dont ce processus audio est un assistant (ou l'appli elle-même).
fn browser_app(bundle: &str) -> Option<&'static str> {
    BROWSERS.iter().copied().find(|app| bundle.strip_prefix(app).is_some_and(|rest| rest.is_empty() || rest.starts_with(".helper")))
}

/// Le script qui dit si une page d'une fenêtre du navigateur lit un son ou une vidéo. Sans réponse (JavaScript refusé : aucune page ne répond)
/// il échoue, et `ask_player` rend `None`.
fn browser_script(app: &str) -> String {
    const JS: &str =
        "Array.from(document.querySelectorAll('video,audio')).some(function(m){return !m.paused && !m.ended && m.readyState > 2}) ? 'true' : 'false'";
    format!(
        "tell application id \"{app}\"\n\
         set answered to 0\n\
         repeat with w in windows\n\
         repeat with t in tabs of w\n\
         try\n\
         set r to (execute t javascript \"{JS}\") as text\n\
         set answered to answered + 1\n\
         if r is \"true\" then return \"true\"\n\
         end try\n\
         end repeat\n\
         end repeat\n\
         if answered is 0 then error \"pas de réponse\"\n\
         return \"false\"\n\
         end tell"
    )
}

/// Comment interroger le lecteur de ce processus : (clé de la mémoire, script, durée maximale). `None` : pas un lecteur connu.
fn player_query(bundle: &str) -> Option<(String, String, Duration)> {
    if let Some((id, script)) = SELF_REPORTING_PLAYERS.iter().find(|(id, _)| *id == bundle) {
        return Some(((*id).to_string(), (*script).to_string(), ASK_TIMEOUT));
    }
    let app = browser_app(bundle)?;
    Some((app.to_string(), browser_script(app), BROWSER_ASK_TIMEOUT))
}

/// Les réponses récentes : l'état lu (`None` : pas de réponse, voir `ask_player`) et quand. Une vérification de pause relit la liste
/// toutes les 50 ms : sans cette mémoire, chaque lecture lancerait `osascript`.
type PlayerStates = HashMap<String, (Instant, Option<bool>)>;
static PLAYER_STATES: Mutex<Option<PlayerStates>> = Mutex::new(None);
const STATE_FRESH: Duration = Duration::from_millis(200);
/// Sans réponse (autorisation d'automatisation non accordée : macOS attend le clic de la personne), on ne redemande pas avant ce délai.
const NO_ANSWER_FRESH: Duration = Duration::from_secs(30);
const ASK_TIMEOUT: Duration = Duration::from_millis(1500);
/// Un navigateur parcourt toutes ses pages : on le questionne moins souvent, et plus longtemps.
const BROWSER_STATE_FRESH: Duration = Duration::from_millis(1000);
const BROWSER_ASK_TIMEOUT: Duration = Duration::from_millis(4000);

/// Lance le script et attend sa réponse au plus `timeout` (un `osascript` bloqué par une demande d'autorisation est tué).
fn ask_player(script: &str, timeout: Duration) -> Option<bool> {
    match run_script(script, timeout)?.as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

/// La sortie du script (sans espaces autour) ; `None` s'il échoue ou dépasse `timeout`.
fn run_script(script: &str, timeout: Duration) -> Option<String> {
    let mut child =
        Command::new("osascript").args(["-e", script]).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().ok()?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => break,
            Ok(Some(_)) | Err(_) => return None,
            Ok(None) if started.elapsed() > timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
        }
    }
    let mut out = String::new();
    child.stdout.take()?.read_to_string(&mut out).ok()?;
    Some(out.trim().to_string())
}

/// Le script JavaScript (une expression rendant un nombre en texte) exécuté dans toutes les pages du navigateur `app` ; rend la somme des
/// nombres. `None` si aucune page ne répond (JavaScript refusé, autorisation d'automatisation non accordée).
fn browser_sum(app: &str, js: &str) -> Option<u32> {
    let script = format!(
        "tell application id \"{app}\"\n\
         set total to 0\n\
         set answered to 0\n\
         repeat with w in windows\n\
         repeat with t in tabs of w\n\
         try\n\
         set r to (execute t javascript \"{js}\") as text\n\
         set answered to answered + 1\n\
         set total to total + (r as integer)\n\
         end try\n\
         end repeat\n\
         end repeat\n\
         if answered is 0 then error \"pas de réponse\"\n\
         return total as text\n\
         end tell"
    );
    run_script(&script, BROWSER_ASK_TIMEOUT)?.parse().ok()
}

/// Met en pause ce qui joue dans les pages du navigateur (chaque média touché est marqué pour la relance) ; rend le nombre de médias arrêtés.
fn browser_pause(app: &str) -> Option<u32> {
    const JS: &str = "var n=0;document.querySelectorAll('video,audio').forEach(function(m){if(!m.paused && !m.ended && m.readyState>2){m.__scriibe=true;m.pause();n++}});String(n)";
    let n = browser_sum(app, JS)?;
    // On sait ce que les pages jouent maintenant : pas besoin de les interroger de nouveau tout de suite.
    remember_state(app, Some(false));
    Some(n)
}

/// Relance les médias que `browser_pause` a arrêtés ; rend leur nombre.
fn browser_resume(app: &str) -> Option<u32> {
    const JS: &str = "var n=0;document.querySelectorAll('video,audio').forEach(function(m){if(m.__scriibe){m.__scriibe=false;m.play().catch(function(){});n++}});String(n)";
    let n = browser_sum(app, JS)?;
    remember_state(app, None);
    Some(n)
}

/// Retient ce que dit le lecteur `app` maintenant (`None` : oublie, la prochaine question sera posée).
fn remember_state(app: &str, state: Option<bool>) {
    let mut slot = PLAYER_STATES.lock().unwrap_or_else(|e| e.into_inner());
    let states = slot.get_or_insert_with(HashMap::new);
    match state {
        Some(_) => {
            states.insert(app.to_string(), (Instant::now(), state));
        }
        None => {
            states.remove(app);
        }
    }
}

/// Ce que dit le lecteur de `bundle` ; `None` si ce n'est pas un lecteur connu ou s'il ne répond pas.
fn player_says_playing(bundle: &str) -> Option<bool> {
    let (name, script, timeout) = player_query(bundle)?;
    let mut slot = PLAYER_STATES.lock().unwrap_or_else(|e| e.into_inner());
    let states = slot.get_or_insert_with(HashMap::new);
    if let Some((at, state)) = states.get(&name) {
        let fresh = match (state.is_some(), browser_app(bundle).is_some()) {
            (false, _) => NO_ANSWER_FRESH,
            (true, true) => BROWSER_STATE_FRESH,
            (true, false) => STATE_FRESH,
        };
        if at.elapsed() < fresh {
            return *state;
        }
    }
    drop(slot);
    let state = ask_player(&script, timeout);
    PLAYER_STATES.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(HashMap::new).insert(name, (Instant::now(), state));
    state
}

/// Un navigateur parmi ces processus n'a pas répondu à la dernière question (JavaScript par Apple Events refusé, ou autorisation
/// d'automatisation non accordée) : la touche Lecture/Pause peut alors relancer une vidéo déjà arrêtée.
fn browser_unanswered(players: &[AudioProcess]) -> bool {
    let slot = PLAYER_STATES.lock().unwrap_or_else(|e| e.into_inner());
    let Some(states) = slot.as_ref() else { return false };
    players.iter().filter_map(|p| browser_app(&p.bundle)).any(|app| states.get(app).is_some_and(|(_, state)| state.is_none()))
}

fn address(selector: u32) -> PropertyAddress {
    PropertyAddress { selector, scope: SCOPE_GLOBAL, element: ELEMENT_MAIN }
}

/// Un nombre de 32 bits d'un objet CoreAudio, ou `default` si la propriété manque.
fn number(object: u32, selector: u32, default: u32) -> u32 {
    let mut value = default;
    let mut size = std::mem::size_of::<u32>() as u32;
    // SAFETY : `value` est un entier de 32 bits valide, `size` dit sa taille ; CoreAudio n'écrit pas au-delà.
    let status = unsafe {
        AudioObjectGetPropertyData(object, &address(selector), 0, std::ptr::null(), &mut size, std::ptr::addr_of_mut!(value).cast())
    };
    if status == 0 {
        value
    } else {
        default
    }
}

/// L'identifiant de bundle d'un processus (vide s'il n'en a pas).
fn bundle_id(object: u32) -> String {
    let mut string: Ref = std::ptr::null();
    let mut size = std::mem::size_of::<Ref>() as u32;
    // SAFETY : CoreAudio dépose une `CFStringRef` (à relâcher) dans `string`, de la taille d'un pointeur.
    let status = unsafe {
        AudioObjectGetPropertyData(
            object,
            &address(PROCESS_BUNDLE_ID),
            0,
            std::ptr::null(),
            &mut size,
            std::ptr::addr_of_mut!(string).cast(),
        )
    };
    if status != 0 || string.is_null() {
        return String::new();
    }
    let mut buffer = [0 as c_char; 512];
    // SAFETY : `string` est une chaîne CoreFoundation valide (obtenue ci-dessus), le tampon a la taille annoncée ; relâchée après lecture.
    let text = unsafe {
        let ok = CFStringGetCString(string, buffer.as_mut_ptr(), buffer.len() as isize, UTF8);
        ffi::CFRelease(string);
        ok.then(|| std::ffi::CStr::from_ptr(buffer.as_ptr()).to_string_lossy().into_owned())
    };
    text.unwrap_or_default()
}

/// Les processus audio de la session, à cet instant. Vide si CoreAudio ne donne pas la liste (macOS avant 14.2).
fn read_processes() -> Vec<AudioProcess> {
    let list = address(PROCESS_OBJECT_LIST);
    let mut size = 0u32;
    // SAFETY : `size` reçoit la taille de la liste ; rien d'autre n'est lu ou écrit.
    if unsafe { AudioObjectGetPropertyDataSize(SYSTEM_OBJECT, &list, 0, std::ptr::null(), &mut size) } != 0 || size == 0 {
        return Vec::new();
    }
    let mut ids = vec![0u32; size as usize / std::mem::size_of::<u32>()];
    // SAFETY : `ids` a la place annoncée par `size`, que CoreAudio met à jour avec ce qu'il a écrit.
    if unsafe { AudioObjectGetPropertyData(SYSTEM_OBJECT, &list, 0, std::ptr::null(), &mut size, ids.as_mut_ptr().cast()) } != 0 {
        return Vec::new();
    }
    ids.truncate(size as usize / std::mem::size_of::<u32>());
    ids.into_iter()
        .map(|id| AudioProcess {
            pid: i64::from(number(id, PROCESS_PID, u32::MAX) as i32),
            bundle: bundle_id(id),
            output: number(id, PROCESS_RUNNING_OUTPUT, 0) != 0,
            input: number(id, PROCESS_RUNNING_INPUT, 0) != 0,
        })
        .filter(|process| !is_own_sound(process.pid))
        .map(|mut process| {
            // Un lecteur qui dit qu'il ne joue pas ne joue pas, même si son flux audio reste ouvert.
            if process.output && player_says_playing(&process.bundle) == Some(false) {
                process.output = false;
            }
            process
        })
        .collect()
}

/// `CGPoint` pour Objective-C.
#[repr(C)]
struct Point {
    x: f64,
    y: f64,
}

// SAFETY : même disposition que `CGPoint` (deux `CGFloat`, donc deux `f64`).
unsafe impl Encode for Point {
    const ENCODING: Encoding = Encoding::Struct("CGPoint", &[Encoding::Double, Encoding::Double]);
}

/// Un `CGEventRef` rendu par `-[NSEvent CGEvent]`.
#[repr(transparent)]
struct CgEvent(Ref);

// SAFETY : `CGEventRef` est un pointeur vers une structure opaque `__CGEvent`.
unsafe impl Encode for CgEvent {
    const ENCODING: Encoding = Encoding::Pointer(&Encoding::Struct("__CGEvent", &[]));
}

/// La touche Lecture/Pause (`NX_KEYTYPE_PLAY` = 16), appuyée puis relâchée : macOS l'envoie à l'appli « en cours de lecture ».
fn post_play_pause() {
    const KEY_PLAY: isize = 16;
    const SYSTEM_DEFINED: usize = 14;
    const MEDIA_KEY_SUBTYPE: i16 = 8;
    for down in [true, false] {
        let state: isize = if down { 0xA } else { 0xB };
        // SAFETY : `NSEvent` existe (AppKit est lié plus haut) ; les types des arguments suivent la signature de
        // `+otherEventWithType:location:modifierFlags:timestamp:windowNumber:context:subtype:data1:data2:`. Le `CGEventRef` rendu appartient à
        // l'`NSEvent` (qui vit jusqu'à la fin de la boucle) : on le poste sans le relâcher.
        unsafe {
            let event: *mut AnyObject = msg_send![
                class!(NSEvent),
                otherEventWithType: SYSTEM_DEFINED,
                location: Point { x: 0.0, y: 0.0 },
                modifierFlags: (state << 8) as usize,
                timestamp: 0.0f64,
                windowNumber: 0isize,
                context: std::ptr::null_mut::<AnyObject>(),
                subtype: MEDIA_KEY_SUBTYPE,
                data1: (KEY_PLAY << 16) | (state << 8),
                data2: -1isize
            ];
            if event.is_null() {
                continue;
            }
            let cg: CgEvent = msg_send![event, CGEvent];
            if !cg.0.is_null() {
                ffi::CGEventPost(ffi::TAP_HID, cg.0);
            }
        }
    }
}

/// Les navigateurs que ces processus audio représentent (chacun une fois).
fn browsers_of(players: &[AudioProcess]) -> Vec<&'static str> {
    let mut apps: Vec<&'static str> = Vec::new();
    for app in players.iter().filter_map(|p| browser_app(&p.bundle)) {
        if !apps.contains(&app) {
            apps.push(app);
        }
    }
    apps
}

/// Les navigateurs arrêtés (ou à relancer) par leurs pages, dans l'ordre de la pause : la relance fait la même chose.
static JS_PAUSED: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());

/// La touche du clavier est nécessaire si un de ces lecteurs n'est pas un navigateur traité par ses pages.
fn needs_key(players: &[AudioProcess], by_page: &[&'static str]) -> bool {
    players.iter().any(|p| browser_app(&p.bundle).is_none_or(|app| !by_page.contains(&app)))
}

/// La précision pour le journal : ce qui est passé par les pages, ou l'alerte d'un navigateur qui ne répond pas.
fn describe_how(players: &[AudioProcess], by_page: &[&'static str]) -> String {
    if !by_page.is_empty() {
        " — commande envoyée aux pages du navigateur (JavaScript)".into()
    } else if browser_unanswered(players) {
        " — le navigateur ne répond pas à JavaScript : une vidéo déjà arrêtée peut être relancée (Chrome : Présentation → Développeur → Autoriser JavaScript provenant d'Apple Events)".into()
    } else {
        String::new()
    }
}

/// Les processus audio vus par CoreAudio et la touche Lecture/Pause du clavier.
pub struct CoreAudioSource;

impl AudioSource for CoreAudioSource {
    fn processes(&self) -> Vec<AudioProcess> {
        read_processes()
    }

    fn play_pause(&self) {
        post_play_pause();
    }

    /// Un navigateur est mis en pause par ses pages (JavaScript) : la touche du clavier n'arrête pas toujours une vidéo de Chrome. Les autres
    /// lecteurs (et un navigateur qui ne répond pas) reçoivent la touche Lecture/Pause.
    fn play_pause_for(&self, players: &[AudioProcess]) -> String {
        let mut by_page: Vec<&'static str> = Vec::new();
        for app in browsers_of(players) {
            if browser_pause(app).is_some() {
                by_page.push(app);
            }
        }
        let key = needs_key(players, &by_page);
        *JS_PAUSED.lock().unwrap_or_else(|e| e.into_inner()) = by_page.clone();
        if key {
            post_play_pause();
        }
        describe_how(players, &by_page)
    }

    fn resume_for(&self, players: &[AudioProcess]) -> String {
        let by_page = std::mem::take(&mut *JS_PAUSED.lock().unwrap_or_else(|e| e.into_inner()));
        for app in &by_page {
            browser_resume(app);
        }
        if needs_key(players, &by_page) {
            post_play_pause();
        }
        describe_how(players, &by_page)
    }

    /// Processus système qui produisent des sons de leur propre chef (alertes, assistant) : ils ne sont pas un média à interrompre.
    fn ignored(&self) -> &'static [&'static str] {
        &[
            "com.apple.audiomxd",
            "com.apple.controlcenter",
            "com.apple.CoreSpeech",
            "com.apple.assistantd",
            "com.apple.accessibility.heard",
            "com.apple.universalaccessd",
            "com.apple.mediaremoted",
        ]
    }

    fn describe(&self) -> String {
        let all = read_processes();
        if all.is_empty() {
            return "CoreAudio ne donne aucun processus (macOS 14.2 ou plus récent requis)".into();
        }
        let shown: Vec<String> = all
            .iter()
            .map(|p| {
                format!(
                    "{}{}{}",
                    if p.bundle.is_empty() { format!("pid {}", p.pid) } else { p.bundle.clone() },
                    if p.output { " ♪" } else { "" },
                    if p.input { " 🎤" } else { "" }
                )
            })
            .collect();
        format!("{} processus audio : {}", all.len(), shown.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_browser_helper_is_matched_to_its_app_and_nothing_else_is() {
        assert_eq!(browser_app("com.google.Chrome.helper"), Some("com.google.Chrome"));
        assert_eq!(browser_app("com.google.Chrome.helper.Renderer"), Some("com.google.Chrome"));
        assert_eq!(browser_app("com.google.Chrome"), Some("com.google.Chrome"));
        assert_eq!(browser_app("com.brave.Browser.helper"), Some("com.brave.Browser"));
        assert_eq!(browser_app("com.google.ChromeRemoteDesktop"), None);
        assert_eq!(browser_app("org.videolan.vlc"), None);
        assert_eq!(browser_app(""), None);
    }

    #[test]
    fn a_browser_is_asked_about_its_pages_and_a_known_player_about_itself() {
        let (key, script, timeout) = player_query("com.google.Chrome.helper").expect("Chrome est interrogé");
        assert_eq!(key, "com.google.Chrome");
        assert!(script.contains("tell application id \"com.google.Chrome\""));
        assert!(script.contains("execute t javascript"));
        assert!(script.contains("error \"pas de réponse\""), "sans page qui répond, pas de réponse (et non « rien ne joue »)");
        assert_eq!(timeout, BROWSER_ASK_TIMEOUT);
        let (key, _, timeout) = player_query("org.videolan.vlc").expect("VLC est interrogé");
        assert_eq!(key, "org.videolan.vlc");
        assert_eq!(timeout, ASK_TIMEOUT);
        assert!(player_query("com.apple.Terminal").is_none());
    }

    fn process(bundle: &str) -> AudioProcess {
        AudioProcess { pid: 1, bundle: bundle.into(), output: true, input: false }
    }

    #[test]
    fn the_keyboard_key_is_only_used_for_players_the_pages_did_not_handle() {
        let chrome = process("com.google.Chrome.helper");
        let vlc = process("org.videolan.vlc");
        assert_eq!(browsers_of(&[chrome.clone(), chrome.clone(), vlc.clone()]), vec!["com.google.Chrome"]);
        assert!(!needs_key(std::slice::from_ref(&chrome), &["com.google.Chrome"]), "Chrome arrêté par ses pages : pas de touche");
        assert!(needs_key(std::slice::from_ref(&chrome), &[]), "Chrome ne répond pas : la touche du clavier");
        assert!(needs_key(&[chrome, vlc.clone()], &["com.google.Chrome"]), "VLC reçoit toujours la touche");
        assert!(needs_key(&[vlc], &["com.google.Chrome"]));
    }
}
