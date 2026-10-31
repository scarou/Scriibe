//! « Parcourir… » : la boîte d'ouverture de fichier de macOS, par `osascript` (`choose file`).

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_script_keeps_the_extensions_and_escapes_the_title() {
        let s = script("Choisir \"un\" modèle", &[("Modèles Whisper (*.bin)", "*.bin")]);
        assert_eq!(s, "POSIX path of (choose file with prompt \"Choisir \\\"un\\\" modèle\" of type {\"bin\"})");
        assert_eq!(script("Fichier", &[]), "POSIX path of (choose file with prompt \"Fichier\")");
    }
}
