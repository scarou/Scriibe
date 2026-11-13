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

Cadrage fait le 13/11/2026 (tâche 1). Les questions 1, 2, 3, 5 et 6 portent sur l'usage, la sécurité ou l'irréversible : l'auteur les a tranchées, chaque fois sur l'option recommandée. Les questions 4 et 7, et les mécanismes techniques, sont des choix techniques : je les ai pris en donnant mes raisons.

### 1. D'où se lance la désinstallation (décision de l'auteur)

**Un bouton « Désinstaller Scriibe » au bas de l'onglet Réglages**, le même sur Mac, Windows et Linux, plus une commande `scriibe desinstaller` pour le dépannage.

- Pourquoi : aucun système n'a d'installateur (simple zip), le programme peut être n'importe où. Le programme en cours sait où il est (`current_exe`), connaît ses dossiers et peut proposer la sauvegarde. Un seul code pour les trois systèmes.
- Écartés : créer un vrai installateur et son désinstalleur par système (gros chantier d'empaquetage, rien d'équivalent sur Mac) ; une procédure manuelle dans l'Aide (un dossier oublié est vite arrivé).
- `scriibe desinstaller --liste` affiche la liste exacte sans rien toucher (essais, tâche 5). Sans `--liste`, la commande demande de taper « oui » ; elle ne propose pas la sauvegarde (qui passe par la fenêtre) et le dit.

### 2. Ce qui est supprimé (décision de l'auteur : « tout, à la Corbeille »)

**Tout ce qui appartient à Scriibe part à la Corbeille** (Corbeille du Mac, Corbeille de Windows, Corbeille freedesktop sous Linux) : c'est récupérable tant qu'elle n'est pas vidée. Une case **« Garder mes modèles téléchargés »**, décochée par défaut, laisse `models/` en place pour une réinstallation.

| Élément | Mac | Windows | Linux | Traitement |
|---|---|---|---|---|
| Programme | `Scriibe.app` en cours (`current_exe`, remonté au `.app`) | `scriibe.exe` en cours, plus `scriibe.exe.old` et `.new` à côté | `scriibe` en cours | Corbeille, en dernier (voir « Programme en cours ») |
| Dossier de données (réglages et modes, dictionnaire, historique, totaux, bancs, journaux, verrou, `models/` sauf case cochée) | `~/Library/Application Support/Scriibe` | `%APPDATA%\Scriibe` | `~/.local/share/scriibe` | Corbeille |
| Moteurs téléchargés | dans le dossier de données | `%LOCALAPPDATA%\Scriibe` (le dossier entier) | `~/.local/share/Scriibe` (le dossier entier) | Corbeille |
| Restes de mise à jour | `Scriibe.app.old`, `.Scriibe-mise-a-jour` à côté de l'app | voir Programme | — | Corbeille |
| Caches de la fenêtre | ceux nommés exactement `fr.scriibe.app` sous `~/Library/Caches`, `~/Library/WebKit`, `~/Library/HTTPStorages` | `%LOCALAPPDATA%\fr.scriibe.app` | ceux nommés `fr.scriibe.app` sous `~/.cache` et `~/.local/share` | Corbeille ; liste exacte à mesurer en tâche 5 et 6 |
| Lanceur | — | — | `~/.local/share/applications/fr.scriibe.Scriibe.desktop` et son icône | Corbeille |
| Entrée de démarrage | voir question 6 | voir question 6 | voir question 6 | Retirée |
| Sons temporaires | `$TMPDIR/scriibe-son-*.wav` | — | `$TMPDIR/scriibe-son-*.wav` | Effacés directement (jetables) |

Pas touchés : tout fichier hors de ces emplacements, dont les modèles choisis ailleurs (question 4) ; l'autorisation « RemoteDesktop » mémorisée par le portail GNOME (hors de Scriibe, citée dans l'Aide).

**Garde-fous (choix technique).** Une suppression de trop est irréversible si la Corbeille est vidée, d'où :
- La liste est construite **uniquement** à partir des emplacements ci-dessus ; aucun chemin venu de `settings.json` n'y entre.
- Chaque racine doit être un chemin **absolu** dont le dernier élément est `Scriibe`, `scriibe`, `engines` ou `fr.scriibe.app` (ou `Scriibe.app`, `scriibe.exe`, `scriibe` pour le programme). Sinon, refus. Raison : `data_folder` retombe sur `.` sous Windows sans `APPDATA`, et sur un chemin relatif sans `HOME` ; `SCRIIBE_DATA=~` viserait tout le dossier personnel.
- Les liens symboliques ne sont **jamais suivis** (`symlink_metadata`) : un lien trouvé est retiré comme lien.
- Si la Corbeille est indisponible (Linux sans Corbeille, autre volume), Scriibe s'arrête et le dit : il ne bascule jamais vers une suppression définitive.
- Corbeille par la crate `trash` (macOS, Windows, freedesktop), plutôt qu'un code maison par système : elle est éprouvée et couvre les trois. C'est une dépendance de code, pas un téléchargement de modèle ou de binaire.

**Programme en cours (choix technique).** Avant de toucher aux fichiers, Scriibe arrête `whisper-server` et `llama-server` (Windows verrouille un exécutable lancé). Mac et Linux savent mettre à la Corbeille un programme en cours. Windows non : Scriibe copie son exécutable dans `%TEMP%`, lance cette copie avec `desinstaller-fin <pid> <chemin>`, puis quitte ; la copie attend la fin du processus, met `scriibe.exe` à la Corbeille, puis s'efface au prochain nettoyage de `%TEMP%`.

**Fin.** Un dernier message dit ce qui est parti, ce qui reste (modèles ailleurs, Corbeille à vider pour libérer N Go, variables d'environnement sous Linux) puis Scriibe quitte.

### 3. Sauvegarde et confirmation (décision de l'auteur : « proposée, cochée »)

Fenêtre en trois temps :
1. **Liste exacte** de ce qui part, avec les tailles, et de ce qui reste (modèles ailleurs, case « Garder mes modèles téléchargés »).
2. Case **« Faire d'abord une sauvegarde »**, cochée : « Enregistrer sous », Documents par défaut, nom `scriibe-sauvegarde-<jour>.scriibe.json`. Si la sauvegarde échoue ou est annulée, **rien n'est supprimé**.
3. Bouton rouge **« Désinstaller »**, puis un dernier **« Oui, désinstaller »**.

**Sauvegarde complète (choix technique).** Aujourd'hui seul l'export des modes existe ; l'export d'un utilisateur (`kind: "profile"`) est cadré mais pas codé et exclut l'historique. On ajoute à l'enveloppe de `docs/CADRAGE_PROFILS.md` (décision 4) un `kind: "backup"` dont `content` reprend les fichiers du dossier de données tels quels : `settings` (`settings.json`), `dictionary`, `history`, `historyTotals`. Jamais les clés d'API, jamais les modèles (seulement leurs chemins, dans les réglages), ni les bancs ni les journaux.
- Raison : réutiliser les lectures tolérantes existantes (`from_value`, `UserDictionary::from_json`) ; quand les utilisateurs (chantier 4) arriveront, le `backup` suivra les fichiers grâce à `schemaVersion`.
- **Import** depuis l'onglet Réglages (« Restaurer une sauvegarde ») : aperçu de ce qui sera remplacé, confirmation, copie des fichiers actuels dans `sauvegarde-avant-import-<date>/` avant d'écraser, puis la page se recharge (même règle que l'import du dictionnaire).
- L'historique contient les textes dictés : le fichier reste où l'auteur le range, la fenêtre le dit.

### 4. Modèles liés (choix technique)

Le constat montre que Scriibe **ne crée aucun lien et ne copie rien** : il garde seulement le chemin (`whisperModelPath`, `llamaCppModel`) dans `settings.json`. Règle :
- Un modèle hors du dossier de Scriibe (`~/.lmstudio/models`, futur Magasin de modèles, tout autre dossier) n'est **jamais** dans la liste ; la fenêtre le cite dans « ce qui reste ».
- Si un lien symbolique se trouve un jour dans un dossier de Scriibe (dossier partagé de la ROADMAP), il est retiré comme lien sans suivre sa cible (garde-fou de la question 2).
- Test exigé (tâche 4) : un lien dans `models/` vers un faux modèle hors du dossier ; après désinstallation factice, le lien a disparu et la cible est intacte.

### 5. Clés d'API (décision de l'auteur : « effacées, listées »)

- **Mac** : entrées du trousseau, service `Scriibe`, comptes `deepgram`, `openrouter`, et l'ancien `github` s'il existe (`SecItemDelete` par service et compte).
- **Windows** : identifiants génériques `Scriibe/deepgram`, `Scriibe/openrouter` et l'ancien `Scriibe/github` s'il existe (`CredDeleteW`).
- **Linux** : aucun coffre ; l'Aide dit de retirer `DEEPGRAM_API_KEY` et `OPENROUTER_API_KEY` de son profil de shell. Scriibe ne touche pas aux fichiers du shell.
- Les clés sont **nommées** dans la liste, jamais leur valeur. La sauvegarde n'en contient jamais.
- À créer dans `crates/platform` : `delete_credential(kind)` derrière l'interface des secrets, plus l'effacement de l'ancien compte `github` (seul endroit où ce nom reste dans le code). Une clé absente n'est pas une erreur.
- Effacées **en dernier avant le programme**, après les fichiers : si la Corbeille échoue, les clés sont encore là.

### 6. Lancement au démarrage

**Décision de l'auteur : décoché par défaut, démarrage discret.** Une case « Lancer Scriibe à l'ouverture de session » dans l'onglet Réglages. Cochée, Scriibe démarre avec `lancer --sans-navigateur` : icône seule, touche de dictée prête, sans fenêtre.

**Mécanismes (choix technique)** — chaque fois un simple fichier ou une valeur, facile à créer, retirer et tester sur des dossiers factices :

| Système | Entrée | Commande lancée | Où l'utilisateur la voit |
|---|---|---|---|
| Mac | `~/Library/LaunchAgents/fr.scriibe.app.plist` (`RunAtLoad`, `AssociatedBundleIdentifiers` = `fr.scriibe.app`) | `/usr/bin/open -g -a <chemin de Scriibe.app> --args lancer --sans-navigateur` | Réglages Système › Général › Ouverture |
| Windows | Valeur `Scriibe` de `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` | `"<chemin>\scriibe.exe" lancer --sans-navigateur` | Gestionnaire des tâches › Applications de démarrage |
| Linux | `$XDG_CONFIG_HOME/autostart/fr.scriibe.Scriibe.desktop` (sinon `~/.config/autostart`) | `Exec="<chemin>/scriibe" lancer --sans-navigateur` | Applications au démarrage de l'environnement |

- **Mac par `open` plutôt que `SMAppService.mainApp`** : `open` rend `Scriibe.app` processus responsable, donc les autorisations (micro, surveillance de l'entrée, accessibilité) restent les siennes (`docs/PIEGES.md`) ; il passe les arguments du démarrage discret, ce que `mainApp` ne fait pas. macOS 13 et plus affiche une notification « élément d'arrière-plan ajouté » : normal. Repli si l'essai réel échoue : `SMAppService` (macOS 14 minimum, `objc2` déjà présent).
- **Vérité = le système** : la case lit la présence de l'entrée, rien n'est rangé dans `settings.json`. Si l'utilisateur retire l'entrée dans les réglages du système, la case se décoche d'elle-même ; et le réglage n'entre pas dans le partage « utilisateur / ordinateur » de `docs/CADRAGE_PROFILS.md`.
- **Programme déplacé** : à chaque lancement, si l'entrée existe et vise un autre chemin que le programme en cours, Scriibe la réécrit.
- Pas de `tauri-plugin-autostart` : la fenêtre Tauri n'est pas le centre de Scriibe, et l'interface de `crates/platform` garde le code propre à chaque système hors du cœur.
- Scribe n'a pas cette option : écart à noter dans `docs/ECARTS.md` avec le code.

**Autorisations macOS à la désinstallation (choix technique).** Scriibe lance `tccutil reset All fr.scriibe.app`, qui ne vise que lui ; un échec n'arrête rien et l'Aide dit où retirer à la main ce qui reste (Réglages Système › Confidentialité et sécurité). Une réinstallation redemande les autorisations, comme une première installation.

### 7. Ordre de livraison (choix technique)

1. **Tâche 3, démarrage** : le plus simple et le plus utile au quotidien ; la désinstallation devra retirer cette entrée, elle vient donc après.
2. **Tâche 4, en deux commits** : (a) sauvegarde complète `kind: "backup"`, export et import, utile seule ; (b) liste exacte (`--liste`), Corbeille, clés, entrée de démarrage, autorisations macOS, programme en cours, fenêtre en trois temps.
3. Tâches 5 et 6 : essais réels, sauvegarde d'abord. Rubriques d'Aide FR et EN à chaque étape ; livraison sans version (tâche 7).

## Relecture

À venir (tâche 2).
