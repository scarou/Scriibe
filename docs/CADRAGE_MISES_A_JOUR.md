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

## Note

Conception, sans code. Les références `fichier:ligne` renvoient à l'état des lieux ci-dessus. Pour chaque besoin, la note donne les options, la recommandation et sa raison, les risques et les tests. Les choix qui reviennent à l'auteur (usage, sécurité, irréversible) sont rassemblés en fin de note pour la tâche 3 ; la recommandation y est déjà écrite et la note la suit.

### 0. Principe : un seul mécanisme, le manifeste signé

Tout ce que Scriibe télécharge pour l'exécuter (l'application et les moteurs) passe par **un manifeste signé**, publié dans chaque Release de Scriibe : `scriibe-manifest.json` et sa signature `scriibe-manifest.json.sig`.

Contenu du manifeste (une Release = un manifeste) :

- `version` (x.y.z, égale à l'étiquette et à `Cargo.toml`), `date`, `minSchema`/`settingsSchema` (numéro de schéma des réglages que cette version écrit), `uiStateVersion` ;
- pour chaque système (`windows-x64`, `macos-arm64`, `linux-x64`) : nom du fichier de l'application, taille, **SHA-256** ;
- `engines` : pour whisper.cpp et llama.cpp, la version épinglée (étiquette `v1.9.5`, numéro de build `bNNNN`), et pour chaque système et variante (Windows CPU/Vulkan) : source (fichier de la Release de Scriibe ou de `ggml-org`), taille, SHA-256 ; plus les options obligatoires (`REQUIRED_OPTIONS`, `llama.rs:60`) vérifiées sur cette version.

Signature : **Ed25519 au format minisign**, vérifiée par une petite bibliothèque Rust pure (`minisign-verify`, sans dépendance système). La clé publique est compilée dans l'exécutable ; la clé privée ne quitte pas le Mac de l'auteur (voir choix D1).

Pourquoi ce choix plutôt que les autres :

| Option | Pour | Contre |
|---|---|---|
| Rien de plus (HTTPS vers github.com, état actuel) | aucun travail | un compte GitHub compromis, ou une CI compromise, publie un exécutable que tous les Scriibe installent ; rien ne vérifie les moteurs |
| Sommes SHA-256 dans un fichier de la Release, sans signature | simple | le même attaquant change le fichier et les sommes ensemble : ne protège que contre la corruption |
| Tauri updater (signature intégrée) | outil existant | `bundle.active` est faux (`tauri.conf.json:8`), son format suppose les installateurs Tauri ; ne couvre pas les moteurs ; ré-architecture du remplacement qui marche déjà |
| Signature du code par système (Authenticode, notarisation Apple) | reconnue par l'OS | payante, différente sur chaque système, ne couvre pas Linux ni les moteurs |
| **Manifeste signé Ed25519 (recommandé)** | un seul mécanisme pour 3 systèmes et pour les moteurs ; clé hors de GitHub ; vérifiable hors ligne ; s'ajoute au remplacement existant sans le refaire | une clé à garder (perte = plus de mise à jour intégrée, voir risques) ; une étape de publication de plus |

Publication : la CI construit et dépose les fichiers comme aujourd'hui (`release-windows.yml`, `release-linux.yml`, `publier_macos.sh`) et écrit leurs sommes ; puis un script `tools/signer_manifeste.sh`, lancé par l'auteur sur son Mac une fois la CI finie, télécharge les fichiers de la Release, recalcule les sommes **lui-même** (il ne fait pas confiance à celles de la CI), écrit le manifeste, le signe et le dépose. **Une Release sans manifeste valide n'est jamais installable** par Scriibe : elle est seulement signalée (« version publiée, pas encore vérifiable »).

Règles de vérification côté Scriibe, avant toute écriture sur le disque :

1. signature du manifeste valide avec la clé compilée ;
2. `version` du manifeste égale à l'étiquette de la Release lue ;
3. fichier téléchargé : taille et SHA-256 égales au manifeste (en plus des contrôles actuels `MZ`/ELF/`codesign`, gardés) ;
4. une version plus ancienne que l'installée n'est acceptée que par l'action explicite « revenir à » (§ 2), jamais par la vérification automatique : un manifeste ancien rejoué ne fait pas redescendre Scriibe en silence.

Risques : perte de la clé privée (plus aucune mise à jour intégrée ; il faut une installation à la main d'une version qui porte une nouvelle clé publique) ; vol de la clé (un attaquant signe ce qu'il veut, mais doit aussi publier sur le dépôt) ; rotation : le manifeste peut porter une `nextKey` signée par l'ancienne clé, prévue dès le lot 1, utilisée seulement en cas de besoin.

Tests : manifeste correct accepté ; signature fausse, clé inconnue, manifeste modifié d'un octet, somme différente, taille différente, mauvais système, version du manifeste différente de l'étiquette, manifeste absent : tous refusés **avant** toute écriture ; manifeste ancien refusé hors de « revenir à ». Ces tests sont unitaires (manifeste et clé de test compilés pour les tests seulement) ; aucun ne touche au réseau. Un essai réel se fait sur un dépôt d'essai (adresse du dépôt réglable par variable d'environnement en développement seulement, jamais dans les réglages).

### 1. Forcer la mise à jour (réinstaller la version installée)

Besoin : réinstaller même si l'on a déjà la dernière version (fichier abîmé, essai, doute).

Options :

- (a) Bouton « Réinstaller cette version » : télécharge le fichier de **la version installée** (Release `vX.Y.Z` lue par son étiquette, `GET /releases/tags/vX.Y.Z`, plus `/latest`), vérifié par son manifeste, puis le remplacement habituel.
- (b) Accepter qu'une Release republie le même numéro avec un autre contenu, et proposer « mettre à jour » quand le contenu change (comparaison du commit `SCRIIBE_COMMIT` ou de la somme).
- (c) Supprimer la comparaison et toujours proposer la dernière.

**Recommandation : (a).** Raison : la version (x.y.z) reste l'identité d'une livraison. Deux livraisons sous le même numéro rendraient faux tout ce qui s'appuie sur lui : l'historique (« Scriibe 0.12.0 » dans `bench_archive`, l'enveloppe d'export `app`), les retours d'erreur (« quelle version ? »), le retour en arrière (« revenir à 0.12.0 » : laquelle ?), et le manifeste signé (deux manifestes valides pour la même version). D'où la règle, à écrire dans `docs/CLOTURE.md` : **toute livraison porte un nouveau numéro** (un correctif minime donne 0.13.1). Avec cette règle, une livraison sans nouveau numéro n'existe pas, et (b) n'a rien à proposer ; « Réinstaller » sert à réparer, pas à recevoir du neuf. `publier_macos.sh` refuse déjà une étiquette existante (à vérifier en lot 1) ; les deux workflows aussi (contrôle étiquette = `Cargo.toml`).

Risques : réinstaller la même version remplace l'exécutable en cours : mêmes risques que la mise à jour (couverts par § 3). Aucun risque pour les données (même version, même schéma).

Tests : route d'installation avec une Release choisie par étiquette ; la page montre « Réinstaller 0.13.0 » même quand aucune version plus récente n'existe ; refus si le manifeste de cette version est absent.

### 2. Revenir à une version antérieure

#### 2.1 Quelle version et d'où

Options :

- (a) **Garder la version précédente sur le disque** (`scriibe.exe.old`, `scriibe.old`, `Scriibe.app.old`) au lieu de la supprimer au lancement suivant (`update.rs:387-402`), et proposer « Revenir à 0.12.0 » : immédiat, hors ligne, sans téléchargement.
- (b) Liste des Releases signées (`GET /releases`, filtrées sur celles qui ont un manifeste valide pour ce système), et choix d'une version.
- (c) Rien dans l'application : un guide pour installer à la main une ancienne Release.

**Recommandation : (a) + (b).** Raison : (a) couvre le cas le plus fréquent (« la mise à jour d'hier me gêne ») sans réseau ni GitHub, et c'est aussi le filet de sécurité du § 3 ; (b) couvre « deux versions plus tôt ». Le coût de (a) est la place d'une version de plus (≈ 20 à 40 Mo, application seule ; les moteurs ont leur propre version précédente, § 4). Les versions sans manifeste (toutes celles d'avant 0.13.0) ne sont pas proposées par (b) : on ne peut pas les vérifier (choix D3).

Limite à dire dans la page : une version d'avant 0.13.0 n'a pas le bouton « revenir » ni le filet du § 3 ; revenir jusqu'à elle est un aller simple vers l'ancien mécanisme (elle saura de nouveau proposer la dernière version, sans plus).

#### 2.2 Les données relues par une version plus ancienne

Ce qui se passe aujourd'hui (état des lieux § 3) :

- schéma de réglages plus récent (`schemaVersion` > connu) : lu, **jamais réécrit**, message « vient d'une version plus récente » : rien n'est perdu, mais l'ancienne version ne peut plus enregistrer de réglage (`storage.rs:92-128`) ;
- même schéma, clés ajoutées par la version plus récente : l'ancienne les ignore puis **les efface à sa prochaine écriture** (`settings.rs:770-780` pour `settings.json` ; `profile.json` les garde, `storage.rs:542-548`) ; `ui-state.json` de même (`ui_state.rs:48-50`) ;
- valeur d'énumération inconnue (nouveau mode, nouveau moteur) : remplacée par la valeur par défaut à la lecture, puis réécrite (perte de la valeur) ;
- élément de liste illisible (`customModes`) : écarté puis effacé à l'écriture (perte du mode).

Options :

- (a) Ne rien changer, prévenir dans la page avant de revenir.
- (b) **Préserver ce qu'on ne comprend pas** : à l'écriture, repartir du JSON lu sur le disque et n'y remplacer que les clés connues ; garder les clés inconnues, et pour une clé connue dont la valeur est illisible, **ne pas la réécrire** tant que l'utilisateur ne l'a pas changée ; pour une liste, garder tels quels les éléments illisibles. Même règle pour `ui-state.json` et `profile.json`.
- (c) **Instantané des données à chaque changement de version** (montée, réinstallation, retour) : copie de `settings.json`, `ui-state.json` et `utilisateurs/` (sans l'historique audio, il n'y en a pas ; `history.json` compris) dans `sauvegardes-versions/<de>-vers-<à>-<date>/`, les 5 derniers gardés. Au retour vers la version X, proposer : « garder tes réglages actuels » (défaut si le schéma est le même) ou « reprendre ceux que tu avais en quittant X » (instantané pris lors du passage de X à la suivante ; les changements faits depuis sont perdus, la page le dit).
- (d) Migrations descendantes (la version récente sait réécrire ses données au format ancien avant de revenir).

**Recommandation : (b) + (c), et une règle de conception.** Raisons : (b) supprime la perte silencieuse, la seule vraie perte de données du retour en arrière ; elle ne protège que les versions qui portent ce code, d'où son arrivée dans le **premier lot** (0.13.0) : tout retour vers 0.13.0 ou plus récent sera sans perte. (c) est le filet pour tout le reste (retour vers une version d'avant 0.13.0, schéma plus récent qui rend l'ancienne version « lecture seule »), et coûte quelques centaines de Ko. (d) est écartée : chaque version devrait connaître toutes les suivantes, ce qui est impossible ; et pour les données, (b) + (c) suffisent. Règle de conception, à écrire dans `docs/ARCHITECTURE.md` : **ajouter une clé ne change jamais le sens d'une clé existante, et ne change pas `schemaVersion`** ; on ne monte `schemaVersion` que pour un changement de structure que l'ancienne version lirait de travers (comme le passage à `utilisateurs/`). Quand on le monte, le retour vers une version plus ancienne propose d'office l'instantané (c), puisque l'ancienne serait en lecture seule.

Cas particulier à traiter : `schemaVersion` plus récent + retour en arrière. L'ancienne version affiche les réglages mais refuse d'enregistrer. Au moment du retour (fait par la version récente, qui sait tout), la page dit : « 0.12.0 ne sait pas lire tes réglages actuels ; elle repartira de ceux que tu avais en la quittant (instantané du 20/11), tes réglages actuels restent sauvegardés ». Rien n'est effacé.

Risques : (b) mal fait peut garder une valeur périmée que l'utilisateur croit avoir changée : la règle « une clé connue est réécrite dès que l'utilisateur la change » doit être testée ; (c) restaure un instantané par-dessus des données plus récentes : on sauvegarde toujours l'état courant juste avant (instantané de plus), jamais d'écrasement sans copie ; un instantané peut contenir des clés d'API ? Non : les clés vivent dans le trousseau du système, pas dans `settings.json` (à revérifier au lot 1 ; si une clé s'y trouvait, l'instantané ne la copie pas).

Tests : clé inconnue gardée après une écriture (`settings.json`, `profile.json`, `ui-state.json`) ; valeur d'énumération inconnue gardée tant qu'on ne la change pas, remplacée dès qu'on la change ; mode personnalisé illisible gardé ; instantané créé à chaque changement de version, 5 gardés, restauration qui sauvegarde d'abord l'état courant ; schéma plus récent : retour qui propose l'instantané. Tests unitaires dans `crates/platform` avec dossiers temporaires (`Path::join`, voir CLAUDE.md).

### 3. Scriibe qui ne redémarre plus après un changement de version

Aujourd'hui : le remplacement est sûr **pendant** l'échange (remis en place si une étape échoue, `update.rs:291-314`, `:359-370`), mais si la nouvelle version plante au démarrage, plus rien ne la remplace : l'utilisateur doit renommer `.old` à la main, et ce `.old` est supprimé au premier lancement réussi… ou à n'importe quel lancement qui va jusqu'à `commands.rs:126`, même s'il plante juste après.

Options :

- (a) Guide « renommer le `.old` » dans l'aide.
- (b) **Veilleur** : au redémarrage, ce n'est plus un `sh` (Mac) ou la nouvelle version directement (Windows, Linux) qui relance, mais **l'ancienne version** lancée avec une sous-commande `scriibe veiller-mise-a-jour` (depuis `.old`, qui reste exécutable). Elle lance la nouvelle, attend jusqu'à 90 s qu'elle écrive « démarrée » dans `mise-a-jour.json` (après le serveur de la page prêt et la première lecture des réglages réussie), et sinon : arrête la nouvelle, remet l'ancienne en place, la relance avec un message « la version 0.13.1 n'a pas démarré, retour à 0.13.0 ».
- (c) Démarrage en deux temps dans la nouvelle version elle-même (elle se surveille) : ne couvre pas un plantage avant ce code.

**Recommandation : (b) + (a).** Raison : seul un processus **extérieur à la nouvelle version** peut constater qu'elle ne démarre pas ; l'ancienne version est déjà là, vérifiée, et connaît l'emplacement de tout. (a) reste pour le cas où les deux versions sont cassées. Le `.old` n'est plus supprimé au lancement : il devient la « version précédente » du § 2.1 et n'est remplacé qu'au changement de version suivant.

Particularités par système : Windows ne permet pas de remplacer un `.exe` en cours d'exécution mais permet de le renommer (déjà utilisé) ; le veilleur tourne depuis `scriibe.exe.old` et ne remplace donc que `scriibe.exe`. macOS : le veilleur est `Scriibe.app.old/Contents/MacOS/Scriibe` ; relance par `open` (les autorisations macOS vont à l'application, voir CLAUDE.md) ; le veilleur ne doit **pas** être lancé depuis le Terminal pour garder les droits d'accessibilité attribués à `Scriibe.app`. Linux : relance directe ; le crochet clavier GNOME ne dépend pas du chemin.

Risques : le veilleur lui-même peut avoir un défaut (il est l'ancienne version, déjà en service : risque faible, mais il n'existe qu'à partir de 0.13.0 comme ancienne version, donc effectif à partir de la mise à jour 0.13.0 → 0.13.1) ; boucle de retours (une version qui démarre une fois sur deux) : un seul retour automatique par version, ensuite on prévient ; fausse alerte sur une machine lente : délai de 90 s, et « démarrée » est écrit tôt (avant le chargement des modèles, qui peut être long).

Tests : unitaires sur l'automate du veilleur (horloge simulée de `testing::FakeWorld`) : démarrée à temps, jamais démarrée, plantée tout de suite, démarrée mais trop tard ; un seul retour automatique. Essai réel sur les 3 systèmes au lot 1 avec une version d'essai volontairement cassée publiée sur le dépôt d'essai.

### 4. Mettre à jour llama.cpp et whisper.cpp

#### 4.1 Version liée à celle de Scriibe ou indépendante

Options :

- (a) **Liée** : chaque version de Scriibe épingle une version de whisper.cpp et de llama.cpp (fichier `engines.lock.json` du dépôt : étiquette, build, sommes par système), vérifiée par la CI avant publication ; le manifeste les reprend. Mettre à jour un moteur = publier une version de Scriibe (0.13.2 « llama.cpp b8xxx »).
- (b) Indépendante : Scriibe propose la dernière Release de `ggml-org` dès qu'elle sort.
- (c) Mixte : liée par défaut, plus un réglage avancé « essayer la dernière version des moteurs ».

**Recommandation : (a).** Raisons : llama.cpp sort plusieurs builds par jour et retire ou renomme des options (d'où `REQUIRED_OPTIONS`, `llama.rs:59-71`, aujourd'hui vérifié seulement par un test sur un texte figé, `engines/tests/fixtures.rs:192`) ; une version non essayée peut casser la dictée sans que l'auteur l'ait vue. Liée, chaque combinaison a été vérifiée par la CI et par l'auteur, et le retour en arrière de Scriibe ramène aussi des moteurs qui marchent avec lui. Le coût (une version de Scriibe pour monter un moteur) est faible : la publication est déjà outillée. (c) ajoute une combinaison non vérifiée à soutenir ; on pourra l'ajouter plus tard si le besoin apparaît.

Conséquence : le téléchargement Windows « sans version fixée » (`downloads.rs:163-175`, `:216-235`) et la llama Linux non épinglée (`release-linux.yml:106-125`) disparaissent ; la première Release « parmi les 30 dernières » aussi (`downloads.rs:163-175`) : Scriibe prend les moteurs **de son propre manifeste**.

#### 4.2 Vérification de ce qui est téléchargé

Les moteurs sont décrits dans le manifeste signé (§ 0) avec leur SHA-256 ; **ils sont vérifiés comme l'application**. Source : pour Windows, les zips de `ggml-org` peuvent rester la source (la somme du manifeste suffit, quelle que soit la source), mais la CI de Scriibe les **recopie dans sa Release** pour ne pas dépendre de la disparition d'un fichier chez `ggml-org` (recommandé : la Release de Scriibe est la seule source). Linux : déjà dans la Release de Scriibe. macOS : voir § 4.5.

#### 4.3 Où ils sont rangés, essai avant bascule

- Dossiers versionnés : `engines/whisper/v1.9.5/`, `engines/llama/b8123-vulkan/`, et un fichier `engines/actuels.json` qui dit lesquels servent. Les deux derniers de chaque moteur sont gardés (version actuelle et précédente) ; les autres supprimés après une bascule réussie.
- **Essai avant bascule**, dans le nouveau dossier, sans toucher à l'actuel :
  1. `--help` contient toutes les options de `REQUIRED_OPTIONS` (llama) ; `whisper-server --help` connaît les options passées (`whisper.rs:200`) ;
  2. lancement sur un port libre de réserve, avec **les modèles réellement configurés** de l'utilisateur ; attente du `/health` (llama, `llama.rs:217`) ou de `/` (whisper, `whisper.rs:160`) ;
  3. une vraie requête courte : une complétion de quelques jetons (llama), la transcription d'un WAV d'une seconde embarqué dans Scriibe (whisper) ; réponse non vide et sans erreur ;
  4. arrêt du moteur d'essai, puis bascule (`actuels.json` réécrit de façon atomique) ; le moteur en service est redémarré à la prochaine dictée inactive, jamais au milieu d'une dictée.
- **Retour automatique** : si le moteur choisi ne démarre pas (n'atteint pas `/health` dans le délai déjà utilisé) deux fois de suite après la bascule, Scriibe repasse à la version précédente gardée et le dit (« llama.cpp b8123 n'a pas démarré, retour à b8001 »). Il ne recommence pas avec la même version avant la version suivante de Scriibe.
- Si l'essai échoue (étape 1 à 3) : rien n'est basculé, le dossier d'essai est supprimé, la page dit à quelle étape et le journal garde la sortie du moteur.

**Pourquoi un essai avec les modèles de l'utilisateur** : c'est là que se cachent les vraies incompatibilités (architecture de modèle inconnue d'un vieux llama, quantification retirée d'un nouveau, voir § 5) ; un essai à vide passerait et la dictée échouerait.

#### 4.4 Ordre de recherche des moteurs

Aujourd'hui un moteur système (Homebrew, `/usr/bin`) passe **avant** celui du dossier des moteurs (`whisper.rs:100-116`, `llama.rs:163-178`) : une mise à jour gérée par Scriibe serait sans effet si un moteur système existe. Recommandation : le moteur géré (`actuels.json`) passe d'abord ; un moteur système n'est pris que s'il n'y en a pas de géré, ou si un réglage avancé « utiliser les moteurs du système » est coché (pour qui compile llama.cpp lui-même). Ce réglage est un choix d'usage (D2 pour le Mac).

#### 4.5 Le Mac

Aujourd'hui l'utilisateur installe les moteurs par Homebrew (`brew install`), hors de la portée de Scriibe : la version n'est ni connue ni vérifiée, `brew upgrade` peut casser Scriibe sans prévenir.

Options : (a) garder Homebrew, Scriibe lit la version (`--version`) et prévient si elle diffère de celle du manifeste ; (b) **moteurs gérés aussi sur le Mac** : la CI (machine macOS arm64 de GitHub) compile `whisper-server` (comme pour Linux) et reprend le zip `macos-arm64` de llama.cpp, vérifiés par le manifeste, rangés comme ailleurs, Homebrew devenant le réglage « moteurs du système ». Recommandation : **(b)**, pour avoir un seul mécanisme sur les trois systèmes et des moteurs essayés ; les binaires téléchargés par Scriibe lui-même ne reçoivent pas l'attribut de quarantaine, Gatekeeper ne les bloque donc pas (à vérifier à l'essai réel ; sinon signature ad hoc par la CI). Choix D2, car il change l'installation de l'auteur sur son Mac.

#### Risques (§ 4)

- Sécurité : un moteur est un exécutable lancé à chaque dictée : sans manifeste, c'est la même faille que l'application ; avec, elle est fermée. Les zips de moteurs sont décompressés **à plat** dans le dossier versionné : refuser tout nom d'entrée avec `..` ou chemin absolu (à vérifier dans le code actuel au lot 3).
- Données : aucune (les moteurs n'écrivent rien de l'utilisateur).
- Dictée qui ne marche plus : couverte par l'essai et le retour automatique ; reste le cas d'un moteur qui démarre mais donne un texte faux (régression de qualité) : hors de portée d'un essai automatique ; c'est l'essai de l'auteur avant publication (banc de dictée) qui le couvre.
- Place : deux versions de chaque moteur, ≈ 50 à 150 Mo selon la variante (Vulkan plus lourde).

#### Tests (§ 4)

`FakeWorld` (processus et horloge simulés) : essai réussi puis bascule ; échec à chaque étape (option manquante dans `--help`, pas de `/health`, réponse vide) sans bascule ; retour automatique après deux échecs de démarrage, un seul ; ordre de recherche avec `actuels.json`, avec et sans moteur système, avec le réglage « moteurs du système » ; nettoyage des versions au-delà de deux. CI : un travail qui télécharge les moteurs épinglés pour Linux et lance `--help` contre `REQUIRED_OPTIONS` (vrai binaire, pas de modèle), ce qui remplace le texte figé du test actuel au moment d'une montée de version.

### 5. Formats de modèles qu'un nouveau moteur doit lire

Compatibilité seule ; le Magasin de modèles reste hors périmètre.

- **llama.cpp** lit le format **GGUF** (version 3 du conteneur, la seule utilisée aujourd'hui). Scriibe propose et trouve des `.gguf` (`downloads.rs:157`, `:188`, `:246-248`) et lit ceux de LM Studio. Ce qui peut casser d'une version de llama.cpp à l'autre n'est pas le conteneur mais : (1) **l'architecture du modèle** (un modèle récent, comme gemma-4 ou le modèle assistant `draft-mtp`, `llama.rs:54`, demande un llama récent : un retour à une version ancienne de llama.cpp peut le rendre illisible) ; (2) **les quantifications retirées** (llama.cpp a déjà supprimé des types, par exemple `Q4_0_4_4` ; un nouveau llama peut refuser un vieux fichier) ; (3) le **modèle de discussion** embarqué lu par `--jinja` (`llama.rs:48`).
- **whisper.cpp** lit les modèles **ggml `.bin`** (`ggml-base.bin`, `ggml-small.bin`, `ggml-large-v3-turbo`, `downloads.rs:244-245`) ; le format est stable depuis longtemps, les risques sont les mêmes en plus faible (quantifications, nouvelles variantes de modèle).
- Règle pour le manifeste : chaque version de moteur épinglée doit lire **les modèles proposés par Scriibe** (liste de `downloads.rs`) **et** les modèles réellement configurés chez l'utilisateur ; les premiers sont vérifiés par l'auteur avant publication, les seconds par l'essai avant bascule (§ 4.3, étape 3), qui est le vrai garde-fou. Le retour en arrière d'un moteur (§ 4.3) refait le même essai : si la version précédente ne lit plus le modèle choisi depuis, elle n'est pas rétablie et la page le dit.

### 6. Écarts relevés à corriger en passant

- `docs/MISE_A_JOUR.md` dit Linux « simple alerte » alors que le code installe (`update.rs:64`, `:451`) : à corriger au lot 1.
- `ui/src/lib/api.ts:73` déclare `update: { sha, … }` alors que le serveur envoie `{version, title, date}` (`mod.rs:820`) : à corriger au lot 1.
- `codesign --verify` ne contrôle pas l'identité du signataire (`update.rs:340-355`) : la somme du manifeste rend ce point secondaire ; on garde `codesign` en plus.
- Casse du dossier des moteurs sous Linux (`Scriibe/engines`) différente du dossier de données (`scriibe`) (`lib.rs:25-37`) : sans conséquence, à ne pas changer (déplacerait les moteurs déjà installés) ; noter dans `docs/ECARTS.md` si utile.

### 7. Choix qui reviennent à l'auteur (pour la tâche 3)

La note suit déjà la recommandation de chacun.

- **D1 · Sécurité : où vit la clé privée de signature.** Recommandé : sur le Mac de l'auteur seulement (fichier minisign protégé par mot de passe, plus une copie hors ligne, par exemple sur une clé USB rangée) ; la signature se fait par `tools/signer_manifeste.sh` après la CI. Autre option : secret GitHub (publication entièrement automatique, mais un dépôt ou une CI compromis peut alors signer). Conséquence de la perte de la clé : plus de mise à jour intégrée jusqu'à une installation à la main.
- **D2 · Usage : moteurs gérés par Scriibe sur le Mac** (au lieu de Homebrew), Homebrew restant possible par un réglage avancé. Recommandé : oui (un seul mécanisme, moteurs essayés). Sinon : le Mac reste à part, Scriibe prévient seulement d'un écart de version.
- **D3 · Sécurité : retour vers une version d'avant 0.13.0** (sans manifeste, donc invérifiable). Recommandé : non proposé dans la liste ; possible seulement à la main (guide). Sinon : proposé avec un avertissement « non vérifié ».
- **D4 · Usage et données : au retour en arrière, que deviennent les réglages par défaut.** Recommandé : garder les réglages actuels si le schéma est le même, reprendre l'instantané de la version visée si son schéma est plus ancien ; dans les deux cas, l'autre choix reste proposé et rien n'est effacé.
- **D5 · Usage : nombre de versions gardées sur le disque.** Recommandé : une version précédente de l'application, deux versions de chaque moteur, cinq instantanés de données.

### 8. Lots d'implémentation

| Lot | Version | Contenu | Pourquoi dans cet ordre |
|---|---|---|---|
| 1 | **0.13.0** | Manifeste signé (format, vérification, `signer_manifeste.sh`, clé, `nextKey` prévue) pour l'application ; « Réinstaller cette version » ; préservation des clés et valeurs inconnues (`settings.json`, `profile.json`, `ui-state.json`) ; instantané des données à chaque changement de version ; `.old` gardé ; veilleur de démarrage avec retour automatique ; règle « toute livraison a un nouveau numéro » (`docs/CLOTURE.md`) et règle d'ajout de clé (`docs/ARCHITECTURE.md`) ; écarts du § 6 | Tout ce qui doit déjà être dans une version pour que les **suivantes** puissent revenir vers elle sans perte et sans danger : c'est la fondation, et la sécurité du téléchargement d'abord |
| 2 | **0.14.0** | Retour en arrière : vers la version précédente gardée (hors ligne) et vers une version choisie dans la liste des Releases signées ; choix des données au retour (D4) ; page « Mise à jour » refaite pour ces actions (textes FR + `en.ts`) | Demande directe du carnet ; s'appuie sur le manifeste, l'instantané et le veilleur du lot 1 ; premier retour possible : 0.14.0 → 0.13.0 |
| 3 | **0.15.0** | Moteurs gérés : `engines.lock.json`, CI qui recopie et vérifie les moteurs pour les 3 systèmes (dont macOS si D2), moteurs dans le manifeste, dossiers versionnés et `actuels.json`, essai avant bascule, retour automatique, nouvel ordre de recherche et réglage « moteurs du système », version des moteurs affichée ; contrôle `..` dans les zips ; fin des téléchargements « dernière Release » | Le plus gros lot et le plus risqué pour la dictée ; il réutilise le manifeste et la vérification du lot 1 ; séparé pour être essayé seul sur les 3 systèmes |
| 4 | **0.15.x** | Retour manuel d'un moteur vers sa version précédente (page Installation) ; première montée de llama.cpp par ce mécanisme, en vrai, comme essai de bout en bout | Finition après usage réel du lot 3 |

Chaque lot se découpe ensuite avec /superviser ; chaque lot finit par un essai réel sur Windows, Mac et Linux (dépôt d'essai pour les lots 1 et 2, version cassée volontaire pour le veilleur).
