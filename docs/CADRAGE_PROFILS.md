# Cadrage des profils et des dictionnaires (chantier 4)

Cette note prépare deux idées du carnet, du 11/11 :

- « Profils : plusieurs profils dans une même installation de Scriibe ? »
- « Dictionnaires : import et export, et plusieurs dictionnaires selon le contexte ou le profil »

C'est un cadrage seulement : on ne code rien. On découpera ensuite l'implémentation à partir de cette note. La note fixe aussi le format d'import et d'export qui servira aux modes (chantier 5).

Sections :

- Constat (tâche 0)
- Cadrage (tâche 1)
- Relecture (tâche 2)

## Constat

Le constat a été fait le 12/11 par un sous-agent de recherche. Il a lu le code, les tests, les fixtures et la doc de Scriibe (v0.3.0 et le chantier 3), ainsi que le code Swift de Scribe. Il n'a ouvert aucun vrai fichier de réglages et aucune clé. Les exemples ci-dessous sont reconstruits d'après le code.

Le résumé tient en quatre points :

- Tout est en JSON, dans un seul dossier de données.
- Les réglages, le prénom, le genre et les modes personnels sont dans `settings.json`. Le dictionnaire est à part, dans `dictionary.json`.
- Il n'y a ni numéro de version, ni copie de secours, ni import ou export, ni notion de profil, que ce soit dans Scriibe ou dans Scribe.
- Les clés d'API ne sont jamais dans les fichiers.

### 1. Dossier de données et fichiers

Le dossier de données est donné par `data_folder()` dans `crates/platform/src/storage.rs:16`. La variable `SCRIIBE_DATA` le remplace partout, ce qui sert aux essais.

| Système | Dossier |
|---|---|
| Windows | `%APPDATA%\Scriibe` (les moteurs sont dans `%LOCALAPPDATA%\Scriibe\engines`) |
| macOS | `~/Library/Application Support/Scriibe` |
| Linux | `$XDG_DATA_HOME/scriibe`, sinon `~/.local/share/scriibe` |

Fichiers du dossier :

| Fichier | Contenu |
|---|---|
| `settings.json` | Réglages, prénom, genre, modes personnels |
| `dictionary.json` | Dictionnaire |
| `history.json` | Historique des dictées : textes et nom des applis |
| `history-totals.json` | Totaux par jour, sans texte |
| `bancs/banc-<date>-<id>.json` | Passages du banc d'essai |
| `models/` | Modèles de voix et de texte |
| `scriibe.log` | Journal technique |
| `instance.lock`, `instance.url` | Garantissent une seule Scriibe par dossier |

Lecture et écriture :

- **Lecture.** Elle passe par `FileConfig` et `Watched<T>` (`storage.rs:58-142`). Un fichier est relu dès que sa date de modification change : une modification à la main s'applique à la dictée suivante.
- **Écriture.** `write_atomic` (`storage.rs:49`) écrit d'abord un fichier temporaire au nom fixe (`settings.tmp`, `dictionary.tmp`), puis le renomme. Il n'y a pas de verrou entre processus.
- **Fichier illisible.** On repart des réglages du premier lancement. Le fichier cassé est écrasé à la sauvegarde suivante, **sans copie `.bak`**. Scribe, lui, met le fichier de côté sous `<nom>.<horodatage>.bak` (`Store.swift:121-132`).

### 2. Réglages (`settings.json`)

**Format.** C'est du JSON indenté, avec des clés camelCase identiques à celles de Scribe.

- **Struct.** C'est `AppSettings` (`crates/core/src/settings.rs:214-413`).
- **Lecture.** Elle est tolérante, champ par champ, dans `AppSettings::from_value` (l.439-523) : une valeur illisible prend sa valeur par défaut.
- **Version.** Le fichier n'a **aucun numéro de version**. La compatibilité ne repose que sur la tolérance de la lecture, par exemple :
  - les anciens `startSound` et `endSound` (booléens) sont convertis en `startSoundName` et `endSoundName` ;
  - l'ancien `rewrites` est converti en `fidelity` ;
  - un `customModeID` qui pointe sur un mode disparu est effacé.
- **Champs inconnus.** Ils sont perdus à la première sauvegarde. C'est le cas d'un champ écrit par une version plus récente, et de même côté Scribe.

**Exposition à l'interface.** Le serveur local est dans `crates/cli/src/web/mod.rs`.

- `GET api/settings` renvoie les réglages.
- `POST api/settings` appelle `merge_settings`, qui fusionne clé par clé. Il refuse une clé inconnue, sauf celles de `OPTIONAL_KEYS = ["historyKeepDays","customModeID"]` (l.698). Le résultat repasse ensuite par `from_value`.
- Côté interface, c'est le store `ui/src/lib/settings.svelte.ts`.
- Le menu de l'icône (`crates/cli/src/tray_source.rs`) relit le même fichier.

Extrait reconstruit :

```json
{
  "hotkey": "rightControl", "language": "fr",
  "transcriptionEngine": "whisper", "whisperModelPath": "",
  "cleanupEnabled": true, "cleanupEngine": "llamaCpp", "llamaCppModel": "",
  "rewriteMode": "simple", "simpleFidelity": "light",
  "speakerGender": "unspecified", "firstName": "",
  "styleOverrides": {}, "customModes": [],
  "microphoneUID": "", "dictationKeys": [], "translateKeys": [],
  "uiLanguage": "auto", "theme": "auto", "setupDone": false
}
```

`customModeID` et `historyKeepDays` ne sont écrits que s'ils ont une valeur.

### 3. Dictionnaire (`dictionary.json`)

**Stockage.** C'est un fichier à part, lu et écrit par `FileConfig.dictionary` et `save_dictionary` (`storage.rs:133-142`). Il est vide par défaut ; celui de Scribe contient 5 mots d'exemple.

**Structure.** Elle est dans `crates/core/src/dictionary.rs:9-39` : `UserDictionary { words: Vec<String>, corrections: Vec<Correction{heard, written}> }`. Une correction n'a pas d'identifiant. La lecture est tolérante.

Exemple reconstruit :

```json
{ "words": ["Claude Code", "to-do"],
  "corrections": [{ "heard": "cri be", "written": "Scriibe" }] }
```

**Usage.** Seuls les `words` sont envoyés aux modèles :

- **Whisper.** Les mots partent dans le champ `prompt` (`engines/src/whisper.rs:339-358`).
- **Deepgram.** Ils partent en `keyterm`, 100 mots au plus (`services.rs:51-66`).
- **Prompt de mise au propre et de traduction.** Les mots sont ajoutés par `cleaner::system_prompt` (`core/src/cleaner.rs:319-358`), sous la forme « écris-le exactement ainsi ».

Les `corrections` ne sont jamais envoyées à un modèle. Elles s'appliquent localement par `dictionary::apply_for` (`dictionary.rs:56-95`), qui :

- applique les signes dictés ;
- remplace « entendu → écrit », sur un mot entier, sans tenir compte de la casse ;
- remet l'orthographe exacte des `words`.

`apply_for` est appelée avant le modèle, pour la traduction et après le modèle (`pipeline/src/controller.rs:850, 866, 905`).

**Modification.**

- La page `ui/src/tabs/Dictionary.svelte` renvoie le dictionnaire **entier** en `POST api/dictionary`, 500 ms après la dernière modification.
- `clean_dictionary` (`web/mod.rs`) retire les blancs et dédoublonne les mots sans tenir compte de la casse. **Il ne dédoublonne pas les corrections**, alors que Scribe les dédoublonne sur `heard`.
- La commande vocale « ajouter au dictionnaire » préremplit la page via `api/prefill`.

### 4. Modes personnels (`customModes`)

**Stockage.** Ils sont dans `settings.json`, sans fichier à part :

- `customModes` : un tableau ;
- `customModeID` : le mode personnel choisi ;
- `styleOverrides` : les phrases de style modifiées des modes livrés, par exemple `{"formal": "…"}`.

**Structure.** Elle est dans `crates/core/src/modes.rs:300-406` :

- `CustomMode { id, name, style, fidelity, vouvoie, examples: [{id, dictation, expected}] }` ;
- l'`id` est un UUID en majuscules, créé par la page avec `crypto.randomUUID().toUpperCase()` ;
- `fidelity` vaut `literal`, `light`, `reformulated` ou `free` ;
- l'écriture ajoute `rewrites`, un booléen déduit de `fidelity`, pour Scribe et les anciens fichiers. Scribe ne connaît que `rewrites` et n'a pas de `fidelity`.

Exemple reconstruit :

```json
{ "customModes": [{
    "id": "6F9619FF-8B86-D011-B42D-00CF4FC964FF", "name": "Mon mode",
    "style": "Écris court et direct.", "fidelity": "reformulated", "rewrites": true,
    "vouvoie": false,
    "examples": [{ "id": "…", "dictation": "…", "expected": "…" }] }],
  "customModeID": "6F9619FF-8B86-D011-B42D-00CF4FC964FF",
  "rewriteMode": "simple" }
```

**Lien avec les modes livrés.**

- Un mode personnel s'appuie toujours sur Simple (`settings.rs:453, 645-670`).
- `active_profile()` (l.622) donne le profil de mode actif, avec le genre global.

**Interface.**

- La page Modes (`ui/src/tabs/Modes.svelte:35-72`) envoie le tableau `customModes` **entier** à chaque sauvegarde ou suppression.
- Le choix d'un mode passe par `api/modes/choose`.
- Le menu de l'icône a un groupe « Mode personnel ». La concordance entre la page et le menu est testée dans `crates/cli/src/mode_sync.rs`.

**Mode par application.** Il n'est pas implémenté. C'est une idée de Scribe, notée « écartée, après la parité » dans `ROADMAP.md:72`. L'appli au premier plan est déjà connue : `platform/src/apps.rs` (`ForegroundApp`) donne l'exécutable sous Windows et Linux, et le bundle sous macOS. Elle est gardée dans l'historique (`appName`, `bundleID`).

### 5. Prénom et genre

**`firstName`** (chantier 3, `settings.rs:249-250`).

- C'est une chaîne de texte, vide par défaut et nettoyée des espaces. Saisie dans `Settings.svelte`, 40 caractères au plus.
- `greeting_name()` (l.612-619) donne le prénom saisi, sinon celui du compte de l'ordinateur (`platform/src/machine.rs:89-129`) :
  - macOS : `id -F` ;
  - Linux : `getent passwd` ;
  - Windows : identifiant de connexion, avec une majuscule.
- Il sert **uniquement** à la salutation du tableau de bord (`api/status` → `first_name` → `Dashboard.svelte`). Il n'a aucun effet sur la dictée ni sur le prompt.
- Scribe n'a pas ce réglage : il calcule le prénom à chaque lancement.

**`speakerGender`** (`settings.rs:246-248`, enum `Gender` dans `modes.rs:49-79`).

- Valeurs : `unspecified` (par défaut), `masculine` ou `feminine`.
- Il ajoute une ligne au prompt système, **seulement pour les modes qui réécrivent** : Soutenu, Apaisé, métiers, et modes personnels Reformulé ou Libre (`cleaner.rs:327-343`).
- Les modes personnels ne portent pas de genre : c'est toujours le réglage global qui s'applique.
- Scribe n'a pas de genre.
- Le code (`settings.rs:249`) et `docs/PETITS_PLUS.md` notent déjà que ces deux champs « sont de la personne » et ont vocation à passer dans un profil.

### 6. Empreinte et stabilité du prompt

- **L'empreinte `4ed86f0c1c154918` ne dépend d'aucune donnée de l'utilisateur.** Elle est calculée par `built_in_prompts_fingerprint()` (`cleaner.rs:454-467`) avec un vocabulaire fixe, le genre « non précisé » et aucune phrase de style modifiée. Ni le dictionnaire, ni le prénom, ni le genre, ni `styleOverrides` ne la changent.
- **Le préfixe du prompt, lui, dépend du profil.** Il change avec :
  - les `words` ;
  - le mode, la phrase de style, `fidelity`, `vouvoie` et les exemples ;
  - le genre ;
  - la langue de traduction.
- `preload_signature` (`controller.rs:106-123`) surveille ces éléments. Quand l'un change, le préchargement du modèle est relancé, ce qui peut prendre jusqu'à 30 s sur un processeur lent.
- La règle « prompt identique d'une dictée à l'autre » reste donc vraie à l'intérieur d'un profil. Changer de profil coûte un préchargement.

### 7. Ce qui dépend du système

- **Le dossier de données.** Seul son emplacement dépend du système ; tout le reste (réglages, dictionnaire, modes, historique) est du JSON commun aux trois systèmes.
- **Les secrets.** Clés `deepgram` et `openrouter`, jamais dans les fichiers ; l'interface ne reçoit que des booléens (`platform/src/secrets.rs`, `lib.rs:46-65`).

  | Système | Où sont rangées les clés |
  |---|---|
  | Windows | Gestionnaire d'identifiants, `Scriibe/deepgram` et `Scriibe/openrouter` (`windows/credentials.rs`) |
  | macOS | Trousseau, service `Scriibe` (`macos/credentials.rs`) |
  | Linux | **Aucun coffre** : seulement les variables d'environnement, et `save_secret` renvoie une erreur |

  Dans les trois cas, les clés sont propres à l'utilisateur du système, pas à un profil.
- **Les champs propres à la machine.** Ils sont dans `settings.json` et ne doivent pas suivre un profil copié d'un ordinateur à l'autre :
  - `hotkey`, `dictationKeys`, `translateKeys` ;
  - `whisperModelPath`, `llamaCppModel` (chemins absolus) ;
  - `microphoneUID`, `microphoneName` ;
  - `llamaDevice`, `lmStudioURL`, `ollamaURL` ;
  - les sons.

  Une `hotkey` absente du système est déjà remplacée à la lecture (`storage.rs:127`). Un chemin de modèle ou un micro absents ne le sont pas.
- **L'état propre à la fenêtre.** Quelques réglages vivent dans le navigateur de la fenêtre (`localStorage`, `sessionStorage`) : cartes repliées, onglet courant.

### 8. Scribe et l'import-export existant

**Scribe** range ses données dans `~/Library/Application Support/Scribe/` (`Sources/Scribe/Store.swift`) :

- Mêmes fichiers et mêmes clés que Scriibe : `settings.json`, `dictionary.json`, `history.json`, `bancs/`. Il n'utilise pas `UserDefaults` pour ses données.
- Les réglages sont écrits 500 ms après la dernière modification.
- Un fichier illisible est mis de côté sous `.bak`.
- Il n'a ni profil, ni import, ni export. Les seules fenêtres de choix de fichier servent à choisir un modèle.

**Import et export : il n'y en a nulle part.** Ce qui s'en rapproche :

- Copier le dossier de Scribe vers Scriibe marche à la main, puisque le format est le même. Scribe perd en revanche les champs propres à Scriibe à sa prochaine écriture.
- Un `bancs/*.json` copié à la main est pris en compte.
- L'icône a un menu « Ouvrir le dossier des données ».
- On peut modifier `settings.json` ou `dictionary.json` à la main : ils sont relus à chaud.

### 9. Points à surveiller pour le cadrage

1. **Version et sauvegarde.** Les fichiers n'ont pas de numéro de version et perdent les champs inconnus ; un fichier illisible est écrasé sans `.bak`. Une migration vers des profils devra ajouter une version et une copie de secours avant d'écraser.
2. **Ce qui est à la personne, ce qui est à la machine.**
   - À la personne : `firstName`, `speakerGender`, `language`, `translationLanguage`, `customModes`, `customModeID`, `styleOverrides`, `simpleFidelity`, le dictionnaire, `voiceCommands`, `customCommands`, `listBullet`.
   - À la machine : les touches, les chemins de modèles, le micro, les URL, `llamaDevice`, les moteurs, l'apparence, les clés.
3. **Le prompt.** Changer de profil relance le préchargement du modèle (voir § 6). Les corrections du dictionnaire n'entrent pas dans le prompt, seuls les mots y entrent.
4. **Import de modes.** Il faudra regénérer les `id` en cas de collision. Un `customModeID` orphelin est déjà remis à vide à la lecture.
5. **Import de dictionnaire.** Il faut choisir une règle de fusion. Les mots sont déjà dédoublonnés sans tenir compte de la casse, mais les corrections ne le sont pas : il faudrait le faire sur `heard`, comme Scribe.
6. **Secrets.** Un export ne doit jamais contenir de clé. Des clés par profil demanderaient de nouveaux noms d'identifiants. Linux n'a pas de coffre.
7. **Données sensibles.** L'historique et `bancs/` contiennent des textes dictés : il faut les exclure d'un export par défaut. Un dictionnaire contient des noms propres, et un export est un fichier en clair.
8. **Fichiers.**
   - Un outil de synchronisation qui remplace `settings.json` prend effet à la dictée suivante.
   - Les noms des fichiers temporaires sont fixes et il n'y a pas de verrou.
   - Un dossier de données par profil (`SCRIIBE_DATA`) dupliquerait `models/`, et deux instances se disputeraient les ports fixes de `whisper-server` et `llama-server`.
9. **Interface.**
   - La page envoie `customModes` et le dictionnaire en bloc : changer de profil pendant une modification pourrait écraser l'autre profil.
   - `merge_settings` refuse les clés inconnues.
   - Le menu de l'icône devra connaître le profil actif.
10. **Compatibilité avec Scribe.** Tout nouveau champ (profils, version) sera perdu par Scribe s'il réécrit le fichier. Il faut décider si on l'accepte.
11. **Mode par application.** Il n'est pas commencé. Il croiserait les profils (un mode par application dans chaque profil ?) et s'appuierait sur `ForegroundApp`.

## Cadrage

Le cadrage a été fait le 12/11 (tâche 1). Les questions 1, 2, 3, 5 et 6 portent sur l'usage, la sécurité ou un choix difficile à défaire : l'auteur les a tranchées. Les questions 4 et 7 sont des choix techniques, je les ai pris en donnant mes raisons.

**Vocabulaire.** Un profil est **un utilisateur**. L'interface dit « utilisateur » ; le code et le format disent `profile`, puisque les identifiants du code sont en anglais.

### Décisions

**1. Contenu d'un utilisateur** (décision de l'auteur : « la personne »)

Chaque utilisateur a les éléments suivants :

| Élément | Champs |
|---|---|
| Nom affiché | (nouveau) |
| Prénom et genre | `firstName`, `speakerGender` |
| Langues | `language`, `translationLanguage` |
| Dictionnaire | `words`, `corrections` |
| Modes personnalisés et mode choisi | `customModes`, `customModeID`, `rewriteMode`, `simpleFidelity`, `styleOverrides` |
| Commandes | `customCommands`, `voiceCommandsEnabled`, `voiceCommands`, `listBullet` |
| Historique et statistiques | `history.json`, `history-totals.json`, et leurs réglages `keepDictationText`, `historyKeepDays`, `typingWPM` |

L'historique et les statistiques ont été ajoutés à la demande de l'auteur, le 12/11 : « les statistiques suivent le profil ». Leurs trois réglages suivent l'utilisateur : sans cela, un utilisateur qui coupe la conservation du texte la couperait aussi pour l'autre (relecture, C1).

Restent communs à tous les utilisateurs, parce qu'ils appartiennent à l'ordinateur. Ce sont tous les autres champs d'`AppSettings` :

| Groupe | Champs |
|---|---|
| Touches | `hotkey`, `dictationKeys`, `translateKeys`, `formatShortcuts` |
| Micro et sons | `microphoneUID`, `microphoneName`, `startSoundName`, `endSoundName` |
| Moteurs et modèles | `transcriptionEngine`, `transcriptionModel`, `whisperModelPath`, `cleanupEngine`, `cleanupModel`, `lmStudioModel`, `ollamaModel`, `llamaCppModel`, `llamaDevice`, `lmStudioURL`, `ollamaURL`, `whisperIdleMinutes`, `llamaIdleMinutes` |
| Comportement | `cleanupEnabled`, `skipWhenClean`, `restoreClipboard`, `pauseMedia` |
| Apparence | `uiLanguage`, `theme`, `overlayEnabled`, `capture*` |
| Banc d'essai | `trialCandidates`, `trialText` ; le dossier `bancs/` reste commun aussi |
| Divers | `checkUpdates`, `setupDone` |
| Nouveaux champs | `schemaVersion`, `activeProfile` |

Les clés d'API restent aussi communes.

**Règle de code.** Un test unitaire échoue si un champ d'`AppSettings` n'est classé ni « utilisateur » ni « ordinateur ». Un futur réglage ne peut donc pas être oublié dans la répartition (relecture, C1).

Pourquoi les clés restent communes :

- elles sont rangées par compte de l'ordinateur ;
- des clés par utilisateur demanderaient de nouveaux noms dans le Trousseau et le Gestionnaire d'identifiants ;
- deux personnes qui partagent une session partagent déjà la facture.

**2. Un dictionnaire par utilisateur** (décision de l'auteur)

Chaque utilisateur a un seul dictionnaire. Un import ajoute ses entrées à ce dictionnaire. Il n'y a ni plusieurs dictionnaires par utilisateur, ni choix selon l'application.

L'idée « mode par application » reste à part dans la ROADMAP. Le format prévoit la place pour l'ajouter plus tard, sans migration : un champ facultatif de l'utilisateur.

**3. Passer d'un utilisateur à l'autre** (décision de l'auteur : « menu de l'icône et Réglages »)

- **Menu de l'icône.** Un sous-menu « Utilisateur » coche l'utilisateur actif. Il n'apparaît que lorsqu'il y a au moins deux utilisateurs, si bien qu'une personne seule ne voit aucun changement.
- **Réglages.** Une liste permet de :
  - créer un utilisateur, vide ou copié depuis l'utilisateur actif. Une copie ne reprend **ni l'historique ni les statistiques** ;
  - le renommer ;
  - le supprimer, après confirmation, avec la proposition d'exporter d'abord. Le dossier est déplacé dans `utilisateurs-supprimes/` et n'est jamais effacé par Scriibe.

Les dossiers sont nommés par un identifiant opaque (`utilisateurs/<uuid>/`), jamais par le nom affiché. Cela évite les caractères interdits sous Windows et les collisions de majuscules sous macOS.
- **Tableau de bord.** Il salue l'utilisateur actif et montre ses statistiques.
- **Pas de raccourci clavier.** On risquerait de changer d'utilisateur sans le voir.

Changer d'utilisateur change le début du prompt (mots, mode, genre) et relance le préchargement du modèle (constat § 6). C'est acceptable parce que c'est rare.

Ce que le changement d'utilisateur demande dans le code :

- Les requêtes de la page qui envoient un bloc entier (dictionnaire, `customModes`, `customCommands`) portent l'identifiant de l'utilisateur. Le serveur les refuse avec le code 409 si ce n'est plus l'utilisateur actif, et la page se recharge. Cela évite d'écraser les données d'un autre utilisateur (constat § 9, point 9).
- Le menu de l'icône envoie lui aussi l'identifiant de l'utilisateur quand il choisit un mode. Sinon, un changement d'utilisateur fait entre l'ouverture du menu et le clic appliquerait le mode à l'autre utilisateur (relecture, C3).
- Un verrou en mémoire protège la suite « lire, fusionner, écrire » des réglages et de `profile.json`. Les fichiers temporaires prennent un nom unique au lieu de `<nom>.tmp` (relecture, C3).
- `preload_signature` lit la vue fusionnée (étape 4a), sans autre changement. Pendant le délai de stabilisation (`PRELOAD_SETTLE`, 1,5 s), la première dictée qui suit un changement peut partir avant le rechargement. C'est acceptable.
- On change d'utilisateur par une route dédiée. `activeProfile` ne s'écrit pas par `POST api/settings`.

**4. Format commun d'import et d'export** (choix technique)

Un export est un seul fichier JSON lisible, d'extension `.scriibe.json`, avec une enveloppe commune :

```json
{
  "format": "scriibe-export",
  "version": 1,
  "kind": "dictionary",
  "app": "Scriibe 0.4.0",
  "exportedAt": "2026-11-12T18:00:00+01:00",
  "content": { "words": ["Claude Code"], "corrections": [{ "heard": "cri be", "written": "Scriibe" }] }
}
```

`kind` prend l'une de ces valeurs :

| `kind` | Contenu de `content` |
|---|---|
| `dictionary` | `{words, corrections}`, comme `dictionary.json` |
| `modes` | `{customModes: […]}` : chaque mode avec `fidelity`, `rewrites` (pour Scribe) et ses exemples |
| `commands` | `{customCommands: […]}` |
| `profile` | Les champs de l'utilisateur listés en décision 1, **sauf la ligne « Historique et statistiques »**, plus `dictionary`, `customModes` et `customCommands` |

Pourquoi ce format :

- Les clés sont celles des fichiers actuels, ce qui permet de réutiliser les lectures tolérantes qui existent déjà (`from_value`, `UserDictionary::from_json`, `CustomMode::from_value`).
- Un fichier reste lisible et modifiable à la main.
- Le chantier 5 (modes) reprendra `kind: "modes"` tel quel.

Règles de lecture :

- Un `format` inconnu ou une `version` plus grande que celle que Scriibe connaît : le fichier est refusé, avec un message clair.
- Les champs inconnus de `content` sont ignorés.
- Le fichier vient peut-être d'un tiers. Il a donc des limites : taille du fichier, nombre d'éléments, longueur des textes. Les raccourcis des commandes passent par `Shortcut::parse` ; un raccourci illisible est signalé dans l'aperçu.

Règles d'écriture (relecture, C6) :

- L'export est construit **champ par champ, d'après une liste blanche**, jamais en sérialisant `AppSettings` : un futur champ sensible ne peut pas partir par défaut.
- `exportedAt` ne garde que le jour.
- Le nom de fichier proposé ne contient pas le nom de l'utilisateur, par exemple `scriibe-dictionnaire-2026-11-12.scriibe.json`.

Règles d'import : un import **ajoute, il ne remplace jamais**, et il montre toujours un aperçu avant (combien d'éléments, et lesquels sont en conflit).

| Élément importé | Règle |
|---|---|
| Mots | Dédoublonnés sans tenir compte des majuscules, comme le fait déjà `clean_dictionary` |
| Corrections | Dédoublonnées sur `heard`, comme Scribe. Si une correction existe déjà avec un autre `written`, celle en place est gardée et le conflit est montré |
| Modes et commandes | Un `id` déjà présent donne un nouvel `id`, et les `id` des exemples suivent la même règle. Un nom déjà pris reçoit un numéro (« Mon mode 2 »), qui ne s'accumule pas d'un import à l'autre |
| Utilisateur entier | Crée toujours un nouvel utilisateur, n'écrase jamais l'existant. `customModeID` suit la table des nouveaux identifiants |

L'import passe par le serveur, puis la page se recharge. Sinon, la sauvegarde automatique de la page Dictionnaire (500 ms après la dernière frappe) pourrait écraser l'import.

N'entrent jamais dans un export :

- les clés d'API ;
- l'historique et les statistiques ;
- le banc d'essai ;
- les champs propres à l'ordinateur (décision 1).

**5. Protection des exports** (décision de l'auteur : « en clair, avec aperçu »)

- Le fichier n'est pas chiffré.
- Avant d'enregistrer, Scriibe montre ce qui part. L'aperçu donne le nombre par catégorie (« 120 mots, 8 corrections, 2 modes »), et un clic sur une catégorie en montre la liste. Il avertit que le fichier peut contenir des noms propres et des textes libres.
- Une case à cocher par catégorie permet de la retirer : mots, corrections, modes, exemples des modes, phrases de style, commandes et leurs phrases.
- Pour un utilisateur entier, la case « prénom et nom affiché » est **décochée par défaut** (relecture, C6).
- L'export se fait toujours à la demande, jamais tout seul. L'emplacement est choisi par l'utilisateur.

Pourquoi pas de chiffrement :

- il faudrait une bibliothèque de plus ;
- un mot de passe perdu ferait perdre le fichier ;
- le contenu le plus sensible, l'historique, n'est jamais exporté.

**6. Migration sans rien perdre** (décision de l'auteur : « déplacer, avec sauvegarde »)

La procédure a été corrigée d'après la relecture (B1, B2, C5).

**Quand elle tourne.** Au lancement de l'application, **après** la prise du verrou d'instance (`single_instance`), et avant :

- la création de `FileHistory` ;
- la première lecture des réglages (`config.settings()`).

Aujourd'hui, `setup()` (`crates/cli/src/commands.rs:18`) crée les réglages avant de prendre le verrou ; il faudra réordonner. Les sous-commandes de diagnostic, qui ne prennent pas le verrou, ne migrent jamais : elles s'arrêtent sur un message si la migration n'est pas faite. La migration travaille dans `data_folder()`, ce qui couvre aussi `SCRIIBE_DATA`.

**Ce qui dit « migré ».** `settings.json` porte `schemaVersion >= 2`. L'existence du dossier `utilisateurs/` ne compte pas.

**Étapes.** Aucune n'est un déplacement avant la dernière :

1. Scriibe copie `settings.json`, `dictionary.json`, `history.json` et `history-totals.json` dans `sauvegarde-avant-utilisateurs-<date>/`. Ce dossier n'est jamais effacé automatiquement. Si la place manque (disque plein), Scriibe s'arrête ici : rien n'a changé.
2. Il construit l'utilisateur dans un dossier de travail, `utilisateurs/.migration-<uuid>/`, à partir de **copies** :
   - `profile.json` avec les champs de la décision 1 ;
   - `dictionary.json`, `history.json` et `history-totals.json`.

   Le nom affiché vaut `firstName`, ou « Principal » si `firstName` est vide. L'identifiant de connexion de l'ordinateur ne devient jamais un nom affiché.
3. Il renomme ce dossier en `utilisateurs/<uuid>/`. Un renommage dans le même dossier est atomique.
4. Il réécrit `settings.json` (écriture atomique) sans les champs de l'utilisateur, avec `schemaVersion: 2` et `activeProfile: "<uuid>"`.
5. Seulement après, il supprime de la racine `dictionary.json`, `history.json` et `history-totals.json`. Leur copie reste dans la sauvegarde.

**Reprises.** Un arrêt brutal peut survenir à n'importe quelle étape :

| Arrêt | Ce que trouve le lancement suivant | Ce que fait Scriibe |
|---|---|---|
| Avant l'étape 3 | Un dossier `.migration-*` | L'efface et recommence |
| Entre les étapes 3 et 4 | Un `utilisateurs/<uuid>/` complet, un `settings.json` sans `schemaVersion` | Le reprend comme utilisateur actif au lieu d'en créer un deuxième |
| Après l'étape 4 | `schemaVersion: 2` | Rien : la migration est faite. Les fichiers de l'étape 5 qui restent à la racine sont simplement effacés |

**Retour à une version antérieure, puis retour ici.**

- Une version antérieure lit le `settings.json` v2 sans erreur. Elle y voit ses champs utilisateur vides et, si elle enregistre, elle perd `schemaVersion` et `activeProfile`.
- Au retour sur la nouvelle version, on trouve un `utilisateurs/` existant et un `settings.json` sans `schemaVersion`. C'est le même cas que la deuxième ligne du tableau : Scriibe reprend l'utilisateur existant (le dernier utilisé, sinon le seul) et ne recrée pas « Principal ».
- Les champs de l'ordinateur modifiés entre-temps par l'ancienne version sont gardés.

**Dossier de Scribe copié par-dessus après la migration.** Un `settings.json` sans `schemaVersion` à côté d'un `utilisateurs/` existant est traité comme ci-dessus. Les champs utilisateur venus de Scribe ne sont pas fusionnés, pour ne rien écraser. Cette limite est notée dans l'Aide.

Revenir à une version antérieure de Scriibe :

- Le dictionnaire, les modes et l'historique y paraîtront vides tant qu'on n'aura pas remis la sauvegarde à la main.
- L'Aide (FR et EN) et le CHANGELOG expliquent comment faire.
- L'auteur a accepté cette conséquence, le 12/11.

Pourquoi déplacer plutôt que garder une copie à l'ancienne place : les données ne sont qu'à un seul endroit, il n'y a donc pas deux copies à tenir d'accord.

**7. Ordre de livraison** (choix technique)

Chaque étape peut être livrée seule. Les étapes 1 et 2 servent même si les utilisateurs ne viennent jamais.

| Étape | Contenu | Utile seule parce que |
|---|---|---|
| 1. Fondations, sans changement visible | Voir le détail sous ce tableau | Protège les données dès maintenant (constat § 9, points 1, 5 et 8) ; la migration en dépend |
| 2. Import et export du dictionnaire | Enveloppe v1 (`kind: "dictionary"`), aperçu et cases à cocher, fusion par le serveur, limites. Page Dictionnaire, Aide FR et EN | Partager ou sauvegarder son vocabulaire, même sans utilisateurs. **Dépend de l'étape 1** (dédoublonnage des corrections) |
| 3. Import et export des modes et des commandes | `kind: "modes"` (chantier 5) et `kind: "commands"` | Même format que l'étape 2. **Prérequis** : le niveau de fidélité de la ROADMAP n°18 pour les modes ; les commandes peuvent partir avant |
| 4a. Séparer la vue des réglages, sans changement visible | Voir le détail sous ce tableau | Prépare l'étape 4b ; sans effet visible tant qu'il n'y a qu'un seul « utilisateur » |
| 4b. Migration | Décision 6, avec un test sur un dossier temporaire pour chaque ligne du tableau des reprises | Les données sont rangées par utilisateur, toujours sans rien à choisir |
| 4c. Utilisateurs visibles | Liste dans les Réglages (créer, copier, renommer, supprimer), sous-menu de l'icône, route de changement, garde-fou 409, verrou en mémoire, tableau de bord et statistiques par utilisateur, Aide FR et EN | La fonction elle-même |
| 5. Export et import d'un utilisateur entier | `kind: "profile"`, case « prénom et nom affiché » décochée | Passer d'un ordinateur à l'autre, ou donner sa configuration à quelqu'un. **Dépend de 4c** |

Contenu de l'étape 1 :

- `schemaVersion` devient un champ d'`AppSettings`, lu et écrit.
- Un drapeau « lecture refusée » dans `FileConfig` couvre un fichier illisible ou d'une version plus récente que celle que Scriibe connaît. Toute sauvegarde de ce fichier échoue alors, et la page affiche un avertissement. Aujourd'hui, la lecture retombe sur les réglages par défaut, que la sauvegarde suivante écrit par-dessus (relecture, B4).
- À la première lecture d'un fichier illisible, Scriibe en fait une copie `<nom>.<date>.bak`, comme Scribe. Elle a lieu une seule fois, à la lecture et non à la sauvegarde.
- Les fichiers temporaires prennent un nom unique.
- Les corrections sont dédoublonnées sur `heard`.

Contenu de l'étape 4a (relecture, B3) :

- `FileConfig` donne toujours une vue `AppSettings` unique, fusion de `settings.json` et `profile.json`. Le reste du code n'a ainsi pas à changer : `active_profile`, `preload_signature`, le menu de l'icône, `api/status`.
- `merge_settings` répartit chaque clé dans le bon fichier, selon le classement de la décision 1.
- Un `Watched` par fichier, avec un chemin qui suit l'utilisateur actif, couvre aussi le dictionnaire, lu à chaque dictée.
- `FileHistory` peut changer de dossier sous son verrou (`switch(folder)`), et `ctx.history` suit.
- Tant que la migration n'existe pas, le « profil » est simplement la racine du dossier de données.

Plus tard, hors de ce cadrage :

- un raccourci pour changer d'utilisateur ;
- un mode ou un dictionnaire par application ;
- plusieurs dictionnaires par utilisateur ;
- des clés par utilisateur ;
- un coffre à secrets sous Linux.

### Conséquences pour les autres chantiers

- **Chantier 3 (prénom).** `firstName` passe dans l'utilisateur à l'étape 4b. D'ici là, le réglage ne change pas.
- **Chantier 5 (modes).** Il adopte l'enveloppe de la décision 4 et les règles d'import des modes. L'export d'un mode doit porter `fidelity`.
- **Compatibilité avec Scribe.** Un dossier de Scribe copié dans une Scriibe **pas encore migrée** est migré comme l'ancien rangement. Copié **après** la migration (étape 4b), ses champs d'utilisateur ne sont pas fusionnés (décision 6). Scribe ne lit pas les fichiers de Scriibe, c'est accepté.
- **Prompt.** L'empreinte `4ed86f0c1c154918` ne change pas, puisqu'aucun texte de prompt n'est modifié.

## Relecture

La relecture a été faite le 12/11 (tâche 2) par un sous-agent relecteur qui n'avait pas écrit la note. Il a vérifié les décisions contre le code. J'ai contrôlé moi-même deux de ses affirmations : l'ordre `setup()` puis `single_instance` dans `commands.rs:18-75`, et les champs non classés dans `settings.rs`. Les deux sont exactes.

**Verdict du relecteur.** « Solide sur le fond, pas encore prêt pour l'implémentation. » Il a relevé 4 défauts bloquants, 7 à corriger et 7 mineurs. Tous sont corrigés dans le § Cadrage ci-dessus. **Aucune décision de l'auteur ne change** : les corrections précisent comment les appliquer.

| Défaut | Correction (où) |
|---|---|
| B1. La migration tournerait avant le verrou d'instance (`setup()` crée les réglages avant `single_instance`), et aussi dans les sous-commandes | Migration après le verrou ; les sous-commandes ne migrent jamais (décision 6) |
| B2. Migration ni atomique ni rejouable : un arrêt après le déplacement des fichiers laissait l'utilisateur vide, et le critère « `utilisateurs/` existe » sautait la reprise | Dossier de travail, copies puis renommage ; critère `schemaVersion >= 2` ; tableau des reprises (décision 6) |
| B3. Étape 4 sous-estimée : `AppSettings` est lu partout, `Watched` et `FileHistory` ont des chemins figés | Nouvelle étape 4a « séparer la vue », étape 4 découpée en 4a, 4b et 4c (décision 7) |
| B4. Le refus d'écraser un fichier plus récent n'était pas réalisable : `from_value` perd `schemaVersion`, la sauvegarde écrit par-dessus les réglages par défaut | `schemaVersion` dans `AppSettings`, drapeau « lecture refusée », `.bak` fait à la lecture (étape 1) |
| C1. Des champs n'étaient pas classés (`keepDictationText`, `historyKeepDays`, `typingWPM`, moteurs, banc, `skipWhenClean`, `setupDone`…) | Tableau complet ; réglages de l'historique dans l'utilisateur ; test unitaire de classement (décision 1) |
| C2. Contradictions : export `profile` et historique, nom affiché pris dans le compte de l'ordinateur, copie d'un utilisateur et historique | Historique exclu explicitement ; « Principal » si `firstName` est vide ; une copie ne reprend pas l'historique |
| C3. Le menu de l'icône écrit sans identifiant d'utilisateur ; nom fixe des fichiers temporaires | Identifiant envoyé par le menu, verrou en mémoire, nom temporaire unique (décisions 3 et 7) |
| C4. Délai de préchargement après un changement d'utilisateur | Noté et accepté (décision 3) |
| C5. Retour à une version antérieure puis retour ici ; dossier de Scribe copié par-dessus | Reprise de l'utilisateur existant, sans recréer « Principal » ; pas de fusion avec Scribe, noté dans l'Aide (décision 6) |
| C6. Fuites possibles : prénom, textes libres (styles, exemples, phrases), heure exacte, nom de fichier ; export par sérialisation | Liste blanche, date au jour, nom de fichier neutre, aperçu détaillé, prénom décoché par défaut (décisions 4 et 5) |
| C7. Import : renvoi de `customModeID`, `id` des exemples, limites, raccourcis, sauvegarde automatique de la page, suffixe cumulé | Table des identifiants, limites, `Shortcut::parse`, import par le serveur, numérotation (décision 4) |
| Mineurs (M1 à M7) | Dépendances des étapes 2, 3 et 5 écrites ; dossier `utilisateurs/<uuid>/` opaque ; disque plein ; `utilisateurs-supprimes/` sans accent ; route dédiée au changement d'utilisateur |

**Un seul choix ajouté de mon côté.** `skipWhenClean` reste à l'ordinateur, ce que le relecteur recommandait aussi.
