//! « Choisir un fichier… » : la boîte d'ouverture standard de Windows (`GetOpenFileNameW`), pour désigner un modèle `.bin` ou `.gguf`.

use std::os::windows::ffi::OsStringExt;
use windows_sys::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
use windows_sys::Win32::UI::Controls::Dialogs::{
    GetOpenFileNameW, OFN_EXPLORER, OFN_FILEMUSTEXIST, OFN_NOCHANGEDIR, OFN_PATHMUSTEXIST, OPENFILENAMEW,
};

/// Ouvre la boîte et rend le chemin choisi (rien si on annule). `filter` : « Modèles Whisper (*.bin) » et « *.bin », par paires.
/// Bloque jusqu'à la fermeture de la boîte : à appeler depuis un fil de travail, jamais depuis celui de l'icône.
pub fn pick_file(title: &str, filter: &[(&str, &str)]) -> Option<String> {
    let wide = |text: &str| text.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
    let mut filters: Vec<u16> = Vec::new();
    for (label, pattern) in filter {
        filters.extend(label.encode_utf16().chain(std::iter::once(0)));
        filters.extend(pattern.encode_utf16().chain(std::iter::once(0)));
    }
    filters.push(0);
    let title = wide(title);
    let mut path = vec![0u16; 1024];
    // SAFETY : la structure est initialisée à zéro puis remplie ; les tampons vivent pendant tout l'appel (qui est bloquant). La boîte
    // moderne de Windows a besoin de COM sur le fil qui l'ouvre (un fil de requête n'en a pas encore).
    let chosen = unsafe {
        let com = CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32) >= 0;
        let mut dialog: OPENFILENAMEW = std::mem::zeroed();
        dialog.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        dialog.lpstrFilter = filters.as_ptr();
        dialog.lpstrFile = path.as_mut_ptr();
        dialog.nMaxFile = path.len() as u32;
        dialog.lpstrTitle = title.as_ptr();
        dialog.Flags = OFN_EXPLORER | OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR;
        let picked = GetOpenFileNameW(&mut dialog) != 0;
        if com {
            CoUninitialize();
        }
        picked
    };
    if !chosen {
        return None;
    }
    let length = path.iter().position(|c| *c == 0).unwrap_or(path.len());
    Some(std::ffi::OsString::from_wide(&path[..length]).to_string_lossy().into_owned())
}
