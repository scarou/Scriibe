//! Clés d'API dans le trousseau de macOS (mots de passe génériques, service « Scriibe », compte « deepgram », « openrouter », « github »).

use crate::secrets::account_name;
use scriibe_engines::runtime::{KeyKind, Secrets};
use security_framework::passwords::{get_generic_password, set_generic_password};

const SERVICE: &str = "Scriibe";

pub struct KeychainSecrets;

impl Secrets for KeychainSecrets {
    fn get(&self, kind: KeyKind) -> Option<String> {
        let bytes = get_generic_password(SERVICE, account_name(kind)).ok()?;
        let value = String::from_utf8_lossy(&bytes).trim().to_string();
        (!value.is_empty()).then_some(value)
    }
}

/// Enregistre une clé (jamais affichée ni journalisée). macOS peut demander à l'utilisateur d'autoriser l'accès au trousseau.
pub fn set_credential(kind: KeyKind, value: &str) -> Result<(), String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("clé vide".into());
    }
    set_generic_password(SERVICE, account_name(kind), value.as_bytes())
        .map_err(|e| format!("le trousseau a refusé d'enregistrer la clé ({e})"))
}
