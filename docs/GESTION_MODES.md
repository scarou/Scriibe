# Gestion des modes (chantier 5)

Cette note prépare deux idées du carnet, du 11/11 :

- « Menu de la barre en haut à droite : choisir les modes affichés » ;
- « Modes : importer et exporter un mode avec ses exemples ».

Le découpage est dans `~/Library/Mobile Documents/com~apple~CloudDocs/IA-OS/1 - Projets/scriibe/decoupages/decoupage-2026-11-12-c5-gestion-modes.md`. Décision du 12/11 : ce chantier ne publie pas de version.

Sections :

- Constat (tâche 0)

Méthode du constat : lecture du code par un sous-agent de recherche, puis sondage de quelques points par le rédacteur (existence des fichiers, `OPTIONAL_KEYS`, `pick_mode`, `menu_view_in`, `pick_file`). Rien n'a été lancé ni mesuré. Les numéros de ligne datent du 12/11/2026 (commit `e2630ea`) et glissent avec le code : se fier d'abord aux noms.

## Constat

Chemins relatifs à la racine du dépôt. Scribe : `~/Documents/Mes projets IA/Scribe/Sources/Scribe/`.

### 1. Rangement des modes

**Modes intégrés : en code, sans fichier.**

- `crates/core/src/modes.rs` : `enum RewriteMode` à 9 valeurs (Literal, Simple, Formal, Wellness, Tech, Letter, Sales, Playful, Calm). L'ordre de `ALL` compte pour l'empreinte des prompts.
- Valeur écrite dans le JSON : `RewriteMode::raw()` (`literal`, `simple`, `formal`, `wellness`, `tech`, `letter`, `sales`, `playful`, `calm`). Libellés : `label()`, `label_en()` (menu de l'icône), `summary()`. Consigne de style : `style()` (Literal, Simple et Playful n'en ont pas).
- Groupes du menu : `RewriteMode::MENU_GROUPS` = `[Literal, Simple, Formal, Calm]`, `[Wellness, Tech, Letter, Sales]`, `[Playful]`.
- **Amusant (`playful`) est abandonné** : la page (`crates/cli/src/web/data.rs`) et le menu (`crates/cli/src/tray_source.rs`) l'écartent et le montrent comme Simple. Il ne doit pas devenir une case de visibilité.
- Fidélité et exemples des modes livrés : `RewriteMode::profile()` (Calm = Free ; les modes qui reformulent, `rewrites_freely()` = Reformulated ; les autres = Light). Les exemples sont Faithful, Style, None (Courrier) ou `Own(3 exemples)` (Apaisé).
- Réglages propres aux modes livrés, dans `settings.json` : `styleOverrides` (phrase de style modifiée par mode), `simpleFidelity` (cran de Simple), `rewriteMode`, `customModeID`, `cleanupEnabled`.

**Modes personnels : `customModes` dans `settings.json`.**

- `AppSettings.custom_modes: Vec<CustomMode>` (`crates/core/src/settings.rs`), clé JSON `customModes`. `customModeID` (mode personnel choisi) est omis quand il est vide.
- `CustomMode` (`modes.rs`) :

| Champ | Type | Rôle |
|---|---|---|
| `id` | texte | UUID en majuscules |
| `name` | texte | nom affiché |
| `style` | texte | la consigne |
| `fidelity` | `Fidelity` | `literal`, `light`, `reformulated` ou `free` (valeur par défaut si absent) |
| `vouvoie` | booléen | vouvoiement |
| `examples` | liste de `ModeExample` | `{id, dictation, expected}` |

- **Les exemples sont dans le mode lui-même** (tableau `examples`), sans fichier à part. Un exemple dont un côté est vide n'est pas envoyé au modèle (`ModeExample::is_complete()`, filtre dans `CustomMode::profile()`). Sans exemple complet : Faithful si le cran ne reformule pas, None si `vouvoie`, sinon Style.
- Écriture : `impl Serialize for CustomMode` écrit aussi `rewrites` (booléen déduit de `fidelity`, pour Scribe et les anciens fichiers), dans l'ordre id, name, style, fidelity, rewrites, vouvoie, examples.
- Lecture tolérante : `CustomMode::from_value` (un `id` illisible reçoit un nouvel UUID ; sans `fidelity`, elle se déduit de l'ancien `rewrites` : vrai = Reformulated, faux = Light), `ModeExample::from_value`, `AppSettings::from_value`. Un `customModeID` orphelin est remis à vide ; un mode personnel choisi force `rewrite_mode = Simple`.
- **Cran de fidélité** : `enum Fidelity` (`modes.rs`). Literal garde au moins 90 % des mots de la dictée, Light au moins 75 % (dictées de 8 mots ou plus), Reformulated a un contrôle souple, Free n'a plus de contrôle de la part de mots gardés. Le modèle ne reçoit aucun chiffre : le cran règle le garde-fou après coup (`cleaner::accepts` dans `crates/core/src/cleaner.rs`, appelé par `crates/engines/src/services.rs`) et le choix de la consigne et des exemples.
- Choix du mode actif (`settings.rs`) : `active_custom_mode`, `active_profile`, `active_mode_label`, `mode_choice` (valeur d'un mode livré, ou id d'un mode personnel), `set_mode_choice`, **`pick_mode`** (change le mode et rallume `cleanup_enabled`).

**Il n'existe aucun champ « masqué » ou « affiché dans le menu »** pour les modes (ni `hidden`, ni `masqu*`, ni `visible` lié aux modes, dans `crates/`, `ui/src` et les docs). Seul `ModeProfile.available` côté page, toujours vrai.

**Fichier de réglages.**

- `settings.json` dans `data_folder()` (`crates/platform/src/storage.rs`) : `%APPDATA%\Scriibe` (Windows), `~/Library/Application Support/Scriibe` (macOS), `$XDG_DATA_HOME/scriibe` ou `~/.local/share/scriibe` (Linux) ; `SCRIIBE_DATA` remplace tout.
- JSON indenté, écriture atomique par `write_atomic` (fichier temporaire `settings.tmp` à nom fixe, puis renommage ; pas de verrou). Un cache est relu quand l'heure de modification change : une édition à la main s'applique à la dictée suivante.
- `AppSettings` dérive seulement `Serialize` ; la lecture passe par `from_value`, écrit à la main et tolérant.

**Serveur local (`crates/cli/src/web/mod.rs`).**

- Écoute sur `127.0.0.1` avec un jeton secret dans l'adresse. **Le corps de chaque requête est lu en entier, sans limite de taille** : à retenir pour un import.
- Routes utiles : `GET api/modes` (`data::modes` : modes livrés, groupes, choix, libellé actif ; **les modes personnels n'y sont pas**, la page les lit dans les réglages) ; `POST api/modes/choose` (→ `pick_mode`) ; `GET` et `POST api/settings` (`patch_settings` → `merge_settings` → `save_settings`) ; `POST api/pick-file` ; `POST api/trial/*` (essais d'un mode).
- **`merge_settings` refuse toute clé absente de l'objet sérialisé** (« réglage inconnu »), sauf `OPTIONAL_KEYS = ["historyKeepDays", "customModeID"]`. Un nouveau champ toujours sérialisé passe seul ; un champ omis quand il est vide doit entrer dans `OPTIONAL_KEYS`.
- **Aucune route d'import ou d'export de mode n'existe.**

**Interface (`ui/src`).**

- `tabs/Modes.svelte` : cartes des modes livrés, carte « Mes modes », carte d'édition (`editing`, copie du mode). Exemples : deux `textarea` par exemple, bouton « × » et « Ajouter un exemple » (id d'exemple = `crypto.randomUUID().toUpperCase()`). Menu du cran : `fidelities` et `<select bind:value={editing.fidelity}>`.
- Sauvegarde : `saveCustom` envoie **tout le tableau** `customModes` par `settings.change`. Suppression : `removeCustom` (avec confirmation). Création : `newMode` (l'id est créé côté page). Choix : `choose` → `api('modes/choose')` puis `settings.load()`.
- La page relit `api/modes` toutes les 1,5 s (`ui/src/lib/modesSync.ts`).
- `ui/src/lib/api.ts` : `api()` (POST JSON) et les types `ModeExample`, `CustomMode`, `BuiltInMode`, `Modes`. `ui/src/lib/settings.svelte.ts` : `load()` et `change(patch)`.

### 2. Menu de l'icône (après le chantier 2)

**Qui construit la liste.**

- `crates/cli/src/tray_source.rs` : `menu_view_in(settings, version, lang) -> MenuView` (fonction pure, celle des tests) ; `menu_view` lui ajoute la langue résolue et l'entrée de mise à jour.
- Le mode coché vient de `settings.rewriting_on().then(mode_choice)` (Amusant ramené à Simple). Groupes livrés : `MENU_GROUPS`, Amusant écarté, groupes vides retirés. Modes personnels : **un seul groupe ajouté après les groupes livrés**, une entrée par `custom_modes`, résumé « Mode personnel » / « Custom mode ».
- Types : `crates/platform/src/tray_menu.rs` (`ModeEntry {id, label, summary, checked}`, `MenuView`, trait `TrayMenuSource`). Clic : `TraySource::choose_mode` → `pick_mode` + `save_settings` (ignoré sans moteur de mise au propre ou avec un id inconnu).

**Reconstruction quand les modes changent.**

- Pas d'événement « menu ouvert » (`docs/ECARTS.md`). Un fil relit `source.view()` **deux fois par seconde** : `crates/cli/src/mac_tray.rs` (macOS et Linux) et `crates/platform/src/windows/tray.rs` (Windows).
- Chaque relecture compare la vue à la précédente (`needs_refresh`). `same_modes` compare la suite `(id, label)` : identique, seules les cases sont mises à jour ; différente, le sous-menu est reconstruit en entier (entrées `tray:mode:<id>`, séparateur entre groupes).
- Depuis le chantier 2 (commits `4ca7ca8`, `01bb995`, `2d8ae62`, `docs/BUGS_AIDE_MODES.md`) : un seul chemin de choix (`pick_mode`), cases remises d'accord après chaque clic, la page suit le menu sur Mac. Tests dans `crates/cli/src/mode_sync.rs` et `mac_tray::tests`.
- Essai réel (`docs/BUGS_AIDE_MODES.md`) : page → menu et menu → page confirmés **sur Mac seulement**. Restent non tranchés : une coche se met-elle à jour dans un sous-menu **resté ouvert** ? Windows et Linux ne sont pas essayés.

**Où filtrer.**

- Le point naturel est **`menu_view_in`** : un filtre sur chaque mode livré et sur `settings.custom_modes`, à partir d'un champ des réglages à créer. Les deux fils de rafraîchissement n'ont **rien à changer** : masquer retire l'id, donc `same_modes` voit un changement et reconstruit.
- Cas à traiter : groupe tout masqué. Les groupes livrés vides sont déjà retirés ; le groupe des modes personnels ne teste que `custom_modes.is_empty()`, à compléter.
- Tests de forme à adapter : `the_menu_lists_the_eight_modes_in_groups_and_checks_the_current_one` (`tray_source.rs`), `mode_sync.rs` (`checked_in_menu` passe par `menu_view_in`), `data::modes` si la page montre l'état.
- **Le mode actif doit-il rester visible ? Rien ne le fixe dans le code ni les docs.** Sans traitement : aucune entrée cochée, le titre « Mode : <mode actif> » reste, et le mode masqué reste choisissable par la page (`pick_mode` ne regarde que l'existence).

### 3. Format d'import et d'export fixé par `docs/CADRAGE_PROFILS.md`

Décisions 4 et 5 de `docs/CADRAGE_PROFILS.md` (section Cadrage), reprises à la relecture ; le chantier 5 « adopte l'enveloppe de la décision 4 et les règles d'import des modes » et l'export d'un mode « doit porter `fidelity` ».

- **Enveloppe** : un seul fichier JSON lisible, extension `.scriibe.json` : `{"format": "scriibe-export", "version": 1, "kind": "modes", "app": "Scriibe x.y.z", "exportedAt": "<jour>", "content": {"customModes": […]}}`. Chaque mode porte `fidelity`, `rewrites` (pour Scribe) et ses exemples. Clés identiques à celles des fichiers actuels, pour réutiliser les lectures tolérantes.
- **Lecture** : `format` inconnu ou `version` plus grande que celle connue → fichier refusé avec un message clair ; champs inconnus de `content` ignorés ; le fichier peut venir d'un tiers, d'où des limites (taille du fichier, nombre d'éléments, longueur des textes). **Aucune valeur chiffrée n'est fixée.**
- **Écriture** : construction champ par champ **d'après une liste blanche**, jamais en sérialisant `AppSettings` ; `exportedAt` au jour ; nom de fichier neutre (sans nom d'utilisateur).
- **Import** : **ajoute, ne remplace jamais** ; aperçu avant (nombre d'éléments, conflits) ; un `id` déjà présent donne un nouvel `id` et **les `id` des exemples suivent la même règle** ; un nom déjà pris reçoit un numéro (« Mon mode 2 ») qui ne s'accumule pas d'un import à l'autre ; **l'import passe par le serveur**, puis la page se recharge (sinon la sauvegarde automatique de la page pourrait l'écraser).
- **Protection (décision 5)** : fichier en clair ; aperçu avant d'enregistrer avec le nombre par catégorie et un avertissement (noms propres, textes libres) ; une case par catégorie, dont « modes », « exemples des modes » et « phrases de style » ; export toujours à la demande, emplacement choisi par l'utilisateur.
- **N'entrent jamais dans un export** : clés d'API, historique et statistiques, banc d'essai, champs propres à l'ordinateur.
- **État de l'implémentation** : l'étape 1 du cadrage (`schemaVersion`, fichiers temporaires à nom unique, drapeau « lecture refusée ») et l'étape 2 (enveloppe et import du dictionnaire) **ne sont pas faites**. Aucune trace de `schemaVersion` dans `storage.rs` ni `settings.rs`, et `storage.rs` écrit toujours `settings.tmp`. Le prérequis fidélité (ROADMAP n°18) est fait. Si les modes passent avant le dictionnaire, l'enveloppe doit être **la même** (`format`, `version: 1`) pour ne pas être refaite.

### 4. Choix d'un fichier selon le système

**Ce qui existe : un seul mécanisme, l'ouverture d'un fichier.**

- Route `POST api/pick-file` → `pick_file(kind)` (`crates/cli/src/web/mod.rs`). Un seul `kind` : `"whisper"`. Réponse `{ok, path}` ou `{ok: false, error}`. Appelant unique : `ui/src/tabs/Settings.svelte` (`chooseWhisperFile`).
- **Windows** : `crates/platform/src/windows/file_dialog.rs`, `pick_file(title, filter)` par `GetOpenFileNameW`.
- **macOS** : `crates/platform/src/macos/file_dialog.rs`, `pick_file(title, filter)` par `osascript` (`choose file … of type {…}`) ; ne garde que les extensions de la forme `*.xxx` ; annulation et échec renvoient tous deux `None`.
- **Linux : rien** (message « le choix d'un fichier n'est pas encore pris en charge sur ce système »). Aucune trace de `zenity`, `kdialog`, `rfd` ni de portail pour les fichiers (`ashpd` ne sert qu'aux raccourcis, `docs/PIEGES.md`). Le bouton « Choisir un fichier… » est pourtant affiché sur tous les systèmes.
- Ni `<input type="file">`, ni `showOpenFilePicker`, ni plugin Tauri `dialog`, ni `rfd`.

**Enregistrer sous et télécharger : rien n'existe.** Pas de `choose file name`, ni de `GetSaveFileNameW`, ni de `Blob`, `createObjectURL` ou `download=` dans `ui/src` ; le serveur ne répond qu'en JSON (et un `audio/wav`), sans `Content-Disposition`. La page « sort » du texte par le presse-papiers (`Journal.svelte`, `History.svelte`). La fenêtre est une WebView Tauri sur l'adresse locale, sans gestionnaire de téléchargement. Ouvrir un dossier : `platform::open::open`.

**Solution la plus réutilisable.**

- (a) Choisir un fichier à importer : étendre `pick_file` avec un `kind` d'import (filtre `.scriibe.json`, donc `*.json` côté macOS) ; le serveur lit le fichier par son chemin, ce qui évite la taille illimitée du corps de requête et correspond à « l'import passe par le serveur ». Linux reste à résoudre.
- (b) Choisir où enregistrer : **rien de réutilisable tel quel**. À écrire : `pick_save` (macOS `choose file name`, Windows `GetSaveFileNameW`, même structure `OPENFILENAMEW`). Repli garanti partout, sans dialogue : écrire dans un dossier connu puis l'ouvrir avec `open::open`, ou copier le JSON dans le presse-papiers.
- Sur Mac, `osascript` dépend du processus responsable (Scriibe.app lancée par `open`, `docs/PIEGES.md`).

### 5. Scribe (référence, Swift)

- **Modes livrés** : `Models.swift`, `enum RewriteMode` à 8 valeurs (pas d'Apaisé ; Amusant présent), `menuGroups` en 3 groupes.
- **Modes personnels** : `Modes.swift`, `struct CustomMode` : `id`, `name`, `style`, **`rewrites: Bool`** (pas de `fidelity`), `vouvoie`, `examples: [ModeExample]` (`id`, `dictation`, `expected`). Champ `customModes` dans `AppSettings` (`Models.swift`). Pas de `simpleFidelity` ni de `speakerGender`.
- **Menu de la barre des menus** : `AppDelegate.swift`. `setupStatusItem` crée les modes livrés une fois ; **`menuWillOpen` recalcule à chaque ouverture** (groupe des modes personnels recréé, coches d'après `rewritingOn ? modeChoice : nil`, titre « Mode : … »). `selectMode` : `modeChoice = choice` et `cleanupEnabled = true`. Pas de sondage.
- **Masquage de modes : aucun. Import et export : aucun** (les seuls `NSOpenPanel` choisissent un modèle ; ni `NSSavePanel`, ni `fileExporter`).
- **Création** : `ModesView.createMode(from:)` : « un nom déjà pris reçoit un numéro » (`Mon mode`, `Mon mode 2`, `Mon mode 3`…), la même règle que celle prévue pour l'import. Exemples édités dans `ModesView.swift`.

| Sujet | Scribe | Scriibe |
|---|---|---|
| Cran d'un mode personnel | `rewrites: Bool` | `fidelity` (4 crans), `rewrites` écrit pour la compatibilité |
| Modes livrés | 8 (dont Amusant) | 9 avec Apaisé, Amusant retiré |
| Cran de Simple, genre de la personne | absents | `simpleFidelity`, `speakerGender` |
| Liste du menu | recréée à chaque ouverture | relue deux fois par seconde, reconstruite si `(id, label)` change |
| Groupes du menu | 3 | 4 + 4 + 1, modes personnels à part |
| Choix de fichier | `NSOpenPanel` | `osascript` (Mac), `GetOpenFileNameW` (Windows), rien (Linux) |
| Nouveau champ (masqué, version…) | perdu si Scribe réécrit `settings.json` | à accepter (`docs/CADRAGE_PROFILS.md`) |

Un champ « modes masqués » ou une enveloppe d'export seront donc des écarts voulus à noter dans `docs/ECARTS.md` au moment de les coder.

### 6. Points d'attention pour l'implémentation

**Réglages et modèle de données.**

- Un champ de visibilité (ex. liste d'ids masqués : `raw()` pour les modes livrés, UUID pour les personnels) demande : la déclaration dans `AppSettings` avec `#[serde(rename)]`, sa valeur par défaut dans `impl Default`, la lecture tolérante dans `from_value` (ids inconnus ignorés en silence), un test de relecture, et `OPTIONAL_KEYS` s'il est omis quand il est vide.
- Ne pas confondre `customModeID` (mode choisi) et la visibilité. **Au moins un mode doit rester visible**, à imposer dans l'interface et à la lecture (un fichier édité à la main peut tout masquer).
- Un champ ajouté à `CustomMode` casserait les littéraux de structure de `crates/core/tests/` (`prompts.rs`, `speaker_gender.rs`, `simple_fidelity_scriibe.rs`) ; un champ côté `AppSettings`, non. `fixtures/settings.json` ne se modifie pas.
- Un import ne touche pas l'empreinte des prompts (`4ed86f0c1c154918`, 8 modes de Scribe). Une consigne importée est aplatie (`ModeProfile::sentence`) mais arrive dans le prompt : limiter sa longueur.

**Import et export.**

- Limites chiffrées à choisir (taille du fichier, nombre de modes, nombre d'exemples, longueur des textes) et à tester : fichier trop gros, mal formé, autre `kind`, version supérieure.
- Cas de conflit à tester : `id` du mode, `id` de chaque exemple, « Mon mode 2 » sans accumulation, `fidelity` invalide (`Fidelity::from_raw`), `customModeID` jamais écrit par l'import.
- La page envoie `customModes` entier : l'import côté serveur doit fusionner puis la page recharger (`settings.load()`).

**Interface, traductions, Aide.**

- Tout texte de page passe par `t('français')` avec sa traduction dans `ui/src/lib/locales/en.ts` (`en-server.ts` pour un texte venu de Rust) ; le menu de l'icône se traduit en Rust. Lire `docs/I18N.md` d'abord.
- Aide : rubrique `modes` de `ui/src/tabs/Help.svelte` à reporter dans `HelpEn.svelte`. **L'Aide dit « Sous Linux, il n'y a pas encore d'icône »** alors que `mac_tray.rs` est décrit comme servant aussi à Linux : à vérifier avant de rédiger l'Aide du menu.
- Un commentaire en tête de `tray_source.rs` dit que seul Windows utilise `menu_view_in` « pour l'instant » : probablement périmé, à vérifier.
- Écrire les chemins des tests avec `Path::join`. Sur Mac, Scriibe.app se lance par `open`, jamais par Bash.

### 7. Décisions à prendre aux tâches suivantes

Recommandations du rédacteur, à confirmer dans les tâches 1 à 3 (choix techniques, pas de question à l'auteur sauf mention) :

1. **Mode actif masqué : le laisser visible et coché dans le menu**, même si sa case est décochée. Raison : un menu sans coche ni mode actif désoriente, et c'est le seul comportement qui ne change pas ce qui se passe à la dictée. La case reste décochée dans la page, avec une note. À soumettre à l'auteur si la tâche 1 le juge nécessaire (usage visible).
2. **Stockage : liste d'ids masqués (`hiddenModes`)**, plutôt qu'un booléen par mode. Raison : « tous affichés » est l'état par défaut, aucun champ n'est à ajouter à `CustomMode`, un mode nouveau ou importé apparaît tout seul, et Scribe n'a rien à comprendre.
3. **Import : par chemin, lu par le serveur** (`kind` d'import ajouté à `api/pick-file`), avec une limite de taille explicite en plus ; repli sans dialogue pour Linux (coller le contenu ou saisir le chemin) à trancher en tâche 3.
4. **Export : repli garanti par le presse-papiers ou un dossier ouvert** ; un `pick_save` natif (macOS, Windows) est un plus, pas un prérequis.
5. **Enveloppe : celle de la décision 4 du cadrage**, sans attendre l'étape 2 ; si le dictionnaire est codé ensuite, il réutilise le même code.

### 8. Ce qui n'a pas pu être établi

- Si un lien `download` ou un `Blob` fonctionne dans WKWebView, WebView2 et WebKitGTK sous Tauri (non utilisé, non essayé).
- Le comportement des dialogues de fichier selon la version du système ; seul le Mac a été essayé (`docs/ESSAI_MACOS.md`).
- Si une coche se met à jour dans un sous-menu resté ouvert ; le menu sous Windows et Linux.
- Le sort de la mention « Linux sans icône » de l'Aide.

## Modes affichés (tâche 1)

**Décision de l'auteur (12/11).** Par défaut, tous les modes sont affichés. Décocher un mode le fait disparaître du menu de l'icône, **y compris le mode actif** (il reste le mode utilisé, sans coche dans le menu, jusqu'à ce qu'un autre soit choisi). Le mode reste choisissable dans la page. Cette décision remplace la recommandation du § 7 du Constat (garder le mode actif visible).

**Ce qui a été fait.**

- Réglage `hiddenModes` (`crates/core/src/settings.rs`) : liste d'identifiants, toujours écrite dans `settings.json` (donc acceptée par `merge_settings`, pas d'entrée dans `OPTIONAL_KEYS`). Lecture tolérante (`clean_hidden_modes`) : doublons, identifiants inconnus (mode supprimé, Amusant) et valeur illisible sont écartés ; si tous les modes étaient masqués, la liste est vidée. `menu_mode_ids()` donne les identifiants proposables, `is_hidden_in_menu(id)` répond pour un mode (jamais « masqué » si cela masquerait tout, par exemple après la suppression d'un mode personnel).
- Menu de l'icône : filtre dans `menu_view_in` (`crates/cli/src/tray_source.rs`), modes livrés et personnels ; un groupe vide disparaît. Les fils de rafraîchissement Windows, macOS et Linux n'ont pas changé (la suite `(id, label)` change, le sous-menu est reconstruit).
- Page : `ui/src/tabs/Modes.svelte`, une case par mode (livrés et personnels), la dernière case affichée est désactivée ; phrase d'explication en tête.
- Aide : rubrique « Les modes » → « Choisir les modes du menu de l'icône », dans `Help.svelte` et `HelpEn.svelte`. Traductions dans `locales/en.ts`. Écart noté dans `docs/ECARTS.md`.

**Tests.** `crates/core/tests/hidden_modes.rs` (défaut, aller-retour, identifiants écartés, tout masqué, mode supprimé) ; deux tests dans `tray_source.rs` (modes livrés masqués dont l'actif, groupe des modes personnels). Mesurés ce jour : `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings` et `cargo test --workspace` sans erreur ; interface : `npm run check` (0 erreur), `npm test` (95 tests passent), `npm run build`.

**Non vérifié.** L'effet réel dans le menu de l'icône (Mac, puis Windows et Linux) : c'est l'essai de la tâche 4.

## Export (tâche 2)

**Ce qui a été fait.**

- Format : `crates/core/src/export.rs`. Enveloppe commune (`envelope`) : `format: "scriibe-export"`, `version: 1`, `kind`, `app` (« Scriibe x.y.z »), `exportedAt` (le jour seul, UTC), `content`. Export de modes : `modes_export` → `kind: "modes"`, `content: {customModes: […]}`. Chaque mode est écrit **champ par champ** (`mode_entry`) : `id`, `name`, `style`, `fidelity`, `rewrites` (pour Scribe), `vouvoie`, `examples` (`id`, `dictation`, `expected` ; liste vide si l'auteur les retire). Jamais de sérialisation des réglages. L'enveloppe est celle que l'import (tâche 3) et le dictionnaire (étape 2 du cadrage) reprendront.
- Nom proposé : `scriibe-mode-<nom du mode simplifié>-<jour>.scriibe.json` (`mode_file_name` : minuscules, sans accent, sans signe, 40 caractères au plus ; jamais le nom de l'utilisateur).
- Serveur : `POST api/modes/export` (`crates/cli/src/web/export.rs`), `{id, examples}`. Il construit le fichier, fait choisir l'emplacement, écrit, et répond `{ok, saved, path}` (`saved: false` si l'utilisateur annule).
- Choix de l'emplacement : nouvelles fonctions `pick_save` — macOS par `osascript` (`choose file name`, `crates/platform/src/macos/file_dialog.rs`), Windows par `GetSaveFileNameW` avec confirmation avant d'écraser (`crates/platform/src/windows/file_dialog.rs`). **Linux : pas de boîte** ; le fichier va dans `~/Documents` (ou le dossier personnel) sous un nom qui ne s'écrase pas (`free_path`, préfixe « 2- », « 3- »…), et la page affiche l'emplacement.
- Page : `ui/src/tabs/Modes.svelte`, bouton « Exporter » sur chaque mode personnel, carte « Exporter le mode « … » » avec le contenu (« 1 mode, N exemple(s) »), la case « Inclure les exemples » et l'**avertissement** (la consigne et les exemples peuvent contenir du texte personnel ; le fichier n'est pas chiffré). Aide FR et EN : « Exporter un mode ». Écart noté dans `docs/ECARTS.md`.

**Décisions techniques** (recommandations de la tâche 0, § 7).

- Enveloppe du cadrage reprise sans attendre l'étape 2 : un seul format pour les modes, les commandes et le dictionnaire.
- Un `pick_save` natif plutôt que le presse-papiers : le cadrage veut que l'emplacement soit choisi par l'utilisateur, et le code Windows ressemble à celui de `pick_file`. Le repli sans boîte ne sert qu'à Linux.
- Le jour est en UTC (pas d'heure locale dans le cœur) : près de minuit, il peut différer d'un jour du jour local ; sans conséquence (ni l'heure ni le fuseau ne partent).
- Les modes livrés ne s'exportent pas : seuls les modes personnels (ils sont dans Scriibe, pas à partager).

**Tests.** `crates/core/tests/export_modes.rs` (6 : enveloppe, liste blanche exacte des clés, exemples retirés, relecture identique par `CustomMode::from_value`, rien de sensible, jour et nom de fichier) ; dans `web/export.rs` : fichier d'un mode, mode inconnu, jamais d'écrasement ; dans `macos/file_dialog.rs` : script de la boîte. Mesurés ce jour : `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (481 tests passent, 0 échec), `npm run check` (0 erreur), `npm test` (95 passent), `npm run build`.

**Non vérifié.**

- Le code Windows (`pick_save`) n'a pas été compilé : `clippy --target x86_64-pc-windows-gnu` s'arrête ici sur l'absence de `x86_64-w64-mingw32-gcc` (compilateur croisé absent du Mac). À vérifier par la CI Windows, puis à la main sur le PC (carnet « à vérifier sur le PC »).
- Les boîtes « Enregistrer sous » ne se sont pas ouvertes (macOS : essai de la tâche 4 ; Windows, Linux : sur le PC) ; la page n'a pas été essayée dans un navigateur.

## Import (tâche 3)

**Décision de l'auteur (12/11).** Un nom déjà pris ne s'écrase jamais sans son accord ; il veut **aussi pouvoir remplacer l'existant**. D'où un choix **par mode** dans l'aperçu : « l'ajouter sous le nom « SMS 2 » » (par défaut) ou « remplacer le mode « SMS » existant » (avec un avertissement).

**Ce qui a été fait.**

- Cœur : `crates/core/src/import.rs`.
  - `parse_modes` contrôle **tout avant de toucher aux réglages** et refuse avec un message clair : fichier de plus de 1 Mo, JSON illisible, `format` autre que `scriibe-export`, `version` absente, nulle ou plus récente, `kind` autre que `modes`, liste vide ou de plus de 50 modes, plus de 50 exemples par mode, texte de plus de 20 000 caractères, nom de plus de 100, mode ou exemple mal formé, cran de fidélité inconnu ou absent. Un fichier de Scribe (case `rewrites` seule) est lu : Reformulé si vrai, Léger sinon.
  - Seuls les champs attendus sont lus (`id`, `name`, `style`, `fidelity`/`rewrites`, `vouvoie`, `examples`) ; tout autre champ est ignoré. Les caractères de contrôle sont retirés des textes, le nom tient sur une ligne, un nom vide devient « Mode importé ».
  - **Une consigne importée reste un texte** : stockée telle quelle, jamais interprétée ni exécutée ; l'import ne modifie aucun autre réglage et ne choisit jamais le mode (`customModeID`, `rewriteMode`, `hiddenModes` intacts).
  - `preview` donne, pour chaque mode, son conflit et le nom qu'il aurait. `apply` fusionne : un `id` de mode ou d'exemple déjà présent est remplacé par un nouveau ; un nom pris reçoit un numéro (« SMS 2 », « SMS 3 »…) calculé sur le radical, donc **sans accumulation** (« SMS 2 » importé ne devient pas « SMS 2 2 ») ; un remplacement garde la place et l'`id` du mode remplacé, donc il reste choisi s'il l'était. Un mode peut être décoché (non importé).
- Serveur : `crates/cli/src/web/import.rs`, trois routes. `api/modes/import/read` (boîte d'ouverture du système ; sous Linux `ask_path`, la page demande le chemin), lit le fichier **par son chemin** (taille contrôlée avant et pendant la lecture), garde le résultat en attente et rend l'aperçu ; `api/modes/import/apply` fusionne dans les réglages, enregistre et rend ce qui a été ajouté ou remplacé ; `api/modes/import/cancel`. Un fichier refusé ne laisse rien en attente. L'import passe par le serveur, puis la page relit les réglages (`settings.load()`), pour que sa sauvegarde ne l'écrase pas.
- Page : carte « Importer des modes » (`ui/src/tabs/Modes.svelte`) : aperçu (nom, cran, exemples), case par mode, choix ajouter/remplacer en cas de nom pris, bouton « Importer » désactivé si rien n'est coché. Messages de refus du serveur traduits (`en-server.ts`). Aide FR et EN : « Importer un mode ». Écart noté dans `docs/ECARTS.md`.
- Choix techniques : limites chiffrées fixées ici (aucune dans le cadrage) : 1 Mo, 50 modes, 50 exemples, 20 000 caractères par texte, 100 pour le nom ; assez larges pour un usage normal, assez étroites pour qu'un fichier tiers ne gonfle pas le prompt.

**Tests.** `crates/core/tests/import_modes.rs` (22 tests : trop gros, mal formé, autre type, version plus récente, liste vide ou trop longue, textes trop longs, cran invalide ou absent, modes mal formés, fichier de Scribe, consigne hostile et champs inconnus ignorés sans toucher aux autres réglages, nom vide, **aller-retour export → import identique**, sans exemples, nom pris (numéro, existant intact), identifiants déjà présents (modes et exemples), numéro non cumulé, deux noms identiques dans un fichier, remplacement (place, `id`, cran, exemples), remplacement par son propre export, remplacement sans homonyme, mode décoché, aperçu) ; `web/import.rs` (4 : fichier refusé = rien en attente et rien de changé, fichier trop gros ou absent, aperçu puis accord ajouter ou remplacer avec attente consommée, chemin tapé). Mesurés ce jour : `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (507 tests passent, 0 échec), `npm run check` (0 erreur), `npm test` (95 passent), `npm run build`.

**Non vérifié.** La boîte « Ouvrir » (macOS : essai de la tâche 4 ; Windows et Linux : sur le PC, dont le champ de chemin de Linux) ; la page n'a pas été essayée dans un navigateur ; le code Windows reste non compilé sur le Mac (voir § Export).

## Essai réel (tâche 4)

Sur le Mac de l'auteur, le 12/11/2026, avec `Scriibe.app` compilée en release (`tools/faire_app_macos.sh`) et lancée par `open`. Les modes de l'auteur ont servi (« Langage Schtroumpfs », « Patrick BOSSO - Grammaire Marseillaise »).

**Contrôlé par la session, sans l'auteur** (routes locales de l'application lancée, fichiers de la session) :

- un fichier abîmé est refusé (« ce fichier n'est pas un fichier JSON lisible ») et `settings.json` reste identique à l'octet près ;
- l'aperçu d'import repère le nom déjà pris (« Langage Schtroumpfs » serait ajouté sous « Langage Schtroumpfs 2 ») ; annulé, rien n'est écrit ;
- après le réimport de l'auteur, le mode « Patrick BOSSO » est **identique à l'original** (identifiants, consigne, cran, 3 exemples comparés à une copie prise avant), il n'y en a qu'un, et aucun mode n'est choisi par l'import.

**Verdict de l'auteur** (essai guidé, une étape à la fois) :

| Étape | Résultat |
|---|---|
| 1. Décocher Soutenu | Il disparaît du menu de l'icône (Mac). OK |
| 2. Décocher le mode actif (Simple) | Il disparaît de la liste, aucun mode n'est coché, le titre reste « Mode : Simple ». OK |
| 3. Quitter et relancer | Les choix sont conservés d'une fois sur l'autre (le fichier le confirme : `hiddenModes`). OK |
| 4A. Exporter « Patrick BOSSO » | Carte, avertissement, boîte « Enregistrer sous », nom proposé et contenu conformes ; aucune clé, aucun historique, aucun nom. OK |
| 4B. Supprimer puis réimporter | Aperçu sans conflit, message « Importé : 1 ajouté(s), 0 remplacé(s). », mode revenu avec ses 3 exemples. OK |
| 4C. Importer un fichier abîmé | Refus clair, sans bouton « Importer », rien de changé ; **le message était noir au lieu d'orange (corrigé, revu orange)**. OK |

**Défauts trouvés à l'essai, corrigés.**

1. **Limite de texte trop basse.** La consigne de « Langage Schtroumpfs » fait 5 064 caractères : la limite de 5 000 aurait refusé l'import de sa propre exportation. Limite relevée à **20 000** (consigne et chaque côté d'un exemple) ; test de régression `a_long_instruction_like_a_real_dictionary_mode_goes_back_and_forth`.
2. **Message de refus d'import en noir** : la classe de couleur dynamique n'était pas retenue par la mise en forme de la page ; remplacée par `class:warn`.

**Non essayé** : l'ajout sous un nom numéroté et le remplacement d'un mode existant (cas couverts par les tests du cœur et du serveur, pas par un essai à l'écran) ; la boîte « Enregistrer sous » et « Ouvrir » sur Windows et Linux (carnet « à vérifier sur le PC ») ; le choix du mode actif dans le menu après un masquage, sous Windows et Linux.

## Ajouts

Aucune nouvelle demande de l'auteur pendant l'essai. À noter, pour qui voudra y revenir :

- **Question de l'auteur** sur les `id` du fichier d'export : ce sont des identifiants tirés au hasard (UUID), sans lien avec la personne ni l'ordinateur ; à l'import, un identifiant déjà présent est remplacé par un nouveau. Rien à changer.
- **Constat** : supprimer un mode personnel retire aussi son identifiant de `hiddenModes` (lecture tolérante) ; un mode réimporté avec le même identifiant redevient donc visible dans le menu, même s'il était masqué avant sa suppression. Cohérent, mais à savoir.
- **Constat** : en cas d'échec d'enregistrement d'un export, le message n'est pas en orange (seul le refus d'import l'est) ; sans gravité.

## Relecture (livraison, tâche 5)

Relecture du diff par un agent distinct de celui qui l'a écrit (aucun bloquant) ; l'auteur a demandé de corriger les défauts importants et mineurs, avant le push.

| Constat | Correction |
|---|---|
| I1. L'accord de l'import n'était pas lié à l'aperçu montré (deux pages, deux clics) | `read` renvoie un **jeton** d'aperçu ; `apply` l'exige et refuse un jeton périmé ou un nombre de choix qui ne correspond pas (l'attente reste alors telle quelle) ; bouton « Importer un mode… » désactivé pendant l'import |
| I2. Deux modes du fichier de même nom, tous deux en « remplacer » : le premier était écrasé en silence | `apply` retient les places déjà remplacées ou ajoutées ; le second est ajouté sous un nom numéroté |
| I3. Le chemin de lecture fourni par la page était accepté partout (sondage de fichiers, chemin réseau Windows) | Honoré **sous Linux seulement** (seul système sans boîte) |
| M1. Export Linux : écrasement possible (course, lien brisé) | Écriture en création exclusive (`create_new`), numéro suivant si le nom existe |
| M2. Annulation et échec de la boîte macOS indistincts | `classify` : seul le code -128 est une annulation, le reste est une erreur claire |
| M4. L'attente de l'import était consommée même si l'écriture échouait | Consommée après l'enregistrement réussi |
| M5. « Rapport 2026 » devenait « Rapport 2 » | Le numéro final n'est retiré que si le radical existe déjà |
| M7. Caractères Unicode invisibles ou inverseurs dans un nom | Retirés du nom |
| M9. Tests : fichiers temporaires jamais supprimés, cas manquants | Dossiers d'essai supprimés ; tests du jeton périmé, du double remplacement, du nombre de choix, du numéro, des caractères invisibles, du fichier non UTF-8, de l'export sans écrasement, de l'annulation macOS |

**Laissés tels quels** : M3 (Windows : chemin de plus de 1 024 caractères pris pour une annulation, extension `.json` si le nom est retapé sans extension, boîte sans fenêtre propriétaire : très improbable ou sans conséquence, comme `pick_file`) ; M6 (l'aperçu peut annoncer un nom numéroté différent de celui obtenu si l'on décoche ou remplace un mode précédent : cosmétique) ; M8 (le nom « Mode importé » donné à un mode sans nom, et les titres des boîtes natives, restent en français quelle que soit la langue de l'interface).

**Mesuré** : `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (513 tests passent, 0 échec), `npm run check` (0 erreur), `npm test` (95 passent), `npm run build`. Essai à la main sur le Mac après les corrections : import d'un fichier dont le nom existe, choix « remplacer », un seul mode au final avec ses 3 exemples. Non vérifiés : le code Windows (CI Windows), les boîtes sous Windows et Linux.
