//! Clés d'API dans le trousseau du bureau sous Linux (Secret Service : Trousseau de GNOME, KWallet…), par l'outil `secret-tool`
//! (paquet `libsecret-tools`) : aucune dépendance de compilation, aucun module de plus à auditer pour un secret.
//!
//! Règles (chantier 6, tâche 1) :
//! - la clé passe par l'**entrée standard** de `secret-tool`, jamais par ses arguments (visibles dans `ps`) ;
//! - le message d'erreur de l'outil n'est **jamais lu** (sa sortie d'erreur part à `/dev/null`) : seuls le code de sortie et la nature de
//!   l'échec remontent, donc aucune trace de la clé dans un message ni dans le journal ;
//! - sans trousseau (outil absent, bus de session absent, trousseau fermé), **rien n'est écrit nulle part** : l'erreur dit de garder la
//!   variable d'environnement (`OPENROUTER_API_KEY`, `DEEPGRAM_API_KEY`).
//!
//! Le code est compilé sur tous les systèmes pour être testé avec un trousseau factice ; seul Linux l'utilise.
#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

use crate::secrets::{account_name, env_name};
use scriibe_engines::runtime::{KeyKind, Secrets};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

const SERVICE: &str = "Scriibe";
const TOOL: &str = "secret-tool";
/// Une lecture qui n'a rien donné (clé absente, trousseau fermé ou absent) n'est pas refaite avant ce délai : une demande de
/// déverrouillage ne doit pas revenir à chaque rafraîchissement de la page.
const ABSENT_FOR: Duration = Duration::from_secs(30);
/// Lire : un trousseau fermé peut afficher une demande de mot de passe ; enregistrer : on laisse le temps de la remplir.
const READ_TIMEOUT: Duration = Duration::from_secs(8);
const WRITE_TIMEOUT: Duration = Duration::from_secs(60);
/// Une clé d'API fait quelques dizaines de signes.
const MAX_KEY_LEN: usize = 4096;

/// Ce que renvoie l'outil : le code de sortie et ce qu'il a écrit en sortie standard (la clé, pour une lecture). Jamais sa sortie d'erreur.
pub struct ToolOutput {
    pub code: Option<i32>,
    pub stdout: String,
}

pub enum ToolError {
    /// `secret-tool` n'est pas installé.
    Missing,
    /// Pas de réponse dans le délai (trousseau fermé, demande de mot de passe laissée sans réponse).
    TimedOut,
    /// Lancement ou échange impossible.
    Io,
}

/// L'outil `secret-tool`, ou son remplaçant dans les tests.
pub trait SecretTool: Send + Sync {
    fn run(&self, args: &[&str], stdin: Option<&str>, timeout: Duration) -> Result<ToolOutput, ToolError>;
}

/// Le vrai `secret-tool`.
pub struct CommandTool;

impl SecretTool for CommandTool {
    fn run(&self, args: &[&str], stdin: Option<&str>, timeout: Duration) -> Result<ToolOutput, ToolError> {
        let mut child = Command::new(TOOL)
            .args(args)
            .stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::null() })
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| if e.kind() == std::io::ErrorKind::NotFound { ToolError::Missing } else { ToolError::Io })?;
        if let (Some(text), Some(mut pipe)) = (stdin, child.stdin.take()) {
            // `secret-tool store` lit jusqu'à la fin de l'entrée : pas de retour à la ligne ajouté, il serait gardé avec la clé.
            if pipe.write_all(text.as_bytes()).is_err() {
                let _ = child.kill();
                let _ = child.wait();
                return Err(ToolError::Io);
            }
        }
        let began = Instant::now();
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if began.elapsed() >= timeout => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(ToolError::TimedOut);
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(20)),
                Err(_) => return Err(ToolError::Io),
            }
        };
        let mut stdout = String::new();
        if let Some(mut out) = child.stdout.take() {
            let _ = out.by_ref().take(MAX_KEY_LEN as u64 * 2).read_to_string(&mut stdout);
        }
        Ok(ToolOutput { code: status.code(), stdout })
    }
}

enum Known {
    Found(String),
    Absent(Instant),
}

pub struct ServiceSecrets<T: SecretTool> {
    tool: T,
    absent_for: Duration,
    known: Mutex<HashMap<&'static str, Known>>,
}

impl<T: SecretTool> ServiceSecrets<T> {
    pub fn new(tool: T) -> Self {
        Self::with_absent_for(tool, ABSENT_FOR)
    }

    pub fn with_absent_for(tool: T, absent_for: Duration) -> Self {
        Self { tool, absent_for, known: Mutex::new(HashMap::new()) }
    }

    fn known(&self) -> MutexGuard<'_, HashMap<&'static str, Known>> {
        self.known.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn lookup(&self, account: &str) -> Result<Option<String>, ToolError> {
        let out = self.tool.run(&["lookup", "service", SERVICE, "account", account], None, READ_TIMEOUT)?;
        let value = out.stdout.trim().to_string();
        Ok((out.code == Some(0) && !value.is_empty()).then_some(value))
    }

    /// Enregistre la clé dans le trousseau. Rien n'est écrit ailleurs, quoi qu'il arrive ; l'erreur ne contient jamais la clé.
    pub fn store(&self, kind: KeyKind, value: &str) -> Result<(), String> {
        let value = value.trim();
        if value.is_empty() {
            return Err("clé vide".into());
        }
        if value.len() > MAX_KEY_LEN || value.chars().any(char::is_control) {
            return Err("cette clé n'est pas lisible : une clé d'API tient sur une ligne, sans signe de commande".into());
        }
        let account = account_name(kind);
        let label = format!("{SERVICE} {account}");
        let args = ["store", "--label", label.as_str(), "service", SERVICE, "account", account];
        let env = env_name(kind);
        match self.tool.run(&args, Some(value), WRITE_TIMEOUT) {
            Ok(out) if out.code == Some(0) => {
                self.known().insert(account, Known::Found(value.to_string()));
                Ok(())
            }
            Ok(out) => Err(format!(
                "le trousseau du bureau n'est pas disponible (code {}) : garde la variable d'environnement {env}. Rien n'a été enregistré.",
                out.code.map_or("?".to_string(), |c| c.to_string())
            )),
            Err(ToolError::Missing) => Err(format!(
                "l'outil secret-tool est absent : installe le paquet libsecret-tools (et un trousseau, comme celui de GNOME), ou garde la variable d'environnement {env}. Rien n'a été enregistré."
            )),
            Err(ToolError::TimedOut) => Err(format!(
                "le trousseau du bureau n'a pas répondu à temps (est-il verrouillé ?) : déverrouille-le et recommence, ou garde la variable d'environnement {env}. Rien n'a été enregistré."
            )),
            Err(ToolError::Io) => Err(format!(
                "le trousseau du bureau n'a pas pu être utilisé : garde la variable d'environnement {env}. Rien n'a été enregistré."
            )),
        }
    }

    /// Efface la clé : `Ok(false)` si elle n'y était pas (ou s'il n'y a pas d'outil, donc rien d'enregistré par Scriibe).
    pub fn delete(&self, account: &'static str) -> Result<bool, String> {
        self.known().remove(account);
        match self.lookup(account) {
            Ok(None) | Err(ToolError::Missing) => return Ok(false),
            Ok(Some(_)) => {}
            Err(_) => return Err("le trousseau du bureau n'a pas répondu".into()),
        }
        match self.tool.run(&["clear", "service", SERVICE, "account", account], None, WRITE_TIMEOUT) {
            Ok(out) if out.code == Some(0) => Ok(true),
            _ => Err("le trousseau du bureau a refusé d'effacer la clé".into()),
        }
    }
}

impl<T: SecretTool> Secrets for ServiceSecrets<T> {
    fn get(&self, kind: KeyKind) -> Option<String> {
        let account = account_name(kind);
        match self.known().get(account) {
            Some(Known::Found(value)) => return Some(value.clone()),
            Some(Known::Absent(since)) if since.elapsed() < self.absent_for => return None,
            _ => {}
        }
        // Le verrou n'est pas tenu pendant l'appel : une demande de déverrouillage ne bloque pas les autres lectures.
        let found = self.lookup(account).ok().flatten();
        self.known().insert(account, found.clone().map_or_else(|| Known::Absent(Instant::now()), Known::Found));
        found
    }
}

/// Le trousseau du programme, partagé entre la lecture des clés et leur enregistrement par la page (un seul cache).
pub fn shared() -> &'static ServiceSecrets<CommandTool> {
    static SHARED: OnceLock<ServiceSecrets<CommandTool>> = OnceLock::new();
    SHARED.get_or_init(|| ServiceSecrets::new(CommandTool))
}

/// La lecture, pour la chaîne des sources de clés.
pub struct DesktopKeyring;

impl Secrets for DesktopKeyring {
    fn get(&self, kind: KeyKind) -> Option<String> {
        shared().get(kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    /// Un trousseau factice : garde les clés dans une table, écrit une trace de chaque appel (arguments seulement, pas l'entrée), et
    /// peut être « absent » ou « fermé ».
    #[derive(Clone, Default)]
    struct FakeTool {
        items: Arc<Mutex<HashMap<String, String>>>,
        calls: Arc<Mutex<Vec<Vec<String>>>>,
        stdin_seen: Arc<Mutex<Vec<String>>>,
        mode: Arc<Mutex<Mode>>,
    }

    #[derive(Clone, Copy, Default, PartialEq)]
    enum Mode {
        #[default]
        Working,
        NoTool,
        NoService,
        Locked,
    }

    impl FakeTool {
        fn set(&self, mode: Mode) {
            *self.mode.lock().unwrap() = mode;
        }
        fn count(&self, verb: &str) -> usize {
            self.calls.lock().unwrap().iter().filter(|c| c[0] == verb).count()
        }
    }

    impl SecretTool for FakeTool {
        fn run(&self, args: &[&str], stdin: Option<&str>, _: Duration) -> Result<ToolOutput, ToolError> {
            self.calls.lock().unwrap().push(args.iter().map(|a| a.to_string()).collect());
            if let Some(text) = stdin {
                self.stdin_seen.lock().unwrap().push(text.to_string());
            }
            match *self.mode.lock().unwrap() {
                Mode::NoTool => return Err(ToolError::Missing),
                Mode::Locked => return Err(ToolError::TimedOut),
                Mode::NoService => return Ok(ToolOutput { code: Some(1), stdout: String::new() }),
                Mode::Working => {}
            }
            let account = args.iter().position(|a| *a == "account").map(|i| args[i + 1].to_string()).unwrap_or_default();
            let found = |value: String| Ok(ToolOutput { code: Some(0), stdout: value });
            match args[0] {
                "store" => {
                    self.items.lock().unwrap().insert(account, stdin.unwrap_or_default().to_string());
                    found(String::new())
                }
                "lookup" => match self.items.lock().unwrap().get(&account) {
                    Some(v) => found(v.clone()),
                    None => Ok(ToolOutput { code: Some(1), stdout: String::new() }),
                },
                "clear" => {
                    self.items.lock().unwrap().remove(&account);
                    found(String::new())
                }
                _ => Err(ToolError::Io),
            }
        }
    }

    const KEY: &str = "sk-or-v1-0123456789abcdef";

    #[test]
    fn a_stored_key_is_read_again_after_a_restart() {
        let tool = FakeTool::default();
        ServiceSecrets::new(tool.clone()).store(KeyKind::OpenRouter, &format!("  {KEY}\n")).unwrap();
        // « Redémarrage » : un nouveau programme, donc un cache vide, sur le même trousseau.
        let after = ServiceSecrets::new(tool.clone());
        assert_eq!(after.get(KeyKind::OpenRouter).as_deref(), Some(KEY));
        assert_eq!(after.get(KeyKind::Deepgram), None, "chaque clé a son compte");
        assert_eq!(*tool.stdin_seen.lock().unwrap(), vec![KEY.to_string()], "sans espace ni retour à la ligne : ils seraient gardés");
    }

    #[test]
    fn the_key_goes_through_standard_input_never_through_the_arguments() {
        let tool = FakeTool::default();
        let keyring = ServiceSecrets::new(tool.clone());
        keyring.store(KeyKind::OpenRouter, KEY).unwrap();
        keyring.get(KeyKind::OpenRouter);
        keyring.delete("openrouter").unwrap();
        for call in tool.calls.lock().unwrap().iter() {
            assert!(call.iter().all(|a| !a.contains(KEY)), "la clé est visible dans les arguments : {call:?}");
        }
        assert_eq!(tool.stdin_seen.lock().unwrap().len(), 1, "seule l'écriture donne la clé à l'entrée");
    }

    #[test]
    fn without_a_keyring_nothing_is_written_and_the_message_names_the_environment_variable() {
        for (mode, wants) in [(Mode::NoTool, "libsecret-tools"), (Mode::NoService, "pas disponible"), (Mode::Locked, "verrouillé")] {
            let tool = FakeTool::default();
            tool.set(mode);
            let err = ServiceSecrets::new(tool.clone()).store(KeyKind::OpenRouter, KEY).unwrap_err();
            assert!(err.contains("OPENROUTER_API_KEY"), "{err}");
            assert!(err.contains(wants), "{err}");
            assert!(err.contains("Rien n'a été enregistré"), "{err}");
            assert!(!err.contains(KEY), "la clé est dans le message d'erreur");
            assert!(tool.items.lock().unwrap().is_empty(), "rien ne doit être écrit");
        }
        let deepgram = ServiceSecrets::new(FakeTool::default());
        deepgram.tool.set(Mode::NoTool);
        assert!(deepgram.store(KeyKind::Deepgram, KEY).unwrap_err().contains("DEEPGRAM_API_KEY"));
    }

    #[test]
    fn a_blank_or_unreadable_key_is_refused_before_any_call() {
        let tool = FakeTool::default();
        let keyring = ServiceSecrets::new(tool.clone());
        assert_eq!(keyring.store(KeyKind::OpenRouter, "  \n").unwrap_err(), "clé vide");
        assert!(keyring.store(KeyKind::OpenRouter, "ab\ncd").is_err());
        assert!(keyring.store(KeyKind::OpenRouter, &"a".repeat(MAX_KEY_LEN + 1)).is_err());
        assert!(tool.calls.lock().unwrap().is_empty());
    }

    #[test]
    fn a_failed_reading_is_not_repeated_at_every_refresh_but_a_stored_key_shows_at_once() {
        let tool = FakeTool::default();
        tool.set(Mode::Locked);
        let keyring = ServiceSecrets::new(tool.clone());
        for _ in 0..5 {
            assert_eq!(keyring.get(KeyKind::OpenRouter), None);
        }
        assert_eq!(tool.count("lookup"), 1, "une seule demande au trousseau fermé");
        tool.set(Mode::Working);
        keyring.store(KeyKind::OpenRouter, KEY).unwrap();
        assert_eq!(keyring.get(KeyKind::OpenRouter).as_deref(), Some(KEY));
        // Après le délai, on redemande.
        let tool = FakeTool::default();
        let keyring = ServiceSecrets::with_absent_for(tool.clone(), Duration::ZERO);
        keyring.get(KeyKind::OpenRouter);
        keyring.get(KeyKind::OpenRouter);
        assert_eq!(tool.count("lookup"), 2);
    }

    #[test]
    fn deleting_removes_the_key_and_says_if_there_was_one() {
        let tool = FakeTool::default();
        let keyring = ServiceSecrets::new(tool.clone());
        assert_eq!(keyring.delete("openrouter"), Ok(false));
        keyring.store(KeyKind::OpenRouter, KEY).unwrap();
        assert_eq!(keyring.delete("openrouter"), Ok(true));
        assert!(tool.items.lock().unwrap().is_empty());
        assert_eq!(keyring.get(KeyKind::OpenRouter), None);
        tool.set(Mode::NoTool);
        assert_eq!(keyring.delete("openrouter"), Ok(false), "pas d'outil : Scriibe n'a rien enregistré");
        tool.set(Mode::Locked);
        assert!(keyring.delete("openrouter").is_err());
    }
}
