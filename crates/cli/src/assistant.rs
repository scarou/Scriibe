//! L'assistant de premier lancement : d'après la machine mesurée, il propose la chaîne (transcription, mise au propre) et dit ce qu'il
//! faut installer. Les temps annoncés sont ceux **mesurés** sur le portable de test (S03, petits échantillons) ; ce qui n'a pas été
//! mesuré est dit tel quel.

use scriibe_core::settings::{AppSettings, CleanupEngine, TranscriptionEngine};
use scriibe_platform::machine::MachineInfo;
use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chain {
    pub id: &'static str,
    pub title: &'static str,
    pub detail: &'static str,
    /// Temps après le relâchement, tel que mesuré (ou « pas encore mesuré »).
    pub timing: &'static str,
    pub measured: bool,
    pub transcription: TranscriptionEngine,
    pub cleanup: CleanupEngine,
    /// Les éléments de la page Installation dont la chaîne a besoin.
    pub needs: &'static [&'static str],
    /// Une clé d'API est nécessaire.
    pub needs_key: Option<&'static str>,
}

const CHAINS: [Chain; 4] = [
    Chain {
        id: "cloud-cleanup",
        title: "Whisper sur cet ordinateur + mise au propre OpenRouter (cloud)",
        detail: "La voix reste sur l'ordinateur ; seul le texte transcrit part au cloud pour être mis au propre.",
        timing: "3,2 à 3,9 s après le relâchement (mesuré sur un portable à 4 cœurs sans carte graphique)",
        measured: true,
        transcription: TranscriptionEngine::Whisper,
        cleanup: CleanupEngine::OpenRouter,
        needs: &["whisper-server", "whisper-base"],
        needs_key: Some("openrouter"),
    },
    Chain {
        id: "local-raw",
        title: "Tout sur cet ordinateur, sans mise au propre",
        detail: "Whisper seul : le texte est collé tel que transcrit (signes dictés, dictionnaire et typographie appliqués). Aucun compte, rien ne quitte l'ordinateur.",
        timing: "2,3 à 3,1 s (mesuré sur un portable à 4 cœurs sans carte graphique)",
        measured: true,
        transcription: TranscriptionEngine::Whisper,
        cleanup: CleanupEngine::None,
        needs: &["whisper-server", "whisper-base"],
        needs_key: None,
    },
    Chain {
        id: "local-small-model",
        title: "Tout sur cet ordinateur, avec un petit modèle de mise au propre",
        detail: "Whisper + Qwen 2.5 1,5 B (llama.cpp). Rien ne quitte l'ordinateur, mais un petit modèle garde parfois les « euh » et peut changer un mot.",
        timing: "4 à 6 s (mesuré sur un portable à 4 cœurs sans carte graphique) ; 15 à 45 s pour la première dictée après une libération du modèle",
        measured: true,
        transcription: TranscriptionEngine::Whisper,
        cleanup: CleanupEngine::LlamaCpp,
        needs: &["whisper-server", "whisper-base", "llama-server", "qwen-1.5b"],
        needs_key: None,
    },
    Chain {
        id: "gpu",
        title: "Tout sur cet ordinateur, avec sa carte graphique",
        detail: "Whisper « small » + Qwen 2.5 3 B sur la carte graphique (llama.cpp Vulkan). Sans doute plus rapide et plus juste que sur le processeur.",
        timing: "pas encore mesuré sur une carte de cette classe",
        measured: false,
        transcription: TranscriptionEngine::Whisper,
        cleanup: CleanupEngine::LlamaCpp,
        needs: &["whisper-server", "whisper-small", "llama-vulkan", "qwen-3b"],
        needs_key: None,
    },
];

pub fn chain(id: &str) -> Option<&'static Chain> {
    CHAINS.iter().find(|c| c.id == id)
}

/// La chaîne conseillée : une vraie carte graphique → la chaîne locale accélérée ; sinon Whisper `base` avec la mise au propre cloud si
/// une clé OpenRouter existe, sans mise au propre dans le cas contraire (les mesures de S03).
pub fn recommended(machine: &MachineInfo, has_openrouter_key: bool) -> &'static str {
    if machine.has_capable_gpu() {
        "gpu"
    } else if has_openrouter_key {
        "cloud-cleanup"
    } else {
        "local-raw"
    }
}

/// Les chaînes à montrer : sans vraie carte graphique, celle qui l'exige n'est pas proposée.
pub fn offered(machine: &MachineInfo) -> Vec<&'static Chain> {
    CHAINS.iter().filter(|c| c.id != "gpu" || machine.has_capable_gpu()).collect()
}

pub fn machine_json(machine: &MachineInfo) -> Value {
    json!({
        "physical_cores": machine.physical_cores,
        "logical_cores": machine.logical_cores,
        "ram_gb": (machine.ram_gb() * 10.0).round() / 10.0,
        "gpus_known": machine.gpus_known,
        "gpus": machine.gpus.iter().map(|g| json!({
            "name": g.name,
            "vram_gb": g.vram_bytes.map(|v| (v as f64 / 1_073_741_824.0 * 10.0).round() / 10.0),
            "integrated": g.is_integrated(),
            "capable": g.is_capable(),
        })).collect::<Vec<_>>(),
        "capable_gpu": machine.has_capable_gpu(),
    })
}

pub fn chain_json(chain: &Chain) -> Value {
    json!({
        "id": chain.id, "title": chain.title, "detail": chain.detail, "timing": chain.timing, "measured": chain.measured,
        "needs": chain.needs, "needs_key": chain.needs_key,
    })
}

/// Applique la chaîne aux réglages (les moteurs ; les modèles gardent « automatique ») et marque l'assistant comme passé.
pub fn apply(chain: &Chain, settings: &mut AppSettings) {
    settings.transcription_engine = chain.transcription;
    settings.cleanup_engine = chain.cleanup;
    settings.setup_done = true;
}

#[cfg(test)]
mod tests {
    use super::*;
    use scriibe_platform::machine::Gpu;

    const GIB: u64 = 1024 * 1024 * 1024;

    fn machine(gpus: Vec<Gpu>) -> MachineInfo {
        MachineInfo { physical_cores: 4, logical_cores: 8, ram_bytes: 16 * GIB, gpus, gpus_known: true }
    }

    fn gpu(name: &str, gb: u64) -> Gpu {
        Gpu { name: name.into(), vram_bytes: Some(gb * GIB) }
    }

    #[test]
    fn the_test_laptop_without_a_real_card_gets_the_measured_chains() {
        let laptop = machine(vec![gpu("AMD Radeon (TM) 500 Series", 2), gpu("Intel(R) UHD Graphics 620", 1)]);
        assert_eq!(recommended(&laptop, true), "cloud-cleanup");
        assert_eq!(recommended(&laptop, false), "local-raw");
        assert!(offered(&laptop).iter().all(|c| c.id != "gpu"), "pas de chaîne pour carte graphique sans vraie carte");
        assert!(offered(&laptop).iter().all(|c| c.measured));
    }

    #[test]
    fn a_machine_with_a_real_card_is_offered_the_accelerated_chain_first_and_told_it_is_not_measured() {
        let desktop = machine(vec![gpu("NVIDIA GeForce RTX 3060", 12)]);
        assert_eq!(recommended(&desktop, true), "gpu");
        assert_eq!(offered(&desktop).len(), 4);
        assert!(!chain("gpu").unwrap().measured);
    }

    #[test]
    fn applying_a_chain_sets_the_engines_and_marks_the_assistant_done() {
        let mut settings = AppSettings::default();
        assert!(!settings.setup_done);
        apply(chain("local-raw").unwrap(), &mut settings);
        assert_eq!(
            (settings.transcription_engine, settings.cleanup_engine, settings.setup_done),
            (TranscriptionEngine::Whisper, CleanupEngine::None, true)
        );
        apply(chain("cloud-cleanup").unwrap(), &mut settings);
        assert_eq!(settings.cleanup_engine, CleanupEngine::OpenRouter);
        assert!(chain("inconnue").is_none());
    }

    #[test]
    fn every_need_is_a_real_download() {
        let ids: Vec<&str> = crate::web::download_ids();
        for c in CHAINS.iter() {
            for need in c.needs {
                assert!(ids.contains(need), "{} : {need} n'existe pas dans la page Installation", c.id);
            }
        }
    }
}
