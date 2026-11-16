# Commandes vocales étendues : implémentation (chantier 6)

Découpage : `decoupage-2026-11-12-c6-implementation-commandes.md` (vault, `1 - Projets/scriibe/decoupages/`). Cadrage : `docs/CADRAGE_COMMANDES.md`.

## Constat

Fait le 15/11/2026 (tâche 0), par un sous-agent de recherche en lecture seule, sur `HEAD` = `907ada5` (version 0.5.0), comparé au cadrage du 13/11 (commit `00b2960`). Le « § 7 » du cadrage désigne deux endroits : le tableau « Ordre de livraison » (décision 7, étapes 1a et 1b) et la liste « Ce qui complique l'idée » (Constat 7). Les deux sont couverts.

**Contrôle** : le rapport du sous-agent n'a été vérifié que par sondage (définitions de `translateKeys`, `open`, `process_recording`, `try_command`, `release`, `Binding`, `HotkeyEvent`, `command_held`, `COMPUTER_FIELDS`, test de classement) : tous exacts. Le reste est à relire dans le code avant de s'en servir.

### En bref

- **Aucun de ces fichiers n'a bougé depuis le cadrage** : `controller.rs`, `core/hotkey.rs`, `platform/keys.rs`, `core/commands.rs`, `platform/open.rs`, `platform/linux/wayland.rs`, `engines/services.rs`, `cleaner.rs`. Les numéros de ligne du cadrage restent exacts pour eux.
- **Ont bougé (chantier 4)** : `core/settings.rs`, `core/backup.rs`, `core/profile_file.rs`, `core/import_commands.rs`, `cli/web/mod.rs`, `cli/web/commands_io.rs` (nouveau), `cli/commands.rs`, `tray_source.rs`, `mac_tray.rs`, `platform/storage.rs`, la carte `VoiceCommands.svelte` et d'autres fichiers de l'interface.
- **Ni `commandKeys`, ni `Intent`, ni ouverture contrôlée n'existent encore.**

### Points d'accroche

| Point | Fichier:ligne actuel | Ce que dit le cadrage | Écart / remarque |
|---|---|---|---|
| (a) `commandKeys` | N'existe pas. Modèle `translateKeys` : `crates/core/src/settings.rs:390-391` (champ), `:466` (défaut), `:512` (classé `COMPUTER_KEYS`), `:682` (`from_value`) ; aussi `crates/core/src/backup.rs:25` (`COMPUTER_FIELDS`), `ui/src/lib/api.ts:140`, `ui/src/components/KeyPicker.svelte:101,107,172` | Nouveau réglage sur le modèle de `translateKeys` ; classement « ordinateur » (décision 4) | Le classement se fait maintenant dans `COMPUTER_KEYS` ; **deux listes** à tenir à jour (`COMPUTER_KEYS` et `backup.rs:COMPUTER_FIELDS`). |
| (b) `Intent` | N'existe pas. Aujourd'hui `Option<TranslationLanguage>` : produit `controller.rs:678` et `:698` ; consommé par `process` (`:812`), `cleaner::skips_cleanup` (`:893`), `:909`, `:922`, `:861`, `log_line` (`:975`, `:1027`) ; `State.translating` (`:81`) | `enum Intent { Dictation, Translation(langue), Command }` de `release()` à `process` | Exact, mais plus d'endroits que cités. Seule `process_recording(recording, translate: bool, input)` (`:686`) est publique, avec **3 appelants externes** : `crates/cli/src/commands.rs:409`, `crates/cli/src/web/mod.rs:1043`, `crates/pipeline/tests/dictation.rs:869`. |
| (c) Crochet, côté `core` | `crates/core/src/hotkey.rs` : `Hotkey::command_held` `:78-80` ; `Binding` `:86-94` ; `from_hotkey` `:97-106` ; `translate_held` `:112-114` ; `HotkeyEvent` `:119-128` ; `letter` `:132-139` ; `HotkeyMonitor` `:143-146` ; `handle_change` `:165-171` (liste `known` `:166`) ; `handle` `:174-194` | Un seul bloc « crochet » | `Binding`, `HotkeyEvent` et la liste `known` sont dans **`core`**, pas dans `platform`. Le crochet système (`macos/keyboard.rs:58`, `windows/keyboard.rs:76`, `linux/keyboard.rs:122`) envoie `KeyEvent::Changed{changed, down, held}` et ne connaît pas les combinaisons. Seul Windows lit les `HookRules` (`windows/keyboard.rs:56-78`). |
| (c) Crochet, côté `platform` | `crates/platform/src/keys.rs` : `Layout::validate` `:290` ; `validate_translate` `:316` ; `Layout::custom_binding` `:342` ; libres `validate` `:439`, `custom_binding` `:444`, `binding` `:455-466` (Linux ignoré `:457`) ; `HookRules` `:470-481` ; `hook_rules` `:483-495` (`mask_menu` `:490`) ; `default_translate_label` `:558` | Idem ; Linux : `binding()` `:457` | Exact. `hook_rules` n'a que dictée et traduction en entrée : sans la touche de commande, Alt gauche ou Windows ne seraient pas masqués. |
| (c) Ce qui l'appelle | `crates/pipeline/src/controller.rs` : `Controller::new` `:164-198` ; `dispatch_key` `:257-298` ; `start_hold` `:302-330` ; `end_hold` `:332-337` ; `start_capture` `:341-353` ; `stop_capture` `:355-365` | `State.pending_*`, `start_hold` | `State.pending_translate` `:92` (écrit `:307`, relu `:319-324`) ; il faudra une `pending_command`. |
| (d) Aiguillage (tout dans `controller.rs`) | `press` `:493-535` ; `request_translation` `:600-616` ; `release` `:635-682` ; `process_recording` `:686-699` ; `try_command` `:703-809` ; `process` `:812` ; appel de `try_command` `:843-846` | Nouvel aiguillage sous la touche ; `try_command` colle aujourd'hui dans plusieurs cas | Exact. `try_command` colle comme texte aux `:713` (aucune sélection), `:726` (sélection illisible), `:760`, `:786`, `:806` (id inconnu) ; ids aiguillés en `match` aux `:732`, `:756`, `:785`, `:804`. Point d'insertion : entre `:843` et `:848`, ou un `try_command` dédié à l'intention « commande ». |
| (d) Neutralisation des essais | `crates/cli/src/web/sinks.rs:66-…` (`SwitchPaster`) ; branché `crates/cli/src/commands.rs:177` | « neutralisées comme `SwitchPaster` (C4) » | Exact. **Aucune neutralisation d'ouverture** : pas de fonction d'ouverture dans le trait `Paster` ni dans `Deps` (`controller.rs:56-74`). |
| (d) Linux | `crates/platform/src/linux/wayland.rs` : constantes `:34-35` ; `dictation_keys` `:195-203` ; `translate_keys` `:205-212` ; deux `NewShortcut` `:227-231` ; extraction `:240-250` ; activation `:287-289` | Sous Wayland, troisième raccourci GNOME à la main | Exact. `Triggers` (`:215`) est un couple : il faudra un triplet, plus `gnome_translate_shortcut()` côté `cli/web/mod.rs:761-762`. |
| (e) Ouverture contrôlée | `crates/platform/src/open.rs:18` (`pub fn open(target: &str)`, 43 lignes) | Seule fonction d'ouverture, schéma non contrôlé, Windows par `cmd /C start` | **N'existe pas** (aucun `ShellExecute`, `gtk-launch`, `mailto` dans `crates`). `open` ne rend rien et n'a aucune restriction hormis `http://127.0.0.1:`. Appelants : `windows/tray.rs:316,338,340` ; `cli/commands.rs:254,259,335` ; `cli/prefill.rs:50` ; `cli/mac_tray.rs:153,161,162` ; `cli/web/mod.rs:220`. |
| Catalogue et `recognize` | `crates/core/src/commands.rs` : `MIN_WORDS` `:16` ; `Needs` `:20` ; `Argument` `:29` ; `CommandDef` `:38` ; `VoicePhrases::cleaned` `:57` ; `CustomCommand::cleaned` `:193` ; `all_defs` `:234` ; `builtin` `:271` ; `normalize` `:436` ; `catalogues` `:456` ; `recognize` `:469` ; `recognize_for` `:506` ; `validate_phrase` `:515` | Chaque fiche reçoit une portée ; `recognize`, `recognize_for`, `validate_phrase`, `CustomCommand::cleaned` prennent un contexte | Références du cadrage exactes ; seule dérive : `settings.rs:339-348` est maintenant `:358-361` (`voiceCommandsEnabled`, `voiceCommands`). |

### Ce qui a changé depuis le 13/11 (chantier 4)

- **Classement des champs** : `USER_KEYS` (~`settings.rs:490-507`) et `COMPUTER_KEYS` (~`:509-550`), avec `Owner`, `owner_of`, `split_object`, `AppSettings::split`, `from_parts`.
  - `translateKeys` est « ordinateur » (`:512`) ; `customCommands`, `voiceCommandsEnabled`, `voiceCommands` sont « utilisateur » (`:497-499`).
  - À classer : `commandKeys`, l'accord de la reformulation (étape 4) et `voiceApps` (étape 3) en **`COMPUTER_KEYS`** ; `snippets` et `webSearches` en **`USER_KEYS`**.
- **Test qui échoue si un champ n'est pas classé** : `crates/core/tests/settings_split.rs:21` (`every_settings_field_is_classified_as_user_or_computer`) ; `:32` (`the_classification_has_no_overlap_and_no_stale_key`) échoue aussi pour une clé classée sans champ : **le champ et son classement arrivent dans le même commit**. `:43` est une table de contrôle (facultatif). Un champ facultatif (`Option` avec `skip_serializing_if`) va aussi dans `every_key_written()` (`:7-18`).
- **Garde-fou 409** (`cli/web/mod.rs:849-850`) : une clé « utilisateur » écrite → 409 si la page croit un autre utilisateur actif ; une clé « ordinateur » (touche de commande) s'écrit sans. `merge_settings` (`:815-838`) refuse toute clé non classée (`:827`).
- **Validation des touches à la fusion** : `web/mod.rs:836` appelle `keys::validate(&merged.dictation_keys, &merged.translate_keys)` : **ajouter `command_keys`**. `capture_view` (`:877-891`, branche `translate` `:886`), la route `api/keys/capture/start` (`:315-320`), `translate_label` (`:762`) et `tray_source.rs:41-45` sont à dupliquer pour la touche de commande.
- **Export d'un utilisateur : liste blanche explicite, rien ne part tout seul.** `profile_file.rs:29-38` (`PREFERENCE_KEYS`, 8 clés) ; `export_profile` `:175-246` écrit champ par champ.
  - Une simple préférence : l'ajouter à `PREFERENCE_KEYS` (avec `preference_label` `:125`).
  - Un nouveau champ de commandes (extraits, sites) : une case dans `Categories` (`:52-66`), `export_default` (`:70-80`), `import_default` (`:85-87`), `from_value` (`:90-103`), `Contents` (`:112-122`), `Incoming` (~`:240`), `parse_profile` (`:325`), `preview` (`:393`), `build` (`:447`), et `UserExport.svelte` / `UserImport.svelte`.
  - Test : `profile_file_tests.rs:89-105` (« clé hors liste blanche »).
  - `commandKeys`, `voiceApps` et l'accord de la reformulation sont des champs d'ordinateur : jamais exportés, rien à faire (`profile_file.rs:5-8`).
  - Le cadrage (l.457-459) prévoit déjà : extraits décochés par défaut, sites cochés à l'export ; extraits et sites décochés à l'import.
- **Commandes perso** : `import_commands.rs:128-163` (`evaluate`) valide chaque phrase par `validate_phrase(…, Lang::Fr/En, &all_defs(settings), …)` (`:129-131`) : touché par le « contexte » de `recognize` et `validate_phrase`. Routes `api/commands`, `validate-phrase`, `validate-shortcut` : `web/mod.rs:240-262` ; `commands_view` (`:1368`) n'expose que `builtin()` ; `validate_command_phrase` (`:1386-1416`). L'écriture de `voiceCommands` par un import d'utilisateur ne contrôle que les tailles (`profile_file.rs:282-306`, `check_voice_commands`), ni `MIN_WORDS` ni les phrases prises ; `VoicePhrases::cleaned` (`commands.rs:57-79`) ne retire pas non plus les phrases d'un mot.
- **Sauvegarde complète** : `backup.rs:22-37`, `COMPUTER_FIELDS` (13 clés à la main, aucun test ne les contrôle). `commandKeys` (et `voiceApps`) y vont à la main, sinon une sauvegarde faite sur un Mac, restaurée sur Windows, y apporterait la touche de commande du Mac.

### Écarts au cadrage

1. **Emplacement des types du crochet** : le cadrage (l.274-278) range `HotkeyEvent`, `Binding`, `known`, `Layout::validate`, `custom_binding`, `mask_menu` dans un seul bloc. Les trois premiers sont dans `core/src/hotkey.rs`, les trois autres dans `platform/src/keys.rs`.
2. **`commands_io.rs`** (`crates/cli/src/web/commands_io.rs`) : ajouté par le chantier 4, absent du cadrage du 13/11 ; dépendance de plus pour le contexte de `validate_phrase` (`docs/PROFILS.md:208`).
3. **`Intent` remplace `Option<TranslationLanguage>` à plus d'endroits que prévu** : 3 appelants externes de `process_recording`, plus `log_line` et `cleaner::skips_cleanup`. Le cadrage ne cite que `release()` et `process`.
4. **`command_held` est aussi une opération du banc de fixtures** (`crates/core/tests/fixtures.rs:248`) ; `HotkeyMonitor::new(hotkey)` et `letter()` (P, R, C, X) sont lus par `hotkey_sequence` (`:316-335`) pour `fixtures/hotkey.json`, qui ne se modifie jamais. La touche de commande doit rester **absente par défaut** de `Binding::from_hotkey`, et un nouveau variant de `HotkeyEvent` doit ajouter une lettre sans changer les anciennes séquences.
5. **`Needs::Nothing`** (`commands.rs:23-24`) cite déjà « extraits, recherche web » ; aucune commande livrée ne s'en sert. Rien à corriger.
6. **Ouverture contrôlée sous Windows** : le cadrage prévoit `ShellExecuteW` ; `crates/platform/Cargo.toml:53-72` ne liste pas la fonctionnalité `Win32_UI_Shell` de `windows-sys` (à confirmer à la compilation). L'étape 1b modifiera `Cargo.toml` et passera par la CI Windows.
7. **Pas de neutralisation d'ouverture dans `SwitchPaster`** : l'étape 1a doit décider où va l'appel d'ouverture (nouveau trait dans `Deps` ou méthode du `Paster`), ce qui fixe où la neutralisation du banc d'essai se branche (`sinks.rs`).
8. **`backup.rs:COMPUTER_FIELDS`** : liste à tenir à la main, non prévue par le cadrage.
9. **`web/mod.rs:836` et `:877-891`** : validation des touches et capture de la page à étendre ; le cadrage dit seulement « la route `validate` ».
10. **Ligne à corriger dans le Constat du cadrage** : `settings.rs:339-348` → `358-361`. Autres références vérifiées encore exactes : `controller.rs:703`, `:812`, `commands.rs:271`, `:469`, `:506`, `:515`, `:205`, `open.rs:18`, `keys.rs:457`, `settings.rs:120`, `services.rs:21`, `http.rs:12`, `cleaner.rs:14`.

### Non vérifié

Le détail des fonctions de `commands_io.rs` ; les numéros exacts de `Owner` et `split` dans `settings.rs` ; `FileConfig::import_profile` et `profile_snapshot` dans `platform/storage.rs` (941 lignes ajoutées, non lues) ; la carte `VoiceCommands.svelte` (103 lignes ajoutées, non lue) ; `cleaner.rs:314` (`fill`) et `llama.rs:29-31` (inutiles à 1a et 1b) ; la fonctionnalité `Win32_UI_Shell` (aucune compilation faite).

## Étape 1a : socle (tâche 1, 15/11/2026)

Code de l'étape 1a de `docs/CADRAGE_COMMANDES.md` (touche de commande, portée des fiches, aiguillage sous la touche), fait d'après le § Constat. **Non publié** (part dans la 0.6.0 avec l'étape 1b). Ligne d'écart : `docs/ECARTS.md` (15/11/2026).

### Ce qui change pour la personne

- **Une touche de commande** à tenir en plus de la touche de dictée (comme la touche de traduction). Sous cette touche, ce qui est dit est **toujours une commande**, **jamais collé**, ni corrigé, ni mis au propre, ni gardé dans l'historique. Réglable dans Réglages → Dictée (« Touche tenue en plus pour dicter une commande ») ; « Rétablir » la remet par défaut.
- **Messages fixes de la pastille** (jamais les mots dits), traduits dans `locales/en-server.ts` : « Aucune sélection », « Sélection illisible », « Commande inconnue », « Commande non reconnue » (phrase **proche** d'une commande), « Commande trop longue » (plus de 25 mots).
- **Les commandes d'aujourd'hui** (dictionnaire, caractères autour, gras / italique / souligné, mes commandes) marchent **aussi sous la touche**. **Sans la touche, rien ne change** : même interrupteur `voiceCommandsEnabled`, mêmes phrases (deux mots au moins), même collage de la phrase quand il n'y a pas de sélection (`commands_104` et les tests d'avant passent tels quels).
- **La commande l'emporte sur la traduction** quand les deux touches sont tenues (indicateur de traduction effacé, modèle de traduction jamais appelé).
- Aide FR et EN (« La touche de commande »), guides Linux FR et EN (troisième raccourci GNOME).

### Choix faits (avec la raison)

1. **Touche par défaut, provisoire à confirmer à l'essai (tâche 3)** : Mac **Maj** (⌘ traduit ; Ctrl + Option est le raccourci de VoiceOver) ; Windows **Windows droite** (le crochet masque déjà son menu ; Alt gauche et Ctrl gauche exclus : menu et AltGr) ; Linux **Ctrl gauche** (Super ouvrirait « Activités » au relâchement). Jamais une touche déjà prise par la dictée ou la traduction : si la touche par défaut est prise (par exemple une dictée réglée sur Maj sur Mac), elle n'est pas donnée et la personne en choisit une. Pas d'essai possible dans cette tâche : l'essai réel est la tâche 3.
2. **`voiceCommandsEnabled` ne garde que les commandes sans la touche.** Sous la touche, la personne a demandé une commande : l'interrupteur coupé ne la rend pas muette. Facile à inverser si l'essai montre que c'est surprenant (`recognize_for`, un test).
3. **`validate_phrase` prend la portée de la commande visée** (deux mots pour une commande qui vaut aussi sans touche, un mot pour « sous la touche seulement ») au lieu d'un contexte passé par l'appelant : on ne peut pas se tromper de contexte, et `CustomCommand::cleaned` (portée « partout » par construction) reste inchangée. Une phrase ne peut toujours appartenir qu'à une seule commande, toutes portées confondues.
4. **Sans reformulation (étape 4)** : une phrase libre de 25 mots au plus donne « Commande inconnue » **sans lire la sélection** (pas de ⌘C inutile) ; « Commande trop longue » vaut avec ou sans sélection. L'étape 4 lira la sélection avant, comme le cadrage.
5. **Sélection illisible** : un seul message fixe pour les trois causes (autorisation manquante, presse-papiers non textuel, échec) ; la cause précise va au journal (texte fixe). Sans la touche, les trois notes d'avant restent.
6. **Préchauffage de la mise au propre** : non fait quand la touche de commande est **déjà tenue à l'appui** (cas testé). Si elle est ajoutée après la touche de dictée, le préchauffage a déjà commencé : inoffensif (on ne le défait pas).
7. **Pas de « Listening » spécial** sur la pastille pendant que la touche de commande est tenue, ni de ligne dans le menu de l'icône : non demandé, à voir à l'essai.
8. **Classement** : `commandKeys` est un champ d'**ordinateur** (`COMPUTER_KEYS`, et `backup.rs:COMPUTER_FIELDS`, qui passe à 14 clés : une sauvegarde d'un autre système ne rapporte pas la touche). Il n'entre donc **pas** dans l'export d'un utilisateur (rien à cocher à l'import). Le test du chantier 4 passe, le champ et son classement sont dans le même commit.
9. `#![recursion_limit = "256"]` dans `crates/cli/src/main.rs` : l'état de la page est un seul `json!` très long, que trois entrées de plus ont fait dépasser la limite par défaut.
10. **Banc d'essai** : la route d'essai accepte `"command": true` (même aiguillage, mêmes sorties neutralisées par `SwitchPaster`) ; test `during_a_trial_a_command_reaches_nothing_of_the_active_application`. Il n'y a pas encore de nouvelle sortie (ouverture, extrait, reformulation) à neutraliser : le test les couvrira à leur arrivée.

### Où est le code

- `crates/core/src/hotkey.rs` : `Binding.command`, `command_key_held`, `HotkeyEvent::CommandKey` (lettre `K`) ; **vide par défaut dans `from_hotkey`**, donc les séquences de `fixtures/hotkey.json` ne changent pas.
- `crates/core/src/commands.rs` : `Scope`, `Context`, `CommandDef.scope`, `recognize` et `recognize_for` avec contexte, `Route`, `route_under_key`, `is_close`, `INSTRUCTION_MAX_WORDS`.
- `crates/core/src/settings.rs`, `backup.rs` : `commandKeys`.
- `crates/platform/src/keys.rs` : `validate`, `custom_binding` à trois listes, `default_command`, `binding` (touche par défaut), `hook_rules` (la touche de commande masque le menu comme les autres), `default_command_label`.
- `crates/platform/src/linux/wayland.rs` : troisième raccourci GNOME (`command`, « CTRL+Down » proposé), `command_keys()` ; `linux/mod.rs` l'exporte.
- `crates/pipeline/src/controller.rs` : `Intent`, `request_command`, `pending_command`, `process_command`, `run_command(Context)`, `refuse`, `nothing_to_do` ; `process_recording` prend un `Intent`.
- `crates/cli/src/web/mod.rs` : validation à la fusion et à la capture (`kind: "command"`), état (`command_label`, `gnome_command_shortcut`, `default_command_label`), route d'essai.
- Interface : `KeyPicker.svelte`, `Settings.svelte`, `Help.svelte`, `HelpEn.svelte`, `api.ts`, `locales/`.

### Tests (mesurés sur le Mac)

- **740 tests Rust** (22 de plus que les 718 d'avant la tâche) et **109 tests de l'interface** sans erreur ; `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` et `svelte-check` sans erreur.
- Nouveaux : `commands_401` à `405` (portée, une phrase d'un mot, interrupteur, aiguillage proche / consigne / trop longue, « à un mot près ») ; 3 tests de la machine à états (`K`, commande avec traduction, absence par défaut) ; 4 tests de `keys.rs` (défauts jamais pris, validation, touche choisie, masque du menu) ; 8 tests du contrôleur (commande connue, interrupteur, messages fixes, aucune sélection ou sélection illisible, **mots dits sans trace** sur plusieurs phrases, commande contre traduction, préchauffage, essai en intention « commande ») ; 1 test de neutralisation (`sinks.rs`).

### Non vérifié

- **Code Linux** (`wayland.rs`, troisième raccourci) et **code Windows** : **non compilés ici**. Le Mac n'a ni cible Linux ni compilateur croisé Windows (`x86_64-w64-mingw32-gcc` absent : `cargo clippy --target x86_64-pc-windows-gnu` échoue dans la compilation d'une dépendance C). La CI de l'envoi (Linux) et un lancement manuel de la CI Windows et macOS (`gh workflow run ci.yml --repo scarou/Scriibe`) les vérifieront ; la tâche 3 les essaie sur les PC. Les changements propres à un système sont petits (une touche de plus rejouée à l'activation du raccourci GNOME, `VK_RWIN` comme défaut sous Windows).
- **Aucun essai à l'écran ni au clavier réel** : la touche par défaut de chaque système, la fenêtre « Changer… » pour la touche de commande, le troisième raccourci GNOME et le masque du menu Démarrer sont à voir à la tâche 3.
- Les nouvelles commandes (recherches, extraits, applications, mail, reformulation) n'existent pas encore : `Scope::UnderKey` n'a que des tests.

### Relais (fin de la tâche 1, contexte à 38 %)

Relais décidé par la règle du `CLAUDE.md` (contexte à 25 % ou plus en fin de tâche : on ne commence pas la tâche suivante dans cette session). Ne change ni l'ordre ni les consignes.

- **Fait** : toute la tâche 1 (étape 1a), commit local `7013dff` (puis ce relais), rien de poussé. 740 tests Rust et 109 tests de l'interface verts, fmt, clippy et svelte-check sans erreur (mesurés).
- **En cours** : rien (arbre propre après le commit de ce relais).
- **Prochain pas** : la tâche 2 du découpage (étape 1b, recherches), consigne de la tâche 2 telle quelle. Points d'accroche déjà établis dans le § Constat : `crates/platform/src/open.rs:18` (`open` n'a aucune restriction : écrire **une autre fonction** d'ouverture contrôlée qui n'accepte que `https:` et `mailto:`, sans shell : `ShellExecuteW` sous Windows, `open` sur Mac, `xdg-open` sous Linux ; elle ne passe **pas** par `open::open`) ; ajouter l'appel d'ouverture au trait `Paster` ou à `Deps` et le **neutraliser dans `SwitchPaster`** (`crates/cli/src/web/sinks.rs`, test à compléter : `during_a_trial_a_command_reaches_nothing_of_the_active_application`) ; `webSearches` : champ **« utilisateur »** (`USER_KEYS`, avec son entrée dans `every_key_written` si facultatif), validé à la lecture dans `AppSettings::from_value`, avec case dans l'export d'un utilisateur (`profile_file.rs` : `Categories`, `export_default` cochée, `import_default` décochée, `Contents`, `Incoming`, `parse_profile`, `preview`, `build`, `UserExport.svelte`, `UserImport.svelte`) ; les quatre sites sont des fiches `Scope::UnderKey` (`Needs::Nothing`, `Argument::Rest`), les sites de la personne se contrôlent comme dit au cadrage (décision 4, « Sites de recherche ») ; « nouvelle recherche <site> … » sous la touche seulement (test de non-régression : dite sans la touche, la phrase est collée comme du texte) ; `Intent::Command` et `run_command` existent déjà (un identifiant `recherche.<n>` s'ajoute à son `match`, journal `recherche.<n>` sans le nom du site ni la requête).
- **Pièges** : (1) le Mac n'a **pas de compilateur croisé** (ni Linux ni Windows) : `ShellExecuteW` demande peut-être la fonctionnalité `Win32_UI_Shell` de `windows-sys` (`crates/platform/Cargo.toml:53-72`), à confirmer par la **CI Windows** (`gh workflow run ci.yml --repo scarou/Scriibe`, après accord de l'auteur pour pousser) ; (2) les tests du contrôleur doivent tourner sur les trois systèmes : dans le banc, Option droite dicte (`Hotkey::RightOption`) et la touche de commande se lit dans `binding()` (voir `command_rig`, `Rig::dictate_under_key`) ; (3) `fixtures/hotkey.json` ne change jamais : ne rien mettre par défaut dans `Binding::from_hotkey` ; (4) `json!` de l'état de la page est près de sa limite de récursion (`#![recursion_limit = "256"]` posé) ; (5) un champ nouveau et son classement arrivent **dans le même commit** (test du chantier 4) ; (6) pas de réponse « oui » à la place de l'auteur : rien n'est poussé sans son accord.
- **Commits** : `7013dff` (étape 1a) sur `0e96ab3`, `5e73975` (tâche 0, constat) et `907ada5` (version 0.5.0, poussée). Trois commits locaux non poussés (`5e73975`, `0e96ab3`, `7013dff`).


## Étape 1b : recherches (tâche 2, 15/11/2026)

Code de l'étape 1b de `docs/CADRAGE_COMMANDES.md` (recherches vocales, fonction d'ouverture contrôlée, `webSearches`), fait d'après le § Constat et le § Relais de la tâche 1. **Non publié** (part dans la 0.6.0 avec l'étape 1a). Ligne d'écart : `docs/ECARTS.md` (15/11/2026).

### Ce qui change pour la personne

- **Sous la touche de commande seulement** : « nouvelle recherche Google », « … YouTube », « … Wikipédia », « … DuckDuckGo » (ou « new search … »), suivi de ce qu'on cherche, ouvre la recherche dans le navigateur par défaut. Wikipédia s'ouvre en français ou en anglais selon la langue de la **phrase dite**. Sans la touche, la phrase est collée comme du texte (test de non-régression).
- **Sites de la personne** : carte « Commandes vocales » (onglet Dictionnaire), partie « Mes sites de recherche » : un nom, une adresse `https://…{q}…` et une phrase (deux mots au moins). L'hôte de chaque site est affiché ; une adresse refusée dit pourquoi. Les phrases des quatre sites livrés se changent comme celles des autres commandes (groupe « Recherches sur internet »).
- **Messages fixes de la pastille** (traduits dans `locales/en-server.ts`) : « Site de recherche inconnu » (« nouvelle recherche » avec un site que la personne n'a pas, ou seul), « Rien à chercher » (la phrase d'un site, seule), « Recherche non ouverte » (le système n'a pas ouvert l'adresse), « Recherche impossible » (réglage devenu invalide au moment de la recherche).
- **Export et import d'un utilisateur** : case « Sites de recherche vocale », **cochée** à l'export, **décochée** à l'import (l'hôte de chaque site est affiché, une adresse refusée est signalée et le site n'est pas importé) ; identifiants toujours neufs ; 20 sites au plus.
- Aide FR et EN (puce « Recherches sur internet »).

### Choix faits (avec la raison)

1. **Identifiants** : `recherche.1` à `.4` pour Google, YouTube, Wikipédia, DuckDuckGo ; `recherche.<n>` avec n ≥ 10 pour la personne. Raison : le journal n'a que cet identifiant (cadrage, décision 5) ; les numéros 5 à 9 restent libres pour un site livré de plus. `recherche.nouveau` est l'identifiant de projet que la page donne à un site pas encore créé pour contrôler sa première phrase.
2. **Fonction d'ouverture** : `open::open_address`, **séparée** d'`open::open` (qui reste aux usages internes). Elle ne prend que `https://` à hôte nommé et `mailto:` (le cadrage, décision 3), jamais de `http:` : un site qui n'est qu'en `http` ne peut pas être ajouté. Le contrôle est `web_search::is_openable` (dans le cœur), **partagé** avec le faux collage des tests, pour que le test et la réalité refusent la même chose. Pas d'interpréteur : `ShellExecuteW` (Windows, `Win32_UI_Shell` ajouté à `windows-sys`), `open` (Mac), `xdg-open` (Linux), l'adresse pour seul argument (elle ne commence jamais par un tiret).
3. **Où est l'appel** : méthode `Paster::open_address` (défaut : la vraie fonction). `SwitchPaster` la neutralise pendant un essai (l'adresse est gardée dans `Captured.opened`, jamais lancée) ; `FakePaster` l'enregistre après le même contrôle ; `ConsolePaster` (`essai`) l'affiche ; `NoPaster` (système sans prise en charge) la refuse.
4. **Modèle d'adresse valable** (`web_search::check_template`) : `https` ; hôte **nommé** (au moins un point, pas d'adresse numérique, pas `localhost`, pas de `@`, pas de `%`) ; un seul `{q}`, ni dans l'hôte ni après un `#` ; que des caractères ASCII d'adresse (ni espace, ni guillemet, ni les signes `< > \ ^ |` ni l'apostrophe inverse, accolades seulement pour `{q}`) ; 300 caractères au plus. Contrôle à la lecture des réglages (`AppSettings::from_value` → `WebSearch::cleaned`), à la page (`api/web-searches/check`), à l'import, **et** une dernière fois sur l'adresse finale (`search_url` : même hôte que le modèle, 2 000 octets au plus). Les tests injectent des réglages **non nettoyés** pour prouver que ce dernier contrôle suffit (`https://{q}`, `http:`, `https://example.com{q}`, `javascript:`, `https://user@…`).
5. **La requête** : coupée à 300 caractères, blancs et contrôles ramenés à une espace, encodée en UTF-8 octet par octet (seuls lettres, chiffres, `- . _ ~` restent) : un `/`, `@`, `#`, `?` ou `&` dit ne change rien à l'adresse. `percent_encode` est dans `web_search` (cœur) ; celui d'`engines` (Deepgram) n'est pas touché. L'étape 3 (mail) pourra réutiliser celui du cœur.
6. **Aiguillage sous la touche** : deux issues de plus à `Route` : `EmptySearch` (la phrase d'un site, seule) et `UnknownSearch` (commence par « nouvelle recherche » / « new search » sans site connu). Elles passent **avant** « proche d'une commande » : une recherche mal dite ne devient jamais une consigne de reformulation (décision de l'auteur : la recherche sur internet commence par « nouvelle recherche »).
7. **Phrases des sites de la personne** : deux mots au moins (nettoyage à la lecture **et** page), alors qu'une phrase d'un site livré peut en avoir un (portée « sous la touche »). Raison : un mot seul comme « cherche » appartient à la reformulation (étape 4). Une phrase déjà prise par un site livré ou un autre site est écartée ; un site ne peut pas reprendre la phrase d'une autre commande (`validate_phrase`, toutes portées confondues).
8. **Classement** : `webSearches` est un champ **utilisateur** (`USER_KEYS`), dans l'export d'un utilisateur (liste blanche), décoché à l'import. Il est toujours écrit (liste vide), donc pas de changement à `every_key_written`.
9. **Langue d'une recherche** : Wikipédia suit `Recognized.lang`. Dans une langue de dictée fixe, seules les phrases de cette langue sont reconnues ; en « plusieurs langues », les deux.

### Où est le code

- `crates/core/src/web_search.rs` (nouveau) : `WebSearch`, `check_template`, `search_url`, `is_openable`, `percent_encode`, `builtin_defs`, `defs`, `target`, `is_search`.
- `crates/core/src/commands.rs` : `all_defs` (sites), `Route::EmptySearch` / `UnknownSearch`, `search_without_target`, `validate_phrase` (`recherche.nouveau`).
- `crates/core/src/settings.rs` : `web_searches`, `USER_KEYS`, lecture tolérante.
- `crates/core/src/profile_file.rs` : `Categories.web_searches`, `Contents`, `Incoming`, `SitePreview`, `Outcome`, `read_web_searches`.
- `crates/platform/src/open.rs` : `open_address`, `launch_address` (par système). `paste.rs` : `Paster::open_address`. `testing.rs`, `unsupported.rs`.
- `crates/pipeline/src/controller.rs` : branche `recherche.<n>` de `run_command`, messages de `process_command`.
- `crates/cli/src/web/` : `sinks.rs` (neutralisation), `mod.rs` (`api/web-searches/check`, liste des commandes, `validate-phrase`), `user_file_io.rs`.
- Interface : `VoiceCommands.svelte`, `UserExport.svelte`, `UserImport.svelte`, `Help.svelte`, `HelpEn.svelte`, `api.ts`, `locales/`.

### Tests (mesurés sur le Mac)

- **755 tests Rust** (15 de plus que les 740 d'avant la tâche) et **109 tests de l'interface** sans erreur ; `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` et `svelte-check` sans erreur.
- Nouveaux : `commands_511` (modèle d'adresse : https, hôte fixe, un `{q}`, cas refusés), `512` (requête encodée, hostile, coupée), `513` (quatre sites, langue de Wikipédia), `514` (sites de la personne nettoyés à la lecture), `515` (identifiants), `516` (aiguillage : commande, « rien à chercher », « site inconnu », sans la touche, phrases prises) ; 2 tests de `open.rs` (adresse non https refusée **avant** tout lancement : `http:`, `javascript:`, `file:`, identifiant `@`, hôte numérique, espace, guillemet, saut de ligne, application, option…) ; 4 tests du contrôleur (adresse encodée **sans trace** de la requête ni du nom du site dans le journal, la pastille, l'historique ; site de la personne ; messages fixes et réglage piégé ; **sans la touche, collée comme du texte** et rien d'ouvert) ; 3 tests d'export et d'import (liste blanche sans identifiant, décoché à l'import, adresses refusées écartées, limites) ; le test de neutralisation de `sinks.rs` couvre l'ouverture (`Captured.opened`).
- Le test du chantier 4 (`every_settings_field_is_classified_as_user_or_computer`) passe : le champ et son classement sont dans le même commit.

### Non vérifié

- **Aucune adresse n'a été ouverte pour de vrai** : les tests n'ouvrent jamais de navigateur (faux collage). `ShellExecuteW`, `open` et `xdg-open` réels sont à voir à la tâche 3 (« nouvelle recherche Wikipédia … » sur les trois machines).
- **Code Windows** : le Mac n'a pas de compilateur C croisé, donc `scriibe-platform` n'a pas été compilé pour Windows. Seule la fonction `launch_address` (`ShellExecuteW`) a été **vérifiée à part** dans une caisse isolée contre `windows-sys 0.59` (`cargo check --target x86_64-pc-windows-gnu`, sans erreur) avec la fonctionnalité `Win32_UI_Shell`. Le reste du crate pour Windows (aucun fichier `windows/` n'a été modifié) sera vu par la CI Windows (`gh workflow run ci.yml --repo scarou/Scriibe`, après accord de l'auteur pour pousser). **Code Linux** : `xdg-open` est le même code que sur Mac, non compilé pour Linux.
- La page n'a pas été vue dans un navigateur (carte « Mes sites de recherche », cases d'export et d'import) : seulement `svelte-check`, les tests et la compilation de l'interface.
- L'Aide dit encore « Elles ne marchent pour l'instant que sur Mac » dans une phrase d'avant (hors de cette tâche).

### Suite

- Tâche 3 (essai du lot 1 sur le Mac et les deux PC) : nouvelle session, `⚙ Sonnet · Élevé`, consigne dans le découpage. Cette session s'arrête là (contexte à 30 % en fin de tâche).
- Trois commits locaux non poussés (plus celui de cette étape) : rien n'est poussé sans l'accord de l'auteur.

## Essai lot 1 (tâche 3, 15/11/2026)

Essai réel du lot 1 (étapes 1a et 1b) : Mac d'abord (copie d'essai `Scriibe-essai.app` sur une copie des données, `SCRIIBE_DATA`), puis PC Windows et PC Linux par `/pilote-pc`. Résultats ci-dessous, au fur et à mesure.

### Ajouts

1. **15/11, pendant l'étape 1 du Mac** — l'auteur ne veut pas une **touche seule** pour les commandes (les touches simples servent à taper) mais une **combinaison**, par exemple « Option droite + Maj gauche » ou « Option droite + Espace ». État du code : la commande se tient **déjà en plus** de la touche de dictée (Option droite + Maj marche), et n'agit jamais seule ; mais le défaut Mac est « l'une ou l'autre des deux Maj » et Espace est exclu des touches proposées (`MAC_CHOICES`, elle sert à taper ; Option + Espace écrirait une espace insécable dans l'appli, l'écoute de touches ne retenant rien). **Décision de l'auteur : « Option droite + Maj gauche »** comme défaut sur Mac (fait, traité tout de suite : `Layout::default_command` = Maj gauche seule, Maj droit libre, libellé « Maj gauche », 3 tests adaptés, Aide FR et EN ; 755 tests Rust et 109 de l'interface verts). **« Option droite + Espace » : non fait**, à proposer comme ligne du carnet (Espace est exclu des touches proposées ; il faudrait retenir l'événement pour qu'Option + Espace n'écrive pas une espace insécable dans l'appli).
2. **15/11, étape 2 du Mac** — premiers essais avec « mets entre guillemets » : 3 sur 4 « phrase proche, non reconnue » (Whisper écrit « Mais entre guillemets », une fois « Né entre guillemets »). **Erreur de consigne de la session, pas du code** : l'auteur avait déjà retenu la forme à l'**infinitif** (« mettre entre guillemets », plus facile à prononcer et à reconnaître). Aucune variante ajoutée ; essai refait avec l'infinitif.

### Résultats (15 et 16/11/2026)

- **Mac (essai fait, 4 étapes sur 4)** : copie d'essai `Scriibe-essai.app` (`SCRIIBE_INSTALL`), lancée par `open -n --env SCRIIBE_DATA=…` sur une **copie** des données (modèles liés, pas copiés). (1) Réglages → Dictée affiche la touche de commande « Maj gauche » (défaut changé pendant l'essai, voir Ajouts 1). (2) « mettre entre guillemets » sous la touche (Option droite + Maj gauche) : texte entouré de guillemets, journal `commande: entourer.guillemets (sélection : N signes)`, aucun collage. (3) « nouvelle recherche Wikipédia sophrologie » : Wikipédia français s'ouvre à la page demandée, journal `commande: recherche.3 (requête : 11 signes)`, sans le nom du site ni les mots. (4) Phrase ordinaire sous la touche : rien n'est collé, pastille « Commande inconnue », journal `commande: aucune commande reconnue — rien n'est collé`. **Journal sans les mots dits** (aucune occurrence des phrases essayées).
- **Envoi** : `main` poussé à la demande de l'auteur (`907ada5` → `f8efed6`) ; CI manuelle sur les trois systèmes (`gh workflow run ci.yml`, run 43769117793) : **macOS vert ; Linux et Windows rouges** (un test de `keys` de plus sous Windows), voir Constats.
- **Compilation sur les PC** (`pilote maj` puis `pilote compiler --mode release`, commit `f8efed6`) : **Linux** et **Windows** compilent sans erreur (Linux 51 s, Windows 2 min 05), donc le code Windows (`ShellExecuteW`, `VK_RWIN` par défaut) et Linux (`xdg-open`, troisième raccourci) est enfin compilé pour de bon.
- **PC Windows (essai fait, 4 étapes sur 4)** : copie des données dans `C:\essai-c6` (modèles par jonction), instance lancée dans la session de l'auteur par une tâche planifiée temporaire (`schtasks /it`), supprimée ensuite ; ton Scriibe habituel (`C:\scriibe\scriibe.exe`) arrêté puis relancé. (1) La touche de commande affichée est « Windows droite ». (2) L'auteur n'a **pas de touche Windows droite** sur son portable : il l'a remplacée par Maj gauche (traduction passée sur Maj droit, sans conflit) ; avec Ctrl droit + Maj gauche, « mettre entre guillemets » entoure le texte (`entourer.guillemets`, 21 signes), menu Démarrer non concerné. (3) « nouvelle recherche Wikipédia sophrologie » ouvre Wikipédia (`recherche.3`, requête de 10 signes quand Whisper `small` entend mal le mot, de 11 quand il l'entend bien : ce n'est pas le code). (4) Phrase ordinaire : rien collé, pastille « Commande inconnue », journal « aucune commande reconnue ». Journal sans les mots dits.
- **PC Linux, GNOME Wayland (5 étapes sur 5)** : copie des données dans `/tmp/essai-c6` (modèles par lien), `whisper-server` de `~/whisper.cpp` rendu visible par le seul PATH de l'instance d'essai (les moteurs de Scriibe étaient à la corbeille depuis un essai précédent). (1) Fenêtre et icône. (2) Le troisième raccourci GNOME « Dicter une commande » (Ctrl + Bas) est proposé avec les deux autres dans la boîte « Add Keyboard Shortcuts » et affiché dans Réglages (« Ctrl + Bas »). (3) « mettre entre guillemets » : **pastille « Sélection illisible », rien collé** (attendu : Linux ne lit pas la sélection, cadrage § Linux ; journal `entourer.guillemets ignorée (sélection illisible) — rien n'est collé`). (4) « nouvelle recherche Wikipédia sophrologie » : Chrome s'ouvre sur la page (une boîte de mot de passe s'est ouverte avant : très probablement le trousseau de GNOME au démarrage de Chrome, titre non relevé). (5) Phrase ordinaire : rien collé, « Commande inconnue ».
- **PC Linux, Xfce X11 (4 étapes sur 4)** : GNOME 50 n'a plus de session X11, seule la session « Xfce Session » existe (vraie session ouverte par l'auteur). (1) Touche de commande « Ctrl gauche » (dictée : Ctrl droit). (2) Commande : « Sélection illisible », rien collé. (3) Recherche Wikipédia : s'ouvre, `recherche.3` (11 signes). (4) Phrase ordinaire : rien collé. Les trois premières dictées de la session n'ont capté aucun son (micro par défaut muet, RMS 0), la suite a marché : remarque d'environnement, pas de défaut de Scriibe. L'instance d'essai a été arrêtée, l'auteur est revenu en GNOME, et son Scriibe habituel (`~/Scriibe/target/release/scriibe lancer`, vraies données) a été relancé.
- **Remise en état** : Windows et Linux : instances d'essai arrêtées après vérification de leur emplacement ; tâches temporaires supprimées. Les copies de données d'essai restent (`C:\essai-c6`, `/tmp/essai-c6`, Mac : scratchpad de la session) ; le Mac n'avait pas d'instance habituelle lancée.

### Constats

1. **CI rouge sous Linux et Windows, non vue sur le Mac (corrigé en local, non poussé)** : `keys::tests::the_effective_binding_falls_back_to_the_default_when_the_settings_are_invalid` (Linux et Windows : `binding()` donne maintenant la touche de commande par défaut, que `Binding::from_hotkey` seul n'a pas ; sur Mac elle est écartée parce que Maj traduit déjà sous Ctrl droit) et `keys::tests::the_hook_masks_the_menu_keys_and_holds_back_ordinary_keys_of_the_combination` (Windows : la touche de commande par défaut, Windows droite, masque le menu Démarrer, ce que le test ne prévoyait pas). Correction : le premier compare à `with_default_command(Binding::from_hotkey(..))`, le second passe une touche de commande sans effet sur les menus (F12) dans son helper. 34 tests de `keys` verts sur le Mac ; **à relancer sur Windows et Linux** (CI) avant la 0.6.0.
2. **Le défaut Windows « Windows droite » n'existe pas sur un portable** (une seule touche Windows, à gauche) : il faut le changer à la main. Proposé au carnet : un défaut de commande qui existe sur tout clavier (par exemple Maj gauche avec la traduction passée sur Maj droit, comme à l'essai) ; changerait le défaut de traduction, donc à décider par l'auteur.
3. **Sous Linux, les commandes qui agissent sur une sélection (guillemets, gras, dictionnaire…) répondent « Sélection illisible »** : limite connue du cadrage (Linux ne lit pas la sélection ni n'envoie de raccourci), jamais dite à la personne par un message plus clair. À mettre au carnet : message « Non disponible sous Linux » (comme prévu à l'étape 4) ou lecture de la sélection par le presse-papiers primaire.
4. **Option droite + Espace** (Ajouts 1) : non fait, au carnet.

### Suite

- Corriger la CI : pousser le correctif (accord de l'auteur, dans `/livrer-git`), relancer la CI des trois systèmes, attendre trois systèmes verts.
- Tâche 4 : version 0.6.0 avec `/livrer-git` (statut d'essai : Mac complet, Windows complet, Linux Wayland et X11 sans commandes sur sélection).

### Relais (fin de la tâche 3, contexte à 27 %)

Relais décidé par la règle du `CLAUDE.md` (contexte à 25 % ou plus en fin de tâche : on ne commence pas la tâche suivante dans cette session). Ne change ni l'ordre ni les consignes.

- **Fait** : toute la tâche 3 (essai du lot 1 sur Mac, Windows, Linux Wayland et Linux X11), résultats et constats ci-dessus ; commits locaux `bed2656` puis celui de ce relais ; `main` poussé jusqu'à `f8efed6` (accord de l'auteur, pour l'essai sur les PC). Un correctif de deux tests de `keys` est **commité mais pas poussé**.
- **En cours** : rien (arbre propre après le commit de ce relais). Machines : instances habituelles relancées sur Windows (`C:\scriibe\scriibe.exe`) et Linux (`~/Scriibe/target/release/scriibe lancer`, session GNOME Wayland) ; aucune sur le Mac. Les deux PC sont à `f8efed6` et compilés en release à ce commit.
- **Prochain pas** : la tâche 4 du découpage (version 0.6.0), consigne telle quelle. **Avant tout** : la CI du dernier envoi est **rouge sous Linux et Windows** (voir Constats, point 1) ; le correctif est local. `/livrer-git` devra pousser le correctif (aperçu et accord de l'auteur), relancer la CI des trois systèmes (`gh workflow run ci.yml --repo scarou/Scriibe`) et n'envoyer la release qu'avec trois systèmes verts. Les notes de version disent le statut d'essai par système : Mac complet ; Windows complet (touche de commande réglée à la main : le défaut « Windows droite » manque sur un portable) ; Linux Wayland et X11 : touche, recherche, rien collé OK, mais **commandes sur sélection « Sélection illisible »** (limite connue). Ligne « livraison » au journal du vault.
- **Pièges** : (1) SSH direct vers les PC : refusé par le contrôle automatique tant que l'auteur ne l'autorise pas dans la conversation (il l'a fait le 15/11 pour cette session) ; sinon passer par `pilote` ; (2) lancer Scriibe sur Linux par SSH demande `XAUTHORITY` (Wayland : fichier `.mutter-Xwaylandauth.*` lu dans la ligne de commande de `Xwayland` ; X11 : `/run/user/1000/gdm/Xauthority`) sinon plantage « Failed to initialize GTK » ; (3) les moteurs de Scriibe sont à la corbeille du PC Linux : `whisper-server` de `~/whisper.cpp` par le PATH de l'instance seulement ; (4) forme à dire dans les essais : l'infinitif « mettre entre guillemets » (en mémoire) ; (5) le défaut de la touche de commande Mac est maintenant Maj gauche seule (décision de l'auteur) ; Windows droite (Windows) et Ctrl gauche (Linux) restent provisoires ; (6) pas de CI qui tourne seule sur Windows et macOS : `workflow_dispatch` seulement.
- **Commits** : `bed2656` (essai Mac, résultats partiels), celui du correctif et des résultats complets, celui de ce relais ; `f8efed6` est le dernier poussé.

### Relecture avant la 0.6.0 (16/11/2026)

Relecture du code du lot 1 par un agent distinct (aucun bloquant, 2 importants, 7 mineurs) ; les deux importants sont corrigés avec l'accord de l'auteur :
1. **Windows : menu Démarrer masqué en permanence** par la touche de commande par défaut (`hook_rules` la mettait dans `mask_menu`). Maintenant `HookRules.command_menu` + `CommandMenuMask` (testé sur Mac : appui seul libre, commande donnée masquée, dictée tenue avant ou après, relâchée avant). Le code du crochet Windows (`windows/keyboard.rs`) n'est compilé que par le PC et la CI.
2. **Touche de commande affichée alors qu'elle n'existe pas** : `keys::command_label(settings)` (touche en vigueur ou `None`), page « aucune (à choisir) ».
Mineurs non corrigés : voir CHANGELOG 0.6.0 (« Connu, non corrigé »). Mesuré sur le Mac : 756 tests Rust, 109 tests de l'interface, `cargo fmt`, `clippy -D warnings`, `svelte-check` sans erreur.

## Étape 2 : extraits (tâche 5, 16/11/2026)

Code de l'étape 2 de `docs/CADRAGE_COMMANDES.md` (extraits vocaux, `Paster::paste_transient`, `snippets`), calqué sur l'étape 1b. **Non publié** (part dans la 0.7.0 avec l'étape 3). Ligne d'écart : `docs/ECARTS.md` (16/11/2026).

### Ce qui change pour la personne

- **Sous la touche de commande seulement**, une phrase qu'elle a choisie (« insérer mon adresse email ») colle un texte qu'elle a écrit (plusieurs lignes, 5 000 signes au plus). Carte « Commandes vocales » (onglet Dictionnaire), partie **« Mes extraits »** : un nom, une ou plusieurs phrases, le texte (modifiable sur place, enregistré en quittant le champ). La phrase doit être **toute** la phrase dite ; un mot seul suffit. Dite sans la touche, la phrase reste du texte collé (test de non-régression).
- L'extrait est collé **tel quel** : ni modèle, ni dictionnaire, ni typographie, ni espace ajoutée. Il n'entre **jamais** dans l'historique ni les statistiques ; le journal n'a que `extrait.<n>` et la longueur.
- Le presse-papiers d'avant est **toujours remis**, même si « Remettre le presse-papiers d'avant après le collage » est décoché ; le texte est marqué « ne pas garder » pour les historiques de presse-papiers.
- **Messages fixes de la pastille** (traduits dans `locales/en-server.ts`) : « Extrait impossible » (extrait disparu ou vide au moment de la phrase) ; « Extrait copié, à coller avec ⌘V — <raison> » (collage refusé, par exemple Accessibilité manquante : le texte reste dans le presse-papiers).
- **Export et import d'un utilisateur** : case « Extraits vocaux (texte compris) », **décochée à l'export** (un extrait peut être une adresse ou un IBAN, le fichier le contient en clair) **et à l'import** ; l'aperçu de l'import montre le **texte** de chaque extrait et ceux qui seront écartés ; identifiants toujours neufs ; 50 extraits au plus.
- Aide FR et EN (puce « Extraits »).

### Choix faits (avec la raison)

1. **Collage direct, sans `process`** : la branche `extrait.<n>` de `run_command` appelle `paste_transient` et rend `Handled`, comme `surround` (C7). Raison : un extrait passé par `process` entrerait dans `rawText`/`finalText` (historique) et serait « corrigé » par le dictionnaire et la typographie.
2. **`Paster::paste_transient(text)` avec une valeur par défaut** (`paste(text, true)`) plutôt qu'une méthode obligatoire : les systèmes réels la **redéfinissent tous** (Mac, Windows, Linux) ; la valeur par défaut ne sert qu'à `NoPaster` (système sans prise en charge). Les doublures : `FakePaster` l'enregistre à part (`pasted_transient`, jamais mêlé à `pasted`), `SwitchPaster` la neutralise pendant un essai (`Captured.snippet_chars`, **la longueur seulement** : le texte n'est jamais gardé), `ConsolePaster` n'affiche que la longueur.
3. **Marque « ne pas garder » sur Mac : les deux** (`ConcealedType` par `arboard`, `TransientType` par `objc2`, comme Scribe). Raison : le cadrage nomme `TransientType` ; `ConcealedType` est la marque plus stricte que respectent aussi les gestionnaires qui ignorent la première. **Vérifié sur le vrai presse-papiers** par un test `#[ignore]` (`cargo test -p scriibe-platform transient -- --ignored`) : les deux types y sont, le texte aussi.
4. **Jamais tapé** : sur Mac, un navigateur Chromium reçoit le menu Édition → Coller puis ⌘V (la frappe de `paste` est sautée pour un extrait : chaque saut de ligne tapé serait une touche Entrée).
5. **Remise du presse-papiers d'avant** : toujours, 1,5 s après, si le presse-papiers contient encore l'extrait ; s'il était vide ou sans texte, il est **vidé** (l'extrait ne reste pas dedans). Si le collage échoue, rien n'est remis ni vidé : le texte reste pour que la personne le colle elle-même.
6. **Phrases** : valables pour toutes les langues (c'est la personne qui les écrit), **une phrase d'un mot permise** (portée « sous la touche seulement » : la phrase doit être toute la transcription), refusées si elles sont celles d'une commande du programme (toutes portées et langues confondues) ou d'un autre extrait, dès la lecture des réglages ; `validate_phrase` contrôle en plus les commandes de la personne et les sites (`extrait.nouveau` pour un extrait pas encore créé, un mot seul permis).
7. **Texte nettoyé à la lecture** (`snippet::clean_text`) : fins de ligne ramenées à `\n`, caractères de contrôle retirés (sauf `\n` et tabulation), blancs de bord retirés ; vide ou plus de 5 000 signes : extrait écarté seul. Raison : un texte avec un `\r` ou un contrôle collé tel quel poserait des problèmes dans une appli ; la page montre le texte nettoyé.
8. **`Debug` de `Snippet` masque le texte, le nom et le nombre de phrases** : un `{:?}` perdu sur les réglages ne fuit rien.
9. **Classement** : `snippets` est un champ **utilisateur** (`USER_KEYS`), dans l'export d'un utilisateur (liste blanche), décoché à l'export **et** à l'import ; toujours écrit (liste vide), donc pas de changement à `every_key_written`. Le test du chantier 4 passe : le champ et son classement sont dans le même commit.
10. **L'aperçu de l'export ne dit que les noms** ; celui de l'import montre le texte (la personne doit pouvoir lire ce qui sera collé sur une phrase qu'elle dira, C11).
11. **Plafonds** : 50 extraits, 5 000 signes, nom 40 caractères, 20 phrases de 200 caractères au plus (mêmes limites que les commandes).

### Où est le code

- `crates/core/src/snippet.rs` (nouveau) : `Snippet`, `cleaned`, `clean_text`, `defs`, `text_of`, `is_snippet`, `next_id`. `commands.rs` : `all_defs`, `validate_phrase` (`extrait.nouveau`). `settings.rs` : `snippets`, `USER_KEYS`, lecture tolérante. `profile_file.rs` : `Categories.snippets`, `Contents`, `Incoming`, `SnippetPreview`, `Outcome`, `read_snippets`, `numbered_snippets`.
- `crates/platform/src/paste.rs` : `Paster::paste_transient`. `macos/paste.rs`, `windows/paste.rs`, `linux/paste.rs` : `paste_with(text, restore, transient)`. `testing.rs` : `FakePaster`.
- `crates/pipeline/src/controller.rs` : branche `extrait.<n>` de `run_command`.
- `crates/cli/src/web/` : `sinks.rs` (neutralisation), `mod.rs` (`api/snippets/check`, `validate-phrase`), `user_file_io.rs`. `crates/cli/src/commands.rs` : `ConsolePaster`.
- Interface : `VoiceCommands.svelte` (« Mes extraits »), `UserExport.svelte`, `UserImport.svelte`, `Help.svelte`, `HelpEn.svelte`, `api.ts`, `locales/`.

### Tests (mesurés sur le Mac)

- **767 tests Rust** sans erreur (2 ignorés : celui d'avant et le test de marque du presse-papiers, lancé à la main : ok), **109 tests de l'interface** ; `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` et `svelte-check` sans erreur.
- Nouveaux : `commands_521` (phrase d'extrait = commande sous la touche seulement, un mot seul, jamais sans la touche), `522` (nettoyage à la lecture : préfixe, doublon, texte vide ou trop long, phrases prises, 50 au plus, `Debug` masqué), `523` (identifiants, prochain numéro, texte par identifiant), `524` (une phrase = une seule commande, `extrait.nouveau`) ; 3 tests du contrôleur (**collé tel quel par `paste_transient` et jamais par `paste`**, rien dans l'historique, ni modèle, ni lecture de la sélection, **aucune trace** du nom, des phrases ni du texte dans le journal, la pastille, l'historique ; **sans la touche, collé comme du texte** ; extrait disparu et collage impossible) ; 3 tests d'export et d'import (décoché des deux côtés, sans identifiant, aperçu avec le texte, identifiants neufs, extraits refusés écartés, limites du fichier sans citer le texte) ; la neutralisation de `sinks.rs` (longueur seulement) ; la route `validate-phrase` (un mot, phrase prise dans les deux langues).

### Non vérifié

- **Aucun extrait n'a été collé pour de vrai** dans une appli (les tests n'utilisent que des doublures) ; seule la **marque du presse-papiers sur Mac** a été vérifiée sur le vrai. À voir à l'essai du lot 2 (tâche 7) : Mac (Chrome et une appli native), Windows, Linux.
- **Code Windows** : `scriibe-platform` ne se compile pas sur le Mac (pas de compilateur C croisé pour `ring`). `windows/paste.rs` a été **vérifié à part** (copie dans une caisse isolée contre `windows-sys 0.59`, `cargo check --target x86_64-pc-windows-gnu`, sans erreur ni avertissement) ; le reste passera par la CI Windows après accord pour pousser. **Code Linux** : non compilé (cible absente) ; l'appel `Set::exclude_from_history` d'`arboard` 3.7 a été vérifié dans ses sources.
- Les formats d'exclusion Windows (`ExcludeClipboardContentFromMonitorProcessing`…) sont ceux du collage existant : un extrait n'apparaît pas dans Win+V, à confirmer à la main.
- La page (carte « Mes extraits », cases d'export et d'import, aperçu du texte importé) n'a pas été vue dans un navigateur : `svelte-check`, tests et compilation seulement.
- Limite connue : un presse-papiers qui contenait une image ou des fichiers n'est pas remis (seul le texte l'est).

### Suite

Tâche 6 (étape 3, applications et mail) : faite, voir § Étape 3 ci-dessous.

### Relais (fin de la tâche 5, contexte à 30 %)

Relais décidé par la règle du `CLAUDE.md` (contexte à 25 % ou plus en fin de tâche : on ne commence pas la tâche suivante dans cette session). Ne change ni l'ordre ni les consignes.

- **Fait** : toute la tâche 5 (étape 2, extraits), résultats ci-dessus ; commit local `79d2610` puis celui de ce relais. Rien n'est poussé (le dernier envoi sur GitHub est la 0.6.0, `1f90af2`).
- **En cours** : rien (arbre propre après le commit de ce relais).
- **Prochain pas** : la tâche 6 du découpage (étape 3 : `voiceApps`, `OptionalRest`, `percent_encode` public, « nouveau mail … »), consigne telle quelle. Points d'accroche déjà en place : `open::open_address` accepte déjà `mailto:` (`web_search::is_openable`) mais son commentaire dit « à resserrer avant "nouveau mail" » (CHANGELOG 0.6.0, « Connu, non corrigé ») ; `web_search::percent_encode` est dans le cœur ; `Argument` n'a que `None` et `Rest` (ajouter `OptionalRest`) ; `Paster::open_address` ne lance que des adresses : une application de la liste demande une autre fonction d'ouverture contrôlée (identifiant de la liste, `gtk-launch` sous Linux). `voiceApps` se classe en **`COMPUTER_KEYS`** (champ d'ordinateur : jamais exporté ; à ajouter aussi à `backup.rs:COMPUTER_FIELDS`) ; le test du chantier 4 échoue sinon.
- **Pièges** : (1) `paste_transient` existe maintenant ; un nouveau `Paster` (doublure ou système) peut s'en passer, la valeur par défaut colle avec remise ; (2) les tests du contrôleur d'extraits sont à la fin de `crates/pipeline/tests/dictation.rs` (modèle : `search_rig`, `rig_with`, `dictate_under_key`) ; (3) `ui/src/lib/i18n.test.ts` échoue pour une traduction qui n'est utilisée que dans une valeur de repli dynamique (`t(x.error ?? '…')`) : la mettre dans `en-server.ts` ; (4) le Mac n'a pas de compilateur C croisé : le code Windows se vérifie par une caisse isolée (copie du fichier contre `windows-sys`, `cargo check --target x86_64-pc-windows-gnu`, dossier de travail temporaire) ; (5) `cargo fmt --all` avant clippy ; (6) la page (carte « Mes extraits ») n'a pas été vue dans un navigateur : la voir à l'essai du lot 2 (tâche 7).
- **Commits** : `79d2610` (tâche 5), celui de ce relais ; `1f90af2` est le dernier poussé.

## Étape 3 : applications et nouveau mail (tâche 6, 16/11/2026)

Code de l'étape 3 de `docs/CADRAGE_COMMANDES.md` (`voiceApps` avec liste autorisée et contrôle au lancement, `Argument::OptionalRest`, « nouveau mail … » avec objet dicté en option). **Non publié** (part dans la 0.7.0 avec l'étape 2). Ligne d'écart : `docs/ECARTS.md` (16/11/2026).

### Ce qui change pour la personne

- **Applications**, sous la touche de commande seulement : carte « Commandes vocales », partie **« Mes applications »** : on choisit une application dans la liste de celles qui sont installées sur cet ordinateur, puis une phrase (« ouvre le navigateur » ; un mot seul suffit ; elle doit être **toute** la phrase dite). Seules les applications de la liste s'ouvrent, rien d'autre : ni commande, ni script, ni option. Pastille fixe « Ouverture de l'application ». Dite sans la touche, la phrase reste du texte (test de non-régression). Une application désinstallée depuis le choix est signalée dans la page et donne « Application introuvable » à la voix.
- **Nouveau mail**, sous la touche seulement : « nouveau mail » (ou « nouveau courriel », « nouvel email », « new email »…) ouvre un message vierge dans la messagerie par défaut ; « nouveau mail réunion de lundi » préremplit l'objet (un « objet » ou « sujet » en tête est ôté ; 200 signes, une seule ligne). Rien n'est envoyé ; **pas de destinataire** : « nouveau mail à Marie » met « à Marie » en objet (dit dans l'Aide). Les phrases se changent dans la partie « Nouveau mail » de la carte, comme les autres.
- **Messages fixes de la pastille** (traduits dans `locales/en-server.ts`) : « Ouverture de l'application », « Application introuvable » (hors de la liste fraîche, ou disparue des réglages), « Application non ouverte » (le système n'a pas lancé), « Mail non ouvert ».
- **Confidentialité** : rien dans l'historique ni les statistiques ; le journal n'a que `appli.<n>` et `mail.nouveau (objet : <n> signes)` ; ni le nom ou l'identifiant d'une application, ni ses phrases, ni l'objet n'apparaissent dans le journal, la pastille ou l'historique (tests « SECRET »).
- **`voiceApps` est un champ d'ordinateur** : jamais dans l'export d'un utilisateur, et une sauvegarde venue d'un autre système ne remplace pas la liste de cet ordinateur (`backup.rs:COMPUTER_FIELDS`). Les identifiants ne valent que sur le système où ils ont été choisis.
- Aide FR et EN (puces « Applications » et « Nouveau mail »).

### Choix faits (avec la raison)

1. **Le réglage ne garde qu'un identifiant ; ce qui est lancé vient de la liste fraîche.** `installed_apps::launch_app(id)` **réénumère** les applications installées à chaque lancement, retrouve `id` dans l'énumération et lance **l'entrée de la liste** (son identifiant de bundle, son nom de `.desktop`, le chemin du `.lnk` qu'elle a elle-même trouvé), jamais une chaîne des réglages. Raison : un `voiceApps` écrit à la main (identifiant piégé `-a Calculator`, chemin, application disparue) ne permet rien de plus que ce que l'ordinateur propose déjà ; le test `commands_555` le prouve avec des chaînes hostiles et le test du contrôleur avec un réglage piégé.
2. **Identifiants par système** : Mac = identifiant de bundle lu dans `Contents/Info.plist` (XML lu directement, `plutil` seulement pour le rare format binaire) des dossiers `/Applications`, `/System/Applications`, `Utilities` et `~/Applications`, à un niveau de sous-dossier près ; lancement `open -b <id>`, jamais un chemin. Linux = nom du fichier `.desktop` (sans l'extension) des dossiers système, utilisateur, Flatpak et Snap, applications visibles seulement (`Type=Application`, ni `NoDisplay` ni `Hidden`) ; conforme à `^[A-Za-z0-9._-]+$`, sans tiret ni point initial ; lancement `gtk-launch <id>`. Windows = raccourcis `.lnk` des **seuls** dossiers du menu Démarrer (commun et utilisateur), trois niveaux de sous-dossier, liens symboliques et désinstallateurs ignorés ; identifiant `commun/…` ou `utilisateur/…` suivi du chemin relatif ; lancement par `ShellExecuteW` (la fonction d'`open.rs` devient `shell_execute`, partagée avec les adresses).
3. **Windows : pas d'application du Store.** Le cadrage dit « AUMID présent dans la liste, **ou** `.lnk` … » : seul le `.lnk` est fait. Raison : l'énumération des AUMID passe par PowerShell (`Get-StartApps`) ou COM, du code Windows que je ne peux ni compiler ni essayer sur le Mac ; les applications classiques et celles du Store qui ont un raccourci sont couvertes. À reprendre si l'essai (tâche 7) montre un manque.
4. **`Paster::open_app(app_id)`** (défaut : la vraie fonction), séparée d'`open_address` : `SwitchPaster` la neutralise pendant un essai (`Captured.app_opened`, **le fait seulement**), `FakePaster` la contrôle contre une liste « installée » (`set_installed_apps`) comme la vraie, `ConsolePaster` n'affiche aucun identifiant, `NoPaster` la refuse.
5. **`percent_encode`** : le cadrage dit « rendu public dans `engines` » ; je réutilise celui du **cœur** (`web_search::percent_encode`, déjà public depuis l'étape 1b). Raison : l'adresse `mailto:` se construit dans le cœur, qui ne dépend pas d'`engines` ; celui d'`engines` (Deepgram, privé) n'est pas touché. Même résultat (RFC 3986 octet par octet).
6. **`mailto:` resserré** (le « à resserrer avant "nouveau mail" » de la 0.6.0) : `is_openable` n'accepte plus que `mailto:` seul et `mailto:?subject=<lettres, chiffres, % . _ ~ ->` : jamais de destinataire ni de corps. `mailto:` seul est désormais permis (message vierge) ; `mailto:a@b.fr`, `mailto:?body=…`, `?subject=a&to=…` sont refusés avant tout lancement.
7. **Mail : une fiche `mail.nouveau`, `OptionalRest`.** La phrase seule suffit, un reste est repris s'il existe (un reste vide donne `None`). Deux fiches (avec et sans objet) se seraient déclarées prises l'une l'autre. L'objet : blancs et contrôles ramenés à une espace, mot « objet »/« sujet »/« subject » ôté en tête (mot entier : « objets perdus » reste), ponctuation de bord retirée, 200 signes, encodé : un `&to=` ou `&body=` dit reste du texte. Phrases livrées : « nouveau mail », « nouveau courriel », « nouvel email », « nouveau email », « nouveau e-mail », « new email », « new mail », « new e-mail » (variantes de Whisper).
8. **Phrases** : comme les extraits, valables pour toutes les langues, un mot seul permis, refusées si prises par une commande du programme (nouvelle fonction `commands::program_phrases`, qui regroupe commandes d'avant, sites livrés et mail ; `snippet.rs` s'en sert aussi) ou par une autre application, dès la lecture ; `validate_phrase` contrôle les commandes de la personne et les sites (`appli.nouveau` pour une application pas encore créée).
9. **`Debug` de `VoiceApp` et d'`InstalledApp`** masque le nom, l'identifiant système et les phrases.
10. **Pastille de réussite** : « Ouverture de l'application » (cadrage) pour une application ; la coche habituelle pour le mail.
11. **Plafonds** : 30 applications, nom 100 caractères, identifiant 300 caractères sans contrôle ni blanc de bord (contrôle de forme ; ce qui est lancé vient de la liste), 20 phrases de 200 caractères au plus.
12. **Page** : la liste des applications installées se charge à l'ouverture (`api/installed-apps`, énumération fraîche, environ 30 ms sur le Mac : 101 applications) ; une application enregistrée mais absente de la liste est signalée.

### Où est le code

- `crates/core/src/mail.rs` (nouveau) : `defs`, `phrases`, `is_mail`, `clean_subject`, `url`. `voice_app.rs` (nouveau) : `VoiceApp`, `is_app`, `app_id_of`, `defs`, `cleaned`, `next_id`. `commands.rs` : `Argument::OptionalRest`, `all_defs`, `program_phrases`, `validate_phrase` (`appli.nouveau`). `settings.rs` : `voice_apps`, `COMPUTER_KEYS`, lecture tolérante. `backup.rs` : `COMPUTER_FIELDS`. `web_search.rs` : `is_openable` resserrée.
- `crates/platform/src/installed_apps.rs` (nouveau) : `InstalledApp`, `installed_apps`, `launch_app`, parties pures par système (`scan_bundles`, `scan_desktop_files`, `scan_start_menu`). `open.rs` : `shell_execute`. `paste.rs` : `Paster::open_app`. `testing.rs`, `unsupported.rs`.
- `crates/pipeline/src/controller.rs` : branches `mail.nouveau` et `appli.<n>` de `run_command`.
- `crates/cli/src/web/` : `sinks.rs` (neutralisation), `mod.rs` (`api/installed-apps`, liste des commandes, `validate-phrase`). `crates/cli/src/commands.rs` : `ConsolePaster`.
- Interface : `VoiceCommands.svelte` (groupe « Nouveau mail », partie « Mes applications »), `Help.svelte`, `HelpEn.svelte`, `api.ts`, `locales/`.

### Tests (mesurés sur le Mac)

- **786 tests Rust** sans erreur (3 ignorés : les deux d'avant et `commands_556`, lancé à la main : ok, 101 applications énumérées) ; **109 tests de l'interface** ; `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` et `svelte-check` sans erreur. 19 tests Rust de plus que les 767 d'avant.
- Nouveaux : `commands_531` (objet nettoyé), `532` (objet **encodé** : `&to=` et `&body=` restent du texte, adresses ouvrables), `533` (« nouveau mail » avec ou sans objet ; sans la touche : du texte), `541` (phrase d'application sous la touche seulement), `542` (lecture : identifiant illisible, phrases prises, 30 au plus, `Debug` masqué), `543` (identifiants), `544` (une phrase = une commande, `appli.nouveau`, phrases du mail prises), `545` (`voiceApps` **d'ordinateur** : rangé côté ordinateur, pas dans le fichier de l'utilisateur, non restauré depuis un autre système), `551` (identifiants simples seulement), `552` (bundles), `553` (`.desktop`), `554` (menu Démarrer), `555` (**application hors de la liste fraîche jamais lancée**, chaînes hostiles) ; `open_address` refuse tout `mailto:` autre que les deux formes ; 5 tests du contrôleur (**objet encodé et rien dans le journal, la pastille ni l'historique** ; messagerie non ouverte ; **sans la touche, du texte** ; application de la liste ouverte sans trace ; **application hors liste refusée** avec réglage piégé, désinstallée ou liste vide ; système qui ne lance pas) ; la neutralisation de `sinks.rs` (`app_opened`) ; la route `validate-phrase` (application, phrases du mail).

### Non vérifié

- **Aucune application ni messagerie n'a été ouverte pour de vrai** : les tests n'utilisent que des doublures ; seule l'énumération réelle des applications du Mac a été essayée (`commands_556`). `open -b`, `gtk-launch`, `ShellExecuteW` sur un `.lnk` et `mailto:` seul (message vierge) sont à voir à l'essai du lot 2 (tâche 7), sur les trois machines.
- **Code Windows** : `scriibe-platform` ne se compile pas sur le Mac. `installed_apps.rs` a été **vérifié à part** (copie dans une caisse isolée avec un faux `open`, `cargo check --target x86_64-pc-windows-gnu`, `-D warnings`, sans erreur) ; `open.rs` (`shell_execute`, déplacée de `launch_address` sans changement de corps) passera par la CI Windows après accord pour pousser. **Code Linux** : cible absente, non compilé ; les parties pures sont testées sur le Mac.
- La page (partie « Mes applications », liste, groupe « Nouveau mail ») n'a pas été vue dans un navigateur : `svelte-check`, tests et compilation seulement.
- Limites connues : sous Windows, pas d'application du Store sans raccourci ; sous Mac, les applications hors des dossiers listés ne sont pas proposées ; une application qui disparaît entre le choix et la phrase donne « Application introuvable ».

### Suite

Tâche 7 (essai du lot 2 : extraits, applications, mail sur le Mac et les deux PC) : nouvelle session, `⚙ Sonnet · Élevé`, consigne dans le découpage ; puis la 0.7.0 (tâche 8).

### Relais (fin de la tâche 6, contexte à 27 %)

Relais décidé par la règle du `CLAUDE.md` (contexte à 25 % ou plus en fin de tâche : on ne commence pas la suivante dans cette session). Ne change ni l'ordre ni les consignes.

- **Fait** : toute la tâche 6 (étape 3), résultats ci-dessus ; commit local `ef2c3ef` puis celui de ce relais. Rien n'est poussé (le dernier envoi sur GitHub est la 0.6.0, `1f90af2`) ; trois commits locaux de C6 en attente (`79d2610`, `ef2c3ef`, relais).
- **En cours** : rien (arbre propre après le commit de ce relais).
- **Prochain pas** : la tâche 7 du découpage (essai du lot 2 sur le Mac et les deux PC : extraits, applications, mail), consigne telle quelle ; puis la tâche 8 (0.7.0, aperçu et accord de l'auteur avant tout push ou Release).
- **Pièges** : (1) l'essai doit **ouvrir pour de vrai** une application, `mailto:` seul (message vierge : jamais vu à l'écran) et coller un extrait ; `open -b`, `gtk-launch` et `ShellExecuteW` sur `.lnk` n'ont tourné nulle part ; (2) Windows : seuls les raccourcis du menu Démarrer sont listés (pas les applications du Store sans raccourci) : si un essai en réclame, voir le choix 3 de § Étape 3 ; (3) le code Windows (`open.rs`, `installed_apps.rs`) n'a été vérifié qu'à part, la CI Windows le verra au push ; (4) la page (« Mes applications », groupe « Nouveau mail ») n'a jamais été vue dans un navigateur ; (5) `voiceApps` est d'ordinateur : une application choisie sur le Mac n'existe pas sur les PC, il faut la choisir sur chaque machine ; (6) le Mac n'a pas de cible Linux : le code `gtk-launch` ne se compile qu'en CI.
- **Commits** : `79d2610`, `ef2c3ef`, celui de ce relais ; `1f90af2` est le dernier poussé.

## Essai lot 2 (tâche 7, 16/11/2026)

Essai réel du lot 2 (étapes 2 et 3 : extraits, applications, nouveau mail) sur le Mac, le PC Windows et le PC Linux (GNOME Wayland), l'auteur seul aux gestes, une étape à la fois. Code essayé : `ecb8bf1` (Mac, Windows) puis `ae964e0` (Linux, avec le correctif des Ajouts). Chaque machine : copie d'essai de Scriibe sur une **copie** des réglages (`SCRIIBE_DATA`), avec un extrait « texte d'essai », une application de la liste (phrase « ouvre … ») et une application **absente de la liste** (identifiant d'une application non installée, « ouvre l'absente ») ajoutés par l'interface locale (`POST api/settings`) ; instance habituelle arrêtée puis relancée à la fin.

### Résultats

| Étape | Mac | Windows | Linux (GNOME Wayland) |
|---|---|---|---|
| Extrait collé sous la touche, presse-papiers restauré | oui (`extrait.1`, 45 signes ; presse-papiers relu : intact) | oui (id. ; presse-papiers relu par une tâche dans la session : intact) | oui (id. ; presse-papiers vérifié par un Ctrl+V de l'auteur : intact) |
| Application de la liste ouverte | oui, Obsidian (`open -b`) ; pastille « Ouverture de l'application » | oui, Explorateur de fichiers (`ShellExecuteW` sur un `.lnk` du menu Démarrer) ; pastille idem | oui, Calculatrice (`gtk-launch`) |
| Application hors liste refusée | oui, « Application introuvable », rien ouvert, journal sans le nom | oui (id.) | oui (id.) |
| « nouveau mail réunion de lundi » | oui : Mail s'ouvre, objet « Réunion de lundi », sans destinataire | **non essayable** : aucune messagerie sur ce PC, pastille « Mail non ouvert » (journal `objet : 16 signes`, puis `messagerie non ouverte`) | **non essayable** : `xdg-open` n'ouvre aucune messagerie ; après correction, « Mail non ouvert » (voir Ajouts) |
| « nouveau mail » seul (message vierge, `mailto:`) | oui : message entièrement vierge | non essayé (pas de messagerie) | non essayé (idem) |

- **Journaux** (lus par la session sur chaque machine) : `commande: extrait.1 (extrait : 45 signes)`, `commande: appli.<n>`, `commande: appli.<n> — application non ouverte (application introuvable)`, `commande: mail.nouveau (objet : 16 signes)` ; **ni le texte de l'extrait, ni le nom d'une application, ni l'objet, ni les mots dits** n'y figurent ; le texte de l'extrait n'est pas dans l'historique de l'instance d'essai (recherche à vide sur le Mac).
- **Mac** : le premier lancement de la copie a fait ouvrir une fenêtre du trousseau (nouvelle compilation ; « Toujours autoriser » cliqué par l'auteur), le temps d'une page « Chargement… ». La migration en utilisateurs (0.5.0) s'est faite sur la copie au premier lancement. La page « Commandes vocales » a été **vue pour la première fois** dans un navigateur : parties « Mes extraits », « Mes applications » (liste de 100 applications installées) et « Nouveau mail » affichées. Mail a demandé de configurer une boîte avant de s'ouvrir (premier usage de l'auteur, pas un défaut).
- **Windows** : touche de commande de la copie réglée sur **Maj gauche** (traduction Maj droit), comme au lot 1 (pas de touche Windows droite sur le portable). La liste du menu Démarrer propose 140 raccourcis. Pas de messagerie par défaut (le choix de la personne pointe sur un paquet de Store absent : `Get-AppxPackage` ne trouve ni « Mail et Calendrier » ni Outlook).
- **Linux** : touches sous Wayland = celles de GNOME (dictée Ctrl + ←, traduction Ctrl + ↑, commande Ctrl + ↓). Une première instance lancée par SSH sans `XDG_SESSION_TYPE=wayland` est partie en mode X11 sans aucun raccourci (**erreur de lancement de la session, pas du code** ; piège écrit dans `docs/PIEGES.md`). 82 applications proposées.
- **Whisper** : « texte d'essai » est souvent mal entendu (« phrase proche d'une commande », ou collé comme texte quand la touche de commande n'est pas tenue) ; 2 à 3 essais par phrase sur les trois machines. Environnement (petits modèles sur les PC), pas le code ; une phrase d'extrait sans apostrophe est plus sûre.
- **Pas d'application du Store sous Windows** : non essayé (limite connue, choix 3 de § Étape 3) ; aucun manque constaté pour l'essai (Explorateur, applications classiques).

### Ajouts

1. **16/11, Linux, nouveau mail** — la pastille a affiché la **coche (« collé »)** alors que rien ne s'ouvrait : `xdg-open mailto:` sort en code 4 (« L'emplacement indiqué n'est pas pris en charge »), et Scriibe ne lisait pas le code de sortie (`spawn()` seul). **Défaut réel, traité tout de suite** : `open::run_launcher` (module `open.rs`) lance le programme sans sortie, attend jusqu'à 2 s son code de sortie (non nul ou programme absent : « Mail non ouvert » / « Application introuvable »/« Application non ouverte » ; encore en route après 2 s : parti, on ne bloque pas), pour les adresses (`open`, `xdg-open`) et les applications (`open -b`, `gtk-launch`). Test `a_launcher_that_fails_is_reported_as_not_opened` (`true`, `false`, `sh -c "exit 4"`, programme absent, `sleep` laissé partir). **787 tests Rust** sans erreur (3 ignorés), `fmt` et `clippy -D warnings` propres. Commit `ae964e0`, poussé à la demande de l'auteur ; **vérifié sur le PC Linux** : « Mail non ouvert », journal `messagerie non ouverte (ouverture impossible)`. Windows : `ShellExecuteW` jugé déjà sur son code de retour, inchangé.
2. **16/11, Windows, nouveau mail** — échec constaté à l'écran et au journal (`ouverture impossible`) alors qu'un petit programme de test appelant le même `ShellExecuteW("open", "mailto:")` rend **42** (« parti » pour Windows) dans la même session, avec ou sans COM initialisé. **Non expliqué** : l'échec de Scriibe est dans le sens sûr (aucune fausse réussite), mais la différence n'est pas comprise. À reprendre si un PC avec messagerie la montre ; non bloquant.
3. **Consigne mal dite à Windows** — une dictée « nouveau mail » sans la suite (erreur de consigne de la session, pas du code) a donné `mail.nouveau` sans longueur d'objet, ce qui est le bon comportement.

### Constats

- Le lot 2 marche sur les trois systèmes pour les extraits et les applications, y compris les chemins jamais lancés avant (`open -b`, `ShellExecuteW` sur `.lnk`, `gtk-launch`, `mailto:` seul sur le Mac).
- **Reste non vu à l'écran** : l'ouverture d'une messagerie avec objet sous Windows et sous Linux (aucune messagerie installée sur ces deux PC) ; l'objet préremplit sur le Mac seulement.
- CI (`workflow_dispatch`, `ecb8bf1` : run `43868472541`) : Windows, macOS, Linux et le zip Windows **verts**. CI du correctif `ae964e0` : run `43889657358` (Linux et Windows verts, macOS à confirmer avant la 0.7.0).
- Machines remises en état : Mac sans instance (copie d'essai arrêtée ; la copie de données reste dans le dossier temporaire de la session) ; Windows : Scriibe habituel relancé (`C:\scriibe\scriibe.exe`), tâches planifiées temporaires supprimées, copie de données dans `C:\essai-c7` ; Linux : Scriibe habituel relancé sur les vraies données (`~/Scriibe/target/release/scriibe lancer`), copie dans `/tmp/essai-c7`.

### Suite

Tâche 8 : `/livrer-git` en mode version 0.7.0 (statut d'essai par système ci-dessus ; aperçu et accord de l'auteur avant la Release).

### Relais (fin de la tâche 7, contexte à 27 %)

Relais décidé par la règle du `CLAUDE.md` (contexte à 25 % ou plus en fin de tâche : on ne commence pas la suivante dans cette session). Ne change ni l'ordre ni les consignes.

- **Fait** : toute la tâche 7 (essai du lot 2 sur Mac, Windows, Linux), résultats ci-dessus ; correctif des lanceurs (`ae964e0`, **poussé** à la demande de l'auteur, avec les 5 commits du lot 2 : `main` = `ae964e0`) ; commit local `870b3e5` (résultats, pièges) et celui de ce relais, **non poussés**.
- **En cours** : rien (arbre propre après le commit de ce relais). Machines : Scriibe habituel relancé sur Windows et Linux, aucune instance sur le Mac ; PC Windows et Linux à `ae964e0` (Linux recompilé en release à ce commit ; Windows compilé à `ecb8bf1`, sans le correctif des lanceurs, sans effet sur Windows).
- **Prochain pas** : la tâche 8 du découpage (`/livrer-git` en mode version **0.7.0**), consigne telle quelle : aperçu et accord de l'auteur avant tout push ou Release ; CI verte sur les trois systèmes **au commit à publier** (celle de `ae964e0` : run `43889657358`, Linux et Windows verts, **macOS restait « en cours » à la clôture** : le relire, relancer au besoin avec `gh workflow run ci.yml --repo scarou/Scriibe`) ; notes de version avec le statut d'essai par système (voir § Essai lot 2 : extraits et applications vus sur les trois ; mail avec objet vu sur le Mac seulement, Windows et Linux sans messagerie : « Mail non ouvert » vérifié) ; ligne « livraison » au journal du vault. Ajouter au CHANGELOG 0.7.0 le correctif des lanceurs et le point « Connu, non vérifié » (ouverture d'une messagerie avec objet sous Windows et Linux ; écart `ShellExecuteW` 42 / échec, Ajouts 2).
- **Pièges** : (1) Linux sous Wayland lancé par SSH : exporter `XDG_SESSION_TYPE=wayland` (et les autres variables de `docs/PIEGES.md`) sinon aucun raccourci GNOME ; touches = Ctrl + ← (dictée), Ctrl + ↑ (traduction), Ctrl + ↓ (commande) ; (2) SSH direct vers les PC : l'auteur l'a autorisé dans cette session seulement ; à redemander ailleurs ; (3) le shell distant de Windows est `cmd` : écrire un `.ps1`, le copier (`scp`) et le lancer, ne pas imbriquer de guillemets ; une tâche planifiée `/it` donne la session de l'auteur ; (4) `pilote verifier` se délègue à un sous-agent ; (5) la copie de données du Mac et le dossier de la session d'essai sont dans un dossier temporaire (les moteurs, les modèles sont liés, pas copiés) ; (6) `AskUserQuestion` : pas de barre oblique inverse dans les options (le JSON casse).
- **Commits** : `79d2610`, `ef2c3ef`, `ae964e0`, `870b3e5`, celui de ce relais ; `ae964e0` est le dernier poussé.

## Livraison 0.7.0 (tâche 8, 16/11/2026)

- **Publié** : Release `v0.7.0` au commit `a745fc2` (`main` poussé jusqu'à lui ; 3 commits depuis `ae964e0` : résultats, relais, version). CI complète (lancement manuel, run `43892448599`) verte sur Linux, Windows, macOS et le zip `scriibe.exe` ; zips Windows et Linux ajoutés par les workflows de Release.
- **Relecture par un agent distinct** (code Rust du lot, lecture seule) : aucun bloquant, aucun défaut de sécurité ; 6 mineurs. **Corrigé** : sous Windows, une dictée ordinaire vidait un presse-papiers qui était vide (régression du lot ; `paste_with(…, clear_if_empty)`, `crates/platform/src/windows/paste.rs`, écart noté dans `docs/ECARTS.md`), compilé par la CI seulement. **Non corrigés** (CHANGELOG 0.7.0) : zombie d'un lanceur lent, doublon d'identifiant `.desktop`, latence d'énumération, phrase commençant par « nouveau mail » plus forte que la commande, boîte « Choisir une application » de Windows.
- **Piège** : `gh release create` (via `tools/publier_macos.sh`) a échoué une première fois (« connection reset by peer ») en laissant un **brouillon** de Release sans fichier ; le script refuse alors de relancer (« existe déjà »). Supprimer le brouillon (`gh release delete v0.7.0 --yes`, aucune étiquette créée) puis relancer ; un 2e essai a aussi échoué (« ReleaseAsset.name already exists », brouillon nettoyé par `gh`), le 3e a réussi. Le script reconstruit et signe l'application à chaque essai.
- **Mesuré** sur le Mac : 787 tests Rust (3 ignorés), 109 tests de l'interface, `fmt`, `clippy -D warnings`, `svelte-check` sans erreur.

### Relais (fin de la tâche 8, contexte à 15 %)

- **Fait** : toute la tâche 8 (version 0.7.0 publiée, journal du vault `IA-OS/log.md` à jour).
- **Prochain pas** : la tâche 9 du découpage (étape 4, reformulation, `⚙ Opus · Élevé`, nouvelle session « C6 · F · tâches 9 et 10 · reformulation »), consigne telle quelle ; commit local, pas de push. Le lot 3 partira en 0.8.0.
- **Pièges** : voir ci-dessus (Release en brouillon) et § Relais de la tâche 7 (Linux sous Wayland par SSH : `XDG_SESSION_TYPE=wayland` ; SSH vers les PC à redemander ; shell distant de Windows = `cmd`).

## Étape 4 : reformulation de la sélection (tâche 9, 16/11/2026)

Code de l'étape 4 de `docs/CADRAGE_COMMANDES.md` (décision 2 et décision 5). **Non publié** (part dans la 0.8.0 après la relecture, tâche 10, et l'essai, tâche 11). Ligne d'écart : `docs/ECARTS.md` (16/11/2026, « Reformulation de la sélection »).

### Ce qui change pour la personne

- **Sous la touche de commande, avec une sélection**, une phrase de 25 mots au plus qui n'est ni une commande ni proche d'une commande devient la **consigne** : « cherche les fautes », « rends ça plus amical », « raccourcis », « traduis en anglais ». Le texte reformulé **remplace la sélection** (collé par `paste_transient`, jamais tapé : un seul ⌘Z ou Ctrl+Z rend l'original ; presse-papiers d'avant remis). Sans sélection : « Commande inconnue » (comme avant).
- **La sélection ne part que vers un moteur sur cet ordinateur** : llama.cpp, ou LM Studio et Ollama dont l'adresse a pour hôte `127.0.0.1`, `localhost` ou `[::1]`. Pour OpenRouter ou un serveur sur une autre machine, une case **« Autoriser l'envoi de la sélection à ce moteur »** apparaît dans Réglages → Moteurs (décochée par défaut) ; l'accord vaut pour ce moteur, ce modèle et cette adresse et tombe dès que l'un des trois change.
- **Messages fixes de la pastille** (jamais la sélection ni la consigne), traduits dans `en-server.ts` : « Non disponible sous Linux », « Reformulation impossible : moteur hors de cet ordinateur, sans accord (Réglages → Moteurs) », « Sélection trop longue pour la reformulation (3 000 signes au plus) », « Rien à changer », « Reformulation impossible : réponse du modèle écartée », « Reformulation impossible — <raison fixe du moteur> », « Reformulation copiée : l'application a changé, à coller avec ⌘V / Ctrl+V », « Reformulation copiée, à coller avec … — <raison> ».
- **Changement d'application pendant l'appel** : le texte n'est pas collé dans la nouvelle application, il est seulement copié.
- **Linux** : « Non disponible sous Linux », sans lire la sélection ni rien envoyer.
- Aide FR et EN (« Reformuler la sélection », onglet Dictionnaire → Commandes vocales) ; rien dans l'historique ni les statistiques.

### Choix faits (avec la raison)

1. **Plafond mesuré : 3 000 signes** (et non 4 000). Mesure ci-dessous : le code compte 2,1 signes par jeton ; avec une réponse aussi longue que la sélection, 4 000 signes de code (≈ 4 400 jetons) dépassent le contexte de 4 096 jetons de LM Studio par défaut. 3 000 signes tiennent avec 10 % de marge (≈ 3 460 jetons) ; c'est encore un long courriel en français (≈ 450 mots).
2. **Contrôle par l'adresse réellement appelée**, juste avant l'appel, dans `Cleaner::rewrite_selection` (seul point de passage) : `is_loopback_address(ep.base)` ou accord égal au triplet (moteur, modèle envoyé, adresse appelée). Lecture **stricte** de l'adresse : `@`, `\`, `%`, blanc, caractère de contrôle, `127.0.0.2` ou `[0:0:0:0:0:0:0:1]` comptent comme « ailleurs » (refuser à tort coûte une case à cocher ; accepter à tort enverrait la sélection).
3. **L'accord tombe à la lecture** (`AppSettings::from_value`) dès qu'il ne correspond plus au moteur choisi : revenir ensuite au moteur d'avant ne le fait pas revivre. **La page n'écrit que `true` ou `false`** : le serveur lie l'accord au moteur choisi après les autres changements du même envoi ; un objet fabriqué par la page est refusé (« accord illisible »). Ainsi l'accord ne peut porter que sur le moteur montré.
4. **Classement** : `rewriteConsent` est un champ d'**ordinateur** (`COMPUTER_KEYS`, et `backup.rs:COMPUTER_FIELDS`, 16 clés) : jamais dans l'export d'un utilisateur (rien à décocher à l'import), jamais restauré depuis une sauvegarde d'un autre système. Toujours écrit (`null` sans accord) : le test de classement du chantier 4 n'a pas eu à changer.
5. **Aucune redirection suivie** pour la reformulation : `HttpRequest.follow_redirects` (faux par `without_redirects()`), deuxième agent `ureq` avec `redirects(0)` ; une réponse 3xx devient une erreur. Les autres requêtes ne changent pas.
6. **Garde-fou** `accepts_rewrite` comme le cadrage, avec deux précisions : un « numéro » compte à partir de **4 chiffres** (espaces, points et tirets entre chiffres ignorés : un téléphone écrit autrement reste reconnu) ; une **première ligne courte finissant par « : »**, absente de la sélection, est une annonce (« Version plus amicale : »). La réponse est d'abord nettoyée sans toucher au contenu (`clean_output` : balises `<texte>` reprises, bloc de code ou guillemets ajoutés, **seulement** si la sélection ne les avait pas ; blancs de début et de fin de la sélection remis, pour qu'une ligne entière sélectionnée garde son retour à la ligne). On n'utilise pas `cleaner::strip`, qui ôterait le gras Markdown ou un bloc de code de la sélection.
7. **Application inconnue au départ** (`target.id` absent) : traitée comme changée (texte copié, pas collé), puisqu'on ne peut pas vérifier.
8. **`Paster::reads_selection`** (faux par défaut, vrai sur Mac et Windows) plutôt qu'un `cfg!(target_os)` dans le chef d'orchestre : le cas Linux se teste sur tous les systèmes.
9. **Pas de préchauffage** (cadrage) : le coût à froid mesuré est faible (≈ 0,8 s pour les 607 jetons du début fixe sur le Mac Studio) et la dictée garde son emplacement (mesure ci-dessous). `preload_signature` ne change pas.
10. **`--no-slots` non ajouté** : `/slots` ne montre aucun texte (mesure ci-dessous).
11. **Exemples du second prompt** : un changement de ton, une demande contenue dans le texte (corrigée, pas exécutée) et un raccourci ; ils sont dans le début fixe, gardé en mémoire par le serveur.
12. `SwitchPaster` délègue maintenant `copy` (neutralisé pendant un essai) et `reads_selection`. Pendant un essai, la sélection n'est pas lue (`NoSelection`) : rien ne part au modèle.

### Mesures (16/11/2026, Mac Studio, `llama-server` 0.8.0 build 11960 de Homebrew, Gemma 4 E4B q4_0, options de Scriibe : `-c 8192 -np 2 -kvu -fa on --jinja -ngl 99`, port d'essai 8190, sortie dans le dossier temporaire de la session)

| Mesure | Résultat |
|---|---|
| Début fixe du second prompt (consigne et 3 exemples) | 607 jetons |
| 4 000 signes de français (procès-verbal) | 1 480 jetons au total, donc 873 pour la sélection (4,6 signes par jeton) ; réponse 119 jetons ; 4,05 s à froid |
| 4 000 signes de code Rust | 2 501 jetons au total, donc 1 894 pour la sélection (2,1 signes par jeton) ; 2,92 s (début fixe déjà en mémoire) |
| Lecture du prompt | ≈ 730 jetons par seconde ; début fixe à froid ≈ 0,8 s, en mémoire ≈ 0,25 s pour une petite sélection |
| Dictée, reformulation, dictée (deux emplacements) | la dictée reprend 1 022 jetons sur 1 074 de sa mémoire après une reformulation ; la reformulation reprend 691 sur 696 : aucune éviction entre les deux |
| `llama-server.log` (sortie d'erreur) et sortie standard, sélection « SECRET-SELECTION-7731 … » et consigne « … CONSIGNE-4412 » | aucune trace du texte (35 lignes : chargement, compteurs, durées) |
| `GET /slots` (activé par défaut) | 200, champs `id`, `n_ctx`, `speculative`, `is_processing`, `id_task`, `n_prompt_tokens…`, `params`, `next_token` : aucun texte ; `--no-slots` inutile |

### Où est le code

- `crates/core/src/rewrite.rs` (nouveau) : `MAX_SELECTION_CHARS`, `RewriteConsent`, `binding`, `needs_consent`, `consent_matches`, `is_loopback_address`, `messages` (second prompt), `clean_output`, `accepts_rewrite`, `Verdict`.
- `crates/core/src/settings.rs` (`rewrite_consent`, chute à la lecture, `COMPUTER_KEYS`), `backup.rs` (`COMPUTER_FIELDS`), `cleaner.rs` (`Debug` de `Message`).
- `crates/engines/src/http.rs` (`Debug` de `HttpRequest` masqué, `follow_redirects`, agent sans redirection) ; `services.rs` (`RewriteError`, `rewrite_selection`, `may_receive_selection`, `complete_on`, `OPENROUTER_BASE`).
- `crates/platform/src/paste.rs` (`reads_selection`), `macos/paste.rs` et `windows/paste.rs` (vrai), `testing.rs` (`copied`, `set_reads_selection`).
- `crates/pipeline/src/controller.rs` : `Route::Instruction` → `rewrite_selection` (lecture de la sélection, appel, comparaison de l'application, collage transitoire ou copie).
- `crates/cli/src/web/mod.rs` (`merge_settings` : `true` / `false` ; état `rewrite`), `web/sinks.rs` (`copy`, `reads_selection`).
- Interface : `Settings.svelte` (case), `api.ts`, `Help.svelte`, `HelpEn.svelte`, `locales/en.ts`, `locales/en-server.ts`.

### Tests (mesurés sur le Mac)

- **815 tests Rust** (3 ignorés), 28 de plus que les 787 de la 0.7.0, et **109 tests de l'interface** sans erreur ; `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` et `svelte-check` sans erreur.
- Nouveaux : 14 dans `rewrite.rs` (adresse de boucle locale, accord lié et qui tombe, classement « ordinateur », prompt et balises neutralisées, début fixe identique, empreinte inchangée, nettoyage, garde-fou, plafond) ; 7 dans `engines/tests/services.rs` (**sans accord, aucune requête ne part** vers OpenRouter, un LM Studio de la maison ou un Ollama en `nas.local` ; accord d'un autre moteur, modèle ou adresse refusé ; avec accord, un seul appel sans redirection ; moteurs de cet ordinateur sans accord ; plafond sans envoi et `max_tokens` ; garde-fou, réponse 302, moteur en panne ; `Debug` sans corps ni clé) ; 6 dans `pipeline/tests/dictation.rs` (reformulation collée en transitoire sans trace, Linux, sans sélection ou illisible, moteur ailleurs puis accordé, **application changée → copie seulement**, échecs sans remplacement) ; 1 dans `web/mod.rs` (accord par `true` / `false`, objet fabriqué refusé). Le test « sans trace » de l'étape 1a vérifie maintenant que la seule requête qui porte la sélection va en `127.0.0.1` ; celui des phrases libres n'attend plus « Commande inconnue » pour « cherche les fautes » avec sélection.

### Non vérifié

- **Code Windows** (`reads_selection` de `WindowsPaster`) : non compilé ici (pas de compilateur croisé) ; à vérifier par la CI Windows avant la 0.8.0. Changement d'une ligne.
- **Aucun essai réel** à l'écran : sélection reformulée sur Mac et Windows, case d'accord, copie quand l'application change, message Linux. C'est la tâche 11.
- **Éditeurs de code** (ligne courante copiée sans sélection) : non mesuré ; dit dans l'Aide comme limite. La piste du cadrage (exiger l'Accessibilité sur Mac dans ces éditeurs) reste à décider après l'essai.
- **Contextes d'Ollama et de LM Studio** : le plafond suppose 4 096 jetons au moins ; un Ollama réglé plus petit tronquerait le début du prompt (supposé, non vérifié).
- **Troisième début de prompt** : avec la traduction en plus de la dictée et de la reformulation, llama.cpp n'a que deux emplacements ; l'un des trois sera relu à froid (non mesuré).
- Traduction anglaise d'une raison de moteur imbriquée dans « Reformulation impossible — {reason} » : la raison reste en français si elle n'a pas sa propre traduction.

### Suite

- Tâche 10 (même session) : relecture par un sous-agent Sonnet, puis corrections et § Relecture reformulation.

### Relais (fin de la tâche 9, contexte à 33 %)

Relais décidé par la règle du `CLAUDE.md` (contexte à 25 % ou plus en fin de tâche : la tâche suivante part d'une nouvelle session). Ne change ni l'ordre ni les consignes.

- **Fait** : toute la tâche 9 (étape 4), commit local `8861771` (puis ce relais), **rien de poussé**. 815 tests Rust (3 ignorés) et 109 tests de l'interface verts, `fmt`, `clippy -D warnings`, `svelte-check` sans erreur (mesurés).
- **En cours** : rien (arbre propre après le commit de ce relais).
- **Prochain pas** : la tâche 10 du découpage (relecture de la reformulation par un sous-agent Sonnet, effort Élevé ; session sur Opus, effort Moyen), consigne de la tâche 10 telle quelle. Fichiers à donner au relecteur : `crates/core/src/rewrite.rs`, `crates/engines/src/services.rs` (`rewrite_selection`, `may_receive_selection`, `complete_on`), `crates/engines/src/http.rs`, `crates/pipeline/src/controller.rs` (`rewrite_selection`), `crates/core/src/settings.rs` (`rewrite_consent`, fin de `from_value`), `crates/cli/src/web/mod.rs` (`merge_settings`, état `rewrite`), `crates/cli/src/web/sinks.rs`, et leurs tests (`engines/tests/services.rs` fin du fichier, `pipeline/tests/dictation.rs` fin du fichier).
- **Points à faire regarder en priorité** : (1) l'adresse contrôlée est `ep.base`, et `complete_on` poste vers `ep.base + "/v1/chat/completions"` : rien d'autre ne doit pouvoir changer l'hôte ; (2) `consent_matches` compare au `binding` tiré des réglages, `may_receive_selection` revérifie moteur, modèle envoyé et adresse ; (3) la chute de l'accord à la lecture (`from_value`) et l'écriture par `true` / `false` seulement ; (4) l'application inconnue au départ traitée comme changée ; (5) `Reformulation impossible — {e}` affiche le message d'un `ServiceError` : vérifier qu'aucun ne peut contenir du texte de la personne.
- **Pièges** : (1) le Mac n'a pas de compilateur croisé : le code Windows (`reads_selection` de `WindowsPaster`, une ligne) n'est pas compilé ici, la CI Windows le fera avant la 0.8.0 (pousser demande l'accord de l'auteur) ; (2) pour une mesure avec le vrai `llama-server`, prendre un autre port que 8179 (8190 a servi) et l'arrêter par `pgrep -x llama-server` + filtre sur le port, jamais `pkill -f` ; (3) le message « 3 000 signes au plus » est écrit en dur : le test `the_cap_is_the_one_the_pill_message_says` le relie à `MAX_SELECTION_CHARS`.
- **Commits** : `8861771` (étape 4) sur `a463780` (tâche 8). Commits locaux non poussés : `8861771` et ce relais.

## Relecture reformulation (tâche 10, 16/11/2026)

Relecture du code de l'étape 4 (commit `8861771`) par un sous-agent Sonnet (effort Élevé, lecture seule), pilotée sur Opus. Questions posées : la sélection peut-elle partir hors de cet ordinateur sans accord ; fuite dans le journal, la pastille ou `llama-server.log` ; collage dans une autre application que celle d'origine ; accord lié à un moteur précis ; tests qui prouvent vraiment ces garanties.

### Défauts trouvés et corrigés

| Gravité | Défaut | Correction |
|---|---|---|
| **Bloquant** | L'accord était repris d'une **sauvegarde du même système** (`restored_settings` ne protégeait `COMPUTER_FIELDS` que d'un autre système). Un fichier `.scriibe.json` fabriqué (adresse `evil.example` et accord assorti) aurait fait partir la sélection sans case cochée ; une sauvegarde honnête emportait aussi l'accord vers un autre ordinateur. | `backup::NEVER_RESTORED` : `rewriteConsent` garde **toujours** sa valeur actuelle (ou est retiré), quel que soit le système. Test `the_selection_consent_is_never_taken_from_a_backup_even_of_the_same_system` (échoue sur l'ancien code). |
| Important | Reformulation **copiée** (application changée) par la copie ordinaire : elle entrait dans l'historique Win+V, le presse-papiers du cloud, les gestionnaires du Mac. | `Paster::copy_transient` (Mac : marques `ConcealedType` et `TransientType` ; Windows : `write_text(text, true)` ; faux par défaut), délégué par `SwitchPaster` (neutralisé pendant un essai). |
| Important | L'application n'était comparée qu'**après** l'appel au modèle : appui dans A, passage à B pendant la transcription (la sélection de B est lue), retour dans A pendant l'appel → texte de B collé dans A. | `same_app` comparé aussi **avant de lire la sélection** : changée ou inconnue → rien n'est lu ni envoyé, « Reformulation impossible : l'application a changé » (traduit). |
| Important | Le garde-fou écartait des **traductions** légitimes (« Voici le document. » → « Here is the document. ») : la formule de tête n'était permise que si la sélection commençait par le même mot. | Formule de tête permise dès que la sélection commence elle-même par une formule (`opens_with_formula`). |
| Important | Une réponse qui **commente** au lieu de réécrire (« Aucune faute détectée. ») remplaçait la sélection. | Écartée si elle ne reprend aucun mot de 4 lettres ou plus (accents ignorés) d'une sélection qui en a deux, sauf consigne de traduction (`shares_no_word`, `is_translation`). |
| Important | Garanties non testées : redirection avec le **vrai** client, application inconnue, copie refusée, `CopiedOnly`. | Tests `a_request_without_redirects_never_reaches_the_redirect_target` (vrai `ureq`, deux serveurs locaux ; la même requête qui suit les redirections atteint bien le second, l'essai sait donc le voir), `…reads_nothing_when_the_application_changed_before…`, `…when_the_application_is_unknown`, `…cannot_be_copied_or_pasted_says_so_without_the_text`. Le banc ne change plus d'application pendant le préchauffage (un jeton), seulement pendant l'appel qui porte le texte. |
| Mineur | Adresse avec `?` ou `#` acceptée (appel cassé, même hôte, pas de fuite). | Refusée par `is_loopback_address`. |
| Mineur | `HttpResponse` montrait son corps (réponse du modèle) au `Debug` ; aucun usage aujourd'hui. | `Debug` masqué (statut et taille), testé. |

### Points vérifiés sans défaut (par le relecteur, dans le code)

- L'hôte appelé est `ep.base`, contrôlé juste avant l'appel avec le même instantané des réglages ; `ureq` 2.12.1 sans `proxy-from-env` (`HTTP_PROXY`, `ALL_PROXY` ignorés) ; aucun autre chemin n'envoie la sélection ; ni repli vers un autre moteur, ni second essai.
- `merge_settings` n'accepte que `true` / `false` / `null` ; l'accord tombe dès qu'un des trois change, n'est jamais exporté avec l'utilisateur.
- Journal et pastille : longueurs, moteur, durées, raisons fixes ; aucun logger branché (le `log` d'`ureq` ne sort nulle part) ; `ServiceError` sans corps ni adresse ; rien dans l'historique ni les statistiques ; `llama-server` lancé sans option de verbosité (mesures de la tâche 9 sur `llama-server.log` et `/slots` non refaites).
- Linux, essais (`SwitchPaster`), plafond en caractères avant tout envoi, `clean_output` sans coupure UTF-8.

### Non corrigé (mineur ou limite, avec la raison)

- **Même application, autre fenêtre, onglet ou document** pendant l'appel : la reformulation y est collée. Relire la sélection avant de coller demanderait une seconde copie (presse-papiers touché deux fois, ⌘C envoyé à une fenêtre peut-être changée) ; limite dite dans l'Aide (« Reste dans la même fenêtre jusqu'au collage »). À revoir après l'essai (tâche 11).
- Message « Réponse … coupée (dictée trop longue…) » de `complete_on` affiché aussi pour une reformulation : partagé avec la dictée et ses traductions ; à reprendre avec le prochain passage sur les messages.
- Ligne courte finissant par « : » ajoutée par le modèle (« Ingrédients » → « Ingrédients : ») : écartée à tort, sans danger (rien n'est remplacé).
- Guillemets d'un autre type retirés à tort dans `clean_output` ; `<texte>` de la sélection rendu en `‹texte›` (neutralisation du gabarit) : rares, notés.
- `assistant_apply` change le moteur sans passer par `merge_settings` : l'accord périmé reste sur disque jusqu'au rechargement, mais le contrôle à l'usage compare toujours le triplet exact, donc il ne vaut que pour le moteur déjà accordé.
- Boucle locale ≠ « cet ordinateur » si un tunnel SSH écoute sur `127.0.0.1` ; plafond de 3 000 signes compté en caractères (une écriture non latine coûte plus de jetons) : limites de conception, inchangées.

### Tests (mesurés sur le Mac)

- **821 tests Rust** (3 ignorés), 6 de plus que les 815 de la tâche 9, et **109 tests de l'interface** sans erreur ; `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` et `svelte-check` sans erreur.
- **Non compilé ici** : `WindowsPaster::copy_transient` (3 lignes, `write_text` existant) s'ajoute à `reads_selection` ; la CI Windows le vérifiera avant la 0.8.0.

### Suite

- Tâche 11 (nouvelle session, ⚙ Sonnet · Élevé) : essai réel du lot 3 sur le Mac puis le PC Windows. Ajouter à l'essai : changer d'application pendant la transcription (rien n'est lu), et vérifier qu'une reformulation copiée n'apparaît pas dans Win+V.
