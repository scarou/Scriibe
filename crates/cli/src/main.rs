//! `scriibe` : la dictée, avec une icône dans la zone de notification (Windows) ou la barre des menus (macOS) et une fenêtre.
//!
//! Sous-commandes (sans argument : `lancer`) :
//! - `verifier` : ce qui est installé, trouvé, manquant (moteurs, modèles, micro, clés) ;
//! - `micro [secondes]` : enregistre le micro et dit si le niveau convient ;
//! - `essai [fichier.wav] [--traduire]` : transcrit un fichier (ou la dernière prise du micro) avec la vraie chaîne, sans toucher au clavier ;
//! - `cle deepgram|openrouter|github` : enregistre une clé d'API dans le Gestionnaire d'identifiants (lue sur l'entrée standard) ;
//! - `lancer` : la dictée, touche maintenue (Ctrl droit sous Windows), icône dans la zone de notification et page d'essais dans le
//!   navigateur (téléchargements, micro, phrases du banc, rapport) ; c'est ce que fait un double-clic.

mod app;
mod assistant;
mod commands;
#[cfg(any(target_os = "macos", target_os = "linux"))]
mod mac_tray;
mod overlay;
mod tray_source;
mod web;
#[cfg(any(windows, target_os = "macos", target_os = "linux"))]
mod window;

use std::process::ExitCode;

const HELP: &str = "Scriibe — dictée vocale\n\n\
Usage : scriibe [commande]\n\n\
  lancer                     la dictée (maintiens la touche de dictée, parle, relâche) + la fenêtre de Scriibe (par défaut ; --navigateur : dans le navigateur ; --sans-navigateur : sans rien ouvrir)\n\
  verifier                   ce qui est installé, trouvé ou manquant\n\
  micro [secondes]           enregistre le micro (3 s par défaut) et mesure le niveau\n\
  essai [fichier.wav] [--traduire]\n\
                             transcrit un fichier WAV (ou la dernière prise) avec la vraie chaîne, sans coller\n\
  cle deepgram|openrouter|github
                             enregistre une clé d'API (lue sur l'entrée standard, jamais affichée ensuite)\n\
  aide                       ce message\n";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        None | Some("lancer") => commands::run(),
        // Relance après une mise à jour d'un programme lancé sans commande.
        Some(flag) if flag == commands::RESTARTED => commands::run(),
        Some("verifier") => commands::check(),
        Some("micro") => commands::microphone(args.get(1).and_then(|s| s.parse().ok()).unwrap_or(3.0)),
        Some("essai") => commands::trial(&args[1..]),
        Some("cle") => commands::key(args.get(1).map(String::as_str)),
        Some("aide" | "--aide" | "-h" | "--help" | "help") => {
            print!("{HELP}");
            Ok(())
        }
        Some(other) => Err(format!("commande inconnue : « {other} »\n\n{HELP}")),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}
