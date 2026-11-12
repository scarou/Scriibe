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

/// Le résultat d'`osascript` : le chemin choisi, `Ok(None)` si l'utilisateur annule (code -128 de macOS, « User canceled »), sinon une
/// erreur (osascript absent, refusé par macOS…) : un échec ne doit pas passer pour une annulation.
fn classify(success: bool, stdout: &str, stderr: &str) -> Result<Option<String>, String> {
    if success {
        let path = stdout.trim().to_string();
        return Ok((!path.is_empty()).then_some(path));
    }
    if stderr.contains("-128") {
        return Ok(None);
    }
    Err("la boîte de choix de fichier n'a pas pu s'ouvrir".into())
}

fn run(script: &str) -> Result<Option<String>, String> {
    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .map_err(|_| "la boîte de choix de fichier n'a pas pu s'ouvrir".to_string())?;
    classify(output.status.success(), &String::from_utf8_lossy(&output.stdout), &String::from_utf8_lossy(&output.stderr))
}

/// Le chemin choisi, ou rien si l'utilisateur annule (ou si la boîte n'a pas pu s'ouvrir).
pub fn pick_file(title: &str, filter: &[(&str, &str)]) -> Option<String> {
    try_pick_file(title, filter).ok().flatten()
}

/// Comme `pick_file`, mais distingue une annulation (`Ok(None)`) d'un échec (`Err`).
pub fn try_pick_file(title: &str, filter: &[(&str, &str)]) -> Result<Option<String>, String> {
    run(&script(title, filter))
}

/// Le script de la boîte « Enregistrer sous » : un nom proposé, que l'utilisateur change et dont il choisit le dossier.
fn save_script(title: &str, default_name: &str) -> String {
    let escape = |text: &str| text.replace('\\', "\\\\").replace('"', "\\\"");
    format!("POSIX path of (choose file name with prompt \"{}\" default name \"{}\")", escape(title), escape(default_name))
}

/// Le chemin choisi pour enregistrer (macOS demande lui-même confirmation avant d'écraser) ; `Ok(None)` si l'utilisateur annule.
pub fn pick_save(title: &str, default_name: &str) -> Result<Option<String>, String> {
    run(&save_script(title, default_name))
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

    #[test]
    fn a_cancel_is_told_apart_from_a_failure() {
        assert_eq!(classify(true, "/Users/x/m.scriibe.json\n", ""), Ok(Some("/Users/x/m.scriibe.json".to_string())));
        assert_eq!(classify(false, "", "execution error: User canceled. (-128)"), Ok(None));
        assert!(classify(false, "", "osascript: not allowed").is_err());
    }
}
