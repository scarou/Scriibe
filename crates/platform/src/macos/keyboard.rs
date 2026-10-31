//! Touche globale : un « tap » d'événements de CoreGraphics en écoute seule (`kCGEventTapOptionListenOnly`) sur le clavier. Il voit chaque
//! touche, quelle que soit l'appli active, sans rien retenir : l'appli active reçoit les touches comme avant. Option, Commande, Ctrl, Maj et
//! Fn arrivent comme changements de modificateurs (`FlagsChanged`), chacun avec le bit de sa touche (gauche et droite se distinguent) ; les
//! autres touches comme appui et relâchement.
//!
//! macOS demande l'autorisation « Surveillance de l'entrée » (Réglages Système → Confidentialité et sécurité) : sans elle, la création du
//! tap échoue. Quand Scriibe est lancée depuis un terminal, c'est le terminal qui doit l'avoir.
//!
//! Écart avec Windows (`docs/ECARTS.md`) : comme sous Linux, une touche ordinaire de la combinaison (F9…) n'est pas retenue.

use super::ffi::{self, Owned, Ref};
use crate::apple::{modifier_of_keycode, vk_of_keycode};
use crate::keys::{translate_key, KeyEvent, KeyListener, Subscription};
use scriibe_core::hotkey::Key;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

pub struct MacKeys;

/// Ce que le rappel du système a besoin de connaître ; il vit tant que le fil du clavier tourne.
struct Context {
    sink: Sender<KeyEvent>,
    held: Mutex<Vec<Key>>,
    /// Le tap, pour le réactiver si macOS l'a coupé (rappel trop lent ou saisie sécurisée).
    tap: AtomicPtr<c_void>,
}

const MESSAGE_DENIED: &str = "macOS refuse la touche globale : autorise Scriibe (ou l'appli qui la lance, par exemple Terminal) dans \
Réglages Système → Confidentialité et sécurité → Surveillance de l'entrée, puis relance Scriibe";

extern "C" fn on_event(_proxy: *mut c_void, kind: u32, event: Ref, user: *mut c_void) -> Ref {
    // SAFETY : `user` est le `Context` du fil du clavier, qui ne le libère qu'après avoir arrêté le tap.
    let context = unsafe { &*(user as *const Context) };
    match kind {
        ffi::TAP_DISABLED_BY_TIMEOUT | ffi::TAP_DISABLED_BY_USER_INPUT => {
            let tap = context.tap.load(Ordering::SeqCst);
            if !tap.is_null() {
                // SAFETY : `tap` est le tap vivant de ce fil.
                unsafe { ffi::CGEventTapEnable(tap, true) };
            }
        }
        ffi::KEY_DOWN | ffi::KEY_UP | ffi::FLAGS_CHANGED => {
            // SAFETY : `event` est valable pendant tout le rappel.
            let code = unsafe { ffi::CGEventGetIntegerValueField(event, ffi::FIELD_KEYCODE) } as u32;
            let change = if kind == ffi::FLAGS_CHANGED {
                // SAFETY : idem.
                let flags = unsafe { ffi::CGEventGetFlags(event) };
                // Les autres changements de modificateurs (verrouillage des majuscules…) ne servent à rien ici.
                modifier_of_keycode(code).map(|(key, mask)| (key, flags & mask != 0))
            } else {
                Some((Key::Vk(vk_of_keycode(code)), kind == ffi::KEY_DOWN))
            };
            if let Some((key, down)) = change {
                let mut held = context.held.lock().unwrap_or_else(|p| p.into_inner());
                if let Some(event) = translate_key(&mut held, key, down) {
                    let _ = context.sink.send(event);
                }
            }
        }
        _ => {}
    }
    event
}

const EVENTS: u64 = (1 << ffi::KEY_DOWN) | (1 << ffi::KEY_UP) | (1 << ffi::FLAGS_CHANGED);

/// Le fil du clavier : crée le tap, annonce qu'il est prêt (ou pourquoi il ne l'est pas), puis tourne jusqu'à l'arrêt.
fn run_tap(sink: Sender<KeyEvent>, stop: Arc<AtomicBool>, ready: Sender<Result<Owned, String>>) {
    let context = Box::new(Context { sink, held: Mutex::new(Vec::new()), tap: AtomicPtr::new(std::ptr::null_mut()) });
    let user = &*context as *const Context as *mut c_void;
    // SAFETY : `on_event` suit la signature de CGEventTapCallBack et `user` reste valable jusqu'à la fin de cette fonction.
    let tap = unsafe { ffi::CGEventTapCreate(ffi::TAP_HID, ffi::TAP_HEAD_INSERT, ffi::TAP_LISTEN_ONLY, EVENTS, on_event, user) };
    if tap.is_null() {
        // Fait apparaître la demande d'autorisation de macOS (une seule fois), puis dit quoi faire.
        // SAFETY : fonctions sans paramètre.
        unsafe {
            if !ffi::CGPreflightListenEventAccess() {
                ffi::CGRequestListenEventAccess();
            }
        }
        let _ = ready.send(Err(MESSAGE_DENIED.into()));
        return;
    }
    let tap = Owned(tap);
    context.tap.store(tap.0 as *mut c_void, Ordering::SeqCst);
    // SAFETY : `tap` est un tap vivant ; la source créée est ajoutée à la boucle d'exécution de ce fil, qui la garde.
    let source = Owned(unsafe { ffi::CFMachPortCreateRunLoopSource(std::ptr::null(), tap.0, 0) });
    if source.0.is_null() {
        let _ = ready.send(Err("macOS n'a pas pu brancher la touche globale".into()));
        return;
    }
    // SAFETY : la boucle d'exécution du fil courant existe toujours ; on en garde une référence pour pouvoir l'arrêter depuis un autre fil.
    let run_loop = unsafe {
        let run_loop = ffi::CFRunLoopGetCurrent();
        ffi::CFRunLoopAddSource(run_loop, source.0, ffi::kCFRunLoopDefaultMode);
        ffi::CGEventTapEnable(tap.0, true);
        Owned(ffi::CFRetain(run_loop))
    };
    let _ = ready.send(Ok(run_loop));
    // Par tranches d'une seconde : si l'arrêt est demandé avant le début de la boucle, l'indicateur le dit quand même.
    while !stop.load(Ordering::SeqCst) {
        // SAFETY : tourne dans la boucle d'exécution de ce fil, où la source est ajoutée.
        unsafe { ffi::CFRunLoopRunInMode(ffi::kCFRunLoopDefaultMode, 1.0, false) };
    }
    // SAFETY : le tap est encore vivant ; coupé avant que `context` soit libéré.
    unsafe { ffi::CGEventTapEnable(tap.0, false) };
    drop(source);
    drop(tap);
    drop(context);
}

struct Listening {
    stop: Arc<AtomicBool>,
    run_loop: Owned,
    handle: Option<JoinHandle<()>>,
}

impl Subscription for Listening {}

impl Drop for Listening {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Réveille la boucle sans attendre la fin de sa tranche.
        // SAFETY : `run_loop` est retenue ; `CFRunLoopStop` s'appelle depuis n'importe quel fil.
        unsafe { ffi::CFRunLoopStop(self.run_loop.0) };
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl KeyListener for MacKeys {
    fn start(&self, sink: Sender<KeyEvent>) -> Result<Box<dyn Subscription>, String> {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let (ready_tx, ready_rx) = mpsc::channel();
        let handle = std::thread::Builder::new()
            .name("scriibe-clavier".into())
            .spawn(move || run_tap(sink, flag, ready_tx))
            .map_err(|e| format!("fil du clavier : {e}"))?;
        match ready_rx.recv() {
            Ok(Ok(run_loop)) => Ok(Box::new(Listening { stop, run_loop, handle: Some(handle) })),
            Ok(Err(message)) => {
                let _ = handle.join();
                Err(message)
            }
            Err(_) => Err("le fil du clavier s'est arrêté avant d'être prêt".into()),
        }
    }
}
