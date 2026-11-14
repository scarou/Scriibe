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
| Limites | Fichier 1 Mo ; **500 mots** (10 000 caractères en tout) ; **500 corrections** ; mot 100 caractères ; « entendu » 200 ; « écrit » 500 ; un import n'ajoute plus de mot quand le dictionnaire en compte **1 000** (dit dans le message de fin). Dépassement dans le fichier : **tout le fichier est refusé** (rien n'est tronqué en silence). *(Plafonds posés après la relecture : au départ 5 000 et 5 000, ce qui laissait un fichier d'un tiers gonfler le prompt du modèle.)* | `import_dictionary.rs` |
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
| Export | Liste blanche champ par champ (`name`, `phrases`, `shortcut`, `needsSelection` ; pas d'`id`), jour seul, nom `scriibe-commandes-<jour>.scriibe.json`. Boîte « Enregistrer sous » (Mac, Windows) ; Linux : dossier Documents, sans rien écraser. Rien à exporter sans commande. | `crates/core/src/export.rs`, `crates/cli/src/web/commands_io.rs` |
| Lecture contrôlée | Contrôle d'enveloppe commun (étape 2). Limites : 1 Mo, 100 commandes, 20 phrases par commande, titre 100, phrase 200, raccourci 100 caractères ; dépassement ou élément mal formé : **tout le fichier refusé**, rien ne change. Textes sur une seule ligne, sans caractère de contrôle ni invisible ; champs inconnus ignorés. | `crates/core/src/import_commands.rs` |
| `id` regénéré | **Jamais exporté** (il contient l'heure exacte de création de la commande : `perso.<millisecondes>`) et **toujours regénéré à l'import** (`perso.<n>` numérique et libre). Un `id` présent dans un fichier est ignoré. *(Après la relecture : au départ l'id était exporté et gardé s'il était libre.)* | `import_commands.rs` (`free_id`) |
| Nom déjà pris numéroté | Même règle que les modes (`free_name` : « Barré 2 », le numéro ne s'accumule pas, y compris entre deux commandes du même fichier). | `import_commands.rs` |
| Raccourci validé | Par `Shortcut::parse` (donc « Mod+Q » refusé comme dans la page). Un raccourci absent ou illisible n'est **pas** une erreur du fichier : il est signalé dans l'aperçu, case grisée, la commande n'est pas importée. | `import_commands.rs` (`Problem::BadShortcut`) |
| Phrases | Une phrase trop courte, déjà prise par une commande du programme ou par une commande en place (dans l'une ou l'autre langue), ou répétée, est écartée et **montrée** dans l'aperçu. Plus aucune phrase utilisable : commande non importée. | `import_commands.rs` (`evaluate`, via `validate_phrase`) |
| Aperçu et accord | Une case par commande (grisée si elle ne peut pas être importée), jeton d'aperçu, statuts **recalculés à l'accord**, l'import n'ajoute que : commandes en place, phrases personnalisées des commandes du programme et autres réglages ne bougent pas. Fichier non consommé tant que l'enregistrement n'a pas réussi. | `commands_io.rs`, routes `api/commands/export`, `import/read`, `import/apply`, `import/cancel` |
| Page et Aide | Boutons « Exporter mes commandes… » et « Importer des commandes… » dans la carte « Commandes vocales » (onglet Dictionnaire), aperçu en ligne. Aide FR et EN (puce « Mes commandes »), traductions de la page et du serveur. | `ui/src/components/VoiceCommands.svelte`, `Help.svelte`, `HelpEn.svelte`, `locales/` |

### Choix techniques et pourquoi

- **Pas de « remplacer »** (comme pour le dictionnaire, contrairement aux modes) : une commande importée appuie un raccourci dans une autre appli, la laisser écraser une commande en place serait plus dangereux que de l'ajouter sous un autre nom.
- **Raccourci illisible : signalé et non importé**, plutôt que fichier refusé (cadrage : « signalé dans l'aperçu ») ; une seule mauvaise commande ne bloque pas les autres.
- **Phrases en conflit écartées plutôt que commande refusée** : on garde ce qui reste utilisable, on dit ce qui a été laissé de côté.
- **L'`id` n'est pas exporté et toujours regénéré** (relecture) : l'aller-retour redonne les mêmes titres, phrases, raccourcis et réglages de sélection, avec des `id` neufs.

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

Essai réel des étapes 1 à 3 sur le code du commit `58e9898` (non publié). **Mac : fait à la main** (l'auteur a fait les gestes des boîtes de fichier). **Linux : fait par `pilote-pc`**, sans geste de l'auteur. **Windows : fait par `pilote-pc`**, l'auteur faisant les gestes des boîtes de fichier (le PC a d'abord été injoignable, puis allumé sur sa demande).

### Méthode (et pourquoi)

- **Données jetables** : variable `SCRIIBE_DATA` vers un dossier d'essai, jamais le dossier de l'auteur. Sur le Mac, une copie de l'application fabriquée hors de `~/Applications` (`SCRIIBE_INSTALL=…`), lancée par `open -n --env SCRIIBE_DATA=…` (règle : Scriibe.app se lance par `open`, jamais par Bash). Sur Linux, `scriibe lancer --sans-navigateur` sur `/tmp/essai-lot1`, page atteinte par un tunnel SSH ; l'instance habituelle du PC n'a pas été touchée. Sur Windows, même chose avec `C:\essai-lot1`, l'instance étant lancée **dans la session ouverte de l'auteur** par une tâche planifiée temporaire (`schtasks … /it`, supprimée ensuite) pour que les boîtes de fichier s'affichent devant lui ; arrêtée à la fin par son identifiant, après avoir vérifié son emplacement (`C:\scriibe-dev\target\release`, pas `C:\scriibe\`).
- **Code transmis aux PC sans rien publier** : paquet git (`git bundle`, 65 Ko) copié sur le PC Linux, `git merge --ff-only`, puis compilation par `pilote compiler linux --mode release`. Même procédé que pour le chantier 7.
- **Le serveur est piloté par ses routes** (`curl`) et par la page (volet navigateur du Mac) ; seules les boîtes « Enregistrer sous » et « Ouvrir » de macOS demandent l'auteur.

### Résultats

| Cas | Mac | Linux | Windows |
|---|---|---|---|
| `settings.json` et `dictionary.json` illisibles : copie `<nom>.<date>.bak` à la lecture, contenu identique, originaux intacts | ✔ | ✔ | ✔ |
| Avertissement : `config_problems` dans `api/status` (un par fichier, avec le nom de la copie) | ✔ | ✔ | ✔ |
| Bandeau rouge dans la page, avec le nom de la copie | ✔ (vu) | non vu (pas d'écran piloté) | non vu (état du serveur seulement) |
| Écriture refusée (dictionnaire et réglages : HTTP 400, message clair), fichiers intacts, aucun `.tmp` | ✔ | ✔ | ✔ |
| Réparation à la main : l'avertissement tombe tout seul, les copies restent | ✔ | ✔ | ✔ |
| `schemaVersion: 1` écrit dans `settings.json` | ✔ | ✔ | ✔ |
| Export du dictionnaire (4 mots, 3 corrections) : enveloppe v1 `dictionary`, rien d'autre | ✔ (boîte « Enregistrer sous ») | ✔ (dossier Documents) | ✔ (boîte « Enregistrer sous ») |
| Import du dictionnaire dans un dictionnaire vidé : aperçu 4 + 3, fichier identique à l'original (mots, corrections, ordre) | ✔ (boîte « Ouvrir ») | ✔ (chemin tapé) | ✔ (boîte « Ouvrir ») |
| Export puis import des commandes (2 commandes) : `id`, champs et ordre identiques | ✔ | ✔ | ✔ |
| Fichier d'un autre type refusé, message clair, attente oubliée | (tests automatiques) | ✔ | — |
| Aucun fichier temporaire laissé | ✔ | ✔ | ✔ |

### Constats

- **Rien à corriger dans le code du lot 1** : tous les cas essayés ont passé du premier coup.
- **Trousseau (macOS)** : la copie d'essai, dans un autre dossier, a redemandé l'accès aux clés. Tant que la fenêtre était ouverte, `api/status` restait bloqué (il lit les clés). Comportement connu (mémoire « trousseau »), pas un défaut du lot ; une session sur le Mac doit s'y attendre quand elle lance une copie d'essai.
- **Capture d'écran du Mac depuis la session** : noire (pas d'autorisation d'enregistrement d'écran pour cet outil) ; la vérification visuelle s'est faite dans le volet navigateur sur la page de l'instance.
- **Windows** : les boîtes proposent le dossier Téléchargements (les fichiers y sont allés) ; une première tentative de casser le dictionnaire l'a **vidé** par erreur de ma commande (`echo [1, 2>` redirige l'erreur) : le serveur a traité le fichier vide comme voulu (pas un problème, écriture possible), puis le cas « illisible » a été refait correctement. La tâche planifiée a d'abord échoué parce que le journal était écrit à la racine de `C:\` (droits) ; il va maintenant dans le dossier d'essai.
- **Linux** : l'export va dans `~/Documents` (comportement voulu, sans boîte) ; l'import prend le chemin tapé, `~/` développé. L'instance habituelle du PC tourne encore sur l'ancien programme (son fichier a été remplacé par la compilation) : à relancer à l'occasion.
- **Reste à essayer** : le bandeau sur Linux et Windows (état du serveur seulement) ; le refus d'un fichier d'un autre type sur Windows et le Mac (tests automatiques) ; un import avec **conflit** (correction du même « entendu », phrase déjà prise) et un second import du même fichier, couverts par les tests automatiques mais pas essayés à l'écran.

### Ajouts

Aucune nouvelle demande de l'auteur pendant l'essai.


## Relecture du lot 1 (tâche 5, 13/11/2026)

Relu par un agent distinct (Sonnet, lecture seule) sur `8171411..HEAD` : **aucun bloquant, 4 importants, 8 mineurs**. Corrigés avec l'accord de l'auteur : les 4 importants et 3 mineurs ; les 5 autres notés plus bas.

| # | Constat | Correction | Test |
|---|---|---|---|
| 1 (important) | `dictionary.json` refusé : l'enregistrement de la page répond une erreur, la page la prend pour un dictionnaire (`words` devient `undefined`) et le composant plante ; modification perdue sans message. **Régression de l'étape 1** que l'essai (fait par `curl`) n'avait pas vue. | `flush()` ne remplace plus le dictionnaire de la page par une réponse d'erreur, garde `dirty`, affiche le message ; le message du serveur n'est plus enveloppé quand le fichier est refusé (il est traduit). | `ui/src/lib/dictionary.test.ts` (`storedDictionary`, `saveError`) |
| 2 (important) | Un dictionnaire importé de 5 000 mots de 100 caractères gonflait le prompt du modèle (tous les mots y sont joints, `cleaner.rs`) : dictée bloquée, consignes cachées dans des « mots ». | Plafonds ci-dessus (500 mots, 10 000 caractères, 1 000 au total) ; l'avertissement de l'aperçu dit que les mots vont au modèle. | `import_dictionary.rs::the_words_of_a_file_cannot_swell_the_prompt`, `…an_import_stops_adding_words_once_the_dictionary_is_full_and_says_so` |
| 3 (important) | L'export des commandes écrivait leur `id` (heure de création à la milliseconde), contre l'esprit du cadrage (`exportedAt` réduit au jour). | `id` non exporté et toujours regénéré (voir étape 3). | `import_commands.rs` (aller-retour, id) ; `commands_io::tests::the_export_has_the_agreed_name_and_the_whitelisted_fields` |
| 4 (important) | Aperçu, fusion et `clean_dictionary` quadratiques (clé recalculée pour chaque élément en place). | Index calculé une fois (`HashSet`/`HashMap`), tenu à jour pendant la fusion ; `clean_dictionary` par ensemble. | `import_dictionary.rs::preview_and_merge_stay_fast_with_thousands_of_words` (< 2 s pour 5 000 + 2 000 en place) |
| 6 (mineur) | L'aperçu des commandes ne disait pas qu'une commande agit sans texte sélectionné. | Ligne d'avertissement dans l'aperçu (français et anglais). | pas de test de composant |
| 7 (mineur) | Un `id` `perso.nouveau` importé échappait au contrôle des phrases (identifiant réservé). | Conséquence du 3 : plus aucun `id` de fichier n'est gardé. | `import_commands.rs::a_colliding_id_is_regenerated_and_a_taken_name_is_numbered_without_piling_up` |
| 10 (mineur) | Une saisie faite pendant l'import pouvait être perdue. | `flush()` avant l'accord, champs d'ajout désactivés pendant la requête. | pas de test de composant |

**Notés, non corrigés** (à mettre au carnet) :
- **5** : `patch_settings` change `llama` (appareil, délais) avant l'écriture ; si le fichier est refusé, le comportement diverge du fichier jusqu'au redémarrage.
- **8** : un fichier vide est traité comme « pas de problème » ; une coupure de courant pendant une écriture (pas de `fsync`) pourrait en laisser un, et les réglages repartent aux valeurs par défaut sans avertissement.
- **9** : une erreur de lecture passagère (antivirus) déclenche la copie `.bak` d'un fichier bon, et `FileHistory::open` en copie sur toute erreur non-`NotFound`.
- **11** : `tray_source.rs` ignore l'échec d'écriture du menu de l'icône (aucun signe avec un fichier refusé).
- **12** : une copie `.bak` par contenu cassé différent, sans plafond ; les `.tmp` laissés par un plantage ne sont jamais nettoyés.

Mesuré après les corrections, sur le Mac : `cargo fmt` et `cargo clippy --all-targets -- -D warnings` sans erreur ; **604 tests Rust** (601 avant) ; **104 tests de l'interface** (102 avant) ; `svelte-check` sans erreur ; interface construite. Les corrections n'ont **pas été revues à l'écran** (ni la page Dictionnaire avec un fichier refusé, ni l'aperçu des commandes).

## Étape 4a · Vue des réglages, sans changement visible (tâche 6, 13/11/2026)

Code de l'étape 4a de `docs/CADRAGE_PROFILS.md` : le code lit et écrit les réglages par une vue unique, prête à être rangée par utilisateur, sans que rien ne change pour l'auteur tant qu'il n'y a pas de migration (4b).

### Ce qui est fait

- **Classement de la décision 1 dans le code** (`crates/core/src/settings.rs`) : `USER_KEYS` et `COMPUTER_KEYS` (les clés JSON d'`AppSettings`), `Owner`, `owner_of(clé)`. 17 clés suivent l'utilisateur, 39 restent à l'ordinateur (`schemaVersion` compris).
- **`AppSettings::split()`** range les réglages en deux objets JSON (ordinateur, utilisateur) ; **`AppSettings::from_parts(ordinateur, utilisateur)`** recompose la vue. Avec `Some(utilisateur)`, ce fichier fait foi pour toutes les clés de l'utilisateur (une clé absente prend sa valeur par défaut, une clé qui traîne dans le fichier de l'ordinateur est ignorée). Avec `None`, la vue est le `settings.json` d'origine, lu comme avant.
- **`FileConfig`** (`crates/platform/src/storage.rs`) :
  - `settings()` donne toujours une `AppSettings` unique (`view()`), donc `active_profile`, `preload_signature`, le menu de l'icône et `api/status` n'ont pas changé ;
  - un `Watched` par fichier : `settings.json`, `profile.json` et `dictionary.json` ; le chemin d'un `Watched` peut changer (`retarget`, qui vide sa mémoire sous le même verrou) ;
  - `switch_profile(Some(dossier))` fait suivre `profile.json` et `dictionary.json` ; `switch_profile(None)` revient au rangement d'origine ; `profile_folder()` dit où l'on en est ;
  - `save_settings` range chaque clé dans le fichier de son propriétaire (ordinateur → `settings.json`, utilisateur → `profile.json` du dossier de l'utilisateur) et refuse d'écrire si `settings.json` **ou** `profile.json` est refusé (illisible, ou plus récent pour `settings.json`) ; `problems()` liste aussi `profile.json` ; la copie `.bak` d'un profil illisible se fait comme pour les autres fichiers.
- **`merge_settings`** (`crates/cli/src/web/mod.rs`) accepte une clé si elle est classée (`owner_of`), au lieu de `OPTIONAL_KEYS` (supprimé : `customModeID` et `historyKeepDays` sont des clés de l'utilisateur). Le rangement dans le bon fichier se fait à l'écriture, par `FileConfig::save_settings`.
- **`FileHistory`** : l'historique et son dossier sont dans un même état sous un seul verrou ; `switch(dossier)` relit `history.json` et `history-totals.json` du nouveau dossier (même copie `.bak` d'un fichier illisible qu'à l'ouverture), sans rien reprendre de l'ancien (dernier texte dicté compris). Changer pour le même dossier ne recharge rien. Le `FileHistory` de `commands.rs` est partagé par la page, l'icône et le chef d'orchestre : un seul `switch` suffit.

### Choix techniques et pourquoi

1. **Sans dossier d'utilisateur, rien n'est écrit à part** (précision apportée au cadrage, qui dit « le profil est la racine du dossier de données »). Tant que `profile_folder()` vaut `None`, `save_settings` écrit tout dans `settings.json`, octet pour octet comme avant, et le dictionnaire reste à la racine ; aucun `profile.json` n'est créé. La séparation ne s'applique qu'après `switch_profile(Some(dossier))`, que seule la migration (4b) appellera. Pourquoi : un `profile.json` à la racine, créé dès la première sauvegarde, aurait retiré les clés de l'utilisateur de `settings.json` avant toute migration. Cela aurait (a) cassé la sauvegarde complète, qui ne connaît que `settings.json`, `dictionary.json` et l'historique (`restore::FILES`, `read_data_files`) ; (b) trompé la migration de 4b, qui lit ces clés dans `settings.json` ; (c) rendu visible un changement que l'étape doit éviter (retour à une version antérieure, modification à la main). Résultat : 4a ne change rien sur le disque, et la séparation est quand même complète et testée.
2. **Trois champs que le tableau du cadrage ne classait pas** (le test de classement les aurait fait échouer) : `hiddenModes` → utilisateur (il porte des identifiants de modes personnels et `clean_hidden_modes` a besoin de `customModes` : ils doivent venir du même fichier) ; `soundVolume` → ordinateur (avec les sons) ; `animatedLogo` → ordinateur (avec l'apparence). Les tableaux du cadrage sont complétés.
3. **Les fichiers sont lus comme du JSON brut et la vue se compose ensuite** (`Watched<Value>`), plutôt que de lire deux `AppSettings` et de les mélanger champ par champ : la règle « le fichier de l'utilisateur fait foi pour toutes ses clés » tient en quelques lignes, et une nouvelle clé n'oblige à écrire aucune ligne de fusion (le test de classement suffit). Coût : la vue est recomposée à chaque lecture au lieu d'être gardée en mémoire ; je ne l'ai pas mesuré.
4. **`profile.json` n'a pas de numéro de version propre** : le format de l'utilisateur sera fixé par 4b. Il est donc refusé s'il est illisible, jamais « trop récent ».
5. **Le classement est une liste de clés JSON, pas un attribut sur chaque champ** : un attribut aurait demandé une macro. Le test lit les clés réellement écrites par `serde` (avec les clés facultatives remplies), donc un champ ajouté sans classement fait échouer la suite avec le nom du champ.

### Tests (12 nouveaux)

| Test | Ce qu'il prouve |
|---|---|
| `every_settings_field_is_classified_as_user_or_computer` (core) | **Test de la décision 1** : échoue avec le nom du champ si un champ d'`AppSettings` n'est ni « utilisateur » ni « ordinateur » (vérifié à la main en retirant `animatedLogo` de la liste : le test échoue et le nomme) |
| `the_classification_has_no_overlap_and_no_stale_key` | aucune clé des deux côtés ; aucune clé classée qui n'existe plus |
| `the_decision_one_table_is_followed` | les lignes du tableau de la décision 1, une à une |
| `split_then_from_parts_gives_back_the_same_settings` | aller-retour exact, rien de l'ordinateur chez l'utilisateur et inversement |
| `without_a_user_file_the_original_settings_json_is_read_as_is` | sans utilisateur, la vue est le fichier d'origine |
| `a_user_file_decides_every_user_key_and_stray_ones_in_the_computer_file_are_ignored` | le fichier de l'utilisateur fait foi, même pour une clé facultative |
| `a_user_file_that_is_not_an_object_gives_no_view` | un profil qui n'est pas un objet ne donne pas de vue |
| `without_a_profile_folder_everything_stays_in_the_original_files` (platform) | rien de nouveau sur le disque : un seul `settings.json`, identique octet pour octet à l'ancienne écriture, pas de `profile.json` |
| `with_a_profile_folder_each_key_goes_to_its_owner_and_the_view_joins_them` | chaque clé dans le fichier de son propriétaire ; un utilisateur neuf a les valeurs par défaut |
| `switching_user_switches_the_settings_and_the_dictionary_and_none_reads_the_root_again` | deux utilisateurs isolés (réglages et dictionnaire), clés de l'ordinateur communes, retour à `None` |
| `a_hand_edit_of_the_profile_is_picked_up_and_an_unreadable_one_blocks_only_the_save` | modification à la main relue ; profil illisible : copie `.bak`, avertissement, sauvegarde refusée, fichier intact |
| `switching_history_swaps_the_whole_history_under_the_lock_and_keeps_each_on_its_own_disk` | `FileHistory.switch` |

### Résultat des tests

Mesuré sur le Mac : `cargo fmt --check` sans écart, `cargo clippy --workspace --all-targets -- -D warnings` sans erreur, `cargo test --workspace` : **616 réussis, 0 échoué, 1 ignoré** ; `npm test` (interface, non modifiée) : **104 réussis**. Non fait : la compilation croisée pour Windows (clippy `x86_64-pc-windows-gnu`) et l'essai sur les PC : 4a ne change rien d'observable, l'essai viendra avec 4b et 4c.

### Limites et à reprendre (pour 4b et 4c)

- **4b (migration)** : construire `profile.json` et `dictionary.json` de l'utilisateur **avant** le premier `switch_profile(Some(…))`, puis appeler `config.switch_profile(Some(dossier))` et `history.switch(dossier)`. Dès que l'utilisateur est actif, la première `save_settings` retire les clés de l'utilisateur de `settings.json` : si la migration lisait `settings.json` après cela, elle ne les y trouverait plus. La migration doit lire la vue (`config.settings()`) ou les copies de la sauvegarde, jamais `settings.json` après une sauvegarde faite avec un utilisateur.
- **4b : `activeProfile`** n'est pas encore un champ d'`AppSettings`. Quand il le deviendra, le test de classement l'obligera à le classer (« ordinateur », cadrage).
- **4b et 4c : sauvegarde complète et désinstallation** (`restore::FILES`, `backup::read_data_files`, `files_to_restore`, `uninstall`) ne connaissent ni `utilisateurs/` ni `profile.json`. Elles marchent aujourd'hui parce que le rangement d'origine est conservé ; elles devront être étendues dans la même étape que la migration, sinon une sauvegarde faite après migration manquerait les clés de l'utilisateur.
- **4c** : le verrou en mémoire qui protège « lire, fusionner, écrire » n'est pas encore là (cadrage, décision 3). `save_settings` écrit maintenant deux fichiers quand un utilisateur est actif (l'ordinateur d'abord, puis l'utilisateur) : un arrêt entre les deux laisse un fichier à jour et l'autre non, sans fichier cassé. Le garde-fou 409 et le rechargement de la page après `switch_profile` sont aussi à faire.
- Notés à la relecture du lot 1 et toujours non corrigés : voir § Relecture du lot 1.

## Étape 4b · Migration vers le rangement par utilisateur (tâche 7, 14/11/2026)

Code de l'étape 4b de `docs/CADRAGE_PROFILS.md` (décision 6). **Jamais lancée sur le vrai dossier de données** : tous les essais se font sur des dossiers temporaires ; la vraie migration se fera à l'essai du lot 2 (tâche 11), après une sauvegarde datée.

### Ce qui est fait

- **`crates/platform/src/migration.rs`** (nouveau) : `run(dossier)` fait la migration et rend `Report { profile, messages }` ; `state(dossier)` dit où l'on en est sans rien écrire (pour les sous-commandes).
  - Étape 1 : copie de `settings.json`, `dictionary.json`, `history.json`, `history-totals.json` dans `sauvegarde-avant-utilisateurs-<date>/` (puis `-2`, `-3`… : une sauvegarde n'est jamais écrasée ni effacée). Les copies sont faites dans un dossier caché, écrites jusqu'au disque (`sync_all`), puis le dossier est renommé : une sauvegarde visible est toujours complète. Si elle échoue (disque plein, droits), on s'arrête : rien n'a changé.
  - Étape 2 : `utilisateurs/.migration-<id>/` reçoit `profile.json` (les clés de l'utilisateur **telles qu'elles sont dans le fichier**, plus `name` = `firstName`, ou « Principal ») et des **copies** des trois fichiers de l'utilisateur.
  - Étape 3 : renommage en `utilisateurs/<id>/` (identifiant opaque, `modes::new_id`).
  - Étape 4 : `settings.json` réécrit sans les clés de l'utilisateur, avec `schemaVersion: 2` et `activeProfile`, par une écriture jusqu'au disque (temporaire, `sync_all`, renommage, dossier).
  - Étape 5 : seulement après, les trois fichiers de l'utilisateur quittent la racine.
- **Marque « migré »** : `SETTINGS_SCHEMA` passe à 2. Un fichier sans numéro **et les réglages par défaut** sont en version 1 (`LEGACY_SCHEMA`) : seule la migration écrit 2, aucune sauvegarde ordinaire ne peut faire croire à une migration.
- **`activeProfile`** devient un champ d'`AppSettings` (ordinateur, classé ; facultatif, absent tant qu'il n'y a pas d'utilisateur) ; lu seulement s'il a la forme d'un identifiant sûr (`is_profile_id` : lettres, chiffres, « - », 64 caractères au plus ; ni `..`, ni `/`, ni dossier caché). La page ne peut le changer (`merge_settings` l'ignore, comme `schemaVersion`).
- **Lancement** (`commands::run`) : le dossier de données est créé, **puis le verrou d'instance est pris**, puis la restauration en attente est appliquée, puis la migration tourne ; ensuite seulement `config.switch_profile(…)`, la création des réglages d'un premier lancement (devenue inutile après une migration réussie), `migrate_engine_options`, le journal (messages de la restauration et de la migration) et `FileHistory` (ouvert dans le dossier de l'utilisateur).
- **Sous-commandes** (`essai`, `verifier`) : plus de restauration, de création ni de migration (pas de verrou). Dossier migré : elles lisent l'utilisateur actif ; sinon elles s'arrêtent sur « lance Scriibe une fois (sans sous-commande), puis recommence ». Noté dans `docs/ESSAI_MACOS.md` et `docs/ESSAI_WINDOWS.md`.
- **`profile.json` garde son nom affiché** : `FileConfig::save_settings` ne remplace que les clés de l'utilisateur et garde les autres (`name`).
- **Sauvegarde complète** (la limite notée à l'étape 4a) : elle garde **le même format** avant et après la migration. À l'export, `settings` est la vue jointe (`migration::joined_settings` : clés de l'ordinateur de `settings.json`, clés de l'utilisateur de `profile.json`) et le dictionnaire et l'historique viennent du dossier de l'utilisateur actif ; un `profile.json` illisible fait refuser la sauvegarde. À la restauration dans un dossier migré, les réglages sont partagés (`migration::split_for_profile` : `settings.json` garde `schemaVersion: 2` et `activeProfile`, `profile.json` garde le nom affiché) et `restore::stage` note l'utilisateur dans la marque (`utilisateur=<id>`, contrôlé) : `apply_pending` remplace ses fichiers dans `utilisateurs/<id>/` après en avoir copié l'état actuel. Une marque qui vise un utilisateur disparu ou un identifiant forgé ne remplace rien.
- La **désinstallation** n'a pas besoin de changer : elle reconnaît le dossier de données à `settings.json` (resté à la racine) et l'envoie entier à la Corbeille.

### Reprises (tableau de la décision 6) et cas voisins

| Ce que trouve le lancement | Ce que fait Scriibe | Test |
|---|---|---|
| Un dossier `.migration-*` (arrêt après l'étape 1, pendant ou après l'étape 2) | L'efface et recommence ; une deuxième sauvegarde est faite, la première est gardée | `row_1_a_stop_before_step_3_…` (3 arrêts) |
| Un dossier caché de sauvegarde inachevée | L'efface ; les sauvegardes complètes restent | `row_1_an_interrupted_backup_…` |
| Un utilisateur complet, `settings.json` sans `schemaVersion >= 2` (arrêt entre 3 et 4) | Sauvegarde, puis reprend cet utilisateur au lieu d'en créer un deuxième | `row_2_a_stop_between_steps_3_and_4_…` |
| Idem avec plusieurs utilisateurs, sans `activeProfile` | Le dernier utilisé (dernière écriture dans son dossier ; à égalité, le plus petit identifiant) | `row_2_with_several_users_…` |
| Idem, `activeProfile` encore là | Celui-là, même s'il n'est pas le dernier utilisé | `row_2_the_noted_active_user_wins_…` |
| `schemaVersion: 2` (arrêt après l'étape 4, ou pendant l'étape 5) | Rien ; les fichiers restés à la racine sont effacés (ils ont une copie identique dans la sauvegarde) | `row_3_after_step_4_…` (2 arrêts) |
| `schemaVersion: 2` et un fichier à la racine **sans** copie identique | Mis de côté dans une nouvelle sauvegarde, pas effacé | `row_3_a_root_file_without_an_identical_copy_…` |
| Retour d'une version antérieure (marque et `activeProfile` perdus, champs de l'utilisateur vides, dictionnaire et historique créés à la racine) | Reprend l'utilisateur existant, garde les champs de l'ordinateur changés entre-temps ; ce qui a été créé à la racine part dans une nouvelle sauvegarde | `back_from_an_older_version_…` |
| Dossier de Scribe copié **après** la migration | Pas de fusion ; l'utilisateur est intact, les fichiers de Scribe sont dans la sauvegarde | `a_scribe_folder_copied_after_…` |
| Dossier de Scribe copié **avant** la migration | Migré comme l'ancien rangement (« Principal » si le prénom est vide) | `a_scribe_folder_copied_before_…` |
| Premier lancement (dossier vide) | Un utilisateur « Principal », réglages d'un premier lancement, pas de sauvegarde (rien à copier) ; les enregistrements suivants vont au bon endroit | `a_first_launch_…` |
| Sauvegarde impossible (dossier en lecture seule) | Arrêt, rien n'a changé, l'ancien rangement sert pour ce lancement | `a_backup_that_cannot_be_written_…` |
| Utilisateur impossible à construire | Rien n'a changé sauf la sauvegarde ; réussi au lancement suivant | `a_user_that_cannot_be_built_…` |
| `settings.json` illisible ou d'une version plus récente (3) | Rien n'est écrit ni déplacé | `an_unreadable_or_newer_settings_file_…` |
| Migré, utilisateur actif disparu ; puis plus aucun utilisateur | Un autre le remplace (noté) ; sans utilisateur, un utilisateur est refait avec ce qui reste à la racine | `a_missing_active_user_…` |

Chaque test compare, après la reprise, l'état complet à celui d'avant la migration (`assert_migrated`) : un seul utilisateur ; `settings.json` avec la marque, l'utilisateur actif et tous les champs de l'ordinateur (inconnus compris) ; `profile.json` avec les champs de l'utilisateur identiques ; dictionnaire et historique identiques à l'octet près ; plus rien à la racine ; une sauvegarde fidèle ; **la vue des réglages, le dictionnaire et l'historique lus par l'application identiques à avant**. Un arrêt brutal est simulé en arrêtant `run_until` après l'étape voulue (paramètre `Stop`, privé : l'application va toujours au bout).

Autres tests : la migration faite, un deuxième lancement ne réécrit rien (arbre du dossier identique) ; `state` n'écrit rien ; `joined_settings` et `split_for_profile` ; `restore` dans un dossier migré et marque forgée ; sauvegarde puis restauration dans un dossier migré (`backup.rs`) ; `activeProfile` lu seulement s'il est sûr ; la page ne change pas l'utilisateur actif ; découpage brut d'un objet de réglages.

### Choix techniques et pourquoi

1. **Migration ratée = l'ancien rangement pour ce lancement**, plutôt que refuser de démarrer. Le cadrage dit « Scriibe s'arrête ici » à l'étape 1 : je le lis comme « la migration s'arrête », parce que rien n'a changé et qu'une appli de dictée qui ne démarre plus est pire ; elle réessaie au lancement suivant, le journal le dit. Une fois l'utilisateur renommé (étape 3), il sert dès ce lancement même si l'étape 4 échoue : le lancement suivant tombe sur la ligne 2 du tableau.
2. **Les fichiers restés à la racine après l'étape 4 ne sont effacés que s'ils ont une copie identique dans une sauvegarde** (le cadrage dit « simplement effacés ») : après un arrêt pendant l'étape 5, c'est toujours le cas ; mais un fichier écrit là après coup (réglages illisibles pendant un lancement, copie à la main) serait perdu. Il est mis de côté dans une nouvelle sauvegarde.
3. **Ligne 2 : une nouvelle sauvegarde avant de réécrire `settings.json`.** Au retour d'une version antérieure ou après une copie de Scribe, `settings.json` et les fichiers de la racine contiennent des choses nouvelles qui ne sont pas fusionnées (cadrage) : elles doivent rester quelque part.
4. **« Le dernier utilisé » = la dernière écriture dans le dossier de l'utilisateur** (`profile.json`, dictionnaire, historique), quand `activeProfile` a été perdu. L'historique s'écrit à chaque dictée ; aucun fichier de plus n'est nécessaire.
5. **Copie brute** des clés de l'utilisateur (`split_object`), sans passer par `AppSettings` : rien n'est relu ni normalisé, une valeur que Scriibe lit mal n'est pas perdue en route.
6. **Écritures jusqu'au disque** (`sync_all` sur les fichiers et les dossiers, sous Unix) pour la migration seulement : elle efface ensuite des fichiers, elle ne peut pas laisser un fichier vide après une coupure de courant (point noté à la relecture du lot 1). Les sauvegardes ordinaires ne changent pas.
7. **`profile.json` sans numéro de version** : le numéro de `settings.json` vaut pour tout le dossier.
8. **Version 2 refusée par une Scriibe du lot 1** : les versions publiées (0.4.0 et avant) ne connaissent pas `schemaVersion` et lisent le fichier v2 sans erreur, comme le cadrage le prévoit ; le lot 1, non publié, part dans la même version (0.5.0) que la migration.

### Résultat des tests

Mesuré sur le Mac : `cargo fmt --check` sans écart, `cargo clippy --workspace --all-targets -- -D warnings` sans erreur, `cargo test --workspace` : **640 réussis, 0 échoué, 1 ignoré** (24 nouveaux : 18 pour la migration, 2 pour la restauration, 1 pour la sauvegarde, 1 pour la page, 2 pour le cœur) ; `npm test` (interface, non modifiée) : **104 réussis**. Non fait : la compilation croisée pour Windows (`x86_64-w64-mingw32-gcc` absent du Mac : rien installé sans accord ; la CI Windows le fera au push) ; aucun essai de l'application elle-même (prévu à la tâche 11).

### Limites et à reprendre

- **Aide (FR et EN) et CHANGELOG** : le dossier `sauvegarde-avant-utilisateurs-<date>/`, comment revenir à une version antérieure (remettre la sauvegarde à la main), la limite « dossier de Scribe copié après la migration : pas de fusion ». À écrire en 4c, avec les utilisateurs visibles (tout part ensemble dans la 0.5.0).
- **Le journal seul signale une migration ratée** : pas encore de bandeau dans la page.
- **4c** : verrou en mémoire, garde-fou 409, route de changement d'utilisateur (qui écrira `activeProfile`), `name` montré et modifiable.
- La restauration d'une sauvegarde préparée par une version **non migrée** puis appliquée par une version migrée remet les fichiers à la racine ; la migration les traite alors comme un retour d'une version antérieure (pas de fusion, tout dans la sauvegarde). Cas improbable (la restauration relance la même version).

## Relecture de la migration (tâche 8, 14/11/2026)

Relecture du commit de l'étape 4b par un sous-agent (Sonnet, effort Élevé) qui ne l'avait pas écrit, en lecture seule. Il a lu `migration.rs`, ses tests, `restore.rs`, `backup.rs`, le lancement et la décision 6, et lancé les tests de la migration et de la restauration. J'ai vérifié chacun de ses constats dans le code avant de corriger. **Verdict du relecteur** : pas de perte de données trouvée ; trois points à corriger avant l'essai réel. **Aucune décision du cadrage ne change.**

| Constat | Gravité | Suite |
|---|---|---|
| Aucun bloquant : rien n'est effacé ou écrasé sans copie complète ailleurs ; la sauvegarde passe avant tout déplacement, reprise comprise ; noms lus dans les fichiers contrôlés | — | — |
| **I1.** Un deuxième utilisateur, vide, pouvait être créé si `utilisateurs/` était mal lu (erreur passagère prise pour « aucun utilisateur ») ou si l'utilisateur avait perdu son `profile.json` | Important | **Corrigé** : `scan_users` réessaie la lecture puis, en cas d'échec, rien n'est touché ; un dossier d'identifiant valide qui a un dictionnaire ou un historique mais pas de `profile.json` est réparé sur place (nom « Principal », données gardées) ; un utilisateur n'est recréé que si `utilisateurs/` ne contient aucun dossier inconnu non vide |
| **I2.** Sous Windows, un renommage de dossier juste après l'écriture échoue souvent (antivirus, indexeur), sans nouvel essai ; chaque nouvel essai refaisait une sauvegarde complète (`-2`, `-3`…) | Important | **Corrigé** : les deux renommages de dossier (et la lecture de `utilisateurs/`) sont réessayés 5 fois à 200 ms ; une sauvegarde existante qui a déjà tous les fichiers à l'identique resert au lieu d'être refaite |
| **I3.** Restauration : un arrêt pendant la copie de « l'état avant import » laissait une copie tronquée que la reprise prenait pour complète, puis l'original était remplacé (défaut d'avant 4b, étendu par 4b aux fichiers de l'utilisateur) | Important | **Corrigé** : copie sous un nom temporaire puis renommage |
| **I4.** Tests manquants | Important | **Ajoutés** (voir ci-dessous) ; pas de test Windows : la CI Windows le fera au push |
| M1. Fichier temporaire d'une réécriture de `settings.json` interrompue jamais effacé | Mineur | **Corrigé** |
| M3. `activeProfile` comparé avec les majuscules (macOS et Windows ne les distinguent pas) ; un dossier d'utilisateur en lien symbolique accepté par la restauration | Mineur | **Corrigé** (comparaison sans les majuscules ; lien refusé). Restreindre l'identifiant à un UUID : non, inutile tant que seul Scriibe les crée |
| M6. Message « profile.json illisible » quand c'est `settings.json` | Mineur | **Corrigé** |
| M7. Messages de la migration en double à l'écran | Mineur | **Corrigé** (le journal les recopie déjà) |
| M2. `settings.json` illisible : l'utilisateur ouvert est le dernier utilisé, pas forcément l'actif (plusieurs utilisateurs) | Mineur | Pour 4c |
| M4. « Dernier utilisé » par date de modification (peu fiable sur FAT, copie de dossier) | Mineur | Pour 4c (choix technique 4 de 4b, acceptable avec un seul utilisateur) |
| M5. Les sauvegardes de la migration gardent l'historique pour toujours, sans tenir compte de « garder le texte » | Mineur | Conforme au cadrage ; à dire dans l'Aide (4c), avec le fait que la désinstallation les envoie à la Corbeille |
| M8. 4c doit créer un utilisateur par dossier de travail puis renommage (sinon I1 revient) | Mineur | Noté pour 4c |
| M9. `"schemaVersion": "2"` lu comme version 1 | Mineur | Laissé : retombe sur la ligne 2 du tableau, sans danger |
| M10. Écritures ordinaires sans `sync_all` (`write_atomic`, d'avant 4b) | Mineur | Déjà noté à la relecture du lot 1 |

### Tests ajoutés (9)

`an_unreadable_users_folder_never_makes_a_second_user`, `a_user_without_its_profile_file_is_repaired_in_place`, `unknown_folders_in_the_users_folder_stop_a_rebuild` (I1) ; `a_missing_or_empty_settings_file_takes_the_existing_user_back`, `after_a_failed_step_4_what_the_app_saved_is_kept_on_the_next_launch`, `a_pending_restore_from_the_old_layout_is_applied_then_migrated` (I4) ; `a_temporary_file_left_by_an_interrupted_rewrite_is_removed` (M1) ; `the_active_user_is_found_whatever_the_case` (M3) ; `an_interrupted_copy_of_the_current_state_is_redone_in_full` (I3, restauration). Le test de la ligne 1 vérifie maintenant qu'aucune sauvegarde n'est faite en double (I2).

### Résultat des tests

Mesuré sur le Mac : `cargo fmt --check` sans écart, `cargo clippy --workspace --all-targets -- -D warnings` sans erreur, `cargo test --workspace` : **649 réussis, 0 échoué, 1 ignoré**. Non fait : la compilation croisée pour Windows (outil absent du Mac).

## Étape 4c · Utilisateurs visibles (tâche 9, 14/11/2026)

Code de l'étape 4c de `docs/CADRAGE_PROFILS.md` (décision 3) : la fonction elle-même. L'interface dit « utilisateur », le code dit `profile`. **Essayée dans l'application : non** (c'est la tâche 11) ; vérifiée par les tests ci-dessous, `svelte-check` et la construction de l'interface.

### Ce que l'on voit

- **Réglages → Dictée, carte « Utilisateurs »** (en tête de l'onglet) : la liste, l'utilisateur actif marqué « actif », et pour chaque ligne « Utiliser » (recharge la page), « Renommer » (en place : Entrée enregistre, Échap annule), « Supprimer » (confirmation). En bas : un nom, une case « Copier l'utilisateur actif (réglages de personne et dictionnaire, sans historique ni statistiques) » et « Créer un utilisateur ». Sans utilisateur rangé à part (migration ratée, voir 4b), la carte le dit au lieu de proposer quoi que ce soit.
- **Icône** (barre des menus du Mac, zone de notification de Windows, barre de GNOME) : sous-menu « Utilisateur : Anne » qui coche l'actif, placé juste avant « Mode ». **Absent tant qu'il n'y a pas deux utilisateurs** : une personne seule ne voit aucun changement.
- **Tableau de bord** : salue l'utilisateur actif (voir choix 9) et montre ses statistiques (l'historique suit l'utilisateur) ; avec au moins deux utilisateurs, une ligne « Utilisateur : Anne · changer » mène à la carte.
- **Aide FR et EN** (Premiers pas) : « Plusieurs utilisateurs » et « Les utilisateurs et les versions précédentes de Scriibe » (dossier `sauvegarde-avant-utilisateurs-<date>`, comment revenir à une version antérieure à la main, dossier de Scribe copié après la migration, `utilisateurs-supprimes`, sauvegardes qui gardent l'historique, sauvegarde complète = utilisateur actif).

### Ce que fait le code

- **`crates/platform/src/profiles.rs`** (nouveau) : `list`, `create` (vide ou copie), `rename`, `delete`, `clean_name`, sur le dossier de données. Aucune de ces fonctions ne connaît l'utilisateur actif ni le verrou ; `FileConfig` les enveloppe.
- **`FileConfig`** (`storage.rs`) : `active_profile_id`, `active_profile_name`, **`lock_edit`** et **`lock_for(identifiant)`** (le verrou en mémoire de la décision 3, avec le contrôle de l'utilisateur actif sous le même verrou), `list_profiles` / `cached_profiles` (liste gardée 2 s pour le menu, qui se relit deux fois par seconde), `create_profile`, `copy_profile`, `rename_profile`, `delete_profile`, **`switch_to_profile(id, suite)`**.
- **Route de changement** `POST api/profiles/switch` (et le clic du menu) : écrit `activeProfile` dans `settings.json` (jamais par `POST api/settings`, que `merge_settings` ignore pour cette clé), relit `profile.json` et le dictionnaire dans le dossier de l'utilisateur, change le dossier de l'historique **dans la même section verrouillée** (la suite `suite` de `switch_to_profile`), abandonne les imports et la restauration qui attendaient un accord (`profiles_io::cancel_pending`). Le préchargement du modèle se relance tout seul : `preload_signature` lit la vue fusionnée (décision 3, délai de stabilisation de 1,5 s).
- **Autres routes** : `GET api/profiles` (liste, utilisateur actif, longueur maximale d'un nom), `POST api/profiles/create` / `rename` / `delete` (réponse : la liste à jour). `api/status` porte `profile` (`{id, name}` ou `null`) et `profile_count`.
- **Garde-fou 409** (`web/profiles_io.rs`, `web/mod.rs`) : la page envoie `profile` (l'utilisateur qu'elle croit actif) avec chaque requête ; si ce n'est plus lui, le serveur répond 409 `{"stale": true}` **sans rien écrire**, et la page se recharge avec un message. Contrôle et écriture sous le même verrou. Requêtes gardées : `POST api/dictionary` ; `POST api/settings` **quand le changement touche une clé de l'utilisateur** (dont `customModes`, `customCommands`), pas pour un réglage de l'ordinateur ; `modes/choose` ; les trois imports (`dictionary`, `modes`, `commands` `/import/apply`) ; `history/clear` et `history/apply` ; `backup/import/apply` ; la copie d'un utilisateur. Le menu de l'icône envoie lui aussi l'utilisateur qu'il montrait quand on choisit un mode (`choose_mode(profile, id)`), sous `lock_for`.
- **Menu** : `MenuView` porte `profile_id`, `profile_title`, `profiles` ; `TrayMenuSource::choose_profile` ; `tray_source::with_profiles` construit le sous-menu. Les trois menus (Mac et Linux : `mac_tray.rs`, Windows : `windows/tray.rs`) insèrent ou retirent le sous-menu selon le nombre d'utilisateurs, avant « Mode ».
- **Page** : `api.ts` ajoute `profile` aux corps de requête (`withProfile`) et recharge sur 409 `stale` (`STALE_FLAG` en `sessionStorage` : la page rechargée prévient que ce qui venait d'être fait n'a pas été enregistré) ; `status.svelte.ts` recharge la page si l'utilisateur actif change ailleurs (relevé toutes les 8 s) ; `components/Users.svelte` est la carte.

### Choix techniques et pourquoi

1. **Verrou explicite, tenu par l'appelant** (`lock_edit` / `lock_for`), plutôt que pris par `save_settings`. La suite qui compte est « lire, fusionner, écrire » : un verrou pris seulement dans `save_settings` n'empêcherait pas qu'un changement d'utilisateur s'intercale entre la lecture et l'écriture. Le verrou n'est pas réentrant : `save_settings` et `save_dictionary` ne le prennent donc pas. Ordre unique : verrou des modifications, puis verrou de l'historique (le changement d'utilisateur et les routes de l'historique le suivent tous).
2. **`activeProfile` forcé à la sauvegarde** : `save_settings` écrit l'identifiant du dossier réellement suivi, pas celui de la copie des réglages qu'on lui donne. Une copie lue avant un changement d'utilisateur ne peut pas remettre l'ancien dans `settings.json`. (Le test 4a `with_a_profile_folder_…` attend maintenant `active_profile: Some("A")` : c'est ce changement voulu.)
3. **Le garde-fou va plus loin que les blocs entiers** du cadrage : effacer l'historique ou le purger, restaurer une sauvegarde, appliquer un import et choisir un mode chez un autre utilisateur que celui que la page montre sont irréparables ou trompeurs. Même mécanisme, mêmes tests.
4. **Clé `profile` absente = pas de contrôle ; `null` = « aucun utilisateur ».** La page ne connaît l'utilisateur actif qu'après son premier `api/status` : avant, ses requêtes n'en disent rien et ne sont pas refusées (sinon une page qui n'a pas pu le lire serait refusée sans fin). Après, `api()` l'ajoute **à toutes** les requêtes POST, ce qui évite d'oublier une route ; les routes qui ne contrôlent pas l'ignorent.
5. **Créer ne change pas d'utilisateur.** On crée, puis on clique « Utiliser » : changer sans le vouloir est ce que la décision 3 cherche à éviter (pas de raccourci clavier pour la même raison).
6. **L'utilisateur actif ne se supprime pas** (la page grise le bouton, le serveur refuse). Cela garantit qu'il en reste toujours un et évite de choisir à la place de la personne celui qui prendra le relais. Supprimer = déplacer dans `utilisateurs-supprimes/<id>-<date>/` (jamais effacé, jamais écrasé : `-2`, `-3`…).
7. **Changement refusé pendant une dictée** (`controller.is_recording() || is_busy()`), pour la page comme pour le menu : la dictée serait enregistrée dans l'historique de l'autre.
8. **Noms** : 40 caractères au plus, sans caractère de commande, **uniques sans tenir compte des majuscules** (deux « anne » dans un menu ne se distingueraient pas). Le nom est la clé `name` de `profile.json` ; renommer ne touche à aucune autre clé et refuse un `profile.json` illisible.
9. **Salutation** : avec un seul utilisateur, rien ne change (prénom choisi, sinon prénom du compte). Avec plusieurs et un prénom vide, le Tableau de bord salue **le nom de l'utilisateur actif** (le prénom du compte serait le même pour tous). Le login de l'ordinateur n'est jamais un nom d'utilisateur (décision 6).
10. **Création dans un dossier de travail caché** (`utilisateurs/.creation-<id>/`, puis renommage), comme la relecture de la migration l'a demandé (M8) : un arrêt brutal ne laisse pas un dossier d'identifiant valide à moitié rempli, que la migration prendrait pour un utilisateur « réparable ». Les restes sont effacés à la création suivante.
11. **Copie** : `profile.json` en entier (donc modes, commandes, mode choisi, prénom, langues…) et `dictionary.json` ; **jamais** `history.json` ni `history-totals.json`. Un `profile.json` illisible empêche la copie (une copie serait un utilisateur vide sans le dire).
12. **Page rechargée** après « Utiliser » (`location.reload()`), plutôt que de rafraîchir chaque onglet un à un : dictionnaire, modes, historique, statistiques, commandes et réglages viennent tous de l'utilisateur actif. Coût : un rechargement, rare.
13. **Menu** : l'identifiant montré est gardé à chaque relevé de la vue (toutes les 500 ms) et renvoyé avec un clic sur un mode ; après un clic sur un utilisateur ou un mode, le menu se remet d'accord tout de suite (le menu natif a déjà coché la case).
14. **Pas d'export proposé avant la suppression** (le cadrage le prévoyait) : l'export d'un utilisateur entier est l'étape 5 (tâche 10) ; la confirmation dit où le dossier est mis de côté et comment le reprendre. À ajouter à l'étape 5.

### Tests (35 nouveaux côté Rust, 5 côté interface)

| Où | Test | Ce qu'il prouve |
|---|---|---|
| `profiles.rs` | 11 tests | noms (vide, trop long, caractères de commande, 40 caractères pas 40 octets) ; liste (dossiers cachés, identifiants refusés, profils sans nom ou illisibles, tri) ; utilisateur vide ; copie sans historique ni statistiques ; nom pris sans tenir compte des majuscules ; copie d'un profil illisible ; dossier de travail et restes effacés ; renommage qui garde les autres clés ; profil illisible jamais réécrit ; suppression mise de côté, jamais effacée, deux le même jour sans écrasement, l'actif refusé |
| `profiles_config_tests.rs` | 7 tests | `lock_for` (casse, autre utilisateur, aucun, sans utilisateur) ; changement (fichiers de l'autre utilisateur, `activeProfile`, la suite reçoit le dossier, la migration suivante retrouve le bon, retour) ; utilisateur inconnu ou identifiant forgé (`../..`) et `settings.json` illisible sans effet ; sauvegarde d'une copie périmée qui ne remet pas l'ancien ; listes et nom actif relus après création, renommage, suppression ; copie contrôlée par `given` ; **course** : un fil écrit chez Alice sous `lock_for` pendant qu'un autre change d'utilisateur 300 fois, aucun mot d'Alice chez Bruno (vérifié à la main : sans le verrou, le test échoue, échoué 1 fois sur 3 essais) |
| `web/profiles_io.rs` | 7 tests | `given` (absent, null, identifiant) ; `guard` ; liste ; création et copie (copie refusée par une page périmée, nom pris, nom vide) ; renommage et suppression (confirmation, actif refusé, dossier mis de côté) ; changement (réglages, dictionnaire, historique de chacun) ; changement refusé pendant une dictée |
| `web/mod.rs` | 4 tests | `settings_guard` (clé de l'utilisateur contrôlée, réglage de l'ordinateur non, clé `profile` retirée) ; dictionnaire d'une page périmée refusé sans rien écrire ; mode choisi par une page périmée refusé ; salutation |
| `tray_source.rs` | 2 tests | pas de sous-menu pour une personne seule mais l'identifiant part quand même ; à partir de deux, utilisateur coché (casse ignorée), titre FR et EN, vues différentes |
| `mode_sync.rs` | 4 tests | mode cliqué dans un menu qui montrait l'autre utilisateur : refusé ; changement par le menu (actif, titre, cases, mode de chacun) ; refus pendant une dictée ; pas de sous-menu pour un seul utilisateur |
| `ui/src/lib/api.test.ts` | 5 tests | `withProfile`, l'envoi dans les POST seulement, 409 `stale` qui recharge et pose la marque, un autre 409 qui ne recharge pas |

Les tests d'Aide (`i18n.test.ts`, `tabs.test.ts`) tiennent l'Aide anglaise à la structure de la française et toutes les chaînes traduites.

### Résultat des tests

Mesuré sur le Mac : `cargo fmt --all --check` sans écart, `cargo clippy --workspace --all-targets -- -D warnings` sans erreur, `cargo test --workspace` : **684 réussis, 0 échoué, 1 ignoré** (649 avant : 35 de plus) ; `npm test` : **109 réussis** (104 avant) ; `svelte-check` : 0 erreur, 0 avertissement ; `npm run build` réussi.

**Non fait** : (a) la compilation du code Windows (`windows/tray.rs`, le sous-menu) : `x86_64-w64-mingw32-gcc` est absent du Mac, la CI Windows le fera au push ; (b) aucun essai de l'application, ni du menu de l'icône sur les trois systèmes, ni de la page à l'écran (tâche 11) ; (c) le test de l'abandon des imports en attente au changement d'utilisateur : leurs états sont des variables globales de leurs modules, que des tests lancés en parallèle se disputeraient ; la ligne `cancel_pending()` est relue, pas testée.

### Limites et à reprendre

- **Tâche 10 (étape 5)** : ajouter l'export proposé avant la suppression (choix 14), et revoir `kind: "profile"` : l'export ne doit jamais porter l'historique, qui suit le dossier de l'utilisateur.
- **Fenêtre restante au changement** : entre la vérification « pas de dictée en cours » et le changement, une dictée peut démarrer ; elle serait enregistrée chez le nouvel utilisateur (le verrou protège les réglages, pas l'enregistrement du chef d'orchestre). Très étroite ; non fermée.
- **Menu : un clic refusé est muet** (dictée en cours, `settings.json` refusé) ; il est noté au journal et le menu se remet d'accord.
- **M2 et M4 de la relecture de la migration restent ouverts** : si `settings.json` est illisible au lancement, c'est « le dernier utilisé » (date de modification) qui s'ouvre, pas forcément l'actif ; sur une copie de dossier ou un système de fichiers FAT, cette date est peu fiable. Rare (il faut perdre `activeProfile`), mais plus probable maintenant qu'il y a plusieurs utilisateurs : à reprendre si l'essai le montre.
- **Sauvegarde complète et désinstallation** : la sauvegarde ne contient que l'utilisateur actif (dit dans l'Aide) ; les sauvegardes de la migration gardent l'historique entier (dit dans l'Aide).
- **Pas de bandeau global** si le rangement par utilisateur échoue au lancement : seule la carte « Utilisateurs » et le journal le disent.
- **Le nom « Principal »** n'est pas traduit (c'est un nom stocké, pas un texte de la page) ; l'utilisateur peut le renommer.
- **CHANGELOG** : section « C4 tâche 9 » ajoutée ; ROADMAP et REPRISE sont à la clôture de session.
