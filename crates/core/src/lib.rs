//! Logique de texte de Scriibe : signes dictés, listes, typographie, garde-fous, modes.
//!
//! Aucune entrée-sortie, aucune dépendance système : des fonctions de texte et des structures de données.
//! Le comportement de référence est celui de Scribe (macOS, Swift) ; les cas de `fixtures/` en sont tirés.

pub mod dictionary;
pub mod spoken_signs;
pub mod text;
pub mod typography;

mod re;
