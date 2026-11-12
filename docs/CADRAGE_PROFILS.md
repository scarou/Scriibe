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
| Historique et statistiques | `history.json`, `history-totals.json` |

L'historique et les statistiques ont été ajoutés à la demande de l'auteur, le 12/11 : « les statistiques suivent le profil ».

Restent communs à tous les utilisateurs (ils appartiennent à l'ordinateur) :

- les touches de dictée et de traduction ;
- le micro et les sons ;
- les moteurs, les modèles et leurs chemins, `llamaDevice`, les URL ;
- les clés d'API ;
- l'apparence et la langue de l'interface, `formatShortcuts` ;
- le banc d'essai, qui mesure les modèles de cet ordinateur.

`cleanupEnabled` et les réglages de la pastille restent aussi communs.

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
  - créer un utilisateur, vide ou copié depuis l'utilisateur actif ;
  - le renommer ;
  - le supprimer, après confirmation, avec la proposition d'exporter d'abord. Le dossier est déplacé dans `utilisateurs-supprimés/` et n'est jamais effacé par Scriibe.
- **Tableau de bord.** Il salue l'utilisateur actif et montre ses statistiques.
- **Pas de raccourci clavier.** On risquerait de changer d'utilisateur sans le voir.

Changer d'utilisateur change le début du prompt (mots, mode, genre) et relance le préchargement du modèle (constat § 6). C'est acceptable parce que c'est rare.

Ce que le changement d'utilisateur demande dans le code :

- Les requêtes de la page qui envoient un bloc entier (dictionnaire, `customModes`, `customCommands`) portent l'identifiant de l'utilisateur. Le serveur les refuse avec le code 409 si ce n'est plus l'utilisateur actif, et la page se recharge. Cela évite d'écraser les données d'un autre utilisateur (constat § 9, point 9).
- Le menu de l'icône et `preload_signature` lisent l'utilisateur actif.

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
| `profile` | Les champs de l'utilisateur listés en décision 1, plus `dictionary`, `customModes` et `customCommands` ; **jamais l'historique ni les statistiques** |

Pourquoi ce format :

- Les clés sont celles des fichiers actuels, ce qui permet de réutiliser les lectures tolérantes qui existent déjà (`from_value`, `UserDictionary::from_json`, `CustomMode::from_value`).
- Un fichier reste lisible et modifiable à la main.
- Le chantier 5 (modes) reprendra `kind: "modes"` tel quel.

Règles de lecture :

- Un `format` inconnu ou une `version` plus grande que celle que Scriibe connaît : le fichier est refusé, avec un message clair.
- Les champs inconnus de `content` sont ignorés.

Règles d'import : un import **ajoute, il ne remplace jamais**, et il montre toujours un aperçu avant (combien d'éléments, et lesquels sont en conflit).

| Élément importé | Règle |
|---|---|
| Mots | Dédoublonnés sans tenir compte des majuscules, comme le fait déjà `clean_dictionary` |
| Corrections | Dédoublonnées sur `heard`, comme Scribe. Si une correction existe déjà avec un autre `written`, celle en place est gardée et le conflit est montré |
| Modes et commandes | Un `id` déjà présent ou un nom déjà pris donne un nouvel `id` et le suffixe « (importé) » |
| Utilisateur entier | Crée toujours un nouvel utilisateur, n'écrase jamais l'existant |

N'entrent jamais dans un export :

- les clés d'API ;
- l'historique et les statistiques ;
- le banc d'essai ;
- les champs propres à l'ordinateur (décision 1).

**5. Protection des exports** (décision de l'auteur : « en clair, avec aperçu »)

- Le fichier n'est pas chiffré.
- Avant d'enregistrer, Scriibe montre ce qui part (« 120 mots, 8 corrections, 2 modes ») et avertit que le fichier contient des noms propres et des phrases d'exemple.
- Des cases à cocher permettent de retirer une partie, par exemple les corrections ou les exemples des modes.
- L'export se fait toujours à la demande, jamais tout seul. L'emplacement est choisi par l'utilisateur.

Pourquoi pas de chiffrement :

- il faudrait une bibliothèque de plus ;
- un mot de passe perdu ferait perdre le fichier ;
- le contenu le plus sensible, l'historique, n'est jamais exporté.

**6. Migration sans rien perdre** (décision de l'auteur : « déplacer, avec sauvegarde »)

Au premier lancement de la version qui apporte les utilisateurs :

1. Scriibe copie `settings.json`, `dictionary.json`, `history.json` et `history-totals.json` dans `sauvegarde-avant-utilisateurs-<date>/`. Ce dossier n'est jamais effacé automatiquement.
2. Il crée l'utilisateur « Principal » dans `utilisateurs/<id>/` :
   - `profile.json` reçoit les champs de la décision 1, et le nom affiché vaut le prénom de la salutation ;
   - `dictionary.json`, `history.json` et `history-totals.json` y sont déplacés.
3. Il réécrit `settings.json` sans les champs de l'utilisateur, avec `schemaVersion: 2` et `activeProfile: "<id>"`.
4. L'opération est sans effet si `utilisateurs/` existe déjà. Si une étape échoue, Scriibe s'arrête avant de réécrire `settings.json` et garde l'ancien rangement : il réessaiera au lancement suivant.

Revenir à une version antérieure de Scriibe :

- Le dictionnaire, les modes et l'historique y paraîtront vides tant qu'on n'aura pas remis la sauvegarde à la main.
- L'Aide (FR et EN) et le CHANGELOG expliquent comment faire.
- L'auteur a accepté cette conséquence, le 12/11.

Pourquoi déplacer plutôt que garder une copie à l'ancienne place : les données ne sont qu'à un seul endroit, il n'y a donc pas deux copies à tenir d'accord.

**7. Ordre de livraison** (choix technique)

Chaque étape peut être livrée seule. Les étapes 1 et 2 servent même si les utilisateurs ne viennent jamais.

| Étape | Contenu | Utile seule parce que |
|---|---|---|
| 1. Fondations, sans changement visible | `schemaVersion` dans `settings.json`. Copie `<nom>.<date>.bak` d'un fichier illisible avant de l'écraser, comme Scribe. Refus d'écraser un fichier d'une version plus récente, la page affiche alors un avertissement. Corrections dédoublonnées sur `heard` | Protège les données dès maintenant (constat § 9, points 1 et 5) et prépare la migration |
| 2. Import et export du dictionnaire | Enveloppe v1 (`kind: "dictionary"`), aperçu et cases à cocher, fusion. Page Dictionnaire, Aide FR et EN | Partager ou sauvegarder son vocabulaire, même sans utilisateurs |
| 3. Import et export des modes et des commandes | `kind: "modes"` (chantier 5, après le niveau de fidélité de la ROADMAP n°18) et `kind: "commands"` | Même format que l'étape 2 : peu de code nouveau |
| 4. Utilisateurs | Migration (décision 6), dossier `utilisateurs/`, liste dans les Réglages, sous-menu de l'icône, tableau de bord, historique et statistiques par utilisateur, garde-fou 409 | La fonction elle-même ; elle s'appuie sur les fondations de l'étape 1 |
| 5. Export et import d'un utilisateur entier | `kind: "profile"` | Passer d'un ordinateur à l'autre, ou donner sa configuration à quelqu'un |

Plus tard, hors de ce cadrage :

- un raccourci pour changer d'utilisateur ;
- un mode ou un dictionnaire par application ;
- plusieurs dictionnaires par utilisateur ;
- des clés par utilisateur ;
- un coffre à secrets sous Linux.

### Conséquences pour les autres chantiers

- **Chantier 3 (prénom).** `firstName` passe dans l'utilisateur à l'étape 4. D'ici là, le réglage ne change pas.
- **Chantier 5 (modes).** Il adopte l'enveloppe de la décision 4 et les règles d'import des modes. L'export d'un mode doit porter `fidelity`.
- **Compatibilité avec Scribe.** Après l'étape 4, on ne peut plus copier le dossier de Scribe tel quel dans Scriibe. La migration le prend pourtant en charge : un `settings.json` de Scribe sans `schemaVersion` est traité comme l'ancien rangement. Scribe ne lit pas les fichiers de Scriibe, c'est accepté.
- **Prompt.** L'empreinte `4ed86f0c1c154918` ne change pas, puisqu'aucun texte de prompt n'est modifié.

## Relecture

(Tâche 2, à venir.)
