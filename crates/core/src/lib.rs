//! Logique de texte de Scriibe : signes dictés, listes, typographie, garde-fous, modes.
//!
//! Aucune entrée-sortie, aucune dépendance système : des fonctions de texte et des structures de données.
//! Le comportement de référence est celui de Scribe (macOS, Swift) ; les cas de `fixtures/` en sont tirés.

pub mod backup;
pub mod bench;
pub mod bench_archive;
pub mod clean_test;
pub mod cleaner;
pub mod commands;
pub mod dictionary;
pub mod export;
pub mod history;
pub mod hotkey;
pub mod import;
pub mod import_commands;
pub mod import_dictionary;
pub mod inline;
pub mod lang;
pub mod media_pause;
pub mod microphone;
pub mod modes;
mod prompt_text;
pub mod retention;
pub mod settings;
pub mod shortcut;
pub mod spacing;
pub mod spoken_signs;
pub mod text;
pub mod translation;
pub mod typography;

mod re;
