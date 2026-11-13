# Système : désinstallation et lancement au démarrage

Chantier 7 (tri du 12/11/2026). Deux idées du carnet (11/11) : prévoir la désinstallation de Scriibe, et une option de lancement automatique au démarrage de l'ordinateur. Cette note réunit le constat (tâche 0), le cadrage (tâche 1), la relecture (tâche 2), puis les résultats du code et des essais.

## Constat

Établi le 13/11/2026 en lecture seule par un sous-agent de recherche ; points clés revérifiés dans le code (`data_folder`, `engines_folder`, service du trousseau, absence de lien symbolique, de suppression de clé et de démarrage automatique). Chemins relatifs à `crates/` sauf mention contraire.

Aucune crate `dirs`, `directories` ni `keyring` : les chemins sont calculés à la main depuis `HOME`, `APPDATA`, `LOCALAPPDATA`, `XDG_DATA_HOME`. `cli/tauri.conf.json` : `"bundle": {"active": false}`, identifiant `fr.scriibe.app`. Pas de plugin Tauri de démarrage automatique.

### Dossiers racines par système

| | Données | Moteurs téléchargés |
|---|---|---|
| Fonction | `platform/src/storage.rs:16` `data_folder` | `engines/src/lib.rs:25` `engines_folder` |
| macOS | `~/Library/Application Support/Scriibe` | `~/Library/Application Support/Scriibe/engines` (en pratique vide : `whisper-server` et `llama-server` viennent de Homebrew, `/opt/homebrew/bin`) |
| Windows | `%APPDATA%\Scriibe` (Roaming) | `%LOCALAPPDATA%\Scriibe\engines` (Local : **autre dossier** que les données) |
| Linux | `$XDG_DATA_HOME/scriibe`, sinon `~/.local/share/scriibe` (minuscule) | `$XDG_DATA_HOME/Scriibe/engines`, sinon `~/.local/share/Scriibe/engines` (**S majuscule** : autre dossier, `docs/PIEGES.md`) |

`SCRIIBE_DATA` remplace le dossier de données (essais) ; il ne déplace pas les moteurs.

### Fichiers du dossier de données (les mêmes sur les trois systèmes)

| Élément | Fichier | Écrit par | Remarque |
|---|---|---|---|
| Réglages, modes personnels, chemins de modèles choisis | `settings.json` (`settings.tmp` pendant l'écriture) | `platform/src/storage.rs` `save_settings`, `write_atomic` | Les modes personnels sont dans ce fichier (`docs/CADRAGE_PROFILS.md`). |
| Dictionnaire | `dictionary.json` | `storage.rs` `save_dictionary` | |
| Historique | `history.json` | `storage.rs` `persist` | Sensible : textes dictés, noms d'applis. |
| Totaux par jour | `history-totals.json` | `storage.rs` `persist` | Sans texte. |
| Banc de dictée | `bancs/banc-<date>-<id>.json` | `platform/src/bench_store.rs` `standard_folder` | |
| Journal technique | `scriibe.log` | `storage.rs` `FileLog::new` | Ni texte ni clé ; tronqué au-delà de 512 Ko. |
| Journal de llama-server | `llama-server.log` | `cli/src/app.rs:69` | Effacé au lancement au-delà de 2 Mo. |
| Verrou d'instance | `instance.lock`, `instance.url` | `platform/src/instance.rs` `acquire` | `instance.url` supprimé à l'arrêt. Le verrou dit si Scriibe tourne. |
| Prise de test du micro | `essai-micro.wav` | `cli/src/commands.rs` | Seulement avec `scriibe micro`. |
| Modèles téléchargés | `models/` (`ggml-*.bin`, `*.gguf`, `.part` en cours) | `cli/src/app.rs` `models_folder`, `cli/src/web/downloads.rs` `fetch` | Vrais fichiers, parfois plusieurs Go. |
| Ancien réglage | `moteurs.json` | `app.rs` `migrate_engine_options` | Lu puis supprimé. |
| Jeton du portail (Linux Wayland) | `portail-wayland.json` | `platform/src/linux/wayland.rs` | Linux seulement. |

### Modèles hors du dossier de Scriibe

- **Aucun lien symbolique ni copie** : aucun `symlink` ni `hard_link` dans `crates/`.
- Scriibe **enregistre seulement un chemin** dans `settings.json` : `whisper_model_path` et `llama_cpp_model` (`core/src/settings.rs`), choisis par la boîte de dialogue (`cli/src/web/mod.rs` `pick_file`). Supprimer `settings.json` efface le chemin, jamais le fichier visé.
- La recherche automatique ne parcourt que `models/` (`app.rs` `detect_models`) : `~/.lmstudio/models` n'est pas exploré (Scribe, lui, l'explore).
- La suppression d'un modèle depuis l'interface est déjà limitée à `models/` (`cli/src/web/hub.rs` `delete_installed`, testé hors dossier).
- Le dossier de modèles partagé avec le Magasin de modèles n'existe pas encore (`ROADMAP.md`, ○). S'il arrive avec des liens, la désinstallation devra retirer le lien sans suivre sa cible.

### Hors du dossier de données

- **Sons temporaires (macOS, Linux)** : `$TMPDIR/scriibe-son-<pid>-<nom>-<volume>.wav` (`platform/src/macos/sounds.rs`, `linux/sounds.rs`), jamais supprimés par Scriibe. Windows joue les sons depuis la mémoire, sans fichier.
- **Audio de dictée** : aucun fichier écrit (`platform/src/recorder.rs`, en mémoire).
- **Lanceur Linux (Wayland seulement)** : `~/.local/share/applications/fr.scriibe.Scriibe.desktop` (`Exec="<exe>" lancer`) et l'icône `~/.local/share/icons/hicolor/scalable/apps/fr.scriibe.Scriibe.svg` (`linux/wayland.rs` `ensure_desktop_file`). C'est un lanceur d'applications, pas un démarrage automatique.
- **Mise à jour** : macOS, dossier caché `.Scriibe-mise-a-jour` et `Scriibe.app.old` à côté de l'app (`cli/src/web/update.rs` `install_app`, `swap_bundle`, nettoyés par `remove_leftovers` au lancement suivant) ; Windows, `scriibe.exe.new` et `scriibe.exe.old` à côté du programme (`update.rs` `replace_program`) ; Linux, alerte seulement.
- **Caches de la fenêtre (WebView)** : la fenêtre charge l'adresse locale sans `data_directory` (`cli/src/window.rs`) ; l'emplacement vient des valeurs par défaut de Tauri/wry, **non vérifiées** (probablement sous `~/Library/WebKit/fr.scriibe.app` et `~/Library/Caches/fr.scriibe.app` sur Mac, `%LOCALAPPDATA%\fr.scriibe.app` sous Windows). La langue est aussi gardée dans le stockage de la page (`ui/src/lib/i18n.svelte.ts`). À mesurer sur une vraie machine.
- **Registre Windows, plist, LaunchAgent, `~/.config`** : rien d'écrit par le code.

### Clés d'API

| Système | Où | Fichiers et fonctions | Repli |
|---|---|---|---|
| macOS | Trousseau, mot de passe générique, service `Scriibe`, comptes `deepgram` et `openrouter` | `platform/src/macos/credentials.rs` (`SERVICE`, `KeychainSecrets::get`, `set_credential`), crate `security-framework` | Variables `DEEPGRAM_API_KEY`, `OPENROUTER_API_KEY` (`secrets.rs` `EnvSecrets`) |
| Windows | Gestionnaire d'identifiants, identifiants génériques `Scriibe/deepgram` et `Scriibe/openrouter`, persistance `CRED_PERSIST_LOCAL_MACHINE` | `platform/src/windows/credentials.rs` (`target`, `set_credential`, lecture `CredReadW`) | Variables d'environnement |
| Linux | Aucun coffre : variables d'environnement seulement | `platform/src/linux/mod.rs` `EnvSecrets` ; `save_secret` renvoie une erreur (`platform/src/lib.rs`) | Aucun fichier clair écrit |

- **Aucune fonction de suppression de clé** (ni `SecItemDelete` ni `CredDeleteW`) : à créer pour la désinstallation.
- Un ancien compte `github` (jeton abandonné en S14, `docs/MISE_A_JOUR.md`) peut subsister dans le trousseau ou le Gestionnaire d'identifiants : le code ne le gère plus.

### Installation par système

| Système | Format et emplacement | Désinstalleur |
|---|---|---|
| macOS | `Scriibe-x.y.z-macos.zip` contenant `Scriibe.app` (`tools/publier_macos.sh`) ; le guide fait glisser l'app dans Applications (`docs/INSTALLATION_MACOS.md`) ; `tools/faire_app_macos.sh` installe dans `~/Applications`. Signature stable, identifiant `fr.scriibe.app`. | Aucun |
| Windows | `scriibe-windows-x64.zip` contenant `scriibe.exe` seul (`.github/workflows/release-windows.yml`), extrait à la main, conseillé dans `C:\scriibe` (`docs/INSTALLATION_WINDOWS.md`). Ni NSIS, ni MSI. | Aucun (pas d'entrée dans « Applications installées ») |
| Linux | `scriibe-linux-x64.zip` contenant `scriibe` (`release-linux.yml`), conseillé dans `~/scriibe` (`docs/INSTALLATION_LINUX.md`). Pas d'AppImage ni de `.deb`. | Aucun |

L'emplacement du programme n'est imposé nulle part : une désinstallation doit partir de l'exécutable en cours (`current_exe`, variante dans `update.rs` `program_path`).

### Lancement au démarrage

**Rien n'existe** : ni plugin, ni LaunchAgent, ni `SMAppService`, ni clé `Run`, ni `~/.config/autostart` (recherche dans `crates/`, `ui/src`, `docs/`, `packaging/`, `.github/`, `ROADMAP.md`, `CHANGELOG.md`). Points d'appui :
- `scriibe lancer --sans-navigateur` démarre sans ouvrir la fenêtre (`docs/ESSAI_WINDOWS.md`) : adapté à un démarrage discret.
- `LSUIElement` vrai sur Mac (`packaging/macos/Info.plist`) : pas d'icône dans le Dock.
- `docs/INSTALLATION_LINUX.md` dit de relancer Scriibe à la main après un redémarrage.

### Autorisations macOS

Micro, Surveillance de l'entrée (`platform/src/macos/ffi.rs` `CGPreflightListenEventAccess`) et Accessibilité (`AXIsProcessTrusted`), accordées à `Scriibe.app` (`fr.scriibe.app`) lancée par `open` ; le trousseau demande aussi l'accès une fois par clé (`docs/PIEGES.md`). Aucun appel à `tccutil` dans le code ; le dépôt ne dit rien de leur retrait à la désinstallation.

### Scribe (référence Swift)

- Données dans `~/Library/Application Support/Scribe/` : mêmes fichiers (`Sources/Scribe/Store.swift`, `DiagnosticLog.swift`, `BenchArchive.swift`).
- Clés : trousseau, service à l'identifiant de l'app de Scribe, comptes `deepgram` et `openrouter` (`Keys.swift`) ; `Keys.remove` existe (`SecItemDelete`) mais ne sert pas à une désinstallation. Repli : `.env` du projet.
- Modèles : dossier `models/` du projet puis `~/.lmstudio/models`, **explorés sans copie ni lien** (`LlamaServer.swift` `defaultFolders`).
- Installation par `scripts/build.sh` dans `~/Applications/Scribe.app`.
- **Ni désinstallation, ni lancement à l'ouverture de session** : rien à reprendre, écart à noter dans `docs/ECARTS.md` quand le code arrivera.

### À vérifier sur une vraie machine

- Emplacement réel des caches WebView (WKWebView, WebView2, WebKitGTK) et du stockage de la page.
- `whisper-server` écrit-il un journal ? Seul `llama-server.log` est créé par Scriibe.
- Windows : traces éventuelles de l'icône de la zone de notification (`windows/tray.rs`).
- Linux : autorisation « RemoteDesktop » mémorisée côté portail GNOME (hors de Scriibe).

## Cadrage

À venir (tâche 1).

## Relecture

À venir (tâche 2).
