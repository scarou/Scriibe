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

(Tâche 1, à venir.)

## Relecture

(Tâche 2, à venir.)
