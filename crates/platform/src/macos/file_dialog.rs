//! « Parcourir… » et « Enregistrer sous… » : les boîtes de fichier de macOS, par `osascript` (`choose file`, `choose file name`).

use std::process::Command;

/// Le script AppleScript de la boîte : `filter` comme sous Windows (« Modèles Whisper (*.bin) », « *.bin »), dont on garde les extensions.
fn script(title: &str, filter: &[(&str, &str)]) -> String {
    let extensions: Vec<String> = filter
        .iter()
        .flat_map(|(_, pattern)| pattern.split(';'))
        .filter_map(|pattern| pattern.trim().strip_prefix("*."))
        .map(|extension| format!("\"{}\"", extension.replace('"', "")))
        .collect();
    let types = if extensions.is_empty() { String::new() } else { format!(" of type {{{}}}", extensions.join(", ")) };
    let prompt = title.replace('\\', "\\\\").replace('"', "\\\"");
    format!("POSIX path of (choose file with prompt \"{prompt}\"{types})")
}

/// Le chemin choisi, ou rien si l'utilisateur annule.
pub fn pick_file(title: &str, filter: &[(&str, &str)]) -> Option<String> {
    let output = Command::new("osascript").arg("-e").arg(script(title, filter)).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!path.is_empty()).then_some(path)
}

/// Le script de la boîte « Enregistrer sous » : un nom proposé, que l'utilisateur change et dont il choisit le dossier.
fn save_script(title: &str, default_name: &str) -> String {
    let escape = |text: &str| text.replace('\\', "\\\\").replace('"', "\\\"");
    format!("POSIX path of (choose file name with prompt \"{}\" default name \"{}\")", escape(title), escape(default_name))
}

/// Le chemin choisi pour enregistrer (macOS demande lui-même confirmation avant d'écraser), ou rien si l'utilisateur annule.
pub fn pick_save(title: &str, default_name: &str) -> Option<String> {
    let output = Command::new("osascript").arg("-e").arg(save_script(title, default_name)).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!path.is_empty()).then_some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_script_keeps_the_extensions_and_escapes_the_title() {
        let s = script("Choisir \"un\" modèle", &[("Modèles Whisper (*.bin)", "*.bin")]);
        assert_eq!(s, "POSIX path of (choose file with prompt \"Choisir \\\"un\\\" modèle\" of type {\"bin\"})");
        assert_eq!(script("Fichier", &[]), "POSIX path of (choose file with prompt \"Fichier\")");
    }

    #[test]
    fn the_save_script_proposes_a_name_and_escapes_quotes() {
        assert_eq!(
            save_script("Enregistrer \"ce\" mode", "scriibe-mode-sms.scriibe.json"),
            "POSIX path of (choose file name with prompt \"Enregistrer \\\"ce\\\" mode\" default name \"scriibe-mode-sms.scriibe.json\")"
        );
    }
}
