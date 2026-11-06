# Espacement entre deux dictées

Carnet du 05/11/2026 : « Espace manquant entre deux dictées ». Décision : méthode A (mémoire de la dernière dictée). Ce fichier suit le découpage : tâche 0 (Constat), tâche 1 (Règle, validée), tâche 2 (code et tests, faite), tâche 3 (essai réel, faite sur Mac).

## Constat

Tâche 0, faite le 05/11/2026 par un sous-agent de recherche (lecture seule). Les points `[LU]` viennent du code, les points `[SUPPOSÉ]` sont des déductions non vérifiées. Les numéros de ligne datent du 05/11 et peuvent bouger.

### Scribe a-t-il le même défaut ? Oui

- `[LU]` `Sources/Scribe/Paster.swift:10-55` : `Paster.paste` met le texte tel quel dans le presse-papiers puis envoie ⌘V. `DictationController.swift:345-346` appelle `TextFixer.frenchTypography(text)` puis `Paster.paste`. Aucun espace n'est ajouté.
- `[LU]` Aucune logique d'espace de tête ni de mémoire de la dernière dictée dans ces deux fichiers (greps `leadingSpace`, `lastDict`, `lastPaste`, `needsSpace`, `trailing`). Le seul `trim` (`DictationController.swift:269`) sert à autre chose.
- Limite : le reste de `Sources/Scribe` n'a pas été parcouru avec ces mots.
- **Conséquence** : il n'y a pas de comportement de référence à porter. La règle est à créer, et la différence avec Scribe est à noter dans `docs/ECARTS.md` (tâche 1).

### Où se fait le collage dans Scriibe

- `[LU]` Assemblage : `crates/pipeline/src/controller.rs`, `Controller::process` (~l.683).
  - `text.trim()` (l.690), puis nettoyage ou traduction.
  - `french_typography` (~l.774-776), seulement si la langue est le français et sans traduction.
  - `deps.paster.selection()` (l.792). Si une sélection est active, `scriibe_core::inline::inside_a_sentence` (`crates/core/src/inline.rs:12`, l.795-797) retire le point final et met la 1re lettre en minuscule.
  - `deps.paster.paste(&text, settings.restore_clipboard)` (l.801).
  - L'historique enregistre `text` (l.827-833) : un espace ajouté à `text` avant ce point se retrouverait dans l'historique (d'où une variable séparée pour le collage) et dans « copier la dernière » (`history.rs:322`).
- `[LU]` Interface : `crates/platform/src/paste.rs:19-31` (`paste`, `selection`, `copy`). Implémentations : `macos/paste.rs:181`, `windows/paste.rs:155`, `linux/paste.rs:81`, faux collage `testing.rs:126-163` (`FakePaster`), enveloppes `crates/cli/src/web/sinks.rs:69` et `crates/cli/src/commands.rs:32`.
- `[LU]` Aucun espace de tête ou de fin n'est ajouté nulle part dans `crates/`. Plusieurs `trim()` en amont (dont `cleaner.rs` l.205-243) retirent ceux de fin.

### Sur quels systèmes

- `[LU]` Le défaut vient du pipeline commun : **identique sur Mac, Windows et Linux X11**. Le texte collé est toujours « Phrase. » sans espace avant ni après.
- `[LU]` Seul le mécanisme de collage diffère : presse-papiers puis ⌘V sur Mac, Ctrl+V sur Windows et Linux. Mac et Windows attendent le relâchement des modificateurs ; le chemin X11 ne le fait pas dans `paste` (`linux/paste.rs:81-86`, alors que `wait_for_modifiers_released` existe l.47).
- `[LU]` Seul le Mac lit la sélection (`macos/paste.rs:164-175`). `[SUPPOSÉ]` Windows et Linux retombent sur `Selection::default()` (`active: false`).
- `[SUPPOSÉ]` Rien ne lit le caractère avant le curseur dans l'appli cible. Seule la mémoire de la dictée précédente est donc faisable de la même façon sur les trois systèmes (confirme l'écart de la méthode B).

### Point d'insertion proposé pour la tâche 1 (`[SUPPOSÉ]`)

- État : `Inner` (`controller.rs:115-126`) vit pendant toute la durée du `Controller` et porte déjà `State` avec `target: AppInfo`. `AppInfo` (`platform/src/apps.rs:4`) a `name` et `id` (identifiant de bundle sous macOS, exécutable sous Windows et Linux). `deps.apps.current()` donne la fenêtre active au collage (l.799).
- Ajouter un champ du genre `last_paste: Mutex<Option<(instant, AppInfo, finit_par_espace_ou_saut_de_ligne)>>`.
- Règle appliquée **tard**, entre `inside_a_sentence` (l.795-797) et `paste` (l.801), à cause des `trim()` en amont : si la dictée précédente a été collée dans la même appli il y a moins de N secondes (horloge `deps.clock.now()`) et ne finit pas par un espace ou un saut de ligne, préfixer un espace.
- Mettre à jour l'état après un collage réussi seulement (`PasteOutcome::Pasted`, pas `CopiedOnly`).
- Exclure le cas « sélection active » (le texte remplace la sélection).
- Fonction pure dans `crates/core` (à côté de `inline.rs`) pour la tester sans plateforme.
- Limite connue : on ne sait pas si le curseur a bougé, si le champ a changé dans la même appli ou si le texte a été effacé. Un délai court et l'identité de l'appli atténuent le risque sans l'annuler.
- Décision de Christophe (tâche 1) : l'espace ajouté ne figure **ni dans l'historique ni dans « copier la dernière »** ; il est ajouté à une variable séparée, envoyée seule à `paste()`.

### Tests proches

- `crates/pipeline/tests/dictation.rs` : bancs `rig` / `rig_with` / `rig_full` (l.125-220), assertions sur `r.paster.pasted()[0].0` (par ex. `without_a_selection_the_text_is_pasted_as_written` l.315). Deux `r.dictate()` à la suite y tiendraient.
- `FakePaster` et `FakeApps` (`crates/platform/src/testing.rs:126`, `:185`) simulent le collage et le changement d'appli.
- `crates/core/src/inline.rs` : tests unitaires modèles pour la fonction pure.
- `fixtures/` : une règle de ce genre irait en fixture JSON, mais comme Scribe n'a pas ce comportement, il n'y a pas de cas issu de Scribe. `fixtures/README.md` et `A_FAIRE.md` n'ont pas été lus : à voir en tâche 1.

## Règle

Validée par Christophe le 05/11/2026 (tâche 1), **élargie le 06/11/2026** (condition 4, voir « Règle élargie » en fin de fichier). Méthode A : mémoire de la dernière dictée.

**Mémoire** : après un collage réussi (`Pasted`, pas `CopiedOnly`), Scriibe retient l'identifiant de l'appli active, l'heure et le dernier caractère du texte collé (sans l'espace ajouté). Après un `CopiedOnly`, la mémoire est effacée. Elle est vide au démarrage.

**Une espace est ajoutée devant la dictée si, et seulement si, ces 6 conditions sont vraies** :
1. une mémoire existe ;
2. l'identifiant de l'appli active est le même et non vide ;
3. le délai depuis le dernier collage est d'au plus **60 s** (constante `MAX_GAP`) ;
4. le dernier caractère collé n'est **ni une espace ou un saut de ligne, ni un signe qui s'attache au mot suivant** : `( [ { « “ ‘ ¿ ¡ - – — / @ #` (toute autre fin, point ou pas, lettre, chiffre, virgule, guillemet fermant, appelle une espace) ; *(règle d'origine du 05/11 : seulement après `. ! ? … : ;`)* ;
5. aucune sélection n'est active pour la nouvelle dictée ;
6. la nouvelle dictée ne commence ni par une espace ou un saut de ligne, ni par une ponctuation qui se colle au mot précédent (`, . ; : ! ? … ) ] } »` et `%`).

La règle est une fonction pure (`crates/core/src/spacing.rs`). L'espace n'est ajoutée qu'à ce qui part vers `paste()`, juste avant le collage, jamais au texte de l'historique ni de « copier la dernière ». Le journal compte 1 signe de plus.

| # | Cas | Résultat |
|---|---|---|
| 1 | Premier collage (ou après redémarrage) | Pas d'espace : pas de mémoire |
| 2 | Le texte précédent finit déjà par un espace (ou la nouvelle dictée commence par un espace) | Pas d'espace |
| 2 bis | Le texte précédent ne finit pas par un point (modèle qui n'en met pas) | **Espace ajoutée** (règle élargie du 06/11) |
| 3 | Retour à la ligne : le précédent finit par `\n` | Pas d'espace |
| 4 | Autre fenêtre ou autre appli | Pas d'espace |
| 5 | Délai long (plus de 60 s) | Pas d'espace |
| 6 | La phrase commence par une ponctuation | Pas d'espace |
| 7 | Traduction en anglais | Même règle, sur le texte final collé |

**Limites assumées** : la méthode A ne voit pas le curseur. Une touche Entrée tapée à la main entre deux dictées, ou un changement de champ ou d'onglet dans la même appli en moins de 60 s, donne une espace en début de ligne ou de champ. Écart avec Scribe (qui n'a pas cette fonction) : noté dans `docs/ECARTS.md`.

## Tests

Tâche 2, faite le 05/11/2026. Code : `crates/core/src/spacing.rs` (règle pure, constante `MAX_GAP` = 60 s), branchement dans `Controller::process` (`crates/pipeline/src/controller.rs`, mémoire `last_paste` dans `Inner`). Pour les tests, `FakeApps` (`crates/platform/src/testing.rs`) permet maintenant de changer l'appli au premier plan.

**Un test par cas**

| # | Cas | Test de la règle (`scriibe_core::spacing`) | Test du branchement (`crates/pipeline/tests/dictation.rs`) |
|---|---|---|---|
| 1 | Premier collage | `case_1_first_paste_gets_no_space` | `spacing_case_1_the_first_paste_has_no_leading_space` |
| 2 | Déjà un espace | `case_2_previous_text_already_ending_with_a_space_or_new_text_starting_with_one` | (texte rogné en amont : non atteignable par le pipeline) |
| 3 | Retour à la ligne | `case_3_previous_text_ending_with_a_line_break` | (idem) |
| 4 | Autre appli | `case_4_another_app_gets_no_space` | `spacing_case_4_another_app_in_front_gets_no_space` |
| 5 | Délai long | `case_5_a_long_delay_gets_no_space` (60 s : espace ; 61 s : non) | `spacing_case_5_a_long_delay_gets_no_space` |
| 6 | Ponctuation initiale | `case_6_a_phrase_starting_with_punctuation_gets_no_space` | (le nettoyage ne laisse pas passer ce cas : règle seule) |
| 7 | Traduction anglaise | `case_7_a_translation_into_english_follows_the_same_rule` | `spacing_case_7_a_translation_into_english_gets_the_space_too` |

Autres tests : cas normal pour chacun des six signes de fin ; (d'origine : pas d'espace sans point final, remplacé le 06/11 par les tests de la règle élargie) ; sélection active ; texte vide ; la deuxième dictée dans la même appli est collée avec l'espace **mais l'historique ne l'a pas** (`spacing_the_second_dictation_in_the_same_app_gets_a_space_but_the_history_does_not`, qui vérifie aussi le compte du journal) ; un collage qui n'a fait que copier efface la mémoire.

**Résultat** (mesuré le 05/11/2026) : `cargo test --workspace` : **331 tests passés, 0 échec** (dont 11 pour la règle et 7 `spacing_*` du pipeline). Les cas de référence sont rejoués par `reference_cases` (core et engines), tous deux passés ; `python3 tools/verifier_fixtures.py` : 399 cas, 0 erreur. `cargo fmt --all --check` et `cargo clippy --all-targets -- -D warnings` : sans erreur. **Non fait** : vérification de compilation Windows (`--target x86_64-pc-windows-gnu`), CI, essai réel (tâche 3).

## Essai réel

Tâche 3, le 05/11/2026, sur le Mac de Christophe (Scriibe.app de développement dans `~/Applications`, compilée avec la règle, Whisper local, journal lu par Claude).

**TextEdit** (document vierge, trois phrases dictées à la suite, moins d'une minute entre elles) : verdict de Christophe, **réussi**.
- Un seul espace entre la phrase 1 et la phrase 2, idem entre la 2 et la 3.
- La première phrase commence sans espace devant.
- Journal : trois collages dans TextEdit (26, 21 et 23 signes), appli active identique à l'appui et au collage.

**Notes** : réussi (verdict de Christophe). Journal : trois collages (37, 36 et 38 signes).

**LibreOffice Writer** : réussi (verdict de Christophe). Journal : trois collages (60, 36 et 38 signes).

**Google Chrome (page Google)** : **irrégulier**. Christophe : la première dictée s'inscrit, la deuxième n'apparaît pas, la troisième s'inscrit. Le journal ne montre aucune erreur : chaque dictée est « envoyée à Google Chrome » avec un collage normal (0,44 s, ⌘C d'essai). Le texte manquant n'est donc pas un refus de Scriibe. Hypothèse (non vérifiée) : c'est l'irrégularité déjà connue du ⌘V dans Chrome (`docs/ECARTS.md`, 04/11, « Collage ⌘V »), sans lien avec l'espace, qui ne change que la chaîne envoyée. Une inconnue : le journal ne dit pas quelle ligne correspond à quelle dictée de l'essai, et le premier collage a vu une « sélection » dans Chrome (donc sans majuscule ni point final). Effet de bord à connaître : Scriibe retient comme collée une dictée que Chrome a ignorée, donc la suivante reçoit son espace de tête même si la précédente n'est pas dans le champ.

**Limite connue confirmée** : avec un retour à la ligne tapé à la main entre deux dictées rapprochées, la deuxième commence par une espace (observé par Christophe).

**Décisions de Christophe (05/11/2026)** : le cas Chrome est **noté pour plus tard, non traité dans cette session** (piste : série de 5 à 6 dictées courtes dans le champ de recherche de Google, comptées et comparées au journal, pour savoir si le ⌘V seul explique les pertes ; voir aussi l'effet de bord sur la mémoire du dernier collage). L'essai Windows (Bloc-notes) **attend la prochaine mise à jour** : les changements ne sont pas commités, donc pas dans la version Windows.

## Relecture et points à reprendre

Relecture du commit `074ec9d` par un agent distinct (05/11/2026) : **rien de bloquant**. Décision de Christophe : tout noter, ne rien coder dans cette session. CI Linux du commit : verte ; Windows et macOS complets et compilation Windows croisée non lancés.

**À corriger plus tard**
1. Le test `spacing_a_paste_that_only_copied_forgets_the_last_dictation` ne prouve pas l'effacement de la mémoire. Scénario à écrire : A collée, B en `CopiedOnly`, C sans espace (`pasted()[2]`).
2. Un texte vide efface la mémoire (`remember` renvoie `None`, le contrôleur écrase `last_paste`). Correctif : ne pas toucher à la mémoire si `text` est vide ; ajouter un test du pipeline.

**À décider**
3. Phrase qui finit par un guillemet ou une parenthèse fermante (`… fin. »`, `(voir plus haut.)`, `"Fini."`) : aucun espace n'est ajouté, car le dernier caractère n'est pas dans la liste. Conforme à la règle validée, mais lacune probable en français : regarder le caractère avant `» ) "`.

**À noter, sans action**
4. Le délai de 60 s court de la fin du collage au début du collage suivant : il inclut l'enregistrement et la transcription de la dictée suivante. La limite exacte (60 s accepté, 61 s refusé) n'est testée que dans la règle pure.
5. L'identifiant d'appli est comparé sans tolérer la casse : à surveiller à l'essai Windows.
6. Début de dictée en tiret ou en puce : reçoit une espace (cohérent) ; à vérifier à la main après « Voici la liste : ». Signes de fin d'autres langues (`¿`, `؟`, `。`) non gérés : sans effet en français et en anglais.
7. Chrome : voir Essai réel.

## Règle élargie (06/11/2026)

**Constat** (essai Windows, Bloc-notes, Scriibe 0.2.4, puis reproduit sous Linux) : avec le modèle Whisper `ggml-small.bin`, les dictées courtes n'ont pas de point final ; la règle d'origine (espace seulement après `. ! ? … : ;`) n'ajoutait alors rien et les phrases se collaient. Avec `ggml-large-v3-turbo-q8_0.bin` sur Mac, le problème n'apparaît pas. Cause du point manquant : le modèle, d'après Christophe ; non mesuré ici.

**Décision de Christophe (06/11/2026)** : élargir la règle (condition 4), **sans la publier tout de suite** : elle part avec la prochaine version après la 0.2.4. Il essaie le modèle `large-v3-turbo-q8_0` sur Windows et Linux.

**Code et tests** : `crates/core/src/spacing.rs` (`JOINING_ENDINGS` remplace `SENTENCE_ENDINGS` ; tests `a_dictation_without_a_final_stop_still_gets_a_space` et `a_previous_text_ending_with_an_opening_or_joining_sign_gets_no_space`) et `spacing_a_model_that_writes_no_final_stop_does_not_glue_the_phrases` dans `crates/pipeline/tests/dictation.rs`.

**Risque connu** : une espace en trop si l'on continue à la main une phrase au même endroit en moins de 60 s ; les guillemets droits `"` et `'` ne figurent pas dans la liste des signes qui collent (ils peuvent fermer une phrase).

**Résultat** (mesuré le 06/11/2026) : `cargo test --workspace` : **335 tests passés, 0 échec** ; 399 cas de référence vérifiés ; `cargo fmt --check` et `cargo clippy -D warnings` sans erreur. Non lancés : CI complète, essai sur le PC (à faire avec le modèle `large-v3-turbo-q8_0` et avec `small`).
