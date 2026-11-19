//! Windows : la reformulation « copiée seule » (`Paster::copy_transient`) et l'historique du presse-papiers (Win+V). Deux essais :
//! 1. **les marques** : le texte est dans le presse-papiers et porte les trois marques « ne pas garder » (`ExcludeClipboardContentFromMonitorProcessing`,
//!    `CanIncludeInClipboardHistory`, `CanUploadToCloudClipboard`, toutes à 0) ; une copie ordinaire n'en porte aucune ;
//! 2. **l'historique** : interrogé par WinRT (`Clipboard.GetHistoryItemsAsync`, via PowerShell), le texte copié ordinairement y entre, le texte
//!    copié seul n'y entre pas. Ce second essai demande une session ouverte et l'historique activé (Paramètres → Système → Presse-papiers) ;
//!    sinon il le dit et ne conclut pas (jamais de réglage du système modifié par un test).
//!
//! Sans session interactive (SSH, service) le presse-papiers n'est pas joignable : l'essai se saute en le disant. Lancement dans la session de
//! l'auteur : tâche planifiée temporaire (voir `docs/ENQUETE_WINDOWS.md` § Correctifs). Les deux essais partagent le presse-papiers : un seul test.
#![cfg(windows)]

use std::process::Command;
use std::time::Duration;
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard, RegisterClipboardFormatW,
};
use windows_sys::Win32::System::Memory::{GlobalLock, GlobalUnlock};

const MARKS: [&str; 3] = ["ExcludeClipboardContentFromMonitorProcessing", "CanIncludeInClipboardHistory", "CanUploadToCloudClipboard"];

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// La marque `name` du presse-papiers : `None` si absente, sinon sa valeur (un entier de 4 octets).
fn mark(name: &str) -> Option<u32> {
    let format = unsafe { RegisterClipboardFormatW(wide(name).as_ptr()) };
    for _ in 0..20 {
        if unsafe { OpenClipboard(std::ptr::null_mut()) } != 0 {
            let value = unsafe {
                let mut found = None;
                if IsClipboardFormatAvailable(format) != 0 {
                    let handle = GetClipboardData(format);
                    if !handle.is_null() {
                        let pointer = GlobalLock(handle) as *const u32;
                        if !pointer.is_null() {
                            found = Some(pointer.read_unaligned());
                            GlobalUnlock(handle);
                        }
                    }
                }
                CloseClipboard();
                found
            };
            return value;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    None
}

/// Le texte actuel du presse-papiers (format Unicode).
fn clipboard_text() -> Option<String> {
    const CF_UNICODETEXT: u32 = 13;
    for _ in 0..20 {
        if unsafe { OpenClipboard(std::ptr::null_mut()) } != 0 {
            let text = unsafe {
                let mut found = None;
                if IsClipboardFormatAvailable(CF_UNICODETEXT) != 0 {
                    let handle = GetClipboardData(CF_UNICODETEXT);
                    if !handle.is_null() {
                        let pointer = GlobalLock(handle) as *const u16;
                        if !pointer.is_null() {
                            let mut length = 0;
                            while *pointer.add(length) != 0 {
                                length += 1;
                            }
                            found = Some(String::from_utf16_lossy(std::slice::from_raw_parts(pointer, length)));
                            GlobalUnlock(handle);
                        }
                    }
                }
                CloseClipboard();
                found
            };
            return text;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    None
}

enum History {
    /// L'historique est activé : les textes qu'il garde.
    Items(Vec<String>),
    /// L'historique n'est pas joignable (désactivé, pas de session…) : la raison.
    Unavailable(String),
}

const HISTORY_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
try {
  Add-Type -AssemblyName System.Runtime.WindowsRuntime
  $null = [Windows.ApplicationModel.DataTransfer.Clipboard, Windows.ApplicationModel.DataTransfer, ContentType = WindowsRuntime]
  $null = [Windows.ApplicationModel.DataTransfer.ClipboardHistoryItemsResult, Windows.ApplicationModel.DataTransfer, ContentType = WindowsRuntime]
  if (-not [Windows.ApplicationModel.DataTransfer.Clipboard]::IsHistoryEnabled()) { 'ETAT:desactive'; exit 0 }
  $as = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' })[0]
  function Await($op, [Type]$t) { $task = $as.MakeGenericMethod($t).Invoke($null, @($op)); $task.Wait(8000) | Out-Null; $task.Result }
  $r = Await ([Windows.ApplicationModel.DataTransfer.Clipboard]::GetHistoryItemsAsync()) ([Windows.ApplicationModel.DataTransfer.ClipboardHistoryItemsResult])
  'ETAT:ok'
  foreach ($i in $r.Items) { if ($i.Content.Contains('Text')) { 'ITEM:' + (Await ($i.Content.GetTextAsync()) ([string])) } }
} catch { 'ETAT:erreur:' + $_.Exception.Message }
"#;

fn history() -> History {
    let script = std::env::temp_dir().join(format!("scriibe-winv-{}.ps1", std::process::id()));
    if std::fs::write(&script, HISTORY_SCRIPT).is_err() {
        return History::Unavailable("script d'interrogation non écrit".into());
    }
    let output = Command::new("powershell").args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"]).arg(&script).output();
    let _ = std::fs::remove_file(&script);
    let Ok(output) = output else { return History::Unavailable("PowerShell introuvable".into()) };
    let text = String::from_utf8_lossy(&output.stdout).to_string();
    let mut lines = text.lines().map(str::trim);
    match lines.next() {
        Some("ETAT:ok") => History::Items(lines.filter_map(|l| l.strip_prefix("ITEM:").map(str::to_string)).collect()),
        Some("ETAT:desactive") => {
            History::Unavailable("l'historique du presse-papiers est désactivé (Paramètres → Système → Presse-papiers)".into())
        }
        Some(other) => History::Unavailable(other.trim_start_matches("ETAT:").to_string()),
        None => History::Unavailable("aucune réponse".into()),
    }
}

#[test]
fn a_reformulation_copied_alone_carries_the_marks_and_stays_out_of_win_v() {
    let paster = scriibe_platform::windows::platform().paster;
    let id = std::process::id();
    let ordinary = format!("SCRIIBE-ESSAI-WINV-{id}-ordinaire");
    let alone = format!("SCRIIBE-ESSAI-WINV-{id}-copie-seule");

    if !paster.copy(&ordinary) {
        eprintln!("essai Win+V sauté : le presse-papiers n'est pas joignable (pas de session interactive ?)");
        return;
    }

    // 1. Les marques. Une copie ordinaire n'en porte aucune.
    assert_eq!(clipboard_text().as_deref(), Some(ordinary.as_str()));
    for name in MARKS {
        assert_eq!(mark(name), None, "une copie ordinaire ne doit pas porter {name}");
    }
    // Le texte copié seul est dans le presse-papiers, avec les trois marques à 0.
    assert!(paster.copy_transient(&alone));
    assert_eq!(clipboard_text().as_deref(), Some(alone.as_str()), "le texte copié seul doit être dans le presse-papiers");
    for name in MARKS {
        assert_eq!(mark(name), Some(0), "le texte copié seul doit porter {name} à 0");
    }

    // 2. L'historique : l'ordinaire y entre, le copié seul n'y entre pas. L'historique de Windows se met à jour avec un léger retard.
    assert!(paster.copy(&ordinary));
    std::thread::sleep(Duration::from_millis(1500));
    assert!(paster.copy_transient(&alone));
    std::thread::sleep(Duration::from_millis(2500));
    match history() {
        History::Unavailable(reason) => {
            eprintln!("historique Win+V non vérifié : {reason} (les marques, elles, sont vérifiées)");
        }
        History::Items(items) => {
            if !items.iter().any(|item| item == &ordinary) {
                eprintln!(
                    "historique Win+V non concluant : le texte copié ordinairement n'y est pas ({} éléments lus) ; le témoin manque, rien n'est conclu",
                    items.len()
                );
            } else {
                assert!(!items.iter().any(|item| item == &alone), "le texte copié seul est entré dans l'historique Win+V");
                eprintln!(
                    "historique Win+V vérifié : le texte ordinaire y est, le texte copié seul n'y est pas ({} éléments lus)",
                    items.len()
                );
            }
        }
    }
    // Ne laisse pas nos textes d'essai dans le presse-papiers.
    paster.copy("");
}
