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
use windows_sys::Win32::Foundation::CloseHandle;
use windows_sys::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_MEDIA_PLAY_PAUSE,
};

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

/// Les processus ayant une session active sur l'appareil par défaut ; pour la lecture, seulement ceux dont le son passe.
fn active_pids(flow: EDataFlow) -> windows::core::Result<Vec<u32>> {
    // SAFETY : appels COM sur un fil où COM est initialisé (`ComScope`) ; chaque interface est libérée à sa sortie de portée.
    unsafe {
        let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device = enumerator.GetDefaultAudioEndpoint(flow, eConsole)?;
        let manager: IAudioSessionManager2 = device.Activate(CLSCTX_ALL, None)?;
        let sessions = manager.GetSessionEnumerator()?;
        let mut pids = Vec::new();
        for i in 0..sessions.GetCount()? {
            let control = sessions.GetSession(i)?;
            if control.GetState()? != AudioSessionStateActive {
                continue;
            }
            let control2: IAudioSessionControl2 = control.cast()?;
            // Les sons du système (bips, alertes) : la session « sons système » n'est pas un média. Réponse S_OK (0) = oui,
            // S_FALSE (1) = non : `is_ok()` vaudrait vrai dans les deux cas, d'où la comparaison avec zéro.
            if control2.IsSystemSoundsSession().0 == 0 {
                continue;
            }
            let pid = control2.GetProcessId()?;
            if pid == 0 {
                continue;
            }
            if flow == eRender {
                let meter: IAudioMeterInformation = control.cast()?;
                if meter.GetPeakValue()? < AUDIBLE {
                    continue;
                }
            }
            pids.push(pid);
        }
        Ok(pids)
    }
}

fn read_processes() -> Vec<AudioProcess> {
    let _com = ComScope::enter();
    let output = active_pids(eRender).unwrap_or_default();
    let input = active_pids(eCapture).unwrap_or_default();
    let mut all: Vec<u32> = output.iter().chain(input.iter()).copied().collect();
    all.sort_unstable();
    all.dedup();
    all.into_iter()
        .map(|pid| AudioProcess {
            pid: i64::from(pid),
            bundle: executable_name(pid),
            output: output.contains(&pid),
            input: input.contains(&pid),
        })
        .collect()
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

    fn ignored(&self) -> &'static [&'static str] {
        &IGNORED
    }
}
