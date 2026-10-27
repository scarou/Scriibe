//! Pause des médias, côté Windows : les sessions audio de WASAPI disent quelles applis sortent du son (appareil de lecture par défaut)
//! ou captent le micro (appareil d'enregistrement par défaut) ; la touche Lecture/Pause est envoyée avec `SendInput`.
//!
//! « Sort du son » = session active **et** son qui passe vraiment (niveau crête non nul) : un navigateur dont la vidéo est en pause
//! garde souvent son flux ouvert en silence, et la touche Lecture/Pause la relancerait. Seul l'appareil par défaut est regardé.

use crate::media::AudioSource;
use scriibe_core::media_pause::AudioProcess;
use windows::core::Interface;
use windows::Win32::Media::Audio::Endpoints::IAudioMeterInformation;
use windows::Win32::Media::Audio::{
    eCapture, eConsole, eRender, AudioSessionStateActive, EDataFlow, IAudioSessionControl2, IAudioSessionManager2, IMMDeviceEnumerator,
    MMDeviceEnumerator,
};
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED};
use windows_sys::Win32::Foundation::{CloseHandle, BOOL, HWND, LPARAM};
use windows_sys::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_MEDIA_PLAY_PAUSE,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindow, GetWindowThreadProcessId, IsWindowVisible, SendMessageTimeoutW, GW_OWNER, SMTO_ABORTIFHUNG,
};

const WM_APPCOMMAND: u32 = 0x0319;
const APPCOMMAND_MEDIA_PLAY_PAUSE: isize = 14;

/// En dessous, la session est du silence (flux ouvert sans son).
const AUDIBLE: f32 = 0.0005;

/// Programmes qui font du bruit sans être un média à interrompre.
const IGNORED: [&str; 1] = ["audiodg.exe"];

pub struct WasapiSessions;

/// Un fil qui a initialisé COM pour la durée de sa vie (les appels COM se font sur un fil dédié à chaque lecture).
struct ComScope(bool);

impl ComScope {
    fn enter() -> Self {
        // SAFETY : initialisation de COM pour ce fil ; la sortie correspondante est dans `Drop`.
        ComScope(unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok())
    }
}

impl Drop for ComScope {
    fn drop(&mut self) {
        if self.0 {
            // SAFETY : appariée à l'initialisation réussie de `enter`.
            unsafe { CoUninitialize() };
        }
    }
}

/// Le nom de l'exécutable d'un processus (« chrome.exe »), vide s'il n'est pas lisible.
fn executable_name(pid: u32) -> String {
    // SAFETY : poignée en lecture limitée, fermée avant de rendre ; le tampon est assez grand pour le chemin demandé.
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return String::new();
        }
        let mut buffer = [0u16; 1024];
        let mut length = buffer.len() as u32;
        let ok = QueryFullProcessImageNameW(handle, 0, buffer.as_mut_ptr(), &mut length) != 0;
        CloseHandle(handle);
        if !ok {
            return String::new();
        }
        let path = String::from_utf16_lossy(&buffer[..length as usize]);
        path.rsplit(['\\', '/']).next().unwrap_or("").to_string()
    }
}

/// Ce que Windows dit d'une session audio (pour la décision, et pour le journal quand rien n'est mis en pause).
struct Session {
    capture: bool,
    pid: u32,
    name: String,
    active: bool,
    system: bool,
    /// Niveau crête du son (0 à 1) ; absent si Windows ne l'a pas donné.
    peak: Option<f32>,
}

impl Session {
    /// Sort du son : active et son qui passe vraiment (un flux ouvert en silence n'est pas un média qui joue).
    fn counts(&self) -> bool {
        self.active && !self.system && self.pid != 0 && (self.capture || self.peak.is_some_and(|p| p >= AUDIBLE))
    }

    fn describe(&self) -> String {
        format!(
            "{} {} (pid {}){}{}{}",
            if self.capture { "micro" } else { "son" },
            if self.name.is_empty() { "?" } else { &self.name },
            self.pid,
            if self.active { " actif" } else { " inactif" },
            if self.system { " système" } else { "" },
            self.peak.map_or_else(|| " crête illisible".to_string(), |p| format!(" crête {p:.3}")),
        )
    }
}

/// Les sessions de l'appareil par défaut. Une session illisible est notée dans `problems` et ne fait pas échouer les autres.
fn read_sessions(flow: EDataFlow, problems: &mut Vec<String>) -> Vec<Session> {
    let capture = flow == eCapture;
    let label = if capture { "enregistrement" } else { "lecture" };
    // SAFETY : appels COM sur un fil où COM est initialisé (`ComScope`) ; chaque interface est libérée à sa sortie de portée.
    let result: windows::core::Result<Vec<Session>> = unsafe {
        (|| {
            let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let device = enumerator.GetDefaultAudioEndpoint(flow, eConsole)?;
            let manager: IAudioSessionManager2 = device.Activate(CLSCTX_ALL, None)?;
            let list = manager.GetSessionEnumerator()?;
            let mut sessions = Vec::new();
            for i in 0..list.GetCount()? {
                let control = list.GetSession(i)?;
                let control2: IAudioSessionControl2 = control.cast()?;
                let pid = control2.GetProcessId().unwrap_or(0);
                sessions.push(Session {
                    capture,
                    pid,
                    name: if pid == 0 { String::new() } else { executable_name(pid) },
                    active: control.GetState()? == AudioSessionStateActive,
                    // Les sons du système (bips, alertes) : la session « sons système » n'est pas un média. Réponse S_OK (0) = oui,
                    // S_FALSE (1) = non : `is_ok()` vaudrait vrai dans les deux cas, d'où la comparaison avec zéro.
                    system: control2.IsSystemSoundsSession().0 == 0,
                    peak: control.cast::<IAudioMeterInformation>().and_then(|meter| meter.GetPeakValue()).ok(),
                });
            }
            Ok(sessions)
        })()
    };
    result.unwrap_or_else(|e| {
        problems.push(format!("appareil d'{label} illisible ({e})"));
        Vec::new()
    })
}

fn read_all(problems: &mut Vec<String>) -> Vec<Session> {
    let _com = ComScope::enter();
    let mut sessions = read_sessions(eRender, problems);
    sessions.extend(read_sessions(eCapture, problems));
    sessions
}

fn read_processes() -> Vec<AudioProcess> {
    let sessions = read_all(&mut Vec::new());
    let mut pids: Vec<u32> = sessions.iter().filter(|s| s.counts()).map(|s| s.pid).collect();
    pids.sort_unstable();
    pids.dedup();
    pids.into_iter()
        .map(|pid| {
            let mine = sessions.iter().filter(|s| s.counts() && s.pid == pid);
            AudioProcess {
                pid: i64::from(pid),
                bundle: sessions.iter().find(|s| s.pid == pid).map(|s| s.name.clone()).unwrap_or_default(),
                output: mine.clone().any(|s| !s.capture),
                input: mine.into_iter().any(|s| s.capture),
            }
        })
        .collect()
}

fn describe_sessions() -> String {
    let mut problems = Vec::new();
    let sessions = read_all(&mut problems);
    let mut parts: Vec<String> = sessions.iter().map(Session::describe).collect();
    parts.extend(problems);
    if parts.is_empty() {
        "Windows ne voit aucune session audio".into()
    } else {
        format!("vu par Windows : {}", parts.join(" ; "))
    }
}

/// Les fenêtres de premier niveau visibles d'un processus (celles qu'un lecteur a à l'écran, sans les fenêtres filles).
fn top_windows(pid: u32) -> Vec<HWND> {
    struct Search {
        pid: u32,
        found: Vec<HWND>,
    }
    unsafe extern "system" fn visit(window: HWND, data: LPARAM) -> BOOL {
        // SAFETY : `data` est l'adresse du `Search` posé sur la pile de `top_windows`, vivant pendant toute l'énumération.
        let search = unsafe { &mut *(data as *mut Search) };
        let mut owner_pid = 0u32;
        // SAFETY : fenêtre donnée par l'énumération ; lecture seule.
        let visible = unsafe {
            GetWindowThreadProcessId(window, &mut owner_pid);
            IsWindowVisible(window) != 0 && GetWindow(window, GW_OWNER).is_null()
        };
        if owner_pid == search.pid && visible {
            search.found.push(window);
        }
        1
    }
    let mut search = Search { pid, found: Vec::new() };
    // SAFETY : la fonction de rappel ne sort pas de cet appel ; `search` reste valide jusqu'à son retour.
    unsafe {
        EnumWindows(Some(visit), &mut search as *mut Search as LPARAM);
    }
    search.found
}

/// Envoie « Lecture/Pause » à la première fenêtre du processus (la commande multimédia de Windows, que VLC et d'autres lecteurs de
/// bureau comprennent) ; faux si le processus n'a pas de fenêtre à lui (l'audio d'un navigateur vit dans un processus sans fenêtre).
fn send_app_command(pid: u32) -> bool {
    let Some(window) = top_windows(pid).into_iter().next() else { return false };
    let mut result = 0usize;
    // SAFETY : envoi d'un message à une fenêtre existante, avec délai maximal (un programme figé ne bloque pas la dictée).
    let sent = unsafe {
        SendMessageTimeoutW(window, WM_APPCOMMAND, window as usize, APPCOMMAND_MEDIA_PLAY_PAUSE << 16, SMTO_ABORTIFHUNG, 500, &mut result)
    };
    sent != 0
}

fn key(up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT { wVk: VK_MEDIA_PLAY_PAUSE, wScan: 0, dwFlags: if up { KEYEVENTF_KEYUP } else { 0 }, time: 0, dwExtraInfo: 0 },
        },
    }
}

impl AudioSource for WasapiSessions {
    fn processes(&self) -> Vec<AudioProcess> {
        // Un fil à part : COM est initialisé et libéré à l'intérieur, sans dépendre du fil qui appelle.
        std::thread::spawn(read_processes).join().unwrap_or_default()
    }

    fn play_pause(&self) {
        let inputs = [key(false), key(true)];
        // SAFETY : tableau d'entrées valide pour la durée de l'appel.
        unsafe {
            SendInput(inputs.len() as u32, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32);
        }
    }

    fn play_pause_for(&self, players: &[AudioProcess]) -> String {
        let mut targeted = Vec::new();
        let mut keyboard = false;
        for player in players {
            if player.pid > 0 && send_app_command(player.pid as u32) {
                targeted.push(player.bundle.clone());
            } else {
                keyboard = true;
            }
        }
        if keyboard {
            self.play_pause();
        }
        match (targeted.is_empty(), keyboard) {
            (true, _) => String::new(),
            (false, false) => format!(" — commande envoyée à la fenêtre de {}", targeted.join(", ")),
            (false, true) => format!(" — commande envoyée à la fenêtre de {}, touche du clavier pour les autres", targeted.join(", ")),
        }
    }

    fn ignored(&self) -> &'static [&'static str] {
        &IGNORED
    }

    fn describe(&self) -> String {
        std::thread::spawn(describe_sessions).join().unwrap_or_default()
    }
}
