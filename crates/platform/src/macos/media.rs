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

/// Les réponses récentes : l'état lu (`None` : pas de réponse, voir `ask_player`) et quand. Une vérification de pause relit la liste
/// toutes les 50 ms : sans cette mémoire, chaque lecture lancerait `osascript`.
type PlayerStates = HashMap<&'static str, (Instant, Option<bool>)>;
static PLAYER_STATES: Mutex<Option<PlayerStates>> = Mutex::new(None);
const STATE_FRESH: Duration = Duration::from_millis(200);
/// Sans réponse (autorisation d'automatisation non accordée : macOS attend le clic de la personne), on ne redemande pas avant ce délai.
const NO_ANSWER_FRESH: Duration = Duration::from_secs(30);
const ASK_TIMEOUT: Duration = Duration::from_millis(1500);

/// Lance le script et attend sa réponse au plus `ASK_TIMEOUT` (un `osascript` bloqué par une demande d'autorisation est tué).
fn ask_player(script: &str) -> Option<bool> {
    let mut child =
        Command::new("osascript").args(["-e", script]).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().ok()?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => break,
            Ok(Some(_)) | Err(_) => return None,
            Ok(None) if started.elapsed() > ASK_TIMEOUT => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
        }
    }
    let mut out = String::new();
    child.stdout.take()?.read_to_string(&mut out).ok()?;
    match out.trim() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

/// Ce que dit le lecteur de `bundle` ; `None` si ce n'est pas un lecteur connu ou s'il ne répond pas.
fn player_says_playing(bundle: &str) -> Option<bool> {
    let (name, script) = SELF_REPORTING_PLAYERS.iter().find(|(id, _)| *id == bundle)?;
    let mut slot = PLAYER_STATES.lock().unwrap_or_else(|e| e.into_inner());
    let states = slot.get_or_insert_with(HashMap::new);
    if let Some((at, state)) = states.get(name) {
        let fresh = if state.is_some() { STATE_FRESH } else { NO_ANSWER_FRESH };
        if at.elapsed() < fresh {
            return *state;
        }
    }
    drop(slot);
    let state = ask_player(script);
    PLAYER_STATES.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(HashMap::new).insert(name, (Instant::now(), state));
    state
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

/// Les processus audio vus par CoreAudio et la touche Lecture/Pause du clavier.
pub struct CoreAudioSource;

impl AudioSource for CoreAudioSource {
    fn processes(&self) -> Vec<AudioProcess> {
        read_processes()
    }

    fn play_pause(&self) {
        post_play_pause();
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
