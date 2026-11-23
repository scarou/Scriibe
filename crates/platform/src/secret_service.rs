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
/// Une clé trouvée n'est gardée en mémoire que ce temps : ensuite elle est effacée et relue (une clé supprimée ailleurs cesse d'être
/// servie, et la clé ne dort pas en mémoire jusqu'à la fermeture de Scriibe).
const FOUND_FOR: Duration = Duration::from_secs(600);
/// Où chercher `secret-tool` avant de se fier au `PATH` du processus (un `PATH` modifié ferait lancer un autre programme avec la clé).
const TOOL_DIRS: [&str; 3] = ["/usr/bin", "/bin", "/usr/local/bin"];
/// Message d'un trousseau qui n'a pas répondu à la désinstallation ; il dit quoi faire.
const LOCKED_MESSAGE: &str =
    "le trousseau du bureau n'a pas répondu : déverrouille-le (Mots de passe et clés) puis recommence, ou supprime à la main les entrées « Scriibe »";
/// Un trousseau qui n'a pas répondu (fermé, outil absent) n'est pas redemandé avant ce délai : pas de fenêtre de déverrouillage en pleine
/// dictée, toutes les 30 s. Une clé enregistrée par la page, elle, apparaît tout de suite.
const UNREACHABLE_FOR: Duration = Duration::from_secs(300);
/// Lire : un trousseau fermé peut afficher une demande de mot de passe ; enregistrer : on laisse le temps de la remplir.
const READ_TIMEOUT: Duration = Duration::from_secs(8);
const WRITE_TIMEOUT: Duration = Duration::from_secs(60);
/// Une clé d'API fait quelques dizaines de signes.
const MAX_KEY_LEN: usize = 4096;

/// Écrase un texte secret avant de le rendre (`black_box` : l'optimiseur ne retire pas l'écriture). Au mieux : une copie faite plus tôt
/// par une bibliothèque n'est pas atteinte.
fn wipe(text: &mut String) {
    let mut bytes = std::mem::take(text).into_bytes();
    bytes.iter_mut().for_each(|b| *b = 0);
    std::hint::black_box(&bytes);
}

/// Une clé gardée en mémoire : écrasée dès qu'elle est lâchée (remplacée, périmée, effacée).
#[derive(Clone)]
struct Secret(String);

impl Secret {
    fn new(text: &str) -> Self {
        Secret(text.to_string())
    }
}

impl Drop for Secret {
    fn drop(&mut self) {
        wipe(&mut self.0);
    }
}

/// Ce que renvoie l'outil : le code de sortie et ce qu'il a écrit en sortie standard (la clé, pour une lecture). Jamais sa sortie d'erreur.
/// La sortie est écrasée quand le résultat est lâché.
pub struct ToolOutput {
    pub code: Option<i32>,
    pub stdout: String,
}

impl Drop for ToolOutput {
    fn drop(&mut self) {
        wipe(&mut self.stdout);
    }
}

pub enum ToolError {
    /// `secret-tool` n'est pas installé.
    Missing,
    /// Pas de réponse dans le délai (trousseau fermé, demande de mot de passe laissée sans réponse).
    TimedOut,
    /// Lancement ou échange impossible.
    Io,
    /// L'outil a fermé son entrée avant d'avoir lu la clé : il a refusé l'écriture (avec son code de sortie, s'il l'a donné).
    Refused(Option<i32>),
}

/// L'outil `secret-tool`, ou son remplaçant dans les tests.
pub trait SecretTool: Send + Sync {
    fn run(&self, args: &[&str], stdin: Option<&str>, timeout: Duration) -> Result<ToolOutput, ToolError>;
}

/// Le vrai `secret-tool`.
pub struct CommandTool {
    program: String,
}

impl CommandTool {
    pub fn new() -> Self {
        Self { program: resolve_program(&TOOL_DIRS) }
    }
}

/// `secret-tool` par chemin absolu dans les dossiers système ; à défaut, son nom seul (le `PATH` en repli).
fn resolve_program(dirs: &[&str]) -> String {
    dirs.iter()
        .map(|dir| std::path::Path::new(dir).join(TOOL))
        .find(|path| path.is_file())
        .map_or_else(|| TOOL.to_string(), |path| path.to_string_lossy().into_owned())
}

impl Default for CommandTool {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretTool for CommandTool {
    fn run(&self, args: &[&str], stdin: Option<&str>, timeout: Duration) -> Result<ToolOutput, ToolError> {
        let mut child = Command::new(&self.program)
            .args(args)
            .stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::null() })
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| if e.kind() == std::io::ErrorKind::NotFound { ToolError::Missing } else { ToolError::Io })?;
        if let (Some(text), Some(mut pipe)) = (stdin, child.stdin.take()) {
            // `secret-tool store` lit jusqu'à la fin de l'entrée : pas de retour à la ligne ajouté, il serait gardé avec la clé.
            if pipe.write_all(text.as_bytes()).is_err() {
                // L'outil a fermé son entrée sans lire la clé : on lui laisse un instant pour dire pourquoi (son code de sortie), puis on
                // le coupe. Un processus tué se retire du bus : le trousseau retire sa demande de déverrouillage.
                drop(pipe);
                let began = Instant::now();
                let code = loop {
                    match child.try_wait() {
                        Ok(Some(status)) => break status.code(),
                        Ok(None) if began.elapsed() < Duration::from_secs(2) => std::thread::sleep(Duration::from_millis(20)),
                        _ => {
                            let _ = child.kill();
                            let _ = child.wait();
                            break None;
                        }
                    }
                };
                return Err(ToolError::Refused(code));
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
    Found(Secret, Instant),
    /// Le trousseau a répondu : pas de clé.
    Absent(Instant),
    /// Le trousseau n'a pas répondu (fermé, outil absent).
    Unreachable(Instant),
}

#[derive(Default)]
struct State {
    known: HashMap<&'static str, Known>,
    /// Compte les écritures et effacements de chaque clé : une lecture qui a commencé avant l'un d'eux ne range pas son résultat (il serait
    /// périmé : clé enregistrée pendant que la lecture attendait un trousseau fermé).
    generation: HashMap<&'static str, u64>,
}

pub struct ServiceSecrets<T: SecretTool> {
    tool: T,
    absent_for: Duration,
    unreachable_for: Duration,
    found_for: Duration,
    state: Mutex<State>,
}

impl<T: SecretTool> ServiceSecrets<T> {
    pub fn new(tool: T) -> Self {
        Self::with_delays(tool, ABSENT_FOR, UNREACHABLE_FOR)
    }

    pub fn with_delays(tool: T, absent_for: Duration, unreachable_for: Duration) -> Self {
        Self { tool, absent_for, unreachable_for, found_for: FOUND_FOR, state: Mutex::new(State::default()) }
    }

    /// Change le temps pendant lequel une clé trouvée reste en mémoire (les essais).
    pub fn with_found_for(mut self, found_for: Duration) -> Self {
        self.found_for = found_for;
        self
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Une écriture ou un effacement : ce que la mémoire sait de la clé change, les lectures en cours deviennent périmées.
    fn changed(&self, account: &'static str, known: Option<Known>) {
        let mut state = self.state();
        *state.generation.entry(account).or_default() += 1;
        match known {
            Some(known) => state.known.insert(account, known),
            None => state.known.remove(account),
        };
    }

    fn lookup(&self, account: &str) -> Result<Option<Secret>, ToolError> {
        let out = self.tool.run(&["lookup", "service", SERVICE, "account", account], None, READ_TIMEOUT)?;
        let value = out.stdout.trim();
        Ok((out.code == Some(0) && !value.is_empty()).then(|| Secret::new(value)))
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
                self.changed(account, Some(Known::Found(Secret::new(value), Instant::now())));
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
            Err(ToolError::Refused(code)) => Err(format!(
                "secret-tool a refusé la clé (code {}) : déverrouille le trousseau et recommence, ou garde la variable d'environnement {env}. Rien n'a été enregistré.",
                code.map_or("?".to_string(), |c| c.to_string())
            )),
            Err(ToolError::Io) => Err(format!(
                "le trousseau du bureau n'a pas pu être utilisé : garde la variable d'environnement {env}. Rien n'a été enregistré."
            )),
        }
    }

    /// Efface la clé : `Ok(false)` si elle n'y était pas (ou s'il n'y a pas d'outil, donc rien d'enregistré par Scriibe).
    pub fn delete(&self, account: &'static str) -> Result<bool, String> {
        self.changed(account, None);
        match self.lookup(account) {
            Ok(None) | Err(ToolError::Missing) => return Ok(false),
            Ok(Some(_)) => {}
            Err(_) => return Err(LOCKED_MESSAGE.into()),
        }
        match self.tool.run(&["clear", "service", SERVICE, "account", account], None, WRITE_TIMEOUT) {
            Ok(out) if out.code == Some(0) => Ok(true),
            _ => Err("le trousseau du bureau a refusé d'effacer la clé".into()),
        }
    }

    /// Efface plusieurs clés. Dès que le trousseau n'a pas répondu, les suivantes ne sont pas tentées (une attente de 8 s chacune, trois
    /// fois, devant une fenêtre de déverrouillage) : elles reçoivent le même message.
    pub fn delete_all(&self, accounts: &[&'static str]) -> Vec<(String, Result<bool, String>)> {
        let mut locked = false;
        accounts
            .iter()
            .map(|account| {
                let result = if locked {
                    self.changed(account, None);
                    Err(LOCKED_MESSAGE.to_string())
                } else {
                    self.delete(account)
                };
                locked |= result.as_ref().is_err_and(|e| e == LOCKED_MESSAGE);
                ((*account).to_string(), result)
            })
            .collect()
    }
}

impl<T: SecretTool> Secrets for ServiceSecrets<T> {
    fn get(&self, kind: KeyKind) -> Option<String> {
        let account = account_name(kind);
        let generation = {
            let state = self.state();
            match state.known.get(account) {
                Some(Known::Found(value, since)) if since.elapsed() < self.found_for => return Some(value.0.clone()),
                Some(Known::Absent(since)) if since.elapsed() < self.absent_for => return None,
                Some(Known::Unreachable(since)) if since.elapsed() < self.unreachable_for => return None,
                _ => {}
            }
            state.generation.get(account).copied().unwrap_or(0)
        };
        // Le verrou n'est pas tenu pendant l'appel : une demande de déverrouillage ne bloque pas les autres lectures.
        let (found, known) = match self.lookup(account) {
            Ok(Some(value)) => (Some(value.0.clone()), Known::Found(value, Instant::now())),
            Ok(None) => (None, Known::Absent(Instant::now())),
            Err(_) => (None, Known::Unreachable(Instant::now())),
        };
        let mut state = self.state();
        // Une clé enregistrée ou effacée pendant la lecture : son résultat est périmé, il ne remplace rien.
        if state.generation.get(account).copied().unwrap_or(0) == generation {
            state.known.insert(account, known);
        }
        found
    }
}

/// Le trousseau du programme, partagé entre la lecture des clés et leur enregistrement par la page (un seul cache).
pub fn shared() -> &'static ServiceSecrets<CommandTool> {
    static SHARED: OnceLock<ServiceSecrets<CommandTool>> = OnceLock::new();
    SHARED.get_or_init(|| ServiceSecrets::new(CommandTool::new()))
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
    type Hook = Arc<dyn Fn() + Send + Sync>;

    #[derive(Clone, Default)]
    struct FakeTool {
        items: Arc<Mutex<HashMap<String, String>>>,
        calls: Arc<Mutex<Vec<Vec<String>>>>,
        stdin_seen: Arc<Mutex<Vec<String>>>,
        mode: Arc<Mutex<Mode>>,
        /// Appelé pendant une lecture, après qu'elle a trouvé sa réponse et avant de la rendre (un trousseau lent).
        during_lookup: Arc<Mutex<Option<Hook>>>,
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
                "lookup" => {
                    let answer = match self.items.lock().unwrap().get(&account) {
                        Some(v) => found(v.clone()),
                        None => Ok(ToolOutput { code: Some(1), stdout: String::new() }),
                    };
                    let hook = self.during_lookup.lock().unwrap().clone();
                    if let Some(hook) = hook {
                        hook();
                    }
                    answer
                }
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
        // Après le délai, on redemande : vite pour « absente », beaucoup plus tard pour un trousseau qui n'a pas répondu.
        let tool = FakeTool::default();
        let keyring = ServiceSecrets::with_delays(tool.clone(), Duration::ZERO, Duration::from_secs(300));
        keyring.get(KeyKind::OpenRouter);
        keyring.get(KeyKind::OpenRouter);
        assert_eq!(tool.count("lookup"), 2, "absente : redemandée après le délai");
        tool.set(Mode::Locked);
        let keyring = ServiceSecrets::with_delays(tool.clone(), Duration::ZERO, Duration::from_secs(300));
        keyring.get(KeyKind::Deepgram);
        keyring.get(KeyKind::Deepgram);
        assert_eq!(tool.count("lookup"), 3, "fermé : pas redemandé, même quand le délai de « absente » est passé");
    }

    #[test]
    fn a_key_stored_while_a_reading_waits_is_not_overwritten_by_that_reading() {
        let tool = FakeTool::default();
        let keyring = Arc::new(ServiceSecrets::new(tool.clone()));
        let other = keyring.clone();
        // Pendant la lecture (trousseau fermé qui se déverrouille), la page enregistre la clé.
        *tool.during_lookup.lock().unwrap() = Some(Arc::new(move || {
            other.store(KeyKind::OpenRouter, KEY).unwrap();
        }));
        assert_eq!(keyring.get(KeyKind::OpenRouter), None, "cette lecture a vu le trousseau avant l'écriture");
        *tool.during_lookup.lock().unwrap() = None;
        assert_eq!(keyring.get(KeyKind::OpenRouter).as_deref(), Some(KEY), "la clé enregistrée n'est pas effacée par une lecture périmée");
    }

    #[test]
    fn an_erased_key_is_not_brought_back_by_a_reading_that_was_waiting() {
        let tool = FakeTool::default();
        let keyring = Arc::new(ServiceSecrets::new(tool.clone()));
        keyring.store(KeyKind::OpenRouter, KEY).unwrap();
        let fresh = Arc::new(ServiceSecrets::new(tool.clone()));
        let other = fresh.clone();
        *tool.during_lookup.lock().unwrap() = Some(Arc::new(move || {
            other.changed("openrouter", None);
        }));
        assert_eq!(fresh.get(KeyKind::OpenRouter).as_deref(), Some(KEY), "la lecture rend ce qu'elle a vu");
        *tool.during_lookup.lock().unwrap() = None;
        tool.items.lock().unwrap().clear();
        assert_eq!(fresh.get(KeyKind::OpenRouter), None, "mais ne le garde pas : la clé effacée n'est plus servie");
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

    /// Un faux `secret-tool` (script d'essai inoffensif, dans un dossier temporaire) : le seul moyen de prouver, sur le vrai lancement de
    /// processus, que la clé arrive par l'entrée standard, que l'entrée est fermée, que la sortie d'erreur n'est pas lue et que le délai coupe.
    #[cfg(unix)]
    fn fake_program(name: &str, script: &str) -> (std::path::PathBuf, CommandTool) {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("scriibe-secret-tool-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let program = dir.join("secret-tool");
        std::fs::write(&program, format!("#!/bin/sh\n{script}\n")).unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
        (dir, CommandTool { program: program.to_string_lossy().into_owned() })
    }

    #[cfg(unix)]
    #[test]
    fn real_tool_gets_the_key_on_standard_input_only_and_its_error_output_is_never_read() {
        let (dir, tool) = fake_program(
            "stdin",
            r#"cat > "$(dirname "$0")/stdin.txt"; echo "$@" > "$(dirname "$0")/args.txt"; echo "ERREUR-DU-TROUSSEAU" >&2; exit 3"#,
        );
        let out = tool.run(&["store", "service", "Scriibe"], Some(KEY), Duration::from_secs(10)).ok().unwrap();
        assert_eq!(out.code, Some(3));
        assert!(!out.stdout.contains("ERREUR"), "la sortie d'erreur n'est jamais lue");
        assert_eq!(
            std::fs::read_to_string(dir.join("stdin.txt")).unwrap(),
            KEY,
            "l'entrée est fermée (le script a fini) et sans retour à la ligne"
        );
        assert!(!std::fs::read_to_string(dir.join("args.txt")).unwrap().contains(KEY), "la clé n'est pas dans les arguments");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[cfg(unix)]
    #[test]
    fn real_tool_is_stopped_after_the_delay_and_a_missing_program_is_reported() {
        let (dir, tool) = fake_program("slow", "sleep 30");
        let began = Instant::now();
        assert!(matches!(tool.run(&["lookup"], None, Duration::from_millis(300)), Err(ToolError::TimedOut)));
        assert!(began.elapsed() < Duration::from_secs(10), "le processus est tué, pas attendu");
        let _ = std::fs::remove_dir_all(dir);
        let absent = CommandTool { program: "/nonexistent/scriibe-secret-tool".into() };
        assert!(matches!(absent.run(&["lookup"], None, Duration::from_secs(1)), Err(ToolError::Missing)));
    }

    #[test]
    fn a_found_key_is_forgotten_after_its_delay_so_a_key_erased_elsewhere_stops_being_served() {
        let tool = FakeTool::default();
        ServiceSecrets::new(tool.clone()).store(KeyKind::OpenRouter, KEY).unwrap();
        let keyring = ServiceSecrets::new(tool.clone()).with_found_for(Duration::from_secs(600));
        assert_eq!(keyring.get(KeyKind::OpenRouter).as_deref(), Some(KEY));
        tool.items.lock().unwrap().clear();
        assert_eq!(keyring.get(KeyKind::OpenRouter).as_deref(), Some(KEY), "dans le délai : servie depuis la mémoire, sans appel");
        assert_eq!(tool.count("lookup"), 1);
        let keyring = ServiceSecrets::new(tool.clone()).with_found_for(Duration::ZERO);
        tool.items.lock().unwrap().insert("openrouter".into(), KEY.into());
        assert_eq!(keyring.get(KeyKind::OpenRouter).as_deref(), Some(KEY));
        tool.items.lock().unwrap().clear();
        assert_eq!(keyring.get(KeyKind::OpenRouter), None, "délai passé : relue, et la clé effacée ailleurs n'est plus servie");
    }

    #[test]
    fn secrets_are_overwritten_when_dropped() {
        let mut text = String::from(KEY);
        wipe(&mut text);
        assert!(text.is_empty(), "le texte est vidé");
        // Les types qui portent une clé se vident à la chute (vérifié sur le tampon avant la libération).
        let mut secret = Secret::new(KEY);
        wipe(&mut secret.0);
        assert!(secret.0.is_empty());
        let out = ToolOutput { code: Some(0), stdout: KEY.into() };
        drop(out);
    }

    #[test]
    fn the_program_is_looked_for_by_absolute_path_first_and_by_name_otherwise() {
        let dir = std::env::temp_dir().join(format!("scriibe-tool-dir-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let none = dir.join("vide");
        std::fs::create_dir_all(&none).unwrap();
        let (none_s, dir_s) = (none.to_string_lossy().into_owned(), dir.to_string_lossy().into_owned());
        assert_eq!(resolve_program(&[none_s.as_str()]), TOOL, "introuvable : le nom seul, cherché dans le PATH");
        std::fs::write(dir.join(TOOL), "").unwrap();
        assert_eq!(resolve_program(&[none_s.as_str(), dir_s.as_str()]), dir.join(TOOL).to_string_lossy(), "chemin absolu dès qu'il existe");
        assert!(TOOL_DIRS.iter().all(|d| d.starts_with('/')));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_locked_keyring_stops_the_uninstall_after_the_first_wait_and_says_how_to_unlock() {
        let tool = FakeTool::default();
        tool.set(Mode::Locked);
        let keyring = ServiceSecrets::new(tool.clone());
        let results = keyring.delete_all(&["deepgram", "openrouter", "github"]);
        assert_eq!(tool.count("lookup"), 1, "une seule attente, pas trois");
        assert_eq!(results.len(), 3);
        for (_, result) in &results {
            let err = result.as_ref().unwrap_err();
            assert!(err.contains("déverrouille-le"), "{err}");
        }
        tool.set(Mode::Working);
        tool.items.lock().unwrap().insert("openrouter".into(), KEY.into());
        let results = ServiceSecrets::new(tool.clone()).delete_all(&["deepgram", "openrouter", "github"]);
        assert_eq!(
            results.into_iter().map(|(a, r)| (a, r.unwrap())).collect::<Vec<_>>(),
            vec![("deepgram".into(), false), ("openrouter".into(), true), ("github".into(), false)]
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_tool_that_closes_its_input_without_reading_gives_a_message_with_its_exit_code_and_no_key() {
        let (dir, tool) = fake_program("refuse", "exec <&-; sleep 0.2; exit 4");
        let big = "a".repeat(1 << 20);
        match tool.run(&["store"], Some(&big), Duration::from_secs(10)) {
            Err(ToolError::Refused(code)) => assert_eq!(code, Some(4)),
            other => panic!("attendu : refus avec code 4, obtenu {}", if other.is_ok() { "un résultat" } else { "une autre erreur" }),
        }
        let keyring = ServiceSecrets::new(tool);
        // La clé d'API tient dans le tube : ce cas passe par le code de sortie ordinaire.
        let err = keyring.store(KeyKind::OpenRouter, KEY).unwrap_err();
        assert!(err.contains("code 4"), "{err}");
        assert!(err.contains("OPENROUTER_API_KEY") && err.contains("Rien n'a été enregistré") && !err.contains(KEY), "{err}");
        let _ = std::fs::remove_dir_all(dir);
    }
}
