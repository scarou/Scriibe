//! Le chef d'orchestre de Scriibe : touche maintenue → micro → transcription → corrections → mise au propre (ou texte brut) →
//! collage → historique. Port de `DictationController.swift` (Scribe), sans le banc de dictée ni le test de frappe (étape 6).
//!
//! Tout ce qui touche le système passe par les traits de `scriibe-platform` ; les moteurs par `scriibe-engines`. Les tests utilisent
//! les faux de `scriibe_platform::testing` et le monde simulé `FakeWorld`.

mod controller;
mod executor;

pub use controller::{CommandSink, Controller, Deps, KeyCapture};
pub use executor::{Executor, InlineExecutor, ThreadExecutor};
