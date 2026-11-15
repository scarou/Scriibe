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
