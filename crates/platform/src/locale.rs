//! La langue du système, pour l'interface en « automatique » (voir `scriibe_core::settings::UiLanguage::resolve`).

/// Le nom de la langue de l'utilisateur (« fr-FR », « en_US.UTF-8 »…), vide si on ne la connaît pas.
pub fn system_language() -> String {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Globalization::GetUserDefaultLocaleName;
        let mut buffer = [0u16; 85];
        // SAFETY: le tampon fait 85 caractères, la taille maximale d'un nom de langue (LOCALE_NAME_MAX_LENGTH) ; la longueur rendue les compte tous.
        let length = unsafe { GetUserDefaultLocaleName(buffer.as_mut_ptr(), buffer.len() as i32) };
        if length > 1 {
            return String::from_utf16_lossy(&buffer[..(length - 1) as usize]);
        }
        String::new()
    }
    #[cfg(not(windows))]
    {
        ["LC_ALL", "LC_MESSAGES", "LANG"].iter().filter_map(|name| std::env::var(name).ok()).find(|v| !v.is_empty()).unwrap_or_default()
    }
}
