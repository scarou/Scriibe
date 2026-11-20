# Cadrage des mises à jour (chantier 8)

## État des lieux

Relevé en lecture seule du code au 20/11/2026 (version `0.12.0`, `Cargo.toml:6`). Chaque affirmation porte son `fichier:ligne`. « N'existe pas » est dit explicitement. Aucune recommandation ici, seulement des faits.

### 1. Mise à jour de l'application

**Source et vérification de la dernière version**

- Source unique : l'API GitHub, `GET https://api.github.com/repos/scarou/Scriibe/releases/latest`, sans identification (`crates/cli/src/web/update.rs:14`, `:179-186`). Dépôt public. En-têtes envoyés : `Accept`, `X-GitHub-Api-Version`, `User-Agent: scriibe`.
- Seule la Release renvoyée par `/releases/latest` est lue (une seule, jamais la liste) : il n'existe aucun code qui liste les Releases de l'application ni qui en choisisse une autre que la dernière (`update.rs:213-215`).
- Erreurs traitées : 404 (« aucune version publiée »), 403/429 (limite de débit), autre (`update.rs:187-192`).
- Version installée = `CARGO_PKG_VERSION` (`update.rs:85-87`). Le commit compilé (`SCRIIBE_COMMIT`, `crates/cli/build.rs:9-15`) sert seulement à l'affichage (`update.rs:90-92`, `crates/cli/src/web/mod.rs:92`), pas à la comparaison.
- Comparaison : étiquette `vX.Y.Z` ou `X.Y.Z` en trois nombres entiers comparés en tuple ; tout autre format (suffixe, deux ou quatre composants, `nightly`) n'est jamais « plus récent » (`update.rs:95-104`). Seule une version **strictement supérieure** est une mise à jour : la même version ou une plus ancienne n'est jamais proposée (`update.rs:102-104`, `:140-142`).
- Choix du fichier dans la Release par nom : Windows `scriibe-windows-x64.zip` ; macOS `Scriibe-*-macos.zip` ; Linux `scriibe-linux-x64.zip` (`update.rs:16`, `:19`, `:45-51`, `:206-208`). Sans le fichier de son système, la Release est une version « non installable » (taille 0, adresse vide) (`update.rs:207-209`).

**Déclenchement**

- Manuel : onglet « Mise à jour », route `POST api/update/check` (`crates/cli/src/web/mod.rs:671`, `check_update` `:1346-1362`), puis `POST api/update/install` avec `confirmed: true` (`mod.rs:675-683`, `install_update` `:1364-1368`). Une confirmation de la page (version, taille, source) précède l'installation (`ui/src/tabs/Update.svelte:50-54`).
- Automatique : réglage `checkUpdates`, **faux par défaut** (`crates/core/src/settings.rs:431-432`, `:508`). Un fil lit la dernière Release 30 s après le lancement puis toutes les 6 h (`mod.rs:146-172`, condition `:156`). Il ne fait qu'alerter (`set_available`, `mod.rs:164`) ; il n'installe jamais. L'alerte alimente la page (`mod.rs:820`) et le menu de l'icône (`crates/cli/src/tray_source.rs:19`).

**Téléchargement, contrôles, remplacement, redémarrage**

- Téléchargement en mémoire, HTTPS seulement, redirections suivies à la main (3 au plus), taille comparée à `Content-Length` ou à la taille annoncée par l'API ; un fichier plus court est refusé (`update.rs:165-168`, `:218-255`).
- **Aucune somme de contrôle** (SHA-256 ou autre) n'est vérifiée, **aucune signature de Release** (minisign, GPG, Tauri updater…) : aucune dépendance de ce genre dans les `Cargo.toml`, `bundle.active` est `false` (`crates/cli/tauri.conf.json:8`). La confiance repose sur HTTPS vers `github.com` et sur les contrôles ci-dessous.
- Windows et Linux : le zip est ouvert en mémoire ; l'entrée nommée `scriibe.exe` (Windows) ou `scriibe` (Linux) est extraite, doit faire au moins 100 000 octets et commencer par `MZ` (Windows) ou `\x7fELF` (Linux) (`update.rs:54-59`, `:262-282`). Remplacement (`replace_program`, `update.rs:291-314`) : écriture de `<exe>.new`, droits `0755` sous Unix, renommage de l'exécutable en cours en `<exe>.old`, renommage de `.new` à sa place ; si une étape échoue l'ancien est remis. Aucune élévation de droits : un dossier non inscriptible donne une erreur (`update.rs:294-296`). L'ancien `.old` est supprimé au lancement suivant (`update.rs:387-402`, appelé `crates/cli/src/commands.rs:126`).
- macOS : zip écrit dans `.Scriibe-mise-a-jour/` à côté de `Scriibe.app`, décompressé par `/usr/bin/ditto`, puis contrôles : `Contents/MacOS/Scriibe` présent, `fr.scriibe.app` dans `Info.plist`, `codesign --verify --deep --strict` valide (`update.rs:340-355`, `:373-383`). Échange par renommage : l'ancienne devient `Scriibe.app.old`, remise en place si échec (`update.rs:359-370`). Refusé hors de `Scriibe.app` (`cargo run`) (`update.rs:374`). Le contrôle `codesign --verify` vérifie la validité de la signature, **pas l'identité du signataire** (le code ne compare ni certificat ni identifiant d'équipe).
- Redémarrage : après 1,5 s, `restart` arrête whisper et llama (`mod.rs:1377-1379`). macOS : un `sh` détaché attend la fin du processus puis lance `/usr/bin/open Scriibe.app` (`mod.rs:1382-1390`). Windows et Linux : relance directe du programme (`program_path`, qui retire le suffixe `.old` ou ` (deleted)`), avec l'argument `RESTARTED` (`mod.rs:1391-1398`, `update.rs:67-82`).
- Publication côté dépôt : macOS par `tools/publier_macos.sh` (compilation et signature « Scribe Dev » sur le Mac de l'auteur, `gh release create`, confirmation « oui » demandée, refus si le numéro diffère de `Cargo.toml`, `publier_macos.sh:8-41`) ; Windows et Linux ajoutés ensuite par `.github/workflows/release-windows.yml` et `release-linux.yml` à la publication de la Release (`on: release: published`), avec contrôle que l'étiquette égale la version de `Cargo.toml` (`release-windows.yml:29-36`, `release-linux.yml:30-38`). Le CI normal ne publie qu'un artefact `scriibe-windows-x64` (30 jours), que l'application ne lit pas (`ci.yml:78-113`).

**Par système, en résumé**

| Système | Vérifier | Installer | Remplacement |
|---|---|---|---|
| Windows | oui | oui (`can_install`, `update.rs:63-65`) | renommage de `scriibe.exe` en `.old` |
| macOS | oui | oui, seulement depuis `Scriibe.app` | renommage de l'application entière |
| Linux | oui | oui (`update.rs:64`, `:451`) ; remplace `current_exe()` | renommage de `scriibe` en `.old` |

Écart de documentation constaté : `docs/MISE_A_JOUR.md` (fin du fichier, section « S15 … Linux ») dit que Linux « reste une simple alerte » (`can_install()` faux) ; le code actuel renvoie vrai pour Linux (`update.rs:64`) et l'installe (`update.rs:451-452`). `docs/MISE_A_JOUR.md` dit aussi que la mise à jour Linux « reste à essayer » avec une vraie Release (en-tête du fichier). Le type TypeScript `ui/src/lib/api.ts:73` déclare encore `update: { sha, … }` alors que le serveur envoie `{version, title, date}` (`mod.rs:820`).

**Ce qui n'existe pas**

- **Forcer une mise à jour** (réinstaller la même version) : n'existe pas ; `is_newer` est strict (`update.rs:102-104`) et le bouton d'installation n'apparaît que pour un fichier installable d'une Release plus récente (`Update.svelte:84-94`) ; `install_update` ne s'appuie que sur `ctx.found` rempli par la dernière vérification (`mod.rs:1364-1365`).
- **Installer une version antérieure** (retour en arrière) : n'existe pas (pas de liste de Releases, pas de choix d'étiquette, pas de paramètre de version dans les routes `api/update*`). La seule ancienne version conservée est le `.old` de la dernière mise à jour, supprimé au lancement suivant (`update.rs:387-402`) ; aucune commande ne le restaure autrement que par l'échec automatique de `replace_program` / `swap_bundle`.
- Canaux (stable/préversion), mise à jour sans confirmation, installation différée, notes de version affichées (seul le titre `name` de la Release est lu), vérification de somme ou de signature, migration de données déclenchée par la mise à jour : n'existent pas.

### 2. llama.cpp (`llama-server`) et whisper.cpp (`whisper-server`)

**Livraison**

- **Rien n'est embarqué** dans l'exécutable ni dans `Scriibe.app` : `tools/faire_app_macos.sh` ne cite ni whisper ni llama ; `packaging/macos/` ne contient que `Info.plist`. Aucune version de whisper.cpp ou de llama.cpp n'est épinglée dans `Cargo.toml` ni lue par le programme (aucun appel à `--version`, aucun numéro stocké côté application).
- Lancés comme **processus externes** en HTTP (`crates/engines/src/whisper.rs:118-124`, `llama.rs:154-160`).
- **macOS** : fournis par l'utilisateur ; le message d'aide indique `brew install whisper-cpp` et `brew install llama.cpp` (`whisper.rs:127-132`, `llama.rs:181-189`). Aucun téléchargement intégré pour macOS (`crates/cli/src/web/downloads.rs:86-88`, `:198-211` : les identifiants Windows et Linux sont refusés ailleurs).
- **Windows** : téléchargés par l'onglet Installation, depuis les Releases de `ggml-org` à l'instant du clic, **sans version fixée** : `whisper-bin-x64.zip` de whisper.cpp, `llama-*-bin-win-cpu-x64.zip` ou `…-bin-win-vulkan-x64.zip` de llama.cpp, première Release correspondante parmi les 30 dernières (`downloads.rs:163-175`, `:216-235`). Zip décompressé à plat dans `…\Scriibe\engines\whisper` ou `\llama` ; pour llama, les anciens `.exe`, `.dll`, `.so` du dossier sont d'abord supprimés (`downloads.rs:252`, `:288-306`). Pour whisper, aucun nettoyage préalable (`clean` vaut `id.starts_with("llama-")`).
- **Linux** : zips `whisper-server-linux-x64.zip` et `llama-server-linux-x64.zip` pris dans la Release **de Scriibe** (`REPO`) par l'onglet Installation (`downloads.rs:81-82`, `:236-243`), première Release correspondante parmi les 30 dernières. Ils sont construits par `.github/workflows/release-linux.yml` à la publication d'une Release : whisper.cpp compilé depuis le tag **épinglé `WHISPER_TAG: v1.9.5`** (`release-linux.yml:71`, `:76-93`) ; llama.cpp : binaire Ubuntu **de la plus récente Release de llama.cpp ayant le fichier** au moment de la CI, remballé en zip, sans épinglage (`release-linux.yml:106-125`). Le numéro de version de llama.cpp embarqué n'est écrit ni dans le nom du zip ni ailleurs dans le dépôt.
- Fichiers de moteur téléchargés en `.part` avec reprise, taille finale comparée à celle annoncée ; **pas de somme de contrôle** (`downloads.rs:366-368`, `:413-420`).

**Où ils sont trouvés au lancement**

- Recherche faite à la **construction du lanceur** (au démarrage, et à chaque lecture de l'état de la page) : `find_binary` prend le premier candidat existant dans l'ordre, puis cherche dans le `PATH` (`crates/engines/src/fs.rs:50-56` ; `LlamaLauncher::new` `llama.rs:210-212`, `WhisperLauncher::new` `whisper.rs:152-154`, relectures `mod.rs:799-800`).
- Ordre des candidats, identique pour les deux : `/opt/homebrew/bin`, `/usr/local/bin`, `/usr/bin`, dossier de l'exécutable, son sous-dossier `whisper/` ou `llama/`, puis le dossier des moteurs (`engines/whisper/`, `engines/llama/`, `engines/`) (`whisper.rs:100-116`, `llama.rs:163-178`). Un moteur système (Homebrew, `/usr/bin`) passe donc **avant** celui téléchargé dans le dossier des moteurs.
- Dossier des moteurs : Windows `%LOCALAPPDATA%\Scriibe\engines`, macOS `~/Library/Application Support/Scriibe/engines`, Linux `$XDG_DATA_HOME/Scriibe/engines` ou `~/.local/share/Scriibe/engines` (casse `Scriibe`, alors que le dossier de données Linux est `scriibe` en minuscules) (`crates/engines/src/lib.rs:25-37`).

**Mise à jour**

- **Aucune mise à jour automatique ni détection de version périmée des moteurs n'existe.** La mise à jour de l'application ne touche pas aux dossiers `engines/` (le zip de l'application ne contient que `scriibe`/`scriibe.exe`/`Scriibe.app`, `update.rs:262-282`, `:340-355`). Une nouvelle version d'un moteur s'obtient en relançant à la main le téléchargement de l'onglet Installation (Windows, Linux ; llama remplace les fichiers du dossier, `downloads.rs:288-306`) ou par `brew upgrade` (macOS). Le programme ne compare jamais la version d'un moteur à celle de l'application.
- Conséquence relevée dans le code : la CI Linux reconstruit les deux zips à chaque Release de Scriibe (`release-linux.yml:63-125`) ; ce qu'un utilisateur Linux obtient dépend de la première Release de Scriibe (parmi les 30 dernières) qui porte le fichier, pas de la Release qu'il utilise (`downloads.rs:163-175`).

### 3. Réglages, profils et `ui-state.json`

**Emplacements**

- Dossier de données (`SCRIIBE_DATA` s'il existe) : Windows `%APPDATA%\Scriibe`, macOS `~/Library/Application Support/Scriibe`, Linux `$XDG_DATA_HOME/scriibe` ou `~/.local/share/scriibe` (`crates/platform/src/storage.rs:32-47`).
- Fichiers à la racine : `settings.json` (réglages de l'ordinateur), `ui-state.json` (`crates/platform/src/ui_state.rs:15`), `activeProfile` dans `settings.json`.
- Par utilisateur, dans `utilisateurs/<id>/` : `profile.json` (réglages de l'utilisateur + clé `name`), `dictionary.json`, `history.json`, `history-totals.json` (`crates/platform/src/migration.rs:30-38`, `storage.rs:288-290`). Corbeille des utilisateurs supprimés : `utilisateurs-supprimes/` (`profiles.rs:21`). Sauvegarde de migration : `sauvegarde-avant-utilisateurs-<date>/` (`migration.rs:40`).
- Écritures atomiques : fichier temporaire `.<nom>.<pid>-<n>.tmp` puis renommage (`storage.rs:68-86`).

**`settings.json` et `profile.json`**

- Format : JSON objet, clés en camelCase (`#[serde(rename)]`, ex. `checkUpdates`, `schemaVersion`) (`settings.rs:263-267`, `:431-432`).
- **Numéro de version de schéma** : `schemaVersion`, constante `SETTINGS_SCHEMA = 2` (`settings.rs:252`), `LEGACY_SCHEMA = 1` pour un fichier sans numéro (`settings.rs:255`, `storage.rs:242`). La version 2 est la marque « migré vers `utilisateurs/<id>/` ». Présent avec la valeur 2 depuis la v0.5.0 (vérifié par `git grep` sur les étiquettes `v0.5.0` à `v0.12.0` : `pub const SETTINGS_SCHEMA: u64 = 2` partout). `profile.json` **n'a pas de numéro propre** ; celui de `settings.json` fait foi pour tout le dossier (`storage.rs:253-260`).
- **Lecture tolérante, sans `deny_unknown_fields`** (aucune occurrence dans `crates/`) : `AppSettings::from_value` lit clé par clé avec `get`, qui renvoie la valeur par défaut si la clé est absente **ou si sa valeur est illisible** (valeur d'énumération inconnue, mauvais type) (`settings.rs:636-638`, `:648-700`). `AppSettings` ne dérive que `Serialize` : aucune lecture `serde` directe, donc aucun `#[serde(default)]` ni `#[serde(other)]` (`settings.rs:263`). Les listes d'objets (`customModes`, `trialCandidates`) sont lues élément par élément et **un élément illisible est écarté** (`settings.rs:655-659`, `:682-686`). Une clé inconnue est ignorée à la lecture.
- **Écriture** : `save_settings` sérialise `AppSettings` entier (`storage.rs:522-556`). Conséquences pour les clés inconnues : sans utilisateur rangé à part, `settings.json` est réécrit à partir des seules clés connues ; avec un utilisateur, `settings.json` (ordinateur) est réécrit depuis `AppSettings::split` (clés connues seulement, `settings.rs:770-780`), alors que `profile.json` conserve ses clés dont le propriétaire n'est pas « utilisateur » (donc `name` **et** les clés inconnues) et remplace les clés de l'utilisateur (`storage.rs:542-548`). Le nom exact des clés par propriétaire : `COMPUTER_KEYS` et `USER_KEYS` (`settings.rs:525`, `:550`), `owner_of` (`settings.rs:597`).
- **Fichier plus récent que connu** : `schemaVersion > SETTINGS_SCHEMA` est lu (valeurs affichées) mais **jamais réécrit** : `save_settings` renvoie une erreur `PermissionDenied` avec le message « vient d'une version plus récente… Mets Scriibe à jour » (`storage.rs:92-128`, `:239-251`, `:526-529`). Le même refus s'applique à la migration (`migration.rs:88`, `:129-133`), à la restauration d'une sauvegarde (`crates/core/src/backup.rs:105-106`), à `settings.json` illisible (copie `.bak` datée à la lecture, aucune écriture, `storage.rs:130-160`, `:203`).
- Une version ancienne à `SETTINGS_SCHEMA = 2` (toute version depuis la v0.5.0) lisant un `settings.json` de schéma 3 : lecture seule, aucun réglage modifié ni perdu. Une version ancienne lisant un schéma 2 qu'elle sait lire mais écrit par une version plus récente (même numéro de schéma, clés ajoutées) : clés inconnues ignorées, clés absentes à la valeur par défaut, puis **réécriture sans les clés qu'elle ne connaît pas** (pas de refus puisque le numéro est inchangé).
- Retour d'une version antérieure à la migration (schéma 1 sans numéro) sur un dossier migré : prévu par la migration, qui sauvegarde et reprend l'utilisateur existant (`migration.rs:71-75`).
- **Migration de schéma** : une seule, de 1 vers 2 (rangement par utilisateur), `crates/platform/src/migration.rs:1-20`, lancée au démarrage après le verrou d'instance (`crates/cli/src/commands.rs:24`). Il n'existe **aucun mécanisme générique** de migration par numéros successifs (pas de table de versions, pas de fonction « de N vers N+1 ») ; l'ajout d'une clé se fait par lecture tolérante et valeur par défaut, sans changer `schemaVersion`.

**Fichiers d'échange et autres fichiers**

- Fichiers d'export (modes, dictionnaire, commandes, profil, sauvegarde `.scriibe.json`) : enveloppe `{format: "scriibe-export", version: 1, kind, app: "Scriibe x.y.z", exportedAt, content}` ; `version` plus grande que 1 refusée à l'import (« version plus récente »), version 0 ou absente refusée (`crates/core/src/export.rs:13-49`). La version de l'application y figure en texte (`app`) mais n'est pas lue à l'import. La sauvegarde refuse aussi un `schemaVersion` des réglages plus grand que connu (`backup.rs:105-106`).
- `dictionary.json` : objet `{words, corrections}`, lecture tolérante (éléments illisibles écartés, `crates/core/src/dictionary.rs:23-35`), pas de numéro de version. `history.json` et `history-totals.json` : listes lues élément par élément par `lossy_list` (`Dictation::from_value`, `DayTotal::from_value`) ; un fichier illisible est mis de côté en `.bak` (`storage.rs:643-665`). Pas de numéro de version non plus.

**`ui-state.json`**

- Racine du dossier de données, propre à l'ordinateur, jamais dans la sauvegarde (`ui_state.rs:5-6`). Présent depuis la v0.11.0 (premier fichier à l'embarquer : `git ls-tree` sur `v0.10.1` ne le contient pas, `v0.11.0` oui ; une v0.10.x n'a donc aucun code qui le lit).
- Format : objet avec `version` (constante `VERSION = 1`, `ui_state.rs:16`) et, au plus, `folds` (booléens par identifiant), `benchCompare`, `dictionaryHeight` (180 à 1400), `uiLanguage` (`fr`/`en`/`auto`) (`ui_state.rs:48-91`). Seules les entrées connues et valides sont gardées et réécrites ; les autres clés sont ignorées puis perdues à la prochaine écriture (`ui_state.rs:50`, `:48-49`).
- Fichier absent, vide, non JSON, sans numéro ou d'un numéro différent de 1 mais inférieur : valeurs par défaut, sans erreur, remplacé à la prochaine écriture. `version > 1` : lu comme `Newer`, valeurs par défaut en lecture et **fichier laissé tel quel, jamais écrasé** ; fichier illisible pour une raison système (antivirus, droits) : jamais écrasé (`ui_state.rs:30-37`, `:98-114`). Aucune migration entre versions de `ui-state.json` n'existe (une seule version, 1).
- Conséquence : une version qui connaît `VERSION = 1` lisant un `ui-state.json` écrit par une version plus récente en `version: 2` l'ignore sans le toucher ; en `version: 1` avec des clés ajoutées, elle ignore les clés qu'elle ne connaît pas et les supprime à sa prochaine écriture.

### Incertitudes

- Les numéros de ligne ont été relevés sur l'état du dépôt du 20/11/2026 ; `commands.rs:24` est la ligne de l'appel à `migration::run` (je n'ai pas relu la fonction qui l'entoure).
- Non vérifié par exécution : le comportement réel de `codesign --verify` sur un zip signé par un autre certificat, et la mise à jour Linux avec une vraie Release (`docs/MISE_A_JOUR.md` la dit « à essayer »).
- Non lus (hors périmètre demandé) : l'historique des commits du chantier de migration au-delà des étiquettes, et les guides d'installation par système.
