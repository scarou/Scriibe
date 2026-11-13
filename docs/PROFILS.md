# Profils et dictionnaires : implémentation (chantier 4)

Suite de `docs/CADRAGE_PROFILS.md`, qui décide quoi faire. Ce fichier suit l'implémentation, lot par lot. Découpage : `decoupage-2026-11-12-c4-implementation-profils.md` (carnet du vault, dossier `decoupages/`).

## Constat

Fait le 13/11/2026 sur `main` au commit `8171411` (v0.4.0, après le chantier 5 « gestion des modes » et le chantier 7 « système »), par un sous-agent de recherche en lecture seule ; les références ont été revérifiées par sondage (13 lignes contrôlées : 12 exactes, une décalée d'une ligne et corrigée ici). Il porte sur `docs/CADRAGE_PROFILS.md`, décision 7 « Ordre de livraison », étapes 1 à 3. Les numéros de ligne sont ceux de ce commit : ils bougeront.

### Synthèse

| Point | Verdict | Où |
|---|---|---|
| 1. Enveloppe d'export v1 | **Existe**, réutilisable telle quelle ; contrôle d'entrée copié dans deux lecteurs | `crates/core/src/export.rs:11-27` |
| 2. `schemaVersion` | **En partie** : lu seulement dans la sauvegarde, jamais dans `AppSettings`, jamais écrit | `crates/core/src/backup.rs:19`, `:108-110` |
| 3. Drapeau « lecture refusée » | **Manque** : un fichier illisible redonne les défauts, la sauvegarde suivante écrase | `crates/platform/src/storage.rs:88-99`, `:144-150` |
| 4. Copies `.bak` | **Manque** ; mécanisme voisin pour la restauration complète seulement | `crates/platform/src/restore.rs:36-79` |
| 5. Fichiers temporaires | **En partie** : écriture atomique, mais nom fixe | `crates/platform/src/storage.rs:72-74` |
| 6. Dédoublonnage des corrections sur `heard` | **Manque** (à la lecture, à l'écriture et à l'ajout) | `crates/cli/src/web/mod.rs:833`, `crates/core/src/dictionary.rs:23` |
| 7. Export des `customCommands` et du dictionnaire | **Manque** pour les deux | `crates/core/src/commands.rs:174` |
| 8. Autres éléments | Voir plus bas : backup, gel des écritures, fonctions de fichier réutilisables | |

### 1. Enveloppe d'export v1 : existe

- Définition : `crates/core/src/export.rs:11` (`FORMAT = "scriibe-export"`), `:13` (`VERSION = 1`), `:15` (`EXTENSION = ".scriibe.json"`), `:18-27` (`envelope(kind, app_version, day, content)`). Champs écrits : `format`, `version`, `kind`, `app`, `exportedAt` (le jour seul, UTC, `day_of` `:30`), `content`.
- Elle est générique : `kind` est un paramètre libre. Deux usages : `modes_export` (`export.rs:54`, `kind: "modes"`, `content: {customModes}`) et `backup_export` (`crates/core/src/backup.rs:49-63`, `kind: "backup"`).
- Le contrôle d'entrée est recopié dans chaque lecteur : `crates/core/src/import.rs:33-43` (modes) et `backup.rs:95-105` (sauvegarde). Aucune fonction partagée du genre `parse_envelope(text, kind)`. Il accepte une `version` de 1 à `VERSION`, refuse un format, une version ou un `kind` inconnus ; `crates/core/tests/import_modes.rs:47` vérifie qu'un fichier `kind: "dictionary"` est refusé à l'import de modes.
- **Manque** pour `dictionary` et `commands` : les constructeurs d'export, les noms de fichier (seul `mode_file_name` existe, `export.rs:61`) et les lecteurs `parse_dictionary` et `parse_commands`.
- Écart avec le cadrage : l'enveloppe est faite et sert à deux `kind`. Le cadrage ne connaissait pas `backup`. Avec `dictionary` et `commands`, le contrôle copié passerait à quatre exemplaires : à factoriser.

### 2. `schemaVersion` : en partie

- `AppSettings` n'a pas de champ `schemaVersion` : struct `crates/core/src/settings.rs:240`, lecture tolérante `:473`, aucune occurrence dans `settings.rs` (vérifié). Le champ n'est donc ni écrit (`save_settings` sérialise `AppSettings`, `storage.rs:135-140`) ni conservé : la lecture tolérante ne garde aucune clé inconnue.
- Seule trace : `backup.rs:19` (`SETTINGS_SCHEMA: u64 = 1`) et `backup.rs:108-110`, qui refuse une sauvegarde dont les réglages portent un `schemaVersion` supérieur à 1 (test `backup.rs:234`, champ injecté à la main). Ce contrôle ne sert à rien en pratique : aucun `settings.json` réel n'a ce champ. Même constat dans `docs/GESTION_MODES.md:105`.
- **Manque** : le champ dans `AppSettings` (lu et écrit), son écriture à la sauvegarde, une constante unique partagée avec `backup.rs`.

### 3. Drapeau « lecture refusée » : manque

- `storage.rs:88-99` (`Watched::get`) : tout échec de lecture ou de lecture JSON retombe sur `default()`, sans drapeau. `storage.rs:144-150` (`FileConfig::settings`) : un fichier illisible redonne `first_run_settings` (`:47`) ; `save_settings` (`:135`) écrit ensuite par-dessus.
- Un `settings.json` d'une version plus récente n'est pas détecté : la lecture tolérante prend ce qu'elle comprend, la sauvegarde suivante supprime les champs inconnus. Le test `storage.rs:369` (`a_broken_settings_file_gives_first_run_settings`) fige ce comportement.
- Même chose pour le dictionnaire (`storage.rs:152-154`, puis `save_dictionary` `:156`) et pour l'historique (`FileHistory::open` `:194-200`, puis `persist` `:222-231`). `create_settings_if_missing` (`:127-133`) ne teste que `exists()`.
- Un seul gel d'écriture existe : `FROZEN` (`storage.rs:16`), posé par `set_frozen` (`:19`), lu par `is_frozen` (`:23`), appelé avant l'assistant de désinstallation (`crates/cli/src/commands.rs:467`). `write_atomic` (`:65-67`) rend alors `Ok(())` sans rien écrire : global, silencieux, jamais levé. Ce n'est pas le drapeau voulu, qui doit être **par fichier** et faire **échouer** la sauvegarde.
- À faire échouer, une fois le drapeau posé : tous les appelants de `save_settings` (`crates/cli/src/app.rs:162`, `tray_source.rs:170`, `web/mod.rs:737`, `:798`, `:807`, `:821`, `:870`), l'import de modes (`web/mod.rs:343-355`) et le POST `api/dictionary` (`web/mod.rs:357-361`).

### 4. Copies `.bak` : manque

- Aucun `.bak` écrit par le code : le seul résultat de la recherche est un nom de fichier de test (`crates/platform/src/uninstall.rs:1347`).
- Mécanisme voisin, pour la restauration complète seulement : `crates/platform/src/restore.rs:19-33` (`stage`, dossier `.restauration/` `:14`, marque `avant-import-<stamp>`) et `:36-79` (`apply_pending`, copie des fichiers actuels dans `<dossier>/avant-import-<stamp>/`, une seule fois par dossier, puis échange par renommage), appelé au démarrage par `crates/cli/src/commands.rs:21`, avant toute lecture. Horodatage `%Y%m%d-%H%M%S` à `crates/cli/src/web/backup.rs:210`. Test `web/backup.rs:319`.
- L'import de modes enregistre directement par `save_settings`, sans copie préalable.
- **Manque** : la copie `<nom>.<date>.bak` à la première lecture d'un fichier illisible.

### 5. Fichiers temporaires : en partie

- `storage.rs:72` : `path.with_extension("tmp")`, puis `write` (`:73`) et `rename` (`:74`). Nom fixe : `settings.tmp`, `dictionary.tmp`, `history.tmp`, `history-totals.tmp`. Trois appelants de `write_atomic` : `storage.rs:137` (réglages), `:158` (dictionnaire), `:228` (historique et statistiques).
- Incident déjà vu en test : `crates/cli/src/mode_sync.rs:17` note que le `settings.tmp` d'un test était renommé par un autre ; le test contourne le défaut par un dossier à compteur au lieu de corriger la cause.
- Autres écritures : lanceur de démarrage (nom fixe `tmp-scriibe`, `crates/platform/src/autostart.rs:150-152`) ; bancs (`bench_store.rs:94`, écriture directe non atomique) ; export de modes (`crates/cli/src/web/export.rs:22-37`, `create_new`, pas de temporaire) ; sauvegarde (`web/backup.rs:87-100`, écriture directe puis relecture) ; restauration (`restore.rs:29` et `:32`) ; téléchargements (`.part` renommé, `web/downloads.rs:436`) ; journal (`FileLog`, `storage.rs:241`, non atomique).
- **Manque** : nom unique (pid + compteur, ou aléa) pour les quatre fichiers de `write_atomic`.

### 6. Dédoublonnage des corrections sur `heard` : manque

- Lecture : `crates/core/src/dictionary.rs:23-39` (`UserDictionary::from_json`) ignore les corrections incomplètes, ne dédoublonne pas.
- Écriture : `crates/cli/src/web/mod.rs:833` (`clean_dictionary`). Les **mots** sont dédoublonnés sans tenir compte de la casse, mais en ASCII seulement (`eq_ignore_ascii_case`, `:837`). Les **corrections** sont seulement nettoyées (espaces, vides), sans dédoublonnage sur `heard`. Le test de nettoyage (`web/mod.rs:1271-1274`) ne couvre pas les doublons.
- Ajout : `ui/src/tabs/Dictionary.svelte:125-130` (`addCorrection`) ne vérifie pas que `heard` existe déjà.
- Application : `dictionary.rs:56-77` parcourt les corrections dans l'ordre : avec deux `heard` identiques, la première gagne, la seconde ne sert à rien.
- **Manque** : une clé de dédoublonnage de `heard` (sans casse, avec la normalisation des espaces et des apostrophes déjà utilisée par `heard_pattern`), appliquée à la lecture, à l'écriture et à l'import.

### 7. Export des `customCommands` et du dictionnaire : manque pour les deux

- Commandes : `CustomCommand` `crates/core/src/commands.rs:174-190` (`id`, `name`, `phrases`, `shortcut`, `needsSelection`, id préfixé `perso.`, `CUSTOM_PREFIX` `:169`). Dans les réglages : `settings.rs:347-348`, `:434`, `:545-547` (lecture tolérante : une commande mal écrite est écartée seule). Validation : `CustomCommand::cleaned` (`commands.rs:193-231` : nom, au moins une phrase d'au moins deux mots `MIN_WORDS` `:16`, raccourci lisible par `Shortcut::parse`, id préfixé et unique, phrases dédoublonnées). Routes `api/commands`, `validate-phrase`, `validate-shortcut` (`web/mod.rs:220-222`) : aucune n'exporte ni n'importe. Interface `ui/src/components/VoiceCommands.svelte:120-163`, sans bouton Exporter ni Importer ; type TS `ui/src/lib/api.ts:84`. Tests : `crates/core/tests/settings_scriibe.rs:90` ; aucun test d'export ni d'import de commandes.
- Dictionnaire : seules les routes `GET` et `POST api/dictionary` existent (`web/mod.rs:357-361`, `dictionary_view` `:825`, `save_dictionary` `:853`). `ui/src/tabs/Dictionary.svelte` n'a ni export ni import. Le dictionnaire figure dans la sauvegarde complète (`backup.rs:112-115`), vérifié par `UserDictionary::from_json` : ce n'est pas un export autonome.
- **Manque** : `parse_dictionary` et `parse_commands` côté `core` (avec limites de taille) ; aperçu à cases à cocher et fusion par le serveur (le patron existe pour les modes : `core/import.rs:153` `free_name`, `:164` `PreviewItem`, `:175` `preview`, `:190` `Choice`, `:205` `Outcome`, `:225` `apply`, mais il est propre aux modes) ; routes et interface ; les phrases importées doivent passer par `validate_phrase` (`commands.rs:515`) pour ne pas entrer en conflit avec une commande existante.

### 8. Éléments utiles que le cadrage n'avait pas vus

- `crates/core/src/import.rs` : limites (`MAX_BYTES` 1 Mo `:14`, `MAX_MODES` 50 `:16`, `MAX_EXAMPLES` `:18`, `MAX_TEXT` 20 000 `:20`, `MAX_NAME` 100 `:22`), `parse_modes` `:28`, nettoyage des textes et des noms (invisibles Unicode, `:110-130`).
- `crates/core/src/backup.rs` : `kind: "backup"`, limite 64 Mo (`:17`), `COMPUTER_FIELDS` (`:23-37`), `parse_backup` `:90`, `restored_settings` `:164`.
- `crates/cli/src/web/export.rs` : `prepare` `:11`, `write_without_overwriting` `:22`, `fallback_folder` `:40`, `choose_target` `:56`, `export_mode` `:74` (boîte « Enregistrer sous » sous Windows et macOS, dossier Documents sous Linux).
- `crates/cli/src/web/import.rs` : `read_file` `:23`, `read_file_max` `:28`, `choose_file` `:89`, `read` `:116`, `apply_pending` `:146`, `consume` `:161`, état en attente par jeton (`pending` `:18`). **Réutilisables** pour le dictionnaire et les commandes.
- Routes : `api/modes/export`, `api/modes/import/read|apply|cancel` (`web/mod.rs:340-356`) ; `api/backup/export` et `api/backup/import/*` (`:400-405`). Interface : `ui/src/tabs/Modes.svelte:79-148`, `ui/src/tabs/System.svelte:36-70`.
- Documentation à corriger plus tard : `docs/GESTION_MODES.md:105` dit encore que les étapes 1 et 2 ne sont pas faites.

### Écarts avec le cadrage

Déjà fait alors que le cadrage le supposait à faire ou absent :

- L'enveloppe v1 commune, et `kind: "modes"` en entier (étape 3, partie modes : export, aperçu, fusion, limites), par le chantier 5.
- `kind: "backup"` avec restauration par redémarrage et copie `avant-import-<date>/` (chantier 7), que le cadrage ne prévoyait pas.
- Un gel global des écritures (`set_frozen`) pour la désinstallation : à ne pas confondre avec le drapeau « lecture refusée ».

Encore à faire :

- **Étape 1 entière** : `schemaVersion` dans `AppSettings` (le seul contrôle actuel est inopérant), drapeau « lecture refusée » par fichier, `.bak` à la première lecture illisible, fichiers temporaires à nom unique, dédoublonnage de `heard`.
- **Étape 2** : tout (export, import, routes, interface, Aide FR et EN).
- **Étape 3** : seul `commands` manque (export, import, interface).
- Factoriser le contrôle d'enveloppe avant d'ajouter `dictionary` et `commands`.

## Étape 1 · Fondations (tâche 1, 13/11/2026)

Code de ce qui manquait de l'étape 1 de `docs/CADRAGE_PROFILS.md` (décision 7), d'après le constat ci-dessus. Aucun changement visible, sauf l'avertissement quand un fichier est refusé. Pas de version : part dans la 0.5.0.

### Ce qui est fait

| Point | Fait | Où |
|---|---|---|
| `schemaVersion` lu et écrit | Champ `AppSettings.schema_version` (JSON `schemaVersion`), **en tête du fichier** ; défaut 1 ; un fichier sans numéro (ou un numéro illisible) vaut 1 ; écrit à chaque sauvegarde. Constante unique `SETTINGS_SCHEMA` (`settings.rs`), reprise par `backup.rs` (son contrôle sert enfin : un vrai `settings.json` porte le champ). La page ne peut pas le changer (`merge_settings` l'ignore sans erreur, car elle renvoie parfois le bloc entier). | `crates/core/src/settings.rs`, `backup.rs`, `crates/cli/src/web/mod.rs` |
| Drapeau « lecture refusée » | Par fichier, dans `FileConfig` : `settings.json` illisible ou de version plus récente ; `dictionary.json` illisible. `save_settings` et `save_dictionary` échouent alors (`PermissionDenied`, message clair). Le refus tombe tout seul quand le fichier est réparé, remplacé ou supprimé (relecture à chaque changement de date). | `crates/platform/src/storage.rs` (`Watched::read_state`, `ConfigProblem`, `Refusal`) |
| Avertissement dans la page | `api/status` rend `config_problems` ; bandeau rouge en haut de chaque page (français et anglais), avec le nom de la copie. Les erreurs de sauvegarde renvoyées au serveur sont traduites (`en-server.ts`). | `ui/src/App.svelte`, `api.ts`, `locales/` |
| Copie `<nom>.<date>.bak` | À la **lecture** d'un fichier illisible, une seule fois par contenu (si une copie identique existe déjà, on la réutilise : pas de pile de copies à chaque lancement). Aussi pour `history.json` et `history-totals.json` (ouverture de l'historique). | `storage.rs` (`set_aside`, `FileHistory::open`) |
| Fichiers temporaires à nom unique | `.<nom>.<pid>-<n>.tmp`, supprimé si l'écriture ou le renommage échoue. Couvre réglages, dictionnaire, historique et totaux (les trois appelants de `write_atomic`). | `storage.rs` (`write_atomic`) |
| Corrections dédoublonnées sur `heard` | Clé `heard_key` : sans casse, espaces multiples réduits, apostrophe courbe = droite (comme `heard_pattern`). La première correction reste (c'est déjà celle qui s'appliquait). Appliqué à la lecture (`from_json`), à l'enregistrement (`clean_dictionary`) et à l'ajout dans la page (une même phrase entendue **remplace** la valeur existante au lieu de doubler la ligne). | `crates/core/src/dictionary.rs`, `web/mod.rs`, `ui/src/lib/dictionary.ts`, `tabs/Dictionary.svelte` |

### Choix techniques et pourquoi

- **Fichier plus récent : lu, jamais réécrit.** Scriibe garde les réglages qu'il comprend pour que l'appli reste utilisable, mais n'écrit rien : la sauvegarde suivante aurait supprimé les champs qu'il ne connaît pas. Fichier illisible : réglages du premier lancement en mémoire, rien d'écrit. Recommandé plutôt que « tout refuser » parce que bloquer la dictée à cause d'un champ futur est pire que bloquer seulement la sauvegarde.
- **Fichier vide ou blanc : pas un problème** (rien à perdre, la sauvegarde peut l'écrire). Une erreur de lecture passagère (accès refusé un instant) n'est pas gardée en cache : elle se retente au lieu de bloquer jusqu'au prochain changement du fichier.
- **Pas de `.bak` pour un fichier lisible mais plus récent** : il n'est jamais écrasé, une copie n'apporterait rien.
- **Un seul exemplaire de copie par contenu** plutôt que « une fois par lancement » : sans cela, chaque lancement avec le même fichier cassé aurait ajouté une copie identique.
- **Même clé de dédoublonnage côté Rust et côté page**, pour que la page ne laisse jamais passer un doublon que le serveur retirerait ensuite sans prévenir.

### Tests (un par cas)

| Cas | Test |
|---|---|
| `schemaVersion` par défaut, lu, illisible, écrit en tête | `settings_scriibe.rs::the_schema_version_defaults_to_one_is_read_when_present_and_is_written_first` |
| Écrit à la sauvegarde ; fichier sans numéro = 1 | `storage::tests::saved_settings_carry_the_schema_version_and_a_file_without_one_counts_as_version_one` |
| Page : numéro non modifiable | `web::tests::the_page_cannot_change_the_schema_version_of_the_settings` |
| Illisible : sauvegarde bloquée, fichier intact, repart une fois réparé | `storage::tests::an_unreadable_settings_file_blocks_every_save_until_it_is_repaired` |
| Plus récent : lu, jamais réécrit, champs inconnus intacts, pas de `.bak` | `storage::tests::a_settings_file_from_a_newer_version_is_read_but_never_rewritten` |
| Dictionnaire illisible : bloqué, réglages non | `storage::tests::an_unreadable_dictionary_blocks_its_save_but_not_the_settings` |
| `.bak` une seule fois, à la lecture, nom rendu à la page, autre contenu = autre copie | `storage::tests::an_unreadable_file_is_copied_aside_once_at_reading_time` |
| Fichier vide : pas un problème | `storage::tests::an_empty_file_is_not_a_problem` |
| Historique illisible : copie avant écrasement | `storage::tests::an_unreadable_history_is_copied_aside_before_the_empty_one_overwrites_it` |
| Écritures simultanées : temporaires distincts, aucun reste | `storage::tests::simultaneous_writes_use_their_own_temporary_files_and_leave_none_behind` (vérifié : il **échoue** avec l'ancien nom fixe `.tmp`) |
| Clé `heard`, première correction gardée, lecture, résultat de la dictée inchangé | `dictionary_corrections.rs` (3 tests) |
| Doublon envoyé par la page | `web::tests::a_heard_phrase_sent_twice_by_the_page_is_kept_once` |
| Ajout d'une correction existante dans la page | `ui/src/lib/dictionary.test.ts` (3 tests) |

Mesuré sur le Mac : `cargo fmt` et `cargo clippy --all-targets -- -D warnings` sans erreur ; **573 tests Rust** passent (559 avant), **102 tests de l'interface** (99 avant), `svelte-check` sans erreur. Pas de nouveau cas dans `fixtures/` : rien de ce qui change n'est de la logique de texte de Scribe (le dédoublonnage ne modifie pas le résultat d'une dictée, vérifié par un test).

### Limites et à reprendre

- **Pas essayé à l'écran** : l'avertissement n'a été vu ni dans la fenêtre du Mac ni sur un PC (tests automatiques du cas seulement). À essayer à la main : casser `settings.json`, relancer, regarder le bandeau, réparer.
- **Historique** : copie `.bak` oui, refus d'écrire non (le cadrage ne parle que de `FileConfig`) ; l'historique vide écrase donc le fichier cassé, dont la copie reste à côté. À revoir à l'étape 4a (`FileHistory` change de dossier).
- **Message en français dans « impossible d'écrire le dictionnaire (…) »** : le motif entre parenthèses n'est pas traduit en anglais (le bandeau, lui, l'est).
- **Autres écritures non atomiques** (bancs, journal, export de sauvegarde, lanceur de démarrage `tmp-scriibe`) : hors de l'étape 1, inchangées.
- **Mots du dictionnaire** : le dédoublonnage reste ASCII seulement (`eq_ignore_ascii_case`), seules les corrections ont changé.
- **Factorisation du contrôle d'enveloppe** (constat § 1) : pas faite ici, elle appartient à l'étape 2 (nouveaux `kind`).
- `docs/GESTION_MODES.md:105` dit encore que les étapes 1 et 2 ne sont pas faites : non touché.

## Étape 2 · Import et export du dictionnaire (tâche 2, 13/11/2026)

Code de l'étape 2 de `docs/CADRAGE_PROFILS.md` (décisions 4 et 5, `kind: "dictionary"`). Même démarche que pour les modes (chantier 5). Pas de version : part dans la 0.5.0.

### Ce qui est fait

| Point | Fait | Où |
|---|---|---|
| Enveloppe v1, `kind: "dictionary"` | `content: {words, corrections}`, clés de `dictionary.json`. Export **champ par champ** (liste blanche), jamais d'autre clé ; une catégorie décochée n'a pas de clé ; `exportedAt` = le jour seul ; nom `scriibe-dictionnaire-<jour>.scriibe.json`. | `crates/core/src/export.rs` (`dictionary_export`, `dictionary_file_name`) |
| Contrôle d'enveloppe **factorisé** | `open_envelope(text, kind, max, too_big, wrong_kind)` : taille, JSON, `format`, `version`, `kind`. Repris par les modes, la sauvegarde complète et le dictionnaire (trois copies du contrôle ramenées à une). Un fichier en version 0 n'est plus annoncé « plus récent ». | `export.rs`, `import.rs`, `backup.rs` |
| Export en clair avec aperçu | Carte « Exporter et importer le dictionnaire » → « Exporter… » : nombre de mots et de corrections, une case par catégorie, listes dépliables, avertissement (fichier non chiffré, noms propres et textes personnels). Boîte « Enregistrer sous » (Mac, Windows) ; Linux : dossier Documents, sans rien écraser. Les modifications en attente dans la page sont enregistrées avant. | `crates/cli/src/web/dictionary_io.rs`, `ui/src/tabs/Dictionary.svelte` |
| Import avec aperçu et cases | Boîte « Ouvrir » (Mac, Windows) ou chemin tapé (Linux). Aperçu : chaque mot et chaque correction avec son statut — **nouveau** (coché), **déjà là** (grisé), **conflit** (même « entendu », autre « écrit » : la correction en place est gardée, le conflit est montré). « Tout cocher / décocher » par catégorie. | `crates/core/src/import_dictionary.rs`, `dictionary_io.rs`, `Dictionary.svelte` |
| Fusion par le serveur, ajoute sans remplacer | `apply` ne fait qu'ajouter à la fin : rien de ce qui est en place n'est modifié, déplacé ni retiré. Les statuts sont **recalculés à l'accord**, face au dictionnaire d'à présent. Jeton d'aperçu : l'accord le rappelle, sinon « l'aperçu n'est plus à jour ». Fichier non enregistré tant que `save_dictionary` n'a pas réussi (attente gardée pour recommencer). Après l'import, la page relit le dictionnaire. | `import_dictionary.rs`, `dictionary_io.rs`, routes `api/dictionary/export`, `import/read`, `import/apply`, `import/cancel` |
| Limites | Fichier 1 Mo ; 5 000 mots ; 5 000 corrections ; mot 100 caractères ; « entendu » 200 ; « écrit » 500. Dépassement : **tout le fichier est refusé** (rien n'est tronqué en silence). | `import_dictionary.rs` |
| Fichier d'un tiers | Seuls `words` et `corrections` sont lus (le reste est ignoré, jamais copié). Un élément mal formé refuse le fichier. Textes nettoyés : une seule ligne, sans caractère de contrôle ni invisible (RTL, largeur nulle). Éléments vides écartés. Doublons du fichier : le premier reste. Un fichier refusé ne change rien et **efface l'attente précédente**. Le texte « écrit » est inséré tel quel dans les dictées (jamais interprété comme motif ni comme `$1`, vérifié dans `dictionary::replace`). | `import_dictionary.rs`, `import.rs` (`one_line`) |
| Aide FR et EN | Section « Exporter et importer le dictionnaire » dans « Dictionnaire et signes » (`Help.svelte`, `HelpEn.svelte`). Traductions de la page et des messages du serveur (`en.ts`, `en-server.ts`). | `ui/src/` |

### Choix techniques et pourquoi

- **Conflit de correction : celle en place gagne, le fichier ne la remplace jamais** (cadrage, décision 4). Pas de choix « remplacer » (contrairement aux modes) : une correction d'un tiers remplace un mot par un autre dans toutes les dictées, la laisser écraser la sienne serait le pire défaut. Qui veut la changer la modifie à la main.
- **Éléments non nouveaux : cases grisées**, pas de choix. Les cocher n'aurait rien changé ; les griser le dit.
- **Refuser plutôt que tronquer** au-delà des limites : un import partiel en silence est pire qu'un refus clair.
- **Même clé de comparaison partout** : `word_key` (sans casse Unicode, espaces réduits) pour les mots, `heard_key` (étape 1) pour les corrections. `clean_dictionary` passe aussi à `word_key` : « École » et « école » ne font plus deux mots (avant, seule la casse ASCII comptait : limite de l'étape 1 levée).
- **Les modifications en attente de la page sont enregistrées avant un export ou un import** (`flush()`), et l'enregistrement automatique est annulé après l'import (la page relit le dictionnaire) : sinon les 500 ms d'attente de la page pouvaient écraser l'import ou exporter un état périmé.
- **Chemin du fichier jamais envoyé par la page** (sauf Linux, sans boîte, où le chemin tapé est le seul moyen) : même règle que les modes.
- **Tutoiement** : les textes nouveaux suivent la page Dictionnaire (« ton dictionnaire »). Les messages de l'étape 1 (bandeau, « Mets Scriibe à jour ») ont été alignés dessus.

### Tests

| Cas | Test |
|---|---|
| Aller-retour identique (mêmes mots, corrections, ordre) | `import_dictionary.rs::an_export_then_an_import_gives_back_the_same_dictionary` |
| Export : enveloppe, catégories choisies, rien d'autre | `import_dictionary.rs::the_export_has_the_envelope_the_chosen_categories_and_nothing_else` ; `web::dictionary_io::tests::the_export_has_the_agreed_name_and_only_the_chosen_categories` |
| Rien à exporter | `dictionary_io::tests::nothing_to_export_is_refused` |
| Fichiers refusés : pas du JSON, autre format, sans version, version 0, version plus récente, autre type (modes, sauvegarde, commandes, profil), contenu absent | `import_dictionary.rs::files_that_are_not_a_dictionary_export_are_refused_with_a_clear_message` |
| Contenu vide ou mal formé | `import_dictionary.rs::an_empty_or_malformed_content_is_refused` |
| Limites : taille, nombre, longueurs (valeur limite acceptée, une de plus refusée) | `import_dictionary.rs::the_limits_on_size_count_and_length_refuse_the_whole_file` |
| Doublons du fichier, nettoyage des textes, champ futur ignoré | `import_dictionary.rs::repeats_inside_the_file_are_collapsed_to_the_first_and_texts_are_cleaned` |
| Aperçu : nouveau, déjà là, conflit | `import_dictionary.rs::the_preview_says_what_is_new_what_is_already_there_and_what_conflicts` |
| N'ajoute que, deux fois le même fichier = rien de plus | `import_dictionary.rs::an_import_only_adds_it_never_replaces_moves_or_removes_what_is_there` |
| Cases décochées non importées | `import_dictionary.rs::only_the_ticked_items_are_imported` ; `dictionary_io::tests::an_unticked_item_is_not_imported` |
| Fichier refusé : rien ne change, attente précédente oubliée | `dictionary_io::tests::a_refused_file_changes_nothing_and_forgets_the_previous_wait` |
| Aperçu puis accord : ajoute sans remplacer, conflit gardé | `dictionary_io::tests::the_preview_then_the_agreement_adds_without_replacing` |
| Jeton ou nombre de cases périmé : refusé, attente gardée | `dictionary_io::tests::a_wrong_token_or_wrong_number_of_boxes_is_stale_and_keeps_the_wait` |
| Mots « École » / « école » | `web::tests::words_that_differ_only_by_an_accented_capital_are_the_same_word` |

Mesuré sur le Mac : `cargo fmt` et `cargo clippy --all-targets -- -D warnings` sans erreur ; **589 tests Rust** passent (573 après l'étape 1) ; **102 tests de l'interface** ; `svelte-check` sans erreur ; l'interface se construit. Les tests de traduction (`i18n.test.ts`) et de structure de l'Aide FR/EN passent. Pas de nouveau cas dans `fixtures/` : aucune logique de texte de Scribe ne change.

### Limites et à reprendre

- **Pas essayé à l'écran** : ni l'export ni l'import n'ont été faits dans la fenêtre de Scriibe, ni sur le Mac ni sur un PC (tests automatiques des fonctions du serveur et du cœur seulement). À essayer à la main : exporter, relire le fichier, importer dans un dictionnaire vide (aller-retour), puis dans un dictionnaire qui a un conflit ; boîtes « Ouvrir » et « Enregistrer sous » sur Windows ; champ de chemin sous Linux.
- **Windows et Linux non compilés ni testés ici** : le code propre au système est celui déjà utilisé par les modes (`choose_file`, `choose_target`) ; la CI complète le dira.
- **Pas de route exercée en HTTP** : les routes sont de simples appels aux fonctions testées (comme pour les modes).
- **Un seul fichier en attente à la fois** par type (dictionnaire, modes) : deux fenêtres ouvertes sur l'import se partageraient l'attente (le jeton fait refuser l'ancien aperçu). Même limite que les modes.
- **Les nombres de l'aperçu** comptent les éléments du fichier après nettoyage et dédoublonnage, pas ceux du fichier brut.
- **Étape 3, partie commandes** : pas faite ici (tâche suivante). Le contrôle d'enveloppe commun et le patron `dictionary_io.rs` sont prêts à servir.

## Étape 3 · Import et export des commandes perso (tâche 3, 13/11/2026)

Reste de l'étape 3 de `docs/CADRAGE_PROFILS.md` (la partie modes a été faite au chantier 5) : `kind: "commands"`, `content: {customCommands}`. Même enveloppe, mêmes règles que le dictionnaire. Les commandes étendues du chantier 6 ne sont pas touchées. Pas de version : part dans la 0.5.0.

### Ce qui est fait

| Point | Fait | Où |
|---|---|---|
| Export | Liste blanche champ par champ (`id`, `name`, `phrases`, `shortcut`, `needsSelection`), jour seul, nom `scriibe-commandes-<jour>.scriibe.json`. Boîte « Enregistrer sous » (Mac, Windows) ; Linux : dossier Documents, sans rien écraser. Rien à exporter sans commande. | `crates/core/src/export.rs`, `crates/cli/src/web/commands_io.rs` |
| Lecture contrôlée | Contrôle d'enveloppe commun (étape 2). Limites : 1 Mo, 100 commandes, 20 phrases par commande, titre 100, phrase 200, raccourci 100 caractères ; dépassement ou élément mal formé : **tout le fichier refusé**, rien ne change. Textes sur une seule ligne, sans caractère de contrôle ni invisible ; champs inconnus ignorés. | `crates/core/src/import_commands.rs` |
| `id` en collision regénéré | Un `id` du fichier libre et préfixé `perso.` est gardé ; sinon (déjà pris, ou sans le préfixe) un nouveau `perso.<n>` libre. | `import_commands.rs` (`free_id`) |
| Nom déjà pris numéroté | Même règle que les modes (`free_name` : « Barré 2 », le numéro ne s'accumule pas, y compris entre deux commandes du même fichier). | `import_commands.rs` |
| Raccourci validé | Par `Shortcut::parse` (donc « Mod+Q » refusé comme dans la page). Un raccourci absent ou illisible n'est **pas** une erreur du fichier : il est signalé dans l'aperçu, case grisée, la commande n'est pas importée. | `import_commands.rs` (`Problem::BadShortcut`) |
| Phrases | Une phrase trop courte, déjà prise par une commande du programme ou par une commande en place (dans l'une ou l'autre langue), ou répétée, est écartée et **montrée** dans l'aperçu. Plus aucune phrase utilisable : commande non importée. | `import_commands.rs` (`evaluate`, via `validate_phrase`) |
| Aperçu et accord | Une case par commande (grisée si elle ne peut pas être importée), jeton d'aperçu, statuts **recalculés à l'accord**, l'import n'ajoute que : commandes en place, phrases personnalisées des commandes du programme et autres réglages ne bougent pas. Fichier non consommé tant que l'enregistrement n'a pas réussi. | `commands_io.rs`, routes `api/commands/export`, `import/read`, `import/apply`, `import/cancel` |
| Page et Aide | Boutons « Exporter mes commandes… » et « Importer des commandes… » dans la carte « Commandes vocales » (onglet Dictionnaire), aperçu en ligne. Aide FR et EN (puce « Mes commandes »), traductions de la page et du serveur. | `ui/src/components/VoiceCommands.svelte`, `Help.svelte`, `HelpEn.svelte`, `locales/` |

### Choix techniques et pourquoi

- **Pas de « remplacer »** (comme pour le dictionnaire, contrairement aux modes) : une commande importée appuie un raccourci dans une autre appli, la laisser écraser une commande en place serait plus dangereux que de l'ajouter sous un autre nom.
- **Raccourci illisible : signalé et non importé**, plutôt que fichier refusé (cadrage : « signalé dans l'aperçu ») ; une seule mauvaise commande ne bloque pas les autres.
- **Phrases en conflit écartées plutôt que commande refusée** : on garde ce qui reste utilisable, on dit ce qui a été laissé de côté.
- **Un `id` du fichier est gardé s'il est libre** : un aller-retour (export puis import dans des réglages vides) redonne exactement les mêmes commandes, ce qui rend le cas testable.

### Tests

| Cas | Test |
|---|---|
| Aller-retour identique | `import_commands.rs::an_export_then_an_import_gives_back_the_same_commands` |
| Fichiers refusés (format, version, autre type, contenu absent, mal formé) | `import_commands.rs::files_that_are_not_a_commands_export_are_refused` |
| Limites (valeur limite acceptée, une de plus refusée) | `import_commands.rs::the_limits_refuse_the_whole_file` |
| `id` regénéré, nom numéroté sans cumul | `import_commands.rs::a_colliding_id_is_regenerated_and_a_taken_name_is_numbered_without_piling_up` |
| Raccourci validé par `Shortcut::parse`, titre vide | `import_commands.rs::the_shortcut_is_checked_with_shortcut_parse_and_a_bad_one_is_flagged_not_imported` |
| Phrases prises, trop courtes, répétées ; commande sans phrase | `import_commands.rs::phrases_already_used_or_too_short_are_dropped_and_a_command_left_without_phrase_is_flagged` |
| Pas de phrase ni de nom partagés dans le fichier | `import_commands.rs::the_preview_does_not_let_two_commands_of_the_file_share_a_phrase_or_a_name` |
| N'ajoute que, cases décochées, second import | `import_commands.rs::an_import_only_adds_and_unticked_commands_stay_out` |
| Export : nom et champs de la liste blanche | `commands_io::tests::the_export_has_the_agreed_name_and_the_whitelisted_fields` |
| Fichier refusé : rien ne change, attente oubliée | `commands_io::tests::a_refused_file_changes_nothing_and_forgets_the_previous_wait` |
| Accord : id regénéré, nom numéroté, phrase prise écartée, raccourci illisible signalé, commande en place intacte | `commands_io::tests::the_agreement_adds_with_a_new_id_and_a_numbered_name_and_keeps_what_is_there` |
| Case décochée, jeton ou nombre de cases périmé | `commands_io::tests::an_unticked_command_is_not_imported_and_a_stale_agreement_is_refused` |

Mesuré sur le Mac : `cargo fmt` et `cargo clippy --all-targets -- -D warnings` sans erreur ; **601 tests Rust** passent (589 après l'étape 2) ; **102 tests de l'interface**, `svelte-check` sans erreur, interface construite (les tests d'i18n et de structure de l'Aide FR/EN passent).

### Limites et à reprendre

- **Pas essayé à l'écran** (ni export ni import dans la fenêtre, Mac ou PC) ; boîtes de fichier Windows et chemin Linux non essayés ; Windows et Linux non compilés ici (CI).
- **Les commandes ne marchent que sur Mac** pour l'instant (Aide) : on peut les exporter et importer partout, elles n'agissent qu'où la fonction existe.
- **Un raccourci valide ici peut ne rien faire dans une autre appli** : `Shortcut::parse` vérifie la forme, pas l'effet.
- **Les phrases sont comparées dans les deux langues**, plus strict que la page (qui contrôle dans la langue de l'interface) : une phrase libre en français mais prise en anglais est écartée.
- **Un seul fichier en attente à la fois** par type, comme pour les modes et le dictionnaire.
- Les étapes 4a à 5 (utilisateurs) viennent ensuite ; `kind: "profile"` réutilisera ces trois exports.

## Essai lot 1 (tâche 4, 13/11/2026)

Essai réel des étapes 1 à 3 sur le code du commit `58e9898` (non publié). **Mac : fait à la main** (l'auteur a fait les gestes des boîtes de fichier). **Linux : fait par `pilote-pc`**, sans geste de l'auteur. **Windows : pas fait** (le PC ne répond pas : éteint, ou NordVPN actif).

### Méthode (et pourquoi)

- **Données jetables** : variable `SCRIIBE_DATA` vers un dossier d'essai, jamais le dossier de l'auteur. Sur le Mac, une copie de l'application fabriquée hors de `~/Applications` (`SCRIIBE_INSTALL=…`), lancée par `open -n --env SCRIIBE_DATA=…` (règle : Scriibe.app se lance par `open`, jamais par Bash). Sur Linux, `scriibe lancer --sans-navigateur` sur `/tmp/essai-lot1`, page atteinte par un tunnel SSH ; l'instance habituelle du PC n'a pas été touchée.
- **Code transmis aux PC sans rien publier** : paquet git (`git bundle`, 65 Ko) copié sur le PC Linux, `git merge --ff-only`, puis compilation par `pilote compiler linux --mode release`. Même procédé que pour le chantier 7.
- **Le serveur est piloté par ses routes** (`curl`) et par la page (volet navigateur du Mac) ; seules les boîtes « Enregistrer sous » et « Ouvrir » de macOS demandent l'auteur.

### Résultats

| Cas | Mac | Linux | Windows |
|---|---|---|---|
| `settings.json` et `dictionary.json` illisibles : copie `<nom>.<date>.bak` à la lecture, contenu identique, originaux intacts | ✔ | ✔ | — |
| Avertissement : `config_problems` dans `api/status` (un par fichier, avec le nom de la copie) | ✔ | ✔ | — |
| Bandeau rouge dans la page, avec le nom de la copie | ✔ (vu) | non vu (pas d'écran piloté) | — |
| Écriture refusée (dictionnaire et réglages : HTTP 400, message clair), fichiers intacts, aucun `.tmp` | ✔ | ✔ | — |
| Réparation à la main : l'avertissement tombe tout seul, les copies restent | ✔ | ✔ | — |
| `schemaVersion: 1` écrit dans `settings.json` | ✔ | ✔ | — |
| Export du dictionnaire (4 mots, 3 corrections) : enveloppe v1 `dictionary`, rien d'autre | ✔ (boîte « Enregistrer sous ») | ✔ (dossier Documents) | — |
| Import du dictionnaire dans un dictionnaire vidé : aperçu 4 + 3, fichier identique à l'original (mots, corrections, ordre) | ✔ (boîte « Ouvrir ») | ✔ (chemin tapé) | — |
| Export puis import des commandes (2 commandes) : `id`, champs et ordre identiques | ✔ | ✔ | — |
| Fichier d'un autre type refusé, message clair, attente oubliée | (tests automatiques) | ✔ | — |
| Aucun fichier temporaire laissé | ✔ | ✔ | — |

### Constats

- **Rien à corriger dans le code du lot 1** : tous les cas essayés ont passé du premier coup.
- **Trousseau (macOS)** : la copie d'essai, dans un autre dossier, a redemandé l'accès aux clés. Tant que la fenêtre était ouverte, `api/status` restait bloqué (il lit les clés). Comportement connu (mémoire « trousseau »), pas un défaut du lot ; une session sur le Mac doit s'y attendre quand elle lance une copie d'essai.
- **Capture d'écran du Mac depuis la session** : noire (pas d'autorisation d'enregistrement d'écran pour cet outil) ; la vérification visuelle s'est faite dans le volet navigateur sur la page de l'instance.
- **Linux** : l'export va dans `~/Documents` (comportement voulu, sans boîte) ; l'import prend le chemin tapé, `~/` développé. L'instance habituelle du PC tourne encore sur l'ancien programme (son fichier a été remplacé par la compilation) : à relancer à l'occasion.
- **Reste à essayer** : Windows (PC à allumer, NordVPN coupé ; les boîtes « Ouvrir » et « Enregistrer sous » y demandent l'auteur) ; le bandeau sur Linux ; un import avec **conflit** (correction du même « entendu », phrase déjà prise) et un second import du même fichier, couverts par les tests automatiques mais pas essayés à l'écran.

### Ajouts

Aucune nouvelle demande de l'auteur pendant l'essai.
