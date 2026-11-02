//! Les quelques fonctions de CoreFoundation, CoreGraphics et ApplicationServices que Scriibe appelle directement (pas de bibliothèque
//! intermédiaire : ce sont des fonctions C simples). Tout ce qui est `unsafe` de la couche macOS passe par ce fichier et par ses
//! appelants directs, avec la raison de chaque appel.

use std::ffi::c_void;

/// Un objet de CoreFoundation ou de CoreGraphics (`CFTypeRef`, `CGEventRef`…). Nul : l'appel a échoué.
pub type Ref = *const c_void;

/// Une structure de CoreFoundation dont on ne lit rien : on n'en prend que l'adresse.
#[repr(C)]
pub struct Opaque([u8; 0]);

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
    pub static kCFBooleanTrue: Ref;
    pub static kCFTypeDictionaryKeyCallBacks: Opaque;
    pub static kCFTypeDictionaryValueCallBacks: Opaque;
    pub fn CFDictionaryCreate(
        allocator: Ref,
        keys: *const Ref,
        values: *const Ref,
        count: isize,
        key_callbacks: *const Opaque,
        value_callbacks: *const Opaque,
    ) -> Ref;
    pub fn CFRelease(object: Ref);
    pub fn CFRetain(object: Ref) -> Ref;
    pub fn CFRunLoopGetCurrent() -> Ref;
    pub fn CFRunLoopRunInMode(mode: Ref, seconds: f64, return_after_source_handled: bool) -> i32;
    pub fn CFRunLoopStop(run_loop: Ref);
    pub fn CFRunLoopAddSource(run_loop: Ref, source: Ref, mode: Ref);
    pub fn CFMachPortCreateRunLoopSource(allocator: Ref, port: Ref, order: isize) -> Ref;
    pub fn CFStringCreateWithCString(allocator: Ref, text: *const std::ffi::c_char, encoding: u32) -> Ref;
    pub fn CFStringGetLength(string: Ref) -> isize;
    pub fn CFStringGetTypeID() -> usize;
    pub fn CFGetTypeID(object: Ref) -> usize;
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
    pub fn AXIsProcessTrustedWithOptions(options: Ref) -> bool;
    pub static kAXTrustedCheckOptionPrompt: Ref;
    pub fn AXUIElementCreateSystemWide() -> Ref;
    /// Rend 0 (`kAXErrorSuccess`) et pose dans `value` un objet qu'il faut relâcher.
    pub fn AXUIElementCopyAttributeValue(element: Ref, attribute: Ref, value: *mut Ref) -> i32;
}

/// `kCFStringEncodingUTF8`.
const UTF8: u32 = 0x0800_0100;

/// Une chaîne de CoreFoundation faite d'un texte ASCII (nom d'attribut) ; à relâcher.
fn cf_string(text: &std::ffi::CStr) -> Owned {
    // SAFETY : `text` est une chaîne C valide pendant l'appel ; CoreFoundation en copie le contenu.
    Owned(unsafe { CFStringCreateWithCString(std::ptr::null(), text.as_ptr(), UTF8) })
}

/// La valeur d'un attribut d'accessibilité de l'élément, si l'appli la donne.
fn attribute(element: Ref, name: &std::ffi::CStr) -> Option<Owned> {
    let key = cf_string(name);
    let mut value: Ref = std::ptr::null();
    // SAFETY : `element` et `key` sont des objets valides pendant l'appel ; `value` reçoit un objet qu'on possède (relâché par `Owned`).
    let status = unsafe { AXUIElementCopyAttributeValue(element, key.0, &mut value) };
    (status == 0 && !value.is_null()).then_some(Owned(value))
}

/// Un texte est-il sélectionné dans l'élément qui a le focus (toutes applis confondues) ? Faux si l'appli n'expose pas sa sélection
/// (c'est le cas de bien des applis Electron ou de pages web tant que l'accessibilité n'y est pas activée) ou si l'autorisation manque.
pub fn selection_is_active() -> bool {
    // SAFETY : fonction sans paramètre.
    if !unsafe { AXIsProcessTrusted() } {
        return false;
    }
    // SAFETY : fonction sans paramètre ; l'élément créé est relâché par `Owned`.
    let system = Owned(unsafe { AXUIElementCreateSystemWide() });
    if system.0.is_null() {
        return false;
    }
    let Some(focused) = attribute(system.0, c"AXFocusedUIElement") else { return false };
    let Some(selected) = attribute(focused.0, c"AXSelectedText") else { return false };
    // SAFETY : `selected` est un objet valide ; on ne lit sa longueur que si c'est bien une chaîne.
    unsafe { CFGetTypeID(selected.0) == CFStringGetTypeID() && CFStringGetLength(selected.0) > 0 }
}

/// Demande l'autorisation « Accessibilité » : macOS montre sa fenêtre (avec un bouton vers les Réglages) et ajoute l'appli à la liste. Rend
/// l'état d'avant la réponse de l'utilisateur.
pub fn ask_accessibility() -> bool {
    // SAFETY : le dictionnaire est créé avec les fonctions de rappel standard de CoreFoundation (les clés et valeurs sont des objets
    // éternels), n'est lu que pendant l'appel et relâché juste après.
    unsafe {
        let keys = [kAXTrustedCheckOptionPrompt];
        let values = [kCFBooleanTrue];
        let options = CFDictionaryCreate(
            std::ptr::null(),
            keys.as_ptr(),
            values.as_ptr(),
            1,
            std::ptr::addr_of!(kCFTypeDictionaryKeyCallBacks),
            std::ptr::addr_of!(kCFTypeDictionaryValueCallBacks),
        );
        let trusted = AXIsProcessTrustedWithOptions(options);
        if !options.is_null() {
            CFRelease(options);
        }
        trusted
    }
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
