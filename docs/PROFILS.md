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
