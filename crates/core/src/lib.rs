//! Logique de texte de Scriibe : signes dictés, listes, typographie, garde-fous, modes.
//!
//! Aucune entrée-sortie, aucune dépendance système : des fonctions de texte et des structures de données.
//! Le comportement de référence est celui de Scribe (macOS, Swift) ; les cas de `fixtures/` en sont tirés.

pub mod bench;
pub mod bench_archive;
pub mod cleaner;
pub mod dictionary;
pub mod history;
pub mod hotkey;
pub mod lang;
pub mod media_pause;
pub mod microphone;
pub mod modes;
mod prompt_text;
pub mod retention;
pub mod settings;
pub mod spoken_signs;
pub mod text;
pub mod translation;
pub mod typography;

mod re;
