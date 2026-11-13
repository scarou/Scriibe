//! Désinstallation depuis la fenêtre (chantier 7 ; `docs/SYSTEME.md` § Cadrage). Le serveur construit la liste exacte (jamais à partir d'un
//! chemin envoyé par la page), la montre, puis, sur accord, coupe toute écriture, arrête les moteurs et lance l'assistant : tout se fait
//! **après la sortie** de Scriibe (`scriibe_platform::uninstall`).
//!
//! Sécurité : la page ne désigne rien à supprimer, elle dit seulement « garder mes modèles » et « une sauvegarde a été faite ». Si la
//! case « sauvegarde » est cochée, la désinstallation exige une sauvegarde **écrite et relue avec succès** il y a moins de 30 minutes.

use super::backup;
use scriibe_engines::runtime::Log;
use scriibe_platform::uninstall::{self, Job, Kind, Plan};
use serde_json::{json, Value};
use std::time::Duration;

/// Le programme de Scriibe, les modèles choisis ailleurs et l'entrée de démarrage : tout ce que la liste lit hors du disque.
pub(crate) fn current_plan(settings: &scriibe_core::settings::AppSettings, keep_models: bool) -> (uninstall::Env, Plan) {
    let exe = std::env::current_exe().unwrap_or_default();
    let env = uninstall::Env::real(exe, keep_models);
    let external = [settings.whisper_model_path.clone(), settings.llama_cpp_model.clone()];
    let startup = scriibe_platform::autostart::status();
    let plan = uninstall::build_plan(&env, &external, startup.enabled || startup.disabled_by_system);
    (env, plan)
}

/// Les dossiers que la désinstallation supprimera, pour refuser une sauvegarde rangée dedans.
pub(crate) fn protected_roots(settings: &scriibe_core::settings::AppSettings) -> Vec<std::path::PathBuf> {
    // Avec « garder mes modèles » aucun dossier ne part en entier : la liste complète (sans cette option) est la plus prudente.
    current_plan(settings, false).1.roots()
}

fn kind_name(kind: Kind) -> &'static str {
    kind.as_str()
}

/// Les éléments pour la fenêtre : les sons temporaires (parfois des dizaines) sont regroupés en une ligne.
fn items_view(plan: &Plan) -> Vec<Value> {
    let mut out: Vec<Value> = plan
        .items
        .iter()
        .filter(|i| i.kind != Kind::TempSound)
        .map(|i| json!({"kind": kind_name(i.kind), "path": i.path.to_string_lossy(), "size": i.size, "link": i.link, "count": 1}))
        .collect();
    let sounds: Vec<_> = plan.items.iter().filter(|i| i.kind == Kind::TempSound).collect();
    if let Some(first) = sounds.first() {
        let folder = first.path.parent().map(|p| p.join("scriibe-son-*.wav")).unwrap_or_default();
        out.push(json!({"kind": "sound", "path": folder.to_string_lossy(), "size": sounds.iter().map(|i| i.size).sum::<u64>(), "link": false, "count": sounds.len()}));
    }
    out
}

/// `POST api/uninstall/plan` : `{"keepModels"?}` → la liste exacte pour la fenêtre.
pub(crate) fn plan_view(settings: &scriibe_core::settings::AppSettings, input: &Value) -> Value {
    let (env, plan) = current_plan(settings, input["keepModels"].as_bool().unwrap_or(false));
    let keys = match env.os {
        uninstall::Os::Mac => "keychain",
        uninstall::Os::Windows => "credman",
        uninstall::Os::Linux => "env",
    };
    json!({
        "ok": true,
        "blocked": plan.blocked,
        "items": items_view(&plan),
        "kept": plan.kept.iter().map(|(k, p, s)| json!({"kind": k, "path": p.to_string_lossy(), "size": s})).collect::<Vec<_>>(),
        "notBackedUp": plan.not_backed_up.iter().map(|(k, s)| json!({"kind": k, "size": s})).collect::<Vec<_>>(),
        "warnings": plan.warnings,
        "startup": plan.startup,
        "keys": keys,
        "trashSize": plan.trash_size(),
        "capacityProblem": uninstall::capacity_problem(&plan),
    })
}

/// `POST api/uninstall/run` : `{"keepModels", "backup", "confirmed"}`. `Ok` : l'assistant est lancé, l'appelant répond puis quitte.
pub(crate) fn run(app: &crate::app::App, input: &Value) -> Result<Value, String> {
    if input["confirmed"].as_bool() != Some(true) {
        return Err("la désinstallation n'a pas été confirmée".into());
    }
    let keep_models = input["keepModels"].as_bool().unwrap_or(false);
    let settings = app.config.settings();
    // Sauvegarde demandée : elle doit exister, écrite et relue avec succès, récemment. Sinon rien ne part.
    if input["backup"].as_bool().unwrap_or(true) && backup::recent_backup(Duration::from_secs(30 * 60)).is_none() {
        return Err("fais d'abord la sauvegarde : aucune sauvegarde relue avec succès n'a été trouvée".into());
    }
    let (env, plan) = current_plan(&settings, keep_models);
    if let Some(reason) = &plan.blocked {
        return Err(reason.clone());
    }
    if plan.items.is_empty() {
        return Err("rien à désinstaller".into());
    }
    if let Some(problem) = uninstall::capacity_problem(&plan) {
        return Err(problem);
    }
    let job = Job {
        exe: env.exe.clone(),
        keep_models,
        pid: std::process::id(),
        external_models: vec![settings.whisper_model_path.clone(), settings.llama_cpp_model.clone()],
        startup_present: plan.startup,
    };
    app.log.write("désinstallation: confirmée, arrêt des moteurs et lancement de l'assistant");
    app.whisper.stop();
    app.llama.stop();
    // Plus aucune écriture (historique, réglages, journal) : le dossier de données part à la Corbeille.
    scriibe_platform::storage::set_frozen(true);
    if let Err(e) = uninstall::start_helper(&job) {
        scriibe_platform::storage::set_frozen(false);
        return Err(e);
    }
    Ok(json!({"ok": true}))
}
