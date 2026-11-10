//! Ce que Scriibe sait de l'ordinateur : cœurs, mémoire, cartes graphiques. Sert à l'assistant de premier lancement, qui propose la
//! chaîne (transcription et mise au propre) adaptée à la machine.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gpu {
    pub name: String,
    /// Mémoire vidéo en octets, si le système la dit.
    pub vram_bytes: Option<u64>,
}

impl Gpu {
    /// Une carte graphique intégrée au processeur : elle partage la mémoire de l'ordinateur et n'accélère pas un modèle de langage.
    pub fn is_integrated(&self) -> bool {
        let name = self.name.to_lowercase();
        ["intel", "uhd", "iris", "basic render", "basic display", "remote display", "microsoft", "virtual", "vmware", "parallels"]
            .iter()
            .any(|word| name.contains(word))
    }

    /// Une vraie carte : dédiée et assez de mémoire vidéo (4 Go) pour porter un petit modèle. Mémoire inconnue : on n'en suppose pas.
    pub fn is_capable(&self) -> bool {
        !self.is_integrated() && self.vram_bytes.is_some_and(|v| v >= 4 * 1024 * 1024 * 1024)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineInfo {
    pub physical_cores: usize,
    pub logical_cores: usize,
    pub ram_bytes: u64,
    pub gpus: Vec<Gpu>,
    /// Vrai si la recherche des cartes a réussi (un ordinateur sans carte détectée et un échec de détection ne se disent pas pareil).
    pub gpus_known: bool,
}

impl MachineInfo {
    pub fn has_capable_gpu(&self) -> bool {
        self.gpus.iter().any(Gpu::is_capable)
    }

    pub fn ram_gb(&self) -> f64 {
        self.ram_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    }
}

/// Lit des lignes « nom|mémoire en octets » (la mémoire peut manquer ou être vide) ; les doublons et les lignes vides sont écartés.
pub fn parse_gpu_lines(text: &str) -> Vec<Gpu> {
    let mut gpus: Vec<Gpu> = Vec::new();
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let (name, memory) = line.split_once('|').unwrap_or((line, ""));
        let name = name.trim();
        if name.is_empty() || gpus.iter().any(|g| g.name == name) {
            continue;
        }
        let vram_bytes = memory.trim().parse::<u64>().ok().filter(|v| *v > 0);
        gpus.push(Gpu { name: name.to_string(), vram_bytes });
    }
    gpus
}

/// Les cartes graphiques, avec leur mémoire quand le registre de Windows la donne (le champ `AdapterRAM` de WMI plafonne à 4 Go).
#[cfg(windows)]
fn probe_gpus() -> Option<Vec<Gpu>> {
    use std::os::windows::process::CommandExt;
    use std::time::Duration;
    let script = "$k='HKLM:\\SYSTEM\\CurrentControlSet\\Control\\Class\\{4d36e968-e325-11ce-bfc1-08002be10318}\\0*'; \
        Get-ItemProperty $k -ErrorAction SilentlyContinue | ForEach-Object { $_.DriverDesc + '|' + $_.'HardwareInformation.qwMemorySize' }";
    let child = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .creation_flags(0x0800_0000)
        .spawn()
        .ok()?;
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(child.wait_with_output());
    });
    let output = receiver.recv_timeout(Duration::from_secs(15)).ok()?.ok()?;
    Some(parse_gpu_lines(&String::from_utf8_lossy(&output.stdout)))
}

#[cfg(not(windows))]
fn probe_gpus() -> Option<Vec<Gpu>> {
    None
}

/// Le prénom du bandeau d'accueil : le premier mot du nom complet du compte, à défaut l'identifiant de connexion (première lettre en
/// majuscule). Vide si le système ne dit rien.
pub fn first_name_from(full_name: &str, login: &str) -> String {
    let word = |text: &str| text.split_whitespace().next().unwrap_or("").to_string();
    let name = match word(full_name) {
        w if !w.is_empty() => w,
        _ => word(login),
    };
    let mut letters = name.chars();
    letters.next().map(|first| first.to_uppercase().chain(letters).collect()).unwrap_or_default()
}

fn account_full_name(login: &str) -> String {
    let run = |program: &str, args: &[&str]| {
        std::process::Command::new(program)
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    };
    if cfg!(target_os = "macos") {
        run("id", &["-F"]).unwrap_or_default()
    } else if cfg!(target_os = "linux") {
        // « login:x:1000:1000:Nom complet,,,:/home/login:/bin/bash »
        run("getent", &["passwd", login])
            .and_then(|line| line.split(':').nth(4).map(|f| f.split(',').next().unwrap_or("").to_string()))
            .unwrap_or_default()
    } else {
        String::new()
    }
}

/// Le prénom de la personne qui utilise l'ordinateur (calculé une fois).
pub fn first_name() -> &'static str {
    static NAME: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    NAME.get_or_init(|| {
        let login = std::env::var("USER").or_else(|_| std::env::var("USERNAME")).unwrap_or_default();
        first_name_from(&account_full_name(&login), &login)
    })
}

pub fn probe() -> MachineInfo {
    let mut system = sysinfo::System::new();
    system.refresh_memory();
    let logical = std::thread::available_parallelism().map_or(1, usize::from);
    let physical = system.physical_core_count().filter(|n| *n > 0).unwrap_or(logical).min(logical);
    let gpus = probe_gpus();
    MachineInfo {
        physical_cores: physical,
        logical_cores: logical,
        ram_bytes: system.total_memory(),
        gpus_known: gpus.is_some(),
        gpus: gpus.unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GIB: u64 = 1024 * 1024 * 1024;

    #[test]
    fn gpu_lines_are_read_with_or_without_memory_and_without_duplicates() {
        let gpus = parse_gpu_lines("AMD Radeon (TM) 500 Series|2147483648\nIntel(R) UHD Graphics 620|\n\nIntel(R) UHD Graphics 620|1\n  |5\nNVIDIA GeForce RTX 3060");
        assert_eq!(gpus.len(), 3);
        assert_eq!(gpus[0].vram_bytes, Some(2 * GIB));
        assert_eq!(gpus[1].vram_bytes, None);
        assert_eq!(gpus[2].name, "NVIDIA GeForce RTX 3060");
    }

    #[test]
    fn the_first_name_comes_from_the_full_name_then_from_the_login() {
        assert_eq!(first_name_from("Jean Martin", "jeanmartin"), "Jean");
        assert_eq!(first_name_from("  marie-claire  Dupont", "mcd"), "Marie-claire");
        assert_eq!(first_name_from("", "scarou"), "Scarou");
        assert_eq!(first_name_from("", ""), "");
    }

    #[test]
    fn only_a_dedicated_card_with_enough_memory_counts_as_capable() {
        let card = |name: &str, gb: u64| Gpu { name: name.into(), vram_bytes: Some(gb * GIB) };
        assert!(card("NVIDIA GeForce RTX 3060", 12).is_capable());
        assert!(card("AMD Radeon RX 6600", 8).is_capable());
        assert!(!card("AMD Radeon (TM) 500 Series", 2).is_capable(), "un portable d'essai : 2 Go ne suffisent pas");
        assert!(!card("Intel(R) UHD Graphics 620", 8).is_capable(), "intégrée : elle partage la mémoire de l'ordinateur");
        assert!(!Gpu { name: "NVIDIA GeForce RTX 3060".into(), vram_bytes: None }.is_capable(), "mémoire inconnue : on n'en suppose pas");
    }

    #[test]
    fn the_machine_is_measured_without_panicking() {
        let machine = probe();
        assert!(machine.physical_cores >= 1 && machine.physical_cores <= machine.logical_cores);
        assert!(machine.ram_bytes > 0);
    }
}
