//! Touche globale : crochet clavier bas niveau (`WH_KEYBOARD_LL`) sur un fil à lui, avec sa boucle de messages.
//! Le crochet ne retient jamais une touche (il rend toujours la main) et ignore les touches envoyées par programme
//! (notre propre Ctrl+V). Windows retire un crochet trop lent : le rappel ne fait que poster l'événement.

use crate::keys::{translate_vk, KeyEvent, KeyListener, Subscription};
use scriibe_core::hotkey::Key;
use std::cell::RefCell;
use std::sync::mpsc::{self, Sender};
use std::sync::Mutex;
use std::thread::JoinHandle;
use windows_sys::Win32::Foundation::{GetLastError, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, PostThreadMessageW, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, HC_ACTION,
    KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

static SINK: Mutex<Option<Sender<KeyEvent>>> = Mutex::new(None);

thread_local! {
    /// Touches de modification tenues ; le crochet s'exécute toujours sur le fil qui l'a installé.
    static HELD: RefCell<Vec<Key>> = const { RefCell::new(Vec::new()) };
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        // SAFETY : pour HC_ACTION, `lparam` pointe vers une KBDLLHOOKSTRUCT valide pendant l'appel.
        let info = unsafe { &*(lparam as *const KBDLLHOOKSTRUCT) };
        if info.flags & LLKHF_INJECTED == 0 {
            let message = wparam as u32;
            let down = message == WM_KEYDOWN || message == WM_SYSKEYDOWN;
            let up = message == WM_KEYUP || message == WM_SYSKEYUP;
            if down || up {
                let event = HELD.with(|held| translate_vk(&mut held.borrow_mut(), info.vkCode, down));
                if let Some(event) = event {
                    if let Ok(sink) = SINK.lock() {
                        if let Some(sink) = sink.as_ref() {
                            let _ = sink.send(event);
                        }
                    }
                }
            }
        }
    }
    // SAFETY : on passe les mêmes arguments au crochet suivant.
    unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) }
}

pub struct WindowsKeys;

struct HookThread {
    thread_id: u32,
    handle: Option<JoinHandle<()>>,
}

impl Subscription for HookThread {}

impl Drop for HookThread {
    fn drop(&mut self) {
        // SAFETY : demande à la boucle de messages du fil du crochet de s'arrêter.
        unsafe {
            PostThreadMessageW(self.thread_id, WM_QUIT, 0, 0);
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        if let Ok(mut sink) = SINK.lock() {
            *sink = None;
        }
    }
}

impl KeyListener for WindowsKeys {
    fn start(&self, sink: Sender<KeyEvent>) -> Result<Box<dyn Subscription>, String> {
        if let Ok(mut slot) = SINK.lock() {
            *slot = Some(sink);
        }
        let (ready_tx, ready_rx) = mpsc::channel::<Result<u32, String>>();
        let handle = std::thread::Builder::new()
            .name("scriibe-clavier".into())
            .spawn(move || {
                // SAFETY : installation du crochet sur ce fil, puis boucle de messages (obligatoire pour qu'il soit appelé).
                unsafe {
                    let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), GetModuleHandleW(std::ptr::null()), 0);
                    if hook.is_null() {
                        let _ = ready_tx.send(Err(format!("crochet clavier refusé par Windows (code {})", GetLastError())));
                        return;
                    }
                    let _ = ready_tx.send(Ok(GetCurrentThreadId()));
                    let mut message: MSG = std::mem::zeroed();
                    while GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) > 0 {
                        TranslateMessage(&message);
                        DispatchMessageW(&message);
                    }
                    UnhookWindowsHookEx(hook);
                }
            })
            .map_err(|e| format!("fil du clavier : {e}"))?;
        match ready_rx.recv() {
            Ok(Ok(thread_id)) => Ok(Box::new(HookThread { thread_id, handle: Some(handle) })),
            Ok(Err(message)) => Err(message),
            Err(_) => Err("le fil du clavier s'est arrêté avant d'être prêt".into()),
        }
    }
}
