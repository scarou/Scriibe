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
- Chaque racine est **reconstruite une à une** par les fonctions du constat, jamais dérivée d'une autre, et doit être un chemin **absolu**. Raison : `data_folder` retombe sur `.` sous Windows sans `APPDATA`, et sur un chemin relatif sans `HOME`.
- **Nom et structure** (relecture, n° 4) : le dernier élément doit être `Scriibe`, `scriibe` ou `fr.scriibe.app` (comparaison sans tenir compte de la casse sous Windows et macOS) ; le dossier de données doit en plus contenir `settings.json` ou `instance.lock` ; le dossier des moteurs se vise par son parent `Scriibe` entier, jamais par un `engines` seul. Le programme (`Scriibe.app`, `scriibe.exe`, `scriibe`) est **exclu** s'il se trouve sous un dossier `target/` ou dans un dépôt `.git` (programme compilé sur un ordinateur de développement) : seules les données partent alors, et la fenêtre le dit.
- **Bouton grisé avec `SCRIIBE_DATA`** (relecture, n° 3) : cette variable déplace les données mais pas les moteurs ni les clés ; une instance d'essai viserait les vraies. La fenêtre explique pourquoi le bouton est grisé.
- **Programme partagé** (relecture, n° 5) : si le programme ou son dossier n'est pas modifiable par l'utilisateur (propriétaire `root`, autre compte), il est retiré de la liste et la fenêtre le dit **avant** de commencer ; seules « mes données » partent.
- Les liens symboliques ne sont **jamais suivis** (`symlink_metadata`, jamais `canonicalize`). Une racine qui est elle-même un lien (dossier de données renvoyé vers un autre disque) part comme lien : sa cible reste en place et la fenêtre le dit. La case « Garder mes modèles téléchargés » oblige à parcourir le dossier de données enfant par enfant : chaque enfant (`models/` exclu) part tel quel, lien compris, sans descendre dedans.
- Si la Corbeille est indisponible, Scriibe s'arrête et le dit : il ne bascule **jamais** vers une suppression définitive. Sous Windows, un fichier plus gros que la place de la Corbeille peut être supprimé définitivement par le système (relecture, n° 7) : Scriibe compare d'abord la taille des éléments à la place de la Corbeille du volume et, si elle manque, s'arrête en proposant de cocher « Garder mes modèles téléchargés » ou d'effacer les modèles à la main.
- Corbeille par la crate `trash` (macOS, Windows, freedesktop), plutôt qu'un code maison par système : elle est éprouvée et couvre les trois. C'est une dépendance de code, pas un téléchargement de modèle ou de binaire. **Avant de l'adopter** (tâche 4) : lire son code pour la version retenue. Sur Mac, elle doit passer par `NSFileManager` et non par le Finder en AppleScript, qui déclencherait une demande « Automatisation ». Il faut aussi vérifier son comportement entre deux volumes et la taille limite sous Windows. Sinon, code maison derrière `crates/platform`.

**Programme en cours : tout se fait après sa sortie** (choix technique ; relecture, n° 1). Le processus vivant réécrit son dossier : `FileHistory::read` appelle `persist` (`storage.rs`), et le journal comme le verrou refont `create_dir_all`. L'historique réapparaîtrait donc dans un dossier neuf. Sous Windows, le verrou, le journal et WebView2 empêchent même de déplacer le dossier. Sur les **trois systèmes** :
1. Le processus vivant fait la liste, la sauvegarde et les confirmations. Ensuite, il **ne supprime rien**.
2. Il pose un drapeau qui coupe toute écriture (historique, réglages, journal). Il arrête `whisper-server`, `llama-server`, la touche, la fenêtre et le serveur local.
3. Il lance un assistant détaché, puis quitte. Cet assistant attend la fin du processus **sur son handle** (Windows) ou par `kill -0` (Mac, Linux, comme `restart` dans `web/mod.rs`), jamais sur un PID seul réutilisable.
4. L'assistant fait la suppression dans l'ordre fixé ci-dessous, avec des reprises en cas de verrou (WebView2, WebKit qui écrit à la sortie).
- L'assistant est une copie du programme placée dans le dossier temporaire et lancée avec `desinstaller-fin`. Sous Windows, il s'efface à la fin par un `cmd /c` différé : Windows ne vide pas `%TEMP%` de lui-même.
- Un petit journal de reprise reste dans le dossier temporaire. Il permet, si l'opération est interrompue, de relancer `scriibe desinstaller` sur ce qui reste. Le dernier message dit ce qui reste.

**Ordre fixé** (relecture, n° 15), pour qu'une interruption ne laisse jamais une entrée de démarrage vers un programme absent :
1. Arrêt des processus.
2. Entrée de démarrage retirée.
3. Données, moteurs, caches, lanceur Linux.
4. Sons temporaires, seulement les fichiers ordinaires à l'utilisateur dont le nom suit exactement `scriibe-son-<chiffres>-…​.wav`.
5. Clés d'API.
6. `tccutil`.
7. Programme, en dernier.

**Commande `scriibe desinstaller`** (relecture, n° 16) : elle refuse si Scriibe tourne déjà (verrou `instance.lock`) et propose de le quitter d'abord.

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

**Corrections de la relecture (n° 2, 12, 14) :**
- **Taille** : l'import commun refuse plus de 1 Mio (`core/src/import.rs`, `MAX_BYTES`), ce qu'un historique dépasse vite. Le `backup` a sa propre limite, 64 Mio.
- **Validation avant suppression** : la sauvegarde écrite est relue et analysée comme pour un import. Si elle ne se relit pas, rien n'est supprimé.
- **Emplacement** : un chemin de sauvegarde situé sous une racine de la liste est refusé, par exemple dans `%APPDATA%\Scriibe` ou à côté de `scriibe.exe`, puisqu'il partirait avec elle.
- **Ce qui n'est pas sauvegardé** : la fenêtre a une ligne « non sauvegardé », avec les tailles. On y trouve les modèles téléchargés (à retélécharger), les bancs, les journaux et les clés d'API.
- **Restauration hors du processus vivant** : l'historique est gardé en mémoire et réécrit à la première lecture. La restauration passe donc par un redémarrage. Scriibe écrit les fichiers dans un dossier de travail, les échange par renommage, puis se relance, comme la migration de `docs/CADRAGE_PROFILS.md` (décision 6). Ainsi, une interruption ne laisse pas un mélange de deux états.
- **Fichier plus récent** : un `settings` dont le `schemaVersion` est plus récent que celui de Scriibe est refusé (B4 de `CADRAGE_PROFILS`).
- **Champs propres à l'ordinateur** (touches, micro, chemins de modèles, moteurs) : ils sont restaurés seulement si la sauvegarde vient du même système (`os` noté dans l'enveloppe). Sinon, ils sont laissés tels quels et l'aperçu le dit.
- **Exception dans `docs/CADRAGE_PROFILS.md`** : cette note y pointe une exception « backup ». C'est le seul export qui contient l'historique et les statistiques, et le seul import qui **remplace** au lieu d'ajouter. Il ne se fait que depuis la sauvegarde complète. Quand les utilisateurs arriveront (`utilisateurs/<uuid>/`), le `backup` suivra les fichiers de chaque utilisateur, avec un test qui le vérifie.

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
- Effacées **après les fichiers** (ordre fixé, question 2) : si la Corbeille échoue, les clés sont encore là.
- **Demandes du trousseau** (relecture, n° 13) : une entrée créée par une autre compilation (ou l'ancien `github`) peut faire redemander l'accès au Mac, jusqu'à une fois par clé, après « Oui, désinstaller ». La fenêtre le prévient. Une clé dont l'effacement est refusé est citée dans le dernier message avec la marche à suivre (Trousseaux d'accès › rechercher « Scriibe »). À l'essai réel : avec une clé créée par une version précédente.

### 6. Lancement au démarrage

**Décision de l'auteur : décoché par défaut, démarrage discret.** Une case « Lancer Scriibe à l'ouverture de session » dans l'onglet Réglages. Cochée, Scriibe démarre avec `lancer --sans-navigateur` : icône seule, touche de dictée prête, sans fenêtre.

**Mécanismes (choix technique)** — chaque fois un simple fichier ou une valeur, facile à créer, retirer et tester sur des dossiers factices :

| Système | Entrée | Commande lancée | Où l'utilisateur la voit |
|---|---|---|---|
| Mac | `~/Library/LaunchAgents/fr.scriibe.app.plist` (`RunAtLoad`, `AssociatedBundleIdentifiers` = `fr.scriibe.app`) | `/usr/bin/open -g -a <chemin de Scriibe.app> --args lancer --sans-navigateur` | Réglages Système › Général › Ouverture |
| Windows | Valeur `Scriibe` de `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` | `"<chemin>\scriibe.exe" lancer --sans-navigateur` | Gestionnaire des tâches › Applications de démarrage |
| Linux | `$XDG_CONFIG_HOME/autostart/fr.scriibe.Scriibe.desktop` (sinon `~/.config/autostart`) | `Exec="<chemin>/scriibe" lancer --sans-navigateur` | Applications au démarrage de l'environnement |

- **Mac par `open` plutôt que `SMAppService.mainApp`** : `open` rend `Scriibe.app` processus responsable, donc les autorisations (micro, surveillance de l'entrée, accessibilité) restent les siennes (`docs/PIEGES.md`) ; il passe les arguments du démarrage discret, ce que `mainApp` ne fait pas. macOS 13 et plus affiche une notification « élément d'arrière-plan ajouté » : normal. Repli si l'essai réel échoue : `SMAppService` (macOS 14 minimum, `objc2` déjà présent).
- **Vérité = le système** : rien n'est rangé dans `settings.json`, et le réglage n'entre pas dans le partage « utilisateur / ordinateur » de `docs/CADRAGE_PROFILS.md`. La case lit **la présence et l'activation** de l'entrée (relecture, n° 10). Désactiver l'élément dans les réglages du système laisse le fichier ou la valeur en place :
  - Mac : `launchctl print-disabled gui/<uid>` ;
  - Windows : valeur `Scriibe` de `HKCU\…\Explorer\StartupApproved\Run`, écrite par le Gestionnaire des tâches ;
  - Linux : `X-GNOME-Autostart-enabled=false` ou `Hidden=true` dans le `.desktop`.

  Désactivée là, la case est décochée. La recocher réactive l'entrée : on efface la marque de désactivation, ou on réécrit l'entrée. À l'essai réel : désactiver depuis les réglages du système, puis relire la case.
- **Windows : pas de fenêtre noire au démarrage** (relecture, n° 9). `scriibe.exe` est un programme console : `docs/INSTALLATION_WINDOWS.md` dit qu'une fenêtre noire s'ouvre et qu'il ne faut pas la fermer. Lancé par la clé `Run`, il l'ouvrirait à chaque connexion, et la fermer tuerait Scriibe. Avec `lancer --sans-navigateur`, Scriibe se détache donc de sa console (`FreeConsole`) dès le démarrage. Le journal reste dans `scriibe.log`. Le double-clic ordinaire garde la console, pour ne rien changer au guide. La valeur du registre demande la fonctionnalité `Win32_System_Registry` de `windows-sys`.
- **Mac, app déplacée** (relecture, n° 17) : `open -a <chemin>` échoue jusqu'au prochain lancement manuel, qui réécrit l'entrée. C'est accepté, et l'Aide le dit. Si l'app tourne déjà, `open` ignore les arguments, ce qui est voulu.
- **Programme déplacé** : à chaque lancement, si l'entrée existe et vise un autre chemin que le programme en cours, Scriibe la réécrit.
- Pas de `tauri-plugin-autostart` : la fenêtre Tauri n'est pas le centre de Scriibe, et l'interface de `crates/platform` garde le code propre à chaque système hors du cœur.
- Scribe n'a pas cette option : écart à noter dans `docs/ECARTS.md` avec le code.

**Autorisations macOS à la désinstallation (choix technique).** Scriibe lance `tccutil reset All fr.scriibe.app`, qui ne vise que lui ; un échec n'arrête rien et l'Aide dit où retirer à la main ce qui reste (Réglages Système › Confidentialité et sécurité). Une réinstallation redemande les autorisations, comme une première installation.

### 7. Ordre de livraison (choix technique)

1. **Tâche 3, démarrage** : le plus simple et le plus utile au quotidien ; la désinstallation devra retirer cette entrée, elle vient donc après.
2. **Tâche 4, en deux commits** : (a) sauvegarde complète `kind: "backup"`, export et import, utile seule ; (b) liste exacte (`--liste`), Corbeille, clés, entrée de démarrage, autorisations macOS, programme en cours, fenêtre en trois temps.
3. Tâches 5 et 6 : essais réels, sauvegarde d'abord. Rubriques d'Aide FR et EN à chaque étape ; livraison sans version (tâche 7).

## Relecture

Relecture faite le 13/11/2026 (tâche 2) par un sous-agent qui n'avait pas écrit la note. Elle a porté sur la suppression hors des dossiers de Scriibe, les liens suivis jusqu'à leur cible, les clés oubliées, les données perdues sans sauvegarde et la faisabilité par système. J'ai revérifié moi-même dans le code les points notés V : `FileHistory::read` puis `persist`, `MAX_BYTES`, l'absence de `windows_subsystem`, l'exclusion de l'historique dans `CADRAGE_PROFILS` et `restart`. **Aucune décision de l'auteur ne change** : toutes les corrections portent sur la mise en œuvre. V = vérifié dans le code, S = supposé et à vérifier à l'essai.

| n° | Gravité | Défaut | Correction (où dans la note) |
|---|---|---|---|
| 1 | bloquant | Le processus vivant réécrit le dossier mis à la Corbeille (`FileHistory::read` → `persist`). Sous Windows, verrous (V) | Tout se fait après la sortie, par un assistant, sur les trois systèmes (question 2) |
| 2 | bloquant | Sauvegarde non restaurable : limite d'import de 1 Mio, `Store` en mémoire qui réécrase (V) | Limite propre de 64 Mio, validation, restauration par redémarrage et renommage (question 3) |
| 3 | bloquant | `SCRIIBE_DATA` déplace les données, pas les moteurs ni les clés (V) | Bouton grisé (question 2) |
| 4 | important | « Dernier élément » ne suffit pas : `target/release/scriibe`, `Scriibe-dev/Scriibe`, `engines` seul (V) | Contrôle de structure, exclusion de `target/` et `.git`, casse ignorée (question 2) |
| 5 | important | Programme partagé ou d'un autre compte (S) | Retiré de la liste et signalé avant de commencer (question 2) |
| 6 | important | Racine qui est un lien ; parcours pour « Garder mes modèles » (V pour la règle) | Racine-lien qui part comme lien, parcours enfant par enfant sans `canonicalize` (question 2) |
| 7 | important | Crate `trash` : suppression définitive possible sous Windows au-delà de la place de la Corbeille ; Finder par AppleScript sur Mac (S) | Taille comparée d'abord ; code de `trash` lu avant de l'adopter (question 2) |
| 8 | important | Caches de la fenêtre seulement supposés ; restes possibles (`Preferences`, `Saved Application State`) (S) | Mesure avant et après un vrai lancement (essais) ; suppression après la sortie, avec des reprises |
| 9 | important | Windows : la console noire s'ouvrirait à chaque connexion, la fermer tue Scriibe (V) | `FreeConsole` en `--sans-navigateur` (question 6) |
| 10 | important | Désactivation dans les réglages du système non vue par la case (S) | Lecture de l'activation par système (question 6) |
| 11 | important | Copie dans `%TEMP%` jamais effacée, attente sur un PID réutilisable | Attente sur le handle, effacement différé (question 2) |
| 12 | important | Sauvegarde : chemin dans un dossier qui part, « non sauvegardé » absent, champs propres à l'ordinateur (V) | Chemin refusé, ligne « non sauvegardé », `os` dans l'enveloppe (question 3) |
| 13 | important | Le trousseau peut redemander l'accès après la confirmation (S) | Prévenu dans la fenêtre, marche à suivre en cas de refus (question 5) |
| 14 | important | Contradiction avec `CADRAGE_PROFILS` : l'historique y est exclu des exports, et l'import n'y remplace jamais (V) | Exception « backup » (question 3), à reporter dans `CADRAGE_PROFILS` |
| 15 | mineur | Ordre et interruption | Ordre fixé et journal de reprise (question 2) |
| 16 | mineur | `scriibe desinstaller` pendant que Scriibe tourne | Refus par le verrou (question 2) |
| 17 | mineur | LaunchAgent qui vise une app déplacée (S) | Accepté, l'Aide le dit (question 6) |

**À vérifier aux essais réels (tâches 5 et 6) :**
- Inventaire `find ~/Library ~/.cache ~/.local/share -iname '*scriibe*'` avant et après un lancement, et l'équivalent sous `%LOCALAPPDATA%` et `%APPDATA%`.
- `tccutil reset All fr.scriibe.app` sans droits d'administrateur, en particulier pour l'Accessibilité et la Surveillance de l'entrée.
- La notification « élément d'arrière-plan ajouté », et le nom affiché grâce à `AssociatedBundleIdentifiers` pour un plist utilisateur.
- Le démarrage automatique XDG sous GNOME Wayland : le portail est-il prêt à l'ouverture de session ?
- Sous Windows, un faux modèle plus gros que la place de la Corbeille.
- Un effacement de clé créée par une version précédente.
- La désactivation depuis les réglages du système, puis la relecture de la case.

**Ce que la relecture confirme (V)** :
- Les chemins des trois systèmes, dont `Scriibe` contre `scriibe` sous Linux.
- L'absence de lien et de copie de modèle.
- Les clés : `deepgram`, `openrouter` et l'ancien `github`, sans aucun autre nom dans l'historique git.
- L'ordre « clés après les fichiers ».
- Le choix de `open` plutôt que `SMAppService`.
