//! L'appli au premier plan : nom de l'exécutable de la fenêtre active.

use crate::apps::{AppInfo, ForegroundApp};
use windows_sys::Win32::Foundation::CloseHandle;
use windows_sys::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION};
use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

pub struct WindowsForeground;

impl ForegroundApp for WindowsForeground {
    fn current(&self) -> AppInfo {
        // SAFETY : appels de lecture ; la poignée du processus est fermée avant de rendre la main.
        let path = unsafe {
            let window = GetForegroundWindow();
            if window.is_null() {
                return AppInfo::unknown();
            }
            let mut pid = 0u32;
            GetWindowThreadProcessId(window, &mut pid);
            let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if process.is_null() {
                return AppInfo::unknown();
            }
            let mut buffer = vec![0u16; 1024];
            let mut size = buffer.len() as u32;
            let ok = QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut size);
            CloseHandle(process);
            if ok == 0 {
                return AppInfo::unknown();
            }
            String::from_utf16_lossy(&buffer[..size as usize])
        };
        let file = scriibe_engines::file_name(&path).to_string();
        let name = file.strip_suffix(".exe").or_else(|| file.strip_suffix(".EXE")).unwrap_or(&file).to_string();
        if name.is_empty() {
            return AppInfo::unknown();
        }
        AppInfo { id: Some(file.to_lowercase()), name }
    }
}
