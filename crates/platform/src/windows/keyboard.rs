//! Touche globale : crochet clavier bas niveau (`WH_KEYBOARD_LL`) sur un fil à lui, avec sa boucle de messages.
//! Le crochet ne retient jamais une touche (il rend toujours la main) et ignore les touches envoyées par programme
//! (notre propre Ctrl+V). Windows retire un crochet trop lent : le rappel ne fait que poster l'événement.

use crate::keys::{current_hook_rules, key_of, translate_vk, KeyEvent, KeyListener, Subscription, VK_LMENU, VK_LWIN, VK_RWIN};
use scriibe_core::hotkey::Key;
use std::cell::RefCell;
use std::sync::mpsc::{self, Sender};
use std::sync::Mutex;
use std::thread::JoinHandle;
use windows_sys::Win32::Foundation::{GetLastError, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, PostThreadMessageW, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, HC_ACTION,
    KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

static SINK: Mutex<Option<Sender<KeyEvent>>> = Mutex::new(None);

thread_local! {
    /// Touches tenues ; le crochet s'exécute toujours sur le fil qui l'a installé.
    static HELD: RefCell<Vec<Key>> = const { RefCell::new(Vec::new()) };
    /// Les touches ordinaires de la combinaison dont l'appui a été retenu (leur relâchement l'est aussi).
    static SWALLOWED: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// Glisse une touche sans effet (code 0xE8, non attribué) entre l'appui et le relâchement d'Alt ou de Windows : sans autre touche entre les
/// deux, Windows ouvre le menu de l'appli (Alt) ou le menu Démarrer (Windows), et le collage perd le focus.
fn mask_menu_key() {
    const UNASSIGNED: u16 = 0xE8;
    let key = |up: bool| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT { wVk: UNASSIGNED, wScan: 0, dwFlags: if up { KEYEVENTF_KEYUP } else { 0 }, time: 0, dwExtraInfo: 0 },
        },
    };
    let inputs = [key(false), key(true)];
    // SAFETY : tableau d'entrées valide pour la durée de l'appel ; les touches envoyées par programme sont ignorées par ce crochet.
    unsafe {
        SendInput(inputs.len() as u32, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32);
    }
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
                let code = info.vkCode;
                let rules = current_hook_rules();
                let mut swallow = false;
                let event = HELD.with(|held| {
                    let mut held = held.borrow_mut();
                    if rules.swallow.contains(&code) {
                        SWALLOWED.with(|taken| {
                            let mut taken = taken.borrow_mut();
                            if down {
                                if taken.contains(&code) || rules.needs.iter().all(|need| held.contains(&key_of(*need))) {
                                    if !taken.contains(&code) {
                                        taken.push(code);
                                    }
                                    swallow = true;
                                }
                            } else if taken.contains(&code) {
                                taken.retain(|c| *c != code);
                                swallow = true;
                            }
                        });
                    }
                    translate_vk(&mut held, code, down)
                });
                if up && rules.mask_menu && [VK_LMENU, VK_LWIN, VK_RWIN].contains(&code) {
                    mask_menu_key();
                }
                if let Some(event) = event {
                    if let Ok(sink) = SINK.lock() {
                        if let Some(sink) = sink.as_ref() {
                            let _ = sink.send(event);
                        }
                    }
                }
                if swallow {
                    return 1;
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
