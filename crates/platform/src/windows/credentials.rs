//! Clés d'API dans le Gestionnaire d'identifiants de Windows (identifiants génériques « Scriibe/deepgram », « Scriibe/openrouter »).

use super::wide;
use crate::secrets::account_name;
use scriibe_engines::runtime::{KeyKind, Secrets};
use windows_sys::Win32::Security::Credentials::{
    CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
};

fn target(kind: KeyKind) -> String {
    format!("Scriibe/{}", account_name(kind))
}

pub struct CredentialSecrets;

impl Secrets for CredentialSecrets {
    fn get(&self, kind: KeyKind) -> Option<String> {
        let name = wide(&target(kind));
        let mut credential: *mut CREDENTIALW = std::ptr::null_mut();
        // SAFETY : `credential` est rempli par Windows et libéré avec CredFree ; le blob est copié avant.
        unsafe {
            if CredReadW(name.as_ptr(), CRED_TYPE_GENERIC, 0, &mut credential) == 0 || credential.is_null() {
                return None;
            }
            let c = &*credential;
            let bytes = if c.CredentialBlob.is_null() {
                &[][..]
            } else {
                std::slice::from_raw_parts(c.CredentialBlob, c.CredentialBlobSize as usize)
            };
            let value = String::from_utf8_lossy(bytes).trim().to_string();
            CredFree(credential as *const _);
            (!value.is_empty()).then_some(value)
        }
    }
}

/// Enregistre une clé (jamais affichée ni journalisée).
pub fn set_credential(kind: KeyKind, value: &str) -> Result<(), String> {
    let mut name = wide(&target(kind));
    let mut user = wide("Scriibe");
    let mut blob = value.trim().as_bytes().to_vec();
    if blob.is_empty() {
        return Err("clé vide".into());
    }
    // SAFETY : la structure est remplie champ par champ et ne sert que pendant l'appel.
    unsafe {
        let mut credential: CREDENTIALW = std::mem::zeroed();
        credential.Type = CRED_TYPE_GENERIC;
        credential.TargetName = name.as_mut_ptr();
        credential.UserName = user.as_mut_ptr();
        credential.CredentialBlobSize = blob.len() as u32;
        credential.CredentialBlob = blob.as_mut_ptr();
        credential.Persist = CRED_PERSIST_LOCAL_MACHINE;
        if CredWriteW(&credential, 0) == 0 {
            return Err(format!("Windows a refusé d'enregistrer la clé (code {})", windows_sys::Win32::Foundation::GetLastError()));
        }
    }
    Ok(())
}
