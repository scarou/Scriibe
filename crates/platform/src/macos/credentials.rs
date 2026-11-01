//! Clés d'API dans le trousseau de macOS (mots de passe génériques, service « Scriibe », compte « deepgram », « openrouter », « github »).

use crate::secrets::account_name;
use scriibe_engines::runtime::{KeyKind, Secrets};
use security_framework::passwords::{delete_generic_password, get_generic_password, set_generic_password};
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

const SERVICE: &str = "Scriibe";

pub struct KeychainSecrets;

/// Ce que le trousseau a déjà répondu pendant ce lancement. Chaque lecture peut faire afficher à macOS une demande d'accès (tant que
/// Scriibe n'a pas de signature stable) : une seule par clé et par lancement, pas une à chaque rafraîchissement de la page.
static READ: Mutex<Option<HashMap<&'static str, Option<String>>>> = Mutex::new(None);

fn cache() -> MutexGuard<'static, Option<HashMap<&'static str, Option<String>>>> {
    READ.lock().unwrap_or_else(|e| e.into_inner())
}

impl Secrets for KeychainSecrets {
    fn get(&self, kind: KeyKind) -> Option<String> {
        let account = account_name(kind);
        if let Some(known) = cache().get_or_insert_with(HashMap::new).get(account) {
            return known.clone();
        }
        let value = get_generic_password(SERVICE, account)
            .ok()
            .map(|bytes| String::from_utf8_lossy(&bytes).trim().to_string())
            .filter(|v| !v.is_empty());
        cache().get_or_insert_with(HashMap::new).insert(account, value.clone());
        value
    }
}

/// Enregistre une clé (jamais affichée ni journalisée). macOS peut demander à l'utilisateur d'autoriser l'accès au trousseau.
pub fn set_credential(kind: KeyKind, value: &str) -> Result<(), String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("clé vide".into());
    }
    // Comme Scribe : supprimer puis recréer, pour que l'élément appartienne à la signature actuelle de l'appli (une mise à jour sur
    // place garde la liste d'accès de celui qui l'avait créé, par exemple un programme lancé depuis le Terminal).
    let _ = delete_generic_password(SERVICE, account_name(kind));
    set_generic_password(SERVICE, account_name(kind), value.as_bytes())
        .map_err(|e| format!("le trousseau a refusé d'enregistrer la clé ({e})"))?;
    cache().get_or_insert_with(HashMap::new).insert(account_name(kind), Some(value.to_string()));
    Ok(())
}
