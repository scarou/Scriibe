use crate::app::{self, build, describe_engines, detect_whisper_models, models_folder, Choices};
use crate::overlay::{OverlayIndicator, OverlayModel};
use crate::web::{SwitchHistory, SwitchPaster, TeeIndicator, TestSinks};
use scriibe_core::microphone::MicLevel;
use scriibe_core::settings::{CleanupEngine, TranscriptionEngine};
use scriibe_engines::fs::RealFs;
use scriibe_engines::llama;
use scriibe_engines::runtime::{KeyKind, Log};
use scriibe_engines::server::Launcher;
use scriibe_platform::indicator::{ConsoleIndicator, Indicator};
use scriibe_platform::instance::{self, Instance, InstanceLock};
use scriibe_platform::paste::{PasteOutcome, Paster};
use scriibe_platform::storage::{FileHistory, FileLog, HistorySink};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn setup(echo_log: bool) -> Result<(Arc<scriibe_platform::storage::FileConfig>, Arc<FileLog>), String> {
    let config = app::open_config();
    // Une sauvegarde à restaurer (préparée avant la relance) est appliquée avant toute lecture du dossier.
    let restored = scriibe_platform::restore::apply_pending(config.folder());
    if config.create_settings_if_missing().map_err(|e| format!("impossible d'écrire {} ({e})", config.settings_path().display()))? {
        eprintln!("Premier lancement : réglages créés dans {}", config.settings_path().display());
    }
    app::migrate_engine_options(&config);
    let log = Arc::new(FileLog::new(config.folder(), echo_log));
    if let Some(message) = restored {
        log.write(&message);
    }
    Ok((config, log))
}

/// `essai` ne colle rien : le texte est affiché.
struct ConsolePaster;

impl Paster for ConsolePaster {
    fn paste(&self, text: &str, _: bool) -> PasteOutcome {
        println!("\n--- texte qui serait collé ---\n{text}\n------------------------------");
        PasteOutcome::Pasted
    }
}

/// `essai` ne garde rien dans l'historique.
struct NoHistory;

impl HistorySink for NoHistory {
    fn record(&self, _: scriibe_core::history::Dictation) {}
}

/// Ajouté par la mise à jour au programme qu'elle relance : l'ancien n'a peut-être pas encore libéré le verrou d'instance.
pub const RESTARTED: &str = "--relance";

/// Prend le verrou d'instance. `None` : une autre Scriibe tourne déjà sur ce dossier de données, elle a été priée de montrer sa fenêtre.
fn single_instance(folder: &std::path::Path) -> Option<InstanceLock> {
    let wait = if std::env::args().any(|a| a == RESTARTED) { Duration::from_secs(15) } else { Duration::ZERO };
    let deadline = Instant::now() + wait;
    loop {
        match instance::acquire(folder) {
            Instance::First(lock) => return Some(lock),
            Instance::Running if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(200)),
            Instance::Running => break,
        }
    }
    eprintln!("Scriibe tourne déjà : sa fenêtre est remise au premier plan.");
    instance::allow_foreground();
    match instance::running_url(folder, Duration::from_secs(5)) {
        Some(url) => {
            if let Err(e) = ureq::post(&format!("{url}api/instance/show")).timeout(Duration::from_secs(5)).send_string("{}") {
                eprintln!("L'autre Scriibe n'a pas répondu ({e}) : ferme-la depuis l'icône de la zone de notification.");
            }
        }
        None => eprintln!("L'autre Scriibe n'a pas annoncé son adresse : ferme-la depuis l'icône de la zone de notification."),
    }
    None
}

pub fn run() -> Result<(), String> {
    // Démarrage discret (entrée de lancement à l'ouverture de session) : sous Windows, pas de console noire (la fermer tuerait Scriibe).
    if std::env::args().any(|a| a == "--sans-navigateur") {
        scriibe_platform::autostart::detach_console();
    }
    crate::web::remove_update_leftovers();
    let (config, log) = setup(true)?;
    let Some(instance_lock) = single_instance(config.folder()) else { return Ok(()) };
    // Scriibe déplacée : l'entrée de démarrage, si elle existe, est réécrite pour viser le nouvel emplacement.
    if scriibe_platform::autostart::refresh() {
        log.write("démarrage: entrée réécrite (Scriibe a changé d'emplacement)");
    }
    let platform = scriibe_platform::system();
    let settings = config.settings();
    eprintln!("Scriibe — {}", describe_engines(&settings));
    eprintln!("Données : {}", config.folder().display());

    // L'adresse de la page d'essais n'est connue qu'une fois son serveur démarré ; l'icône la lit au moment du clic.
    let page_url: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    // Ce que le menu de l'icône montre et fait : connu une fois l'appli construite.
    #[cfg(any(windows, target_os = "macos", target_os = "linux"))]
    let menu_source: Arc<Mutex<Option<Arc<dyn scriibe_platform::tray_menu::TrayMenuSource>>>> = Arc::new(Mutex::new(None));
    #[cfg(windows)]
    let (host, real_indicator): (_, Arc<dyn Indicator>) = {
        let english = settings.ui_language.resolve(&scriibe_platform::locale::system_language()) == scriibe_core::settings::UiLang::En;
        let tip = if english {
            "Scriibe — hold Right Ctrl to dictate (+ Shift: translate)"
        } else {
            "Scriibe — maintiens Ctrl droit pour dicter (+ Maj : traduire)"
        };
        let (host, indicator) = scriibe_platform::windows::tray::TrayHost::create(
            tip,
            config.folder().to_path_buf(),
            log.path().to_path_buf(),
            page_url.clone(),
            menu_source.clone(),
        )?;
        (host, Arc::new(indicator))
    };
    // macOS : l'icône est créée par Tauri sur le fil principal, plus tard ; l'indicateur et elle se partagent `tray_state`.
    #[cfg(target_os = "macos")]
    let (tray_state, real_indicator): (_, Arc<dyn Indicator>) = {
        let english = settings.ui_language.resolve(&scriibe_platform::locale::system_language()) == scriibe_core::settings::UiLang::En;
        let tip = if english {
            "Scriibe — hold Right Option to dictate (+ ⌘: translate)"
        } else {
            "Scriibe — maintiens Option droite pour dicter (+ ⌘ : traduire)"
        };
        let (state, indicator) = crate::mac_tray::indicator(tip);
        (state, Arc::new(indicator))
    };
    // Linux : comme macOS, l'icône (barre du haut, AppIndicator) est créée par Tauri sur le fil principal.
    #[cfg(target_os = "linux")]
    let (tray_state, real_indicator): (_, Arc<dyn Indicator>) = {
        let english = settings.ui_language.resolve(&scriibe_platform::locale::system_language()) == scriibe_core::settings::UiLang::En;
        let tip = if english { "Scriibe — voice dictation" } else { "Scriibe — dictée vocale" };
        let (state, indicator) = crate::mac_tray::indicator(tip);
        (state, Arc::new(indicator))
    };
    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    let real_indicator: Arc<dyn Indicator> = Arc::new(ConsoleIndicator);

    // L'état de l'indicateur à l'écran suit l'icône : tout ce qui est montré à l'une l'est à l'autre.
    let overlay = Arc::new(OverlayModel::default());
    let real_indicator: Arc<dyn Indicator> = Arc::new(OverlayIndicator { inner: real_indicator, model: overlay.clone() });

    let settings_source = config.clone();
    let history = Arc::new(FileHistory::open(config.folder().to_path_buf(), move || settings_source.settings()));
    let sinks = TestSinks::new(platform.paster.clone(), history.clone(), real_indicator);
    let choices = Choices {
        paster: Some(Arc::new(SwitchPaster(sinks.clone()))),
        history: Some(Arc::new(SwitchHistory(sinks.clone()))),
        indicator: Arc::new(TeeIndicator(sinks.clone())),
    };
    let app = Arc::new(build(config, log, platform, choices));
    #[cfg(any(windows, target_os = "macos", target_os = "linux"))]
    if let Ok(mut slot) = menu_source.lock() {
        *slot = Some(Arc::new(crate::tray_source::TraySource {
            config: app.config.clone(),
            history: history.clone(),
            paster: app.platform.paster.clone(),
            version: crate::web::build_id(),
        }));
    }

    let (sink, events) = std::sync::mpsc::channel();
    let keys = app.platform.keys.start(sink);
    let keys_error = keys.as_ref().err().cloned();
    if let Some(e) = &keys_error {
        eprintln!("Touche globale indisponible : {e}");
    }
    let controller = app.controller.clone();
    let (whisper, llama) = (app.whisper.clone(), app.llama.clone());
    std::thread::Builder::new()
        .name("scriibe-evenements".into())
        .spawn(move || {
            event_loop(controller, events, move || {
                whisper.release_if_idle();
                llama.release_if_idle();
            })
        })
        .map_err(|e| e.to_string())?;

    let (_context, url) = crate::web::start(app.clone(), history, sinks, overlay.clone(), keys_error)?;
    if let Ok(mut slot) = page_url.lock() {
        *slot = Some(url.clone());
    }
    instance_lock.publish(&url);
    eprintln!("Interface : {url}");
    // macOS et Linux : Tauri prend le fil principal pour de bon (fenêtre, icône, pastille sur Mac) et ne le rend pas avant « Quitter ».
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        app.controller.preload();
        #[cfg(target_os = "macos")]
        eprintln!("Prêt : maintiens Option droite, parle, relâche. Icône dans la barre des menus (menu : ouvrir Scriibe, quitter).");
        #[cfg(target_os = "linux")]
        {
            let key = scriibe_platform::linux::wayland_trigger()
                .map_or_else(|| "Ctrl droit".to_string(), |t| format!("{t} (raccourci de GNOME)"));
            eprintln!("Prêt : maintiens {key}, parle, relâche. Icône dans la barre du haut (menu : ouvrir Scriibe, quitter).");
        }
        let quitting = app.clone();
        let setup = crate::window::MainSetup {
            url: url.clone(),
            show_window: !std::env::args().any(|a| a == "--sans-navigateur" || a == "--navigateur"),
            log: app.log.clone() as Arc<dyn Log>,
            overlay: overlay.clone(),
            config: app.config.clone(),
            tray: crate::mac_tray::Shell {
                state: tray_state,
                source: menu_source,
                page_url: page_url.clone(),
                data_folder: app.config.folder().to_path_buf(),
                log_file: app.log.path().to_path_buf(),
                quit: Arc::new(move || {
                    quitting.whisper.stop();
                    quitting.llama.stop();
                    std::process::exit(0);
                }),
            },
        };
        if std::env::args().any(|a| a == "--navigateur") {
            scriibe_platform::open::open(&url);
        }
        if let Err(e) = crate::window::run_main(setup) {
            // Sans la boucle de Tauri il n'y a ni fenêtre ni icône : l'interface reste atteignable dans le navigateur.
            app.log.write(&format!("fenêtre: impossible ({e}) : ouverture dans le navigateur"));
            scriibe_platform::open::open(&url);
            loop {
                std::thread::park();
            }
        }
        drop(keys);
        Ok(())
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        if !std::env::args().any(|a| a == "--sans-navigateur") {
            open_interface(&url, &app, &overlay);
        }
        app.controller.preload();
        if cfg!(windows) {
            eprintln!("Prêt : maintiens Ctrl droit, parle, relâche. Icône dans la zone de notification (menu : ouvrir Scriibe, quitter).");
        } else {
            eprintln!("Prêt : maintiens Ctrl droit, parle, relâche. Pas d'icône sur ce système pour l'instant : l'interface s'ouvre dans le navigateur (adresse ci-dessus).");
        }

        #[cfg(windows)]
        {
            host.run();
            app.whisper.stop();
            app.llama.stop();
            drop(keys);
            Ok(())
        }
        #[cfg(not(windows))]
        loop {
            std::thread::park();
        }
    }
}

/// Ouvre l'interface : dans la fenêtre de Scriibe (Windows) ou, si elle ne peut pas s'ouvrir ou sur demande (`--navigateur`), dans le
/// navigateur. (macOS : voir `window::run_main`.)
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn open_interface(url: &str, app: &app::App, overlay: &Arc<OverlayModel>) {
    #[cfg(windows)]
    if !std::env::args().any(|a| a == "--navigateur") {
        match crate::window::start(url.to_string(), app.log.clone() as Arc<dyn Log>) {
            Ok(()) => {
                crate::window::start_overlay(url.to_string(), overlay.clone(), app.config.clone(), app.log.clone() as Arc<dyn Log>);
                return;
            }
            Err(e) => app.log.write(&format!("fenêtre: impossible ({e}) : ouverture dans le navigateur")),
        }
    }
    let _ = (app, overlay);
    scriibe_platform::open::open(url);
}

/// Les événements du clavier, une à deux fois par seconde de quoi surveiller le micro, et toutes les 30 s de quoi libérer les moteurs.
fn event_loop(
    controller: scriibe_pipeline::Controller,
    events: std::sync::mpsc::Receiver<scriibe_platform::keys::KeyEvent>,
    mut release_idle: impl FnMut(),
) {
    let mut last_release = std::time::Instant::now();
    loop {
        match events.recv_timeout(Duration::from_millis(100)) {
            Ok(event) => controller.handle_key(event),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return,
        }
        controller.tick();
        if last_release.elapsed() >= Duration::from_secs(30) {
            last_release = std::time::Instant::now();
            release_idle();
        }
    }
}

pub fn microphone(seconds: f64) -> Result<(), String> {
    let platform = scriibe_platform::system();
    let microphones = platform.recorder.microphones();
    if microphones.is_empty() {
        println!("Aucun micro détecté par le système.");
    }
    for m in &microphones {
        println!("  micro : {}", m.label());
    }
    let info = platform.recorder.start("")?;
    println!("Enregistrement de {seconds:.0} s sur : {} — parle maintenant…", info.description);
    std::thread::sleep(Duration::from_secs_f64(seconds.max(0.5)));
    let recording = platform.recorder.stop();
    let verdict = match MicLevel::assess(recording.level) {
        MicLevel::Silent => "muet : rien n'est capté (mauvais micro choisi ? micro coupé ? autorisation du système ?)",
        MicLevel::Weak => "faible : rapproche-toi du micro ou monte le volume d'entrée",
        MicLevel::Ok => "correct",
    };
    println!("Durée {:.1} s — niveau {:.3} ({verdict})", recording.duration, recording.level);
    let path = scriibe_platform::storage::data_folder().join("essai-micro.wav");
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    std::fs::write(&path, &recording.wav).map_err(|e| format!("impossible d'écrire {} ({e})", path.display()))?;
    println!("Prise gardée dans {} (réécoute-la, ou lance : scriibe essai)", path.display());
    Ok(())
}

pub fn trial(args: &[String]) -> Result<(), String> {
    let translate = args.iter().any(|a| a == "--traduire");
    let file: PathBuf = args
        .iter()
        .find(|a| !a.starts_with("--"))
        .map(PathBuf::from)
        .unwrap_or_else(|| scriibe_platform::storage::data_folder().join("essai-micro.wav"));
    let bytes = std::fs::read(&file).map_err(|e| format!("impossible de lire {} ({e})", file.display()))?;
    let recording = scriibe_platform::audio::read_wav(&bytes)?;
    println!("Fichier {} — {:.1} s, niveau {:.3}", file.display(), recording.duration, recording.level);
    let (config, log) = setup(true)?;
    let app = build(
        config,
        log,
        scriibe_platform::system(),
        Choices { paster: Some(Arc::new(ConsolePaster)), history: Some(Arc::new(NoHistory)), indicator: Arc::new(ConsoleIndicator) },
    );
    let settings = app.config.settings();
    println!("{}", describe_engines(&settings));
    // La première dictée charge les modèles : les temps suivants sont ceux d'un usage normal. On passe donc deux fois.
    for pass in 1..=2 {
        println!("\n=== passage {pass}{} ===", if pass == 1 { " (chargement des modèles compris)" } else { " (modèles chargés)" });
        app.controller.process_recording(recording.clone(), translate, "fichier");
    }
    app.whisper.stop();
    app.llama.stop();
    println!("\nLe détail des temps est dans {}", app.log.path().display());
    Ok(())
}

/// `scriibe desinstaller [--liste] [--garder-les-modeles]` : la liste exacte de ce qui partirait ; sans `--liste`, demande « oui » puis lance
/// l'assistant (tout se fait après la sortie de ce programme). Pas de sauvegarde ici : elle passe par la fenêtre.
pub fn uninstall(args: &[String]) -> Result<(), String> {
    use scriibe_platform::uninstall::{self, human_size, Kind};
    let list_only = args.iter().any(|a| a == "--liste");
    let keep_models = args.iter().any(|a| a == "--garder-les-modeles");
    if let Some(unknown) = args.iter().find(|a| !matches!(a.as_str(), "--liste" | "--garder-les-modeles")) {
        return Err(format!("option inconnue : « {unknown} » (usage : scriibe desinstaller [--liste] [--garder-les-modeles])"));
    }
    let folder = scriibe_platform::storage::data_folder();
    let settings = scriibe_platform::storage::FileConfig::new(folder.clone()).settings();
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let env = uninstall::Env::real(exe, keep_models);
    let external = [settings.whisper_model_path.clone(), settings.llama_cpp_model.clone()];
    let startup = scriibe_platform::autostart::status();
    let plan = uninstall::build_plan(&env, &external, startup.enabled || startup.disabled_by_system);
    if let Some(reason) = &plan.blocked {
        return Err(reason.clone());
    }
    println!("Ce qui part à la Corbeille ({}) :", human_size(plan.trash_size()));
    for item in plan.items.iter().filter(|i| i.kind != Kind::TempSound) {
        let what = match item.kind {
            Kind::Data => "données",
            Kind::Engines => "moteurs",
            Kind::Cache => "cache de la fenêtre",
            Kind::Launcher => "lanceur",
            Kind::Leftover => "reste de mise à jour",
            Kind::Program => "programme",
            Kind::TempSound => "",
        };
        let link = if item.link { " [lien : sa cible reste en place]" } else { "" };
        println!("  • {what} : {} ({}){link}", item.path.display(), human_size(item.size));
    }
    let sounds: Vec<_> = plan.items.iter().filter(|i| i.kind == Kind::TempSound).collect();
    if !sounds.is_empty() {
        println!(
            "  • {} son(s) temporaire(s) « scriibe-son-*.wav » du dossier temporaire ({}, effacés directement)",
            sounds.len(),
            human_size(sounds.iter().map(|i| i.size).sum())
        );
    }
    if plan.startup {
        println!("  • entrée de démarrage (retirée)");
    }
    println!(
        "Clés d'API : effacées du trousseau ou du Gestionnaire d'identifiants (deepgram, openrouter, ancien github), jamais affichées."
    );
    for (kind, path, _) in &plan.kept {
        println!("Reste en place : {} ({kind})", path.display());
    }
    for warning in &plan.warnings {
        println!("Attention : {warning}");
    }
    if list_only {
        println!("(--liste : rien n'a été touché.)");
        return Ok(());
    }
    if plan.items.is_empty() {
        return Err("rien à désinstaller".into());
    }
    println!(
        "\nAttention : cette commande ne propose pas de sauvegarde (elle passe par le bouton « Désinstaller Scriibe » de la fenêtre)."
    );
    let lock = match instance::acquire(&folder) {
        Instance::First(lock) => lock,
        Instance::Running => return Err("Scriibe tourne : quitte-la d'abord (icône → Quitter), puis recommence.".into()),
    };
    println!("Tape « oui » pour désinstaller (tout part à la Corbeille, récupérable tant qu'elle n'est pas vidée) :");
    let mut answer = String::new();
    std::io::stdin().read_line(&mut answer).map_err(|e| e.to_string())?;
    if answer.trim() != "oui" {
        return Err("abandon : rien n'a été supprimé.".into());
    }
    let job = uninstall::Job {
        exe: env.exe.clone(),
        keep_models,
        pid: std::process::id(),
        external_models: external.to_vec(),
        startup_present: plan.startup,
    };
    scriibe_platform::storage::set_frozen(true);
    uninstall::start_helper(&job)?;
    println!("L'assistant termine après la fin de cette commande ; un message dira ce qui est parti.");
    drop(lock);
    Ok(())
}

pub fn key(kind: Option<&str>) -> Result<(), String> {
    let kind = match kind {
        Some("deepgram") => KeyKind::Deepgram,
        Some("openrouter") => KeyKind::OpenRouter,
        _ => return Err("usage : scriibe cle deepgram|openrouter (puis colle la clé et valide)".into()),
    };
    if !cfg!(any(windows, target_os = "macos")) {
        return Err(format!(
            "pas encore pris en charge ici : définis la variable d'environnement {}",
            scriibe_platform::secrets::env_name(kind)
        ));
    }
    eprintln!("Colle la clé puis valide (elle s'affiche pendant la saisie, efface la fenêtre ensuite) :");
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).map_err(|e| e.to_string())?;
    let store = scriibe_platform::save_secret(kind, &line)?;
    eprintln!("Clé enregistrée dans le {store} (Scriibe/{}).", scriibe_platform::secrets::account_name(kind));
    Ok(())
}

pub fn check() -> Result<(), String> {
    let (config, log) = setup(false)?;
    let platform = scriibe_platform::system();
    let settings = config.settings();
    let fs = RealFs;
    let folder = config.folder().to_path_buf();
    let models = models_folder(&folder);
    let mark = |ok: bool| if ok { "✔" } else { "✘" };

    println!("Données        : {}", folder.display());
    println!("Réglages       : {}", config.settings_path().display());
    println!("Modèles        : {} (dépose ici les fichiers ggml-*.bin et *.gguf)", models.display());
    println!("Journal        : {}", log.path().display());
    if let Some(engines) = scriibe_engines::engines_folder() {
        println!(
            "Moteurs        : {} (sous-dossiers whisper\\ et llama\\ : le programme et ses DLL ; script : tools/windows/installer-moteurs.ps1)",
            engines.display()
        );
    }
    println!("{}\n", describe_engines(&settings));

    let path_var = std::env::var_os("PATH");
    let whisper_launcher =
        scriibe_engines::whisper::WhisperLauncher::new(Arc::new(RealFs), scriibe_engines::whisper::PORT, path_var.as_deref());
    let llama_launcher = llama::LlamaLauncher::new(Arc::new(RealFs), llama::PORT, None, path_var.as_deref());

    let wants_whisper = settings.transcription_engine == TranscriptionEngine::Whisper;
    let whisper_binary = whisper_launcher.binary();
    println!(
        "{} whisper-server : {}",
        mark(whisper_binary.is_some()),
        whisper_binary.map_or_else(|| format!("introuvable — {}", scriibe_engines::whisper::install_hint()), |p| p.display().to_string())
    );
    let whisper_models = if settings.whisper_model_path.is_empty() {
        detect_whisper_models(&fs, &models)
    } else {
        vec![settings.whisper_model_path.clone()]
    };
    println!(
        "{} modèle Whisper : {}",
        mark(!whisper_models.is_empty()),
        whisper_models.first().cloned().unwrap_or_else(|| "aucun (ggml-small.bin, ggml-large-v3-turbo.bin…)".into())
    );

    let wants_llama = settings.cleanup_engine == CleanupEngine::LlamaCpp;
    let llama_binary = llama_launcher.binary();
    println!(
        "{} llama-server   : {}",
        mark(llama_binary.is_some()),
        llama_binary.map_or_else(|| format!("introuvable — {}", llama::install_hint()), |p| p.display().to_string())
    );
    let llama_models = if settings.llama_cpp_model.is_empty() {
        llama::detect_models(&fs, std::slice::from_ref(&models), Some(&models))
    } else {
        vec![settings.llama_cpp_model.clone()]
    };
    println!("{} modèle GGUF    : {}", mark(!llama_models.is_empty()), llama_models.first().cloned().unwrap_or_else(|| "aucun".into()));
    if let Some(first) = llama_models.first() {
        println!(
            "    assistant MTP : {}",
            llama::assistant(&fs, first).unwrap_or_else(|| "aucun (le modèle tourne sans décodage spéculatif)".into())
        );
    }

    for (kind, name) in [(KeyKind::Deepgram, "Deepgram"), (KeyKind::OpenRouter, "OpenRouter")] {
        let present = platform.secrets.get(kind).is_some();
        println!(
            "{} clé {name} : {}",
            mark(present),
            if present { "présente" } else { "absente (inutile si tout tourne sur cet ordinateur)" }
        );
    }

    let microphones = platform.recorder.microphones();
    println!("{} micros : {}", mark(!microphones.is_empty()), microphones.iter().map(|m| m.label()).collect::<Vec<_>>().join(" | "));

    let mut missing = Vec::new();
    if wants_whisper && (whisper_launcher.binary().is_none() || whisper_models.is_empty()) {
        missing.push("Whisper (programme ou modèle)");
    }
    if wants_llama && (llama_launcher.binary().is_none() || llama_models.is_empty()) {
        missing.push("llama.cpp (programme ou modèle)");
    }
    if missing.is_empty() {
        println!("\nTout ce que les réglages demandent est là.");
    } else {
        println!(
            "\nIl manque : {}. Les réglages sont dans le fichier ci-dessus (transcriptionEngine, cleanupEngine…).",
            missing.join(", ")
        );
    }
    log.write("verifier: exécuté");
    Ok(())
}
