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
    pub fn CFStringGetMaximumSizeForEncoding(length: isize, encoding: u32) -> isize;
    pub fn CFStringGetCString(string: Ref, buffer: *mut std::ffi::c_char, size: isize, encoding: u32) -> bool;
    pub fn CFStringGetTypeID() -> usize;
    pub fn CFGetTypeID(object: Ref) -> usize;
    pub fn CFArrayGetTypeID() -> usize;
    pub fn CFArrayGetCount(array: Ref) -> isize;
    pub fn CFArrayGetValueAtIndex(array: Ref, index: isize) -> Ref;
    pub fn CFStringGetCharacterAtIndex(string: Ref, index: isize) -> u16;
    pub fn CFNumberGetValue(number: Ref, kind: i32, value: *mut std::ffi::c_void) -> bool;
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
    /// Pose le texte que la touche « tape » (jusqu'à 20 unités UTF-16 par événement), quelle que soit la touche virtuelle.
    pub fn CGEventKeyboardSetUnicodeString(event: Ref, length: usize, chars: *const u16);
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
    pub fn AXUIElementCreateApplication(pid: i32) -> Ref;
    /// Rend 0 (`kAXErrorSuccess`) et pose dans `value` un objet qu'il faut relâcher.
    pub fn AXUIElementCopyAttributeValue(element: Ref, attribute: Ref, value: *mut Ref) -> i32;
    /// Rend 0 (`kAXErrorSuccess`) si l'action (« AXPress ») a été faite.
    pub fn AXUIElementPerformAction(element: Ref, action: Ref) -> i32;
}

/// `kCFStringEncodingUTF8`.
const UTF8: u32 = 0x0800_0100;

/// Une chaîne de CoreFoundation faite d'un texte ASCII (nom d'attribut) ; à relâcher.
fn cf_string(text: &std::ffi::CStr) -> Owned {
    // SAFETY : `text` est une chaîne C valide pendant l'appel ; CoreFoundation en copie le contenu.
    Owned(unsafe { CFStringCreateWithCString(std::ptr::null(), text.as_ptr(), UTF8) })
}

/// Ce que l'accessibilité dit de la sélection de l'élément qui a le focus (pour le journal, sans jamais le texte).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionState {
    Active,
    Empty,
    /// L'appli ne donne pas cet élément ou cet attribut : (ce qui manque, code d'erreur d'accessibilité).
    Unknown(&'static str, i32),
    NotTrusted,
}

impl SelectionState {
    pub fn describe(&self) -> String {
        match self {
            Self::Active => "active".into(),
            Self::Empty => "aucune".into(),
            Self::Unknown(what, code) => format!("inconnue ({what}, code {code})"),
            Self::NotTrusted => "inconnue (Accessibilité non autorisée)".into(),
        }
    }
}

/// La valeur d'un attribut d'accessibilité de l'élément, ou le code d'erreur de l'appli.
fn attribute(element: Ref, name: &std::ffi::CStr) -> Result<Owned, i32> {
    let key = cf_string(name);
    let mut value: Ref = std::ptr::null();
    // SAFETY : `element` et `key` sont des objets valides pendant l'appel ; `value` reçoit un objet qu'on possède (relâché par `Owned`).
    let status = unsafe { AXUIElementCopyAttributeValue(element, key.0, &mut value) };
    if status == 0 && !value.is_null() {
        Ok(Owned(value))
    } else {
        Err(if status == 0 { -1 } else { status })
    }
}

/// Un texte est-il sélectionné dans l'élément qui a le focus (toutes applis confondues) ? « Inconnue » si l'appli n'expose pas sa
/// sélection (bien des applis Electron ou pages web tant que l'accessibilité n'y est pas activée) ou si l'autorisation manque.
pub fn selection_state() -> SelectionState {
    // SAFETY : fonction sans paramètre.
    if !unsafe { AXIsProcessTrusted() } {
        return SelectionState::NotTrusted;
    }
    // SAFETY : fonction sans paramètre ; l'élément créé est relâché par `Owned`.
    let system = Owned(unsafe { AXUIElementCreateSystemWide() });
    if system.0.is_null() {
        return SelectionState::Unknown("système", -1);
    }
    let focused = match attribute(system.0, c"AXFocusedUIElement") {
        Ok(f) => f,
        Err(code) => return SelectionState::Unknown("élément ayant le focus", code),
    };
    let selected = match attribute(focused.0, c"AXSelectedText") {
        Ok(s) => s,
        Err(code) => return SelectionState::Unknown("texte sélectionné", code),
    };
    // SAFETY : `selected` est un objet valide ; on ne lit sa longueur que si c'est bien une chaîne.
    let is_text = unsafe { CFGetTypeID(selected.0) == CFStringGetTypeID() };
    if !is_text {
        return SelectionState::Unknown("type de la sélection", -2);
    }
    // SAFETY : voir ci-dessus.
    if unsafe { CFStringGetLength(selected.0) } > 0 {
        SelectionState::Active
    } else {
        SelectionState::Empty
    }
}

/// Ce que la lecture du texte sélectionné par l'accessibilité a donné (le texte n'est lu que pour une commande vocale).
pub enum SelectedByAx {
    Text(String),
    Empty,
    /// L'appli n'expose pas sa sélection : un ⌘C d'essai peut encore la lire.
    Unknown,
    NotTrusted,
}

/// Le texte sélectionné dans l'élément qui a le focus, lu par l'accessibilité : le presse-papiers n'est pas touché.
pub fn selected_text() -> SelectedByAx {
    // SAFETY : fonction sans paramètre.
    if !unsafe { AXIsProcessTrusted() } {
        return SelectedByAx::NotTrusted;
    }
    // SAFETY : fonction sans paramètre ; l'élément créé est relâché par `Owned`.
    let system = Owned(unsafe { AXUIElementCreateSystemWide() });
    if system.0.is_null() {
        return SelectedByAx::Unknown;
    }
    let Ok(focused) = attribute(system.0, c"AXFocusedUIElement") else { return SelectedByAx::Unknown };
    let Ok(selected) = attribute(focused.0, c"AXSelectedText") else { return SelectedByAx::Unknown };
    // SAFETY : `selected` est un objet valide ; on ne lit que si c'est bien une chaîne.
    if unsafe { CFGetTypeID(selected.0) != CFStringGetTypeID() } {
        return SelectedByAx::Unknown;
    }
    // SAFETY : voir ci-dessus.
    let length = unsafe { CFStringGetLength(selected.0) };
    if length <= 0 {
        return SelectedByAx::Empty;
    }
    // SAFETY : la taille maximale en UTF-8 est donnée par CoreFoundation ; le tampon la couvre, avec l'octet nul final.
    let capacity = unsafe { CFStringGetMaximumSizeForEncoding(length, UTF8) };
    if capacity <= 0 {
        return SelectedByAx::Unknown;
    }
    let mut buffer = vec![0u8; capacity as usize + 1];
    // SAFETY : `buffer` a `capacity + 1` octets et reste valide pendant l'appel.
    let copied = unsafe { CFStringGetCString(selected.0, buffer.as_mut_ptr().cast(), capacity + 1, UTF8) };
    let text = if copied {
        let end = buffer.iter().position(|b| *b == 0).unwrap_or(buffer.len());
        match String::from_utf8(buffer[..end].to_vec()) {
            Ok(t) => Some(t),
            Err(e) => {
                // Octets invalides : on les écrase avant de les laisser tomber.
                let mut bytes = e.into_bytes();
                bytes.fill(0);
                None
            }
        }
    } else {
        None
    };
    buffer.fill(0);
    match text {
        Some(t) => SelectedByAx::Text(t),
        None => SelectedByAx::Unknown,
    }
}

/// Cherche dans les menus de l'appli le « Coller » (⌘V, sans autre modificateur) et le déclenche, comme un clic dans le menu Édition :
/// aucune touche simulée, donc rien que Chrome puisse ignorer. `Err` : l'élément n'existe pas, est grisé ou refuse l'action ; l'appelant
/// retombe alors sur ⌘V. Le nom du menu ne compte pas (langue du système) : on reconnaît l'élément à son raccourci.
pub fn press_paste_menu_item(pid: i32) -> Result<(), String> {
    // SAFETY : fonction sans paramètre.
    if !unsafe { AXIsProcessTrusted() } {
        return Err("Accessibilité non autorisée".into());
    }
    // SAFETY : fonction sans autre condition que son argument ; l'élément créé est relâché par `Owned`. (L'élément « système » ne rend pas
    // toujours l'appli active — code -25212 constaté avec Chrome — d'où le numéro de processus.)
    let app = Owned(unsafe { AXUIElementCreateApplication(pid) });
    if app.0.is_null() {
        return Err("appli active".into());
    }
    let bar = attribute(app.0, c"AXMenuBar").map_err(|c| format!("barre des menus (code {c})"))?;
    find_and_press_paste(bar.0, 0)
}

/// Profondeur utile : barre → menu de la barre → menu → élément (Édition → Coller), avec une marge pour un sous-menu.
const MENU_DEPTH: u32 = 5;

fn find_and_press_paste(element: Ref, depth: u32) -> Result<(), String> {
    if depth > MENU_DEPTH {
        return Err("Coller introuvable".into());
    }
    let Ok(children) = attribute(element, c"AXChildren") else { return Err("Coller introuvable".into()) };
    // SAFETY : on ne lit le tableau que si c'est bien un tableau ; ses éléments restent valides tant que `children` l'est.
    if unsafe { CFGetTypeID(children.0) != CFArrayGetTypeID() } {
        return Err("Coller introuvable".into());
    }
    // SAFETY : voir ci-dessus.
    let count = unsafe { CFArrayGetCount(children.0) };
    for i in 0..count {
        // SAFETY : `i` est dans les bornes du tableau ; l'élément rendu n'est pas à nous (pas de relâchement).
        let child = unsafe { CFArrayGetValueAtIndex(children.0, i) };
        if child.is_null() {
            continue;
        }
        if is_paste_item(child) {
            let action = cf_string(c"AXPress");
            // SAFETY : `child` et `action` sont des objets valides pendant l'appel.
            let status = unsafe { AXUIElementPerformAction(child, action.0) };
            return if status == 0 { Ok(()) } else { Err(format!("Coller refusé (code {status})")) };
        }
        if find_and_press_paste(child, depth + 1).is_ok() {
            return Ok(());
        }
    }
    Err("Coller introuvable".into())
}

/// L'élément de menu dont le raccourci est ⌘V seul (modificateurs : 0 = ⌘ seul).
fn is_paste_item(item: Ref) -> bool {
    let Ok(key) = attribute(item, c"AXMenuItemCmdChar") else { return false };
    // SAFETY : on ne lit la chaîne que si c'en est une.
    let is_v = unsafe {
        CFGetTypeID(key.0) == CFStringGetTypeID()
            && CFStringGetLength(key.0) == 1
            && matches!(CFStringGetCharacterAtIndex(key.0, 0), 0x56 | 0x76)
    };
    if !is_v {
        return false;
    }
    let Ok(modifiers) = attribute(item, c"AXMenuItemCmdModifiers") else { return false };
    let mut value: i32 = -1;
    // SAFETY : `modifiers` est un objet valide ; `kCFNumberSInt32Type` = 3 ; `value` a la taille attendue.
    let read = unsafe { CFNumberGetValue(modifiers.0, 3, std::ptr::addr_of_mut!(value).cast()) };
    read && value == 0
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
