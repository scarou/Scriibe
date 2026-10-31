//! Les quelques fonctions de CoreFoundation, CoreGraphics et ApplicationServices que Scriibe appelle directement (pas de bibliothèque
//! intermédiaire : ce sont des fonctions C simples). Tout ce qui est `unsafe` de la couche macOS passe par ce fichier et par ses
//! appelants directs, avec la raison de chaque appel.

use std::ffi::c_void;

/// Un objet de CoreFoundation ou de CoreGraphics (`CFTypeRef`, `CGEventRef`…). Nul : l'appel a échoué.
pub type Ref = *const c_void;

/// `CGEventTapCallBack` : (proxy, type d'événement, événement, `userInfo`) -> l'événement.
pub type TapCallback = extern "C" fn(*mut c_void, u32, Ref, *mut c_void) -> Ref;

// `CGEventType`.
pub const KEY_DOWN: u32 = 10;
pub const KEY_UP: u32 = 11;
pub const FLAGS_CHANGED: u32 = 12;
pub const TAP_DISABLED_BY_TIMEOUT: u32 = 0xFFFF_FFFE;
pub const TAP_DISABLED_BY_USER_INPUT: u32 = 0xFFFF_FFFF;

// `CGEventField`.
pub const FIELD_KEYCODE: u32 = 9;

// `CGEventTapLocation`, `CGEventTapPlacement`, `CGEventTapOptions`.
pub const TAP_HID: u32 = 0;
pub const TAP_HEAD_INSERT: u32 = 0;
pub const TAP_LISTEN_ONLY: u32 = 1;

/// `kCGEventSourceStateCombinedSessionState` : l'état des touches vu par la session, matériel et programmes confondus.
pub const SOURCE_COMBINED_SESSION: i32 = 0;

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    pub static kCFRunLoopDefaultMode: Ref;
    pub fn CFRelease(object: Ref);
    pub fn CFRetain(object: Ref) -> Ref;
    pub fn CFRunLoopGetCurrent() -> Ref;
    pub fn CFRunLoopRunInMode(mode: Ref, seconds: f64, return_after_source_handled: bool) -> i32;
    pub fn CFRunLoopStop(run_loop: Ref);
    pub fn CFRunLoopAddSource(run_loop: Ref, source: Ref, mode: Ref);
    pub fn CFMachPortCreateRunLoopSource(allocator: Ref, port: Ref, order: isize) -> Ref;
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    pub fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        events_of_interest: u64,
        callback: TapCallback,
        user_info: *mut c_void,
    ) -> Ref;
    pub fn CGEventTapEnable(tap: Ref, enable: bool);
    pub fn CGEventGetIntegerValueField(event: Ref, field: u32) -> i64;
    pub fn CGEventGetFlags(event: Ref) -> u64;
    pub fn CGEventCreateKeyboardEvent(source: Ref, key: u16, down: bool) -> Ref;
    pub fn CGEventSetFlags(event: Ref, flags: u64);
    pub fn CGEventPost(tap: u32, event: Ref);
    pub fn CGEventSourceFlagsState(state: i32) -> u64;
    /// Autorisation « Surveillance de l'entrée » (macOS 10.15 et après).
    pub fn CGPreflightListenEventAccess() -> bool;
    /// Montre la demande d'autorisation « Surveillance de l'entrée » (une seule fois par appli).
    pub fn CGRequestListenEventAccess() -> bool;
}

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    /// Autorisation « Accessibilité », nécessaire pour envoyer ⌘V à une autre appli.
    pub fn AXIsProcessTrusted() -> bool;
}

/// Un objet de CoreFoundation qu'on possède : relâché à la fin.
pub struct Owned(pub Ref);

// SAFETY : les objets de CoreFoundation que Scriibe garde (boucle d'exécution, source) se retiennent et se relâchent depuis n'importe quel fil.
unsafe impl Send for Owned {}

impl Drop for Owned {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY : l'objet a été obtenu par une fonction « Create » ou « Retain » et n'est relâché qu'ici.
            unsafe { CFRelease(self.0) };
        }
    }
}
