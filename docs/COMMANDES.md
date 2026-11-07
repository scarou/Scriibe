# Commandes vocales sur du texte sélectionné

Ligne du carnet du 03/11 « Mode commande sur du texte sélectionné » (Décision du 07/11, découpage `decoupage-2026-11-07-commandes-selection.md`). Ce document suit les tâches : Constat (tâche 0), Conception (1), Tests (2), puis Relecture (4) et Essai réel (5).

## Constat

Écrit le 07/11/2026 (tâche 0) d'après une lecture du code par un sous-agent, sondée ensuite par l'assistant sur les points qui comptent (appels du contrôleur, fonctions du Mac, absence de `ConcealedType`). Rien n'a été exécuté. Chemins relatifs à la racine du dépôt.

### 1. Détection de la sélection (Mac seulement)

- `crates/platform/src/paste.rs:13-31` : trait `Paster` avec `paste`, `selection()` (par défaut : aucune) et `copy`. `Selection {active, report}` ne porte **jamais le texte**. Seul le Mac l'implémente ; Windows et Linux n'ont pas la méthode (ROADMAP n°3).
- `crates/platform/src/macos/ffi.rs:141` `selection_state()` : accessibilité (`AXIsProcessTrusted`, élément ayant le focus, `AXSelectedText`) ; seule la **longueur** est lue. Résultats : `Active`, `Empty`, `Unknown`, `NotTrusted`.
- `crates/platform/src/macos/paste.rs:164` `MacPaster::selection()` : `Active` → active ; `Empty` et `NotTrusted` → aucune ; `Unknown` (Electron, pages web) → repli `selection_by_copy()` (ligne 126).
- `selection_by_copy()` : lit le presse-papiers texte, attend le relâchement des modificateurs, envoie ⌘C, sonde 150 ms (`COPY_WAIT`, ligne 104) ; si le contenu change, la sélection est active ; remet l'ancien contenu et le vérifie ; ne rend que le nombre de caractères. Limites : presse-papiers conservé **en texte seulement** (images, fichiers, RTF perdus) ; touche C/V figée en QWERTY (code 8 et 9).
- Usage actuel : un seul, la dictée par-dessus une sélection (S14, `docs/ECARTS.md`). `controller.rs:795` appelle `selection()` ; si elle est active, `core::inline::inside_a_sentence` retire la majuscule et la ponctuation finale, et `spacing.rs` n'ajoute pas d'espace devant. Il n'y a ni mode sélection ni traitement de la sélection.
- Collage (`macos/paste.rs:181`) : sauvegarde du texte du presse-papiers (réglage `restore_clipboard`, vrai par défaut), écriture du texte dicté, ⌘V (`send_command_v`, ligne 82), remise 1,5 s après si le presse-papiers contient encore le texte dicté. Si l'ancien contenu n'était pas du texte, il n'est pas restauré.

### 2. Chemin transcription → collage

`crates/pipeline/src/controller.rs`, fonction `process` (lignes 686-872), dans l'ordre :

1. `:692` transcription → `raw` ; `:702-714` filtre des phrases inventées par Whisper et texte vide ;
2. `:716` `join_spelled_code` puis `dictionary::apply` (signes dictés, corrections de l'utilisateur, signes de nouveau, orthographe des mots) ;
3. `:727-754` traduction éventuelle (puis `dictionary::apply`) ;
4. `:758-789` mise au propre par le modèle, avec garde-fou (puis `dictionary::apply`) ;
5. `:791` typographie française ; `:795-800` sélection puis `inside_a_sentence` ; `:805` `spacing::for_paste` ; `:812` `paster.paste` ;
6. `:817-832` journal « collage » ; `:840` journal « dictée » ; `:852-858` historique.

**Où intercepter une phrase de commande** : juste après `:714`, sur `raw`, **avant** `dictionary::apply` (`:716`). Après, `spoken_signs::apply` a déjà transformé certaines phrases (« à la ligne » devient un saut de ligne) et les corrections de l'utilisateur ont pu les réécrire. La comparaison se fait sur `raw` normalisé (sans ponctuation, casse ni accents).

**Séquencement** : aujourd'hui `selection()` n'est appelée qu'à `:795`, après le modèle, et ne rend jamais le texte. Une commande devra donc (a) reconnaître la phrase sur `raw`, (b) **seulement alors** lire la sélection (nouvelle méthode du trait `Paster` qui rend le texte). Lire la sélection avant de savoir qu'il s'agit d'une commande déclencherait un ⌘C inutile à chaque dictée.

### 3. Ce qui est journalisé

- **Journal** (`scriibe.log`, `crates/platform/src/storage.rs:221`, rotation à 512 Ko) : étapes, durées, niveau du micro, nom de l'appli, motif d'une phrase inventée, nombre de signes collés, **état** de la sélection (longueur seulement). Le commentaire du code dit « jamais le texte dicté ». Aucun accès au texte, à la sélection ou au presse-papiers trouvé. **Non vérifié** : les messages d'erreur (`{e}` de la transcription, `e.message` du modèle) pourraient contenir du texte venant des moteurs.
- **Historique** (`history.json`, `storage.rs:166`, struct `Dictation` dans `crates/core/src/history.rs:55`) : `rawText`, `finalText`, `appName`. Texte conservé par défaut (`keep_dictation_text` vrai), purgeable par durée. Ni sélection ni presse-papiers aujourd'hui ; mais le texte collé y entre, donc **un texte de commande ne doit pas y passer**.
- **Rapport** (« Copier le rapport », `ui/src/lib/report.ts`, `ui/src/tabs/Journal.svelte:26`) : version, réglages, moteurs, micros, extrait de `llama-server.log` (180 caractères par ligne), résultats du banc (phrases de banc, pas des dictées), 25 dernières lignes du journal. Ni historique, ni sélection, ni presse-papiers. **Non vérifié** : si `llama-server.log` peut contenir le texte envoyé au modèle selon la verbosité de llama.cpp.

### 4. Signes dictés

- Définition : `crates/core/src/spoken_signs.rs:28` (`ALL`, 11 entrées, **français seulement**) ; application `replace` (`:329`) avec garde-fous (impératif, rien après une apostrophe, mots descriptifs `DESCRIBING_WORDS`) ; `tidy` (`:397`).
- Reconnus : ouvre/ferme parenthèse, guillemets, crochet ; points de suspension ; point à la ligne ; retour/passe à la ligne ; nouveau paragraphe ; « à la ligne » seul. Listes (début de ligne, coupées en anglais) : tiret, puce, « point un », « numéro suivant ».
- **Manquaient** (inventaire de la tâche 6, 07/11/2026) : aucun signe anglais ; point d'exclamation, point d'interrogation, deux-points, point-virgule, arobase, barre oblique, dièse, etc. **Ajoutés en S18** : voir « Tâche 6 » plus bas et `docs/ECARTS.md`.
- Ajout par l'utilisateur : corrections « entendu → écrit » (`dictionary.json`, onglet Dictionnaire, routes `api/dictionary` dans `crates/cli/src/web/mod.rs`) ; si l'écriture est de la ponctuation, elle suit les règles des signes intégrés (`kind_of`, `spoken_signs.rs:302`). Exemple affiché : « trois points d'exclamation » → `!!!`. Carte « Signes dictés » : `ui/src/tabs/Dictionary.svelte`.

### 5. Équivalent dans Scribe (Swift)

Aucun. `Paster.swift` ne fait que copier et coller (⌘V) ; pas de lecture de la sélection, pas de ⌘C d'essai, pas de commande vocale. Les idées 12 bis (« Commandes vocales ») et 12 ter (« Mode commande sur du texte sélectionné ») ne sont que dans `Scribe/ROADMAP.md` (lignes 61-62), à cadrer. Cela concorde avec `docs/ECARTS.md` (S14 : « Scribe colle toujours la phrase entière »). L'idée 12 ter de Scribe suppose un modèle de langage qui réécrit la sélection ; la Décision du 07/11 de Scriibe l'écarte (phrases connues, pas de modèle) : **écart à noter dans `docs/ECARTS.md` à la tâche 1**.

### Points d'attention pour la conception

- **Presse-papiers et vie privée** : le ⌘C d'essai met la sélection dans le presse-papiers système pendant 0,2 à 0,5 s ; un gestionnaire tiers (Alfred, Raycast, Paste, presse-papiers universel d'Apple) peut l'enregistrer ou la synchroniser. Sous Windows, `paste.rs:103` pose des drapeaux d'exclusion ; sur Mac rien d'équivalent (`org.nspasteboard.ConcealedType` n'est pas utilisé : à examiner, non testé).
- **Trait `Selection`** : interdit par conception de lire le texte ; une nouvelle méthode qui le rend devra être suivie jusqu'à `log.write`, `Dictation` et le rapport (aucun `{e}` d'erreur ne doit l'inclure).
- **Faux positifs** : « ajouter au dictionnaire » dit sans sélection doit rester du texte ; une sélection fantôme est possible si `selection_by_copy` voit un changement dû à une autre copie en même temps.
- **Limites du Mac** : accessibilité obligatoire (sinon `NotTrusted`, aucune commande) ; Electron et pages web retombent sur ⌘C ; presse-papiers conservé en texte seulement.
- **Non vérifié** : comportement réel de la sonde ⌘C dans Chrome et Electron (le code mentionne une copie en deux temps de Chrome, `macos/paste.rs:122`) ; aucun test lancé.

## Conception

Validée par Christophe le 07/11/2026 (tâche 1), avec les 6 points ci-dessous et les 8 cas. Cible : Mac d'abord (tâches 2 à 5) ; Windows et Linux dépendent de la détection de la sélection (ROADMAP n°3) et sont portés dans leurs carnets.

### Principe

Une seule couche de commandes « verbe + objet », en logique pure (`crates/core/src/commands.rs`, rien de propre à un système). Elle est réutilisée par les extraits (ROADMAP n°4) et la recherche web (n°7), qui ajouteront leurs fiches sans toucher à la reconnaissance.

- **Fiche de commande** : identifiant « verbe.objet » (ex. `dictionnaire.ajouter`), phrases par défaut en français et en anglais (au moins 2 mots), `needs` (sélection exigée ou non), `argument` (aucun, ou « le reste de la phrase » pour la recherche web). Les extraits et les sites de recherche de l'utilisateur ajoutent des fiches à l'exécution.
- **Reconnaissance** : la phrase doit être **toute** la transcription (option A). Comparaison sur une forme normalisée : minuscules, accents retirés (œ → oe), apostrophes, tirets, ponctuation et espaces ramenés à un seul espace. Une fiche avec argument reconnaît « phrase + reste non vide ».
- **Où** : sur `raw`, juste après le filtre des phrases inventées (`controller.rs:714`), **avant** `dictionary::apply` (les signes dictés ou les corrections réécriraient la phrase). Un module pur `recognize(raw, catalogue, langue, …)` rend la fiche et l'argument.
- **Sélection lue seulement après la correspondance** : une dictée ordinaire ne déclenche jamais de ⌘C. Une commande à sélection sans sélection lisible → la phrase est collée comme du texte (sans message ; avec un message dans la pastille si la lecture est impossible : accessibilité refusée, presse-papiers non textuel). Une fiche sans `needs` s'exécute avec ou sans sélection.
- **Langues** : catalogue de la langue de dictée ; « plusieurs langues » (`multi`) teste les deux. Sur `raw`, donc la langue parlée, avant toute traduction.
- **Phrases réglables** : réglage par commande et par langue (clé `voiceCommands`, plus un interrupteur général, **activé par défaut**), carte « Commandes vocales » dans l'onglet Dictionnaire. Les phrases de l'utilisateur **remplacent** celles par défaut (bouton « Rétablir »). Refusées : une phrase déjà prise par une autre commande (comparée sous forme normalisée), une phrase de moins de 2 mots.
- **Lecture de la sélection** : nouvelle méthode du trait `Paster`, `selected_text()`, qui rend un type dédié (`SelectedText`) : ni `Debug`/`Display` du contenu, ni `Serialize`, effacé de la mémoire à la fin. Mac : accessibilité (`AXSelectedText`, sans toucher au presse-papiers) ; **repli ⌘C** si l'appli n'expose pas sa sélection, avec presse-papiers texte sauvegardé puis remis (vérifié). **Si le presse-papiers contient autre chose que du texte (image, fichiers), pas de ⌘C** : la commande n'est pas exécutée, la phrase est collée comme du texte et la pastille le dit. Marquer la copie comme « masquée » pour les gestionnaires de presse-papiers (`org.nspasteboard.ConcealedType`) : à examiner en tâche 3, non garanti.
- **Exécution** : un seul répartiteur par identifiant. « Ajouter au dictionnaire » : le texte passe à la page par un **canal en mémoire lu une seule fois** (jamais écrit sur disque ni journalisé) ; la fenêtre s'ouvre sur l'onglet Dictionnaire, « Entendu » prérempli, curseur dans « Écrit ». Refus si la sélection dépasse 80 caractères ou fait plusieurs lignes (message dans la pastille).
- **Retour** : message court dans la pastille, **sans jamais afficher le texte sélectionné** (ex. « Ajouter au dictionnaire — complète « Écrit » »). Les messages venus de Rust ont leur traduction dans `locales/en-server.ts`.
- **⌘Z** : « ajouter au dictionnaire » ne modifie pas le texte (rien à annuler) ; les commandes b et c (caractères autour, gras/italique/souligné) passent par un collage ou un raccourci de l'appli, que ⌘Z annule en une fois, comme une dictée.
- **Aucune trace** : journal = identifiant de la commande et longueur de la sélection seulement ; **pas d'entrée d'historique** pour une commande ; messages d'erreur à texte fixe (jamais `{e}` pouvant contenir du texte) ; rapport inchangé. Un test injecte une sélection piège (« SECRET-123 ») et vérifie son absence du journal, de l'historique, de la pastille, des erreurs et du rapport.

### Les 8 cas (sélection active, sauf le 4)

| # | Tu dis | Ce qui se passe |
|---|---|---|
| 1 | « Ajouter au dictionnaire » | La commande s'exécute, rien n'est collé. |
| 2 | « Ajoute au *Dictionnaire* ! » / « Ajouter au dictionnaire. » | Idem : ponctuation, majuscules et accents sont ignorés. |
| 3 | « Ajouter au dictionnaire le mot cri be » | Phrase plus longue : pas une commande, la dictée remplace la sélection (⌘Z la restaure). |
| 4 | « Ajouter au dictionnaire » sans sélection | La phrase est collée comme du texte, sans message. |
| 5 | « Add to dictionary » (langue anglaise) | Idem en anglais ; en « plusieurs langues », les deux catalogues sont testés. |
| 6 | Phrase changée dans les réglages (« mémorise ») | La nouvelle marche, l'ancienne non ; phrase déjà prise ou d'un seul mot : refusée. |
| 7 | Presse-papiers occupé, repli ⌘C | Le texte est sauvegardé puis remis ; image ou fichiers : pas de ⌘C, commande non exécutée. |
| 8 | Une commande s'exécute | Rien de la sélection dans le journal (longueur seulement), l'historique, le rapport, la pastille ou les erreurs. |

### Les 6 points validés

1. Activé par défaut, avec un interrupteur dans les réglages.
2. Phrases de l'utilisateur d'au moins 2 mots (les phrases par défaut aussi).
3. Lecture par l'accessibilité d'abord, ⌘C en repli (la Décision du 07/11 disait « par ⌘C simulé » : plus privé, un gestionnaire de presse-papiers ne voit rien passer).
4. Presse-papiers non textuel : pas de ⌘C, commande non exécutée.
5. Une commande n'entre pas dans l'historique des dictées.
6. Une fiche déclare si elle exige une sélection et si elle prend un argument (extraits et recherche web n'exigent pas de sélection ; la règle « sans sélection, la phrase reste du texte » vaut pour les commandes sur sélection).

### À vérifier plus tard

- Sonde ⌘C dans Chrome et Electron (copie en deux temps de Chrome) : tâche 5.
- Messages d'erreur des moteurs et `llama-server.log` : peuvent-ils contenir du texte ? tâche 4.
- Windows et Linux : après la ROADMAP n°3 (carnets Windows et Linux).

## Tests

Tâche 2 (07/11/2026) : couche de reconnaissance codée dans `crates/core/src/commands.rs` (module pur), réglages `voiceCommandsEnabled` (actif par défaut) et `voiceCommands` (phrases par langue, nettoyées à la lecture) dans `settings.rs`. **Pas encore branchée** au contrôleur : c'est la tâche 3 (appel de `recognize_for` après `controller.rs:714`).

- **Fonctions** : `normalize` (minuscules, accents retirés, œ → oe, ponctuation/apostrophes/tirets/espaces → un espace), `recognize` / `recognize_for` (phrase = toute la transcription ; complément non vide pour une fiche `Argument::Rest` ; phrase la plus longue d'abord ; moins de 2 mots ignoré), `effective_phrases` (phrases de la personne, sinon celles par défaut), `validate_phrase` (au moins 2 mots, pas déjà prise), `builtin()` (une fiche : `dictionnaire.ajouter`, 7 phrases en français, 6 en anglais).
- **Un test par cas de la Conception** (dans `commands::tests`) : `commands_001_exact_phrase` (cas 1), `commands_002_punctuation_case_and_accents_are_ignored` (2), `commands_003_a_longer_sentence_is_not_a_command` (3), `commands_004_the_definition_says_whether_a_selection_is_needed` (4 : la reconnaissance ne dépend pas de la sélection ; sans sélection, c'est l'appelant qui colle la phrase, testé en tâche 3), `commands_005_english_and_both_languages` (5), `commands_006_custom_phrase_replaces_the_defaults_and_is_checked` et `commands_006_one_word_phrases_from_a_file_are_ignored` (6).
- **Tests en plus** : `commands_extra_argument_is_the_rest_of_the_sentence`, `commands_extra_the_longest_phrase_wins`, `commands_extra_settings_switch_and_file` (interrupteur, fichier de réglages, valeurs incorrectes).
- **Pas dans cette couche** : cas 7 (presse-papiers restauré, image ou fichiers) et cas 8 (aucune trace de la sélection), qui relèvent de la lecture de la sélection et de l'exécution : tâches 3 et 4.
- **Résultat mesuré** : `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings` sans erreur ; `cargo test --workspace` : 345 tests passés, 0 échec (dont 10 de `commands`) ; `python3 tools/verifier_fixtures.py` : 399 cas, 0 erreur (les cas de référence sont rejoués par `crates/core/tests/fixtures.rs` et `crates/engines/tests/fixtures.rs`, verts). Non lancés : clippy pour Windows (aucun code propre à un système touché), tests de l'interface (aucun fichier de `ui/` touché).

### Tâche 3 — « ajouter au dictionnaire » sur Mac (07/11/2026)

Branchée de bout en bout, **jamais essayée à la main** (tâche 5).

- **Lecture de la sélection** (conforme à la Conception validée : accessibilité d'abord, ⌘C en repli) : `Paster::selected_text()` (`crates/platform/src/paste.rs`) rend `SelectedRead::{Text(SelectedText), NoSelection, Unreadable(Unreadable)}`. `SelectedText` n'a ni `Debug` du contenu, ni clone, ni sérialisation, et s'écrase à la fin ; `Unreadable` a des raisons à texte fixe. Mac (`macos/ffi.rs` `selected_text`, `macos/paste.rs`) : `AXSelectedText` sans toucher au presse-papiers ; si l'appli ne l'expose pas, ⌘C d'essai avec presse-papiers texte sauvegardé puis remis (remise vérifiée). Si le presse-papiers est vide ou non textuel, `osascript -e 'clipboard info'` dit s'il contient une image ou des fichiers (marqueurs `PNGf`, `TIFF`, `JPEG`, `GIFf`, `furl`…) : dans ce cas, ou si on ne sait pas le dire, **pas de ⌘C**. Windows et Linux : méthode par défaut (« sélection illisible ») jusqu'à la ROADMAP n°3.
- **Contrôleur** (`crates/pipeline/src/controller.rs`, `try_command`) : après le filtre des phrases inventées et **avant** `dictionary::apply`, `commands::recognize_for(raw)` ; la sélection n'est lue qu'après une correspondance. Sans sélection : la phrase est collée comme du texte, sans message. Lecture impossible : phrase collée, remarque à texte fixe dans la pastille. Sélection de plus de 80 signes, vide ou sur plusieurs lignes : rien n'est collé, message. Succès : `CommandSink::add_to_dictionary`, message « Ajouter au dictionnaire — complète « Écrit » », **pas d'entrée d'historique**, pas de modèle appelé.
- **Canal vers la page** (`crates/cli/src/prefill.rs`) : texte tenu en mémoire seulement, 60 s au plus, pris une seule fois par `POST api/prefill` ; `api/activity` ne donne que `prefill: true/false` (la page le lit chaque seconde). La fenêtre est ouverte par `open::open` (même chemin que le menu de l'icône).
- **Page** : `status.svelte.ts` prend le texte, `nav.go('dictionary', 'dictionary-corrections-entendu-ecrit')` ouvre l'onglet et déplie la carte, `Dictionary.svelte` met le texte dans « Entendu », vide « Écrit » et lui donne le focus. Les nouveaux messages de la pastille sont traduits dans `locales/en-server.ts`.
- **Tests** (`crates/pipeline/tests/dictation.rs`, `commands_1xx`, avec `FakePaster::set_selected` / `selected_reads` et un faux `CommandSink`) : `commands_101` (cas 1-2 : la commande marche, rien collé, rien dans l'historique, modèle non appelé), `commands_101b` (une dictée ordinaire ne lit jamais la sélection), `commands_103` (cas 3), `commands_104` (cas 4), `commands_105` (cas 5), `commands_106` (cas 6 et interrupteur), `commands_107` (cas 7 : presse-papiers non textuel → phrase collée et message), `commands_108` (sélection longue ou multiligne), `commands_109` (échec de la fenêtre), **`commands_110` (cas 8 : une sélection piège « SECRET-123 mot » n'apparaît ni dans le journal — qui garde « commande: dictionnaire.ajouter (sélection : 14 signes) » —, ni dans l'historique, la pastille, le collage ou les requêtes aux services)**. S'y ajoutent deux tests de `prefill.rs` (texte pris une fois, périmé après 60 s, `Debug` sans contenu).
- **Résultat mesuré** : `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings` sans erreur ; `cargo test --workspace` : 357 tests passés, 0 échec ; `npm run check`, `npm test` (83) et `npm run build` verts dans `ui/`.
- **Pas fait / non vérifié** : (1) la carte « Commandes vocales » des réglages (phrases réglables et interrupteur dans l'interface) : les réglages `voiceCommandsEnabled` et `voiceCommands` marchent par `settings.json` seulement ; (2) le marquage « presse-papiers masqué » (`org.nspasteboard.ConcealedType`) du ⌘C d'essai : non fait ; (3) clippy pour Windows (cible non installée ici) : la CI le dira ; (4) le ⌘C d'essai et l'accessibilité en vrai, dans TextEdit, Notes et Chrome : tâche 5 ; (5) Aide (français et anglais) : rien ajouté sur les commandes vocales.

### Tâche 6 — signes dictés manquants (07/11/2026)

Inventaire par un sous-agent de recherche (Sonnet), liste présentée à Christophe, lots A, B et C retenus (le lot D, « signe plus/moins/égal/inférieur/supérieur », et les noms nus ne sont **pas** codés).

- **Codé** (`crates/core/src/spoken_signs.rs`, tableau `ALL`) : A. `point d'exclamation`, `point d'interrogation`, `point-virgule`, `deux-points`, et en anglais `exclamation mark`, `question mark`, `semicolon`, `colon` ; B. `arobase`/`at sign`, `point com` (et fr, org, net, io, eu, be, ch)/`dot com` (org, net, io, fr, eu), `barre oblique`/`forward slash`, `antislash`/`backslash`/`barre oblique inverse`, `underscore`/`tiret bas`/`tiret du 8`, `tilde`, `esperluette`/`ampersand`, `astérisque`/`asterisk`, `hashtag`/`croisillon`/`hash sign`/`pound sign`/`number sign`, accolades (`ouvrez/fermez l'accolade`, `open/close curly bracket`) ; C. anglais : `open/close parenthesis`, `open/close bracket`, `open/close quote`, `ellipsis`, `new line`, `new paragraph`.
- **Langue** : chaque `Sign` porte `langs` (`Fr`, `En`, `Both`) ; `spoken_signs::apply_for` / `dictionary::apply_for` reçoivent `Active` (`Active::of(settings.language)`) ; `apply` et `dictionary::apply` gardent leur signature (français seul, utilisés par le rejeu de Scribe). Dictée anglaise : signes anglais + `Both` ; plusieurs langues : tous ; texte traduit : `Active::NEUTRAL` (seuls les `Both`, comme `underscore` ou `tilde`).
- **Nouveaux types d'espacement** : `Glued` (collé des deux côtés : `@ / \ _ ~ *`), `Spaced` (`&`, espace de chaque côté sauf en fin de phrase), `Suffix` (collé au mot précédent : `.com`). `#` est un `Opening` (collé au mot suivant), `{ }` suivent `( )`.
- **Garde-fous** : `DESCRIBING_WORDS` reçoit une liste anglaise (the, a, my, to, will, I, you…) ; « deux points » en deux mots ne vaut un signe qu'avec un impératif (« écris deux points »), sinon le trait d'union est obligatoire ; « colon cancer » exclu ; « semi-colon » n'est jamais lu comme « colon ». Les signes au pluriel (« trois points d'exclamation ») restent au dictionnaire de l'utilisateur (cas `spoken_signs-015` à `018` de Scribe, inchangés).
- **Non codés** (faux positifs) : `virgule`, `point`, `plus`, `moins`, `égal`, `étoile`, `star`, `euro`, `dollar`, `degré`, `espace`, `tab` nu, `slash` nu, `dièse` nu, `tiret` en phrase.
- **Tests** : un test par signe dans `crates/core/tests/spoken_signs_scriibe.rs` (cas positif, phrases ordinaires inchangées, idempotence) + `english_signs_stay_off_in_french_dictation_and_french_ones_in_english` + `ordinary_words_never_become_signs`.
- **Résultat mesuré** : `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings` sans erreur ; `cargo test --workspace` : 397 tests passés, 0 échec (rejeu de Scribe inchangé : 96 cas `spoken_signs` passent).
- **Pas fait / non vérifié** : jamais essayé à la dictée réelle (Whisper peut écrire ces mots autrement : « arrobase », « hashtag » en `#`, « point com » en `.com` ou `point-com`) ; Windows et Linux : le code est commun, seule la compilation Windows n'est pas relancée ici (aucun fichier système touché) ; pas de carte dans l'interface listant les nouveaux signes ni traduction de l'Aide (`Help.svelte`/`HelpEn.svelte` non touchées).

### Tâche 7 — « caractères autour » (07/11/2026)

Branchée de bout en bout, **jamais essayée à la main** (tâche 9).

- **Commandes** (`crates/core/src/commands.rs`, `builtin()`) : `entourer.parentheses` `( )`, `entourer.guillemets` « » en français (espaces insécables) et `" "` en anglais, `entourer.crochets` `[ ]`, `entourer.accolades` `{ }`. Phrases par défaut : « entre parenthèses », « mets/mettre/mettez entre … », « entoure/entourer de … » ; en anglais « in parentheses », « put in/put it in … », « wrap in … » (« in quotes », « in quotation marks », « in brackets », « in square brackets », « in curly brackets », « in braces »). Même fiche que « ajouter au dictionnaire » : sélection exigée, pas de complément, phrase = toute la transcription, réglables (`voiceCommands`, clés `entourer.*`).
- **Langue des guillemets** : `Recognized.lang` rend la langue du catalogue d'où vient la phrase dite (et non celle des réglages) : en « plusieurs langues », « entre guillemets » donne « » et « in quotes » donne " ". Choix : c'est la langue que Christophe vient de parler.
- **Texte entouré** (`commands::surround`, pur) : les blancs et sauts de ligne de bord restent dehors (« un mot␠ » → « (un mot)␠ ») pour que le signe se colle au texte ; une sélection de blancs seulement n'est pas entourée (pastille « Rien à entourer (la sélection est vide) », traduite dans `en-server.ts`).
- **Exécution** (`controller.rs`, `try_command`) : la sélection, lue par l'accessibilité ou le repli ⌘C, **reste active** ; le texte entouré est collé par `Paster::paste` (même chemin qu'une dictée, `restore_clipboard` respecté) : **un seul collage, donc un seul ⌘Z**. Le collage ne passe ni par `dictionary::apply`, ni par la typographie, ni par la mise au propre ; la mémoire d'espacement entre dictées (`last_paste`) est effacée. Collage seulement copié : même message que pour une dictée (« Copié, à coller avec ⌘V »). Succès : coche de la pastille.
- **Aucune trace** : journal = `commande: entourer.guillemets (sélection : N signes)` ; pas d'entrée d'historique ; le texte entouré est tenu dans un `SelectedText` (effacé de la mémoire) ; le test `commands_209` injecte « SECRET-123 mot » et vérifie son absence du journal, de l'historique, de la pastille et des requêtes (le collage la contient forcément).
- **Tests** : `commands::tests` `commands_201` à `206` (reconnaissance dans les deux langues et en « plusieurs langues », sélection exigée et phrase plus longue = texte, aucune phrase en double entre commandes, paires écrites, blancs de bord, refus d'une phrase prise) ; `crates/pipeline/tests/dictation.rs` `commands_201` à `210` (collage unique, guillemets français et anglais, crochets et accolades, blancs, sans sélection ou phrase plus longue, sélection vide, lecture impossible, collage seulement copié, aucune trace, phrase réglée à la main). Le test existant `commands_006_…` cherche désormais « dictionnaire.ajouter » par son identifiant (la liste a changé d'ordre).
- **Résultat mesuré** : `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings` sans erreur ; `cargo test --workspace` : 413 tests passés, 0 échec ; `npm test` (interface) : 83 tests passés (dont `i18n.test.ts`).
- **Pas fait / non vérifié** : jamais essayé dans une vraie appli (le ⌘Z unique et le repli ⌘C de Chrome restent à voir en tâche 9) ; la carte « Commandes vocales » des réglages et l'Aide ne les mentionnent pas encore (tâche 8) ; Windows et Linux : pas de lecture de la sélection (ROADMAP n°3), la phrase y reste du texte.

### Tâche 8 — gras, italique, souligné et carte « Commandes vocales » (07/11/2026)

Branchée de bout en bout, **jamais essayée dans une vraie appli** (tâche 9) ; la carte a été vue dans la page (lancée à la main, données jetables) et ses routes essayées.

- **Commandes** (`commands.rs`) : `format.gras`, `format.italique`, `format.souligne`, sur sélection, sans complément, réglables comme les autres. Phrases : « mets/mettre/mettez/passe en gras », « en gras » (idem italique) ; « mets en souligné », « souligne le texte », « souligne ça » ; en anglais « make it bold », « make bold », « put in bold », « in bold » (idem italic/italics) ; « make it underlined », « underline it », « underline this », « put in underline ». Un mot seul (« gras ») reste du texte.
- **Pas de Markdown** : le contrôleur appuie le raccourci de l'appli qui a la sélection (`Paster::press_shortcut`), que ⌘Z annule en une fois ; rien n'est collé, rien n'entre dans l'historique ni le journal (identifiant et longueur seulement). Raison : du Markdown s'écrirait tel quel dans un traitement de texte.
- **Raccourcis réglables** : réglage `formatShortcuts` (`bold`, `italic`, `underline`), défaut `Mod+B`, `Mod+I`, `Mod+U` (`Mod` = ⌘ sur Mac, Ctrl ailleurs). Lus par `core::shortcut::Shortcut::parse` (modificateurs `Mod/Cmd/⌘/Ctrl/⌃/Alt/Option/⌥/Maj/Shift/⇧`, une lettre ou un chiffre, **au moins Mod, Ctrl ou Alt**) ; un raccourci illisible redevient celui par défaut à la lecture du fichier, et s'il est illisible au moment de l'appui la pastille le dit (« Raccourci invalide dans les réglages (exemple : Mod+B) »).
- **Envoi (Mac)** : `macos/paste.rs::send_shortcut` : vraie frappe (modificateurs appuyés avant la touche, relâchés après, toujours relâchés même après une erreur), après attente du relâchement des modificateurs, avec l'accessibilité exigée ; touche à sa place QWERTY (même limite que ⌘C/⌘V). Raisons d'échec à texte fixe (`SHORTCUT_*` de `platform/paste.rs`), dites dans la pastille (« Raccourci non envoyé — … ») et traduites dans `en-server.ts`. Windows et Linux : `press_shortcut` répond « non pris en charge » (la sélection n'y est de toute façon pas lue).
- **Carte « Commandes vocales »** (`ui/src/components/VoiceCommands.svelte`, dans l'onglet Dictionnaire, ancre `dictionary-commandes-vocales`) : interrupteur général (`voiceCommandsEnabled`), pour chaque commande ses phrases françaises et anglaises (puces à retirer, champ pour en ajouter), « Rétablir » (visible quand les phrases ont changé), et pour gras/italique/souligné le champ du raccourci avec sa forme lue (« → ⌘B »). Routes (`crates/cli/src/web/mod.rs`) : `GET api/commands`, `POST api/commands/validate-phrase` (refus : moins de deux mots, ou phrase prise par une autre commande, qu'il nomme), `POST api/commands/validate-shortcut`. Les phrases changées s'enregistrent dans `voiceCommands` (seules celles qui diffèrent des phrases d'origine) ; une commande garde toujours au moins une phrase par langue.
- **Aide** : section « Commandes vocales » dans l'onglet Dictionnaire de `Help.svelte` et `HelpEn.svelte` (même structure, test de comparaison vert) ; textes de la carte dans `en.ts`, messages de la pastille dans `en-server.ts`.
- **Tests** : `commands::tests` `commands_301` à `304`, `shortcut::tests` (3), `settings_scriibe` (`format_shortcuts_…`), `crates/pipeline/tests/dictation.rs` `commands_301` à `307` (raccourci appuyé sans collage ni historique, six phrases dans les deux langues, raccourci réglé, raccourci illisible, échec à texte fixe, sans sélection ou phrase plus longue, aucune trace, interrupteur coupé).
- **Résultat mesuré** : `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings` sans erreur ; `cargo test --workspace` : 428 tests passés, 0 échec ; interface : `npm run check` sans erreur, `npm test` 83 passés, `npm run build` ; routes essayées sur un Scriibe lancé avec des données jetables (phrase prise, phrase trop courte, raccourci lisible et illisible, enregistrement et rétablissement).
- **Pas fait / non vérifié** : le raccourci n'a jamais été envoyé à une vraie appli (⌘B dans TextEdit, Notes, Google Docs : tâche 9) ; la saisie et la suppression des phrases dans la carte n'ont pas été cliquées une à une (seulement les routes) ; Google Docs peut avoir d'autres raccourcis (réglables).

### Essai réel b-c (tâche 9, 07/11/2026, Mac Studio)

Essai guidé par Christophe, une étape à la fois, Scriibe.app compilée en release.

- **TextEdit, gras (⌘B)** : marche. Premier essai raté : l'aiguillage `SwitchPaster` (`crates/cli/src/web/sinks.rs`) ne transmettait pas `press_shortcut` (« non pris en charge sur ce système ») ; corrigé (`f7c84b1`). Les tests n'utilisent pas cet aiguillage : un test de bout en bout par la page reste à écrire.
- **Reconnaissance par Whisper** : « mets en gras » est souvent entendu « mais en gras » (ou autre) ; la phrase ratée est collée par-dessus la sélection (⌘Z la ramène). Décision de Christophe : phrase principale « **mettre** en gras / italique / souligné / barré » (plus facile à articuler et à distinguer), « mets », « met », « mais » gardées en variantes. Essai : « mettre en italique » et « mettre en souligné » reconnus du premier coup.
- **Ajouts demandés pendant l'essai** : « mettre en barré », ajouté puis **retiré** (ni TextEdit ni Notes n'ont de raccourci pour le barré, seulement le menu Format ; Christophe : anecdotique, à recréer au besoin par « Mes commandes ») ; la carte « Commandes vocales » et la liste des signes dictés n'affichent que la langue de l'interface (les signes et phrases de l'autre langue restent actifs en dictée) ; **commandes de la personne** (« Mes commandes » : un titre, une ou plusieurs phrases, un raccourci de l'appli, avec ou sans sélection exigée ; réglage `customCommands`, identifiants `perso.<n>`, mêmes règles de phrase que les autres).
- **Notes, « mettre entre parenthèses »** : marche, un seul ⌘Z suffit pour revenir au texte d'origine (essayé par Christophe).
- **Notes, gras, italique, souligné** : marchent. Whisper écrit parfois « souligner » au lieu de « souligné » : « mettre en souligner » ajouté aux phrases par défaut (Christophe l'avait ajouté à la main dans la carte).
- **Google Docs (Chrome), gras, italique, souligné** : marchent tous les trois (lecture de la sélection comprise).
- **Reste à essayer** : Notes (barré, commande de la personne), Google Docs dans Chrome (gras, italique, souligné, barré), « mettre entre parenthèses » et les autres « caractères autour », et les nouveaux signes dictés tels que Whisper les écrit.

## Relecture

Tâche 4 (07/11/2026) : relecture du commit 3e6724b par un sous-agent Sonnet qui n'avait pas écrit le code (lecture seule, rien exécuté), puis corrections. Chemins suivis : types de la sélection, lecture Mac (accessibilité et ⌘C), contrôleur, canal vers la page, serveur local, banc de dictée, interface, rapport, messages d'erreur.

**Vérifié dans le code, sans trace** : journal (identifiant de commande, longueur et raisons à texte fixe seulement), historique (pas d'entrée pour une commande), « Copier le rapport » et `api/log` (ne lisent que le journal), pastille et erreurs (textes fixes, aucun `{}` ni `{:?}` sur la sélection), banc de dictée (`SwitchPaster` rend « aucune sélection » pendant une capture), moteurs (la sélection n'est jamais envoyée : `try_command` retourne avant toute transcription ou mise au propre ; une dictée ordinaire ne lit jamais la sélection), réseau (aucune requête), interface (ni `localStorage`, ni `sessionStorage`, ni console, ni adresse ; `nav.dictionaryPrefill` remis à `null` aussitôt ; « Entendu » n'est enregistré qu'au clic sur « Ajouter »), serveur local (route en POST, réponse `no-store`, jeton dans le chemin, aucun journal de corps de requête).

**Défauts trouvés et corrigés** (`crates/platform/src/macos/paste.rs`, `macos/ffi.rs`) :
1. *Presse-papiers non remis si l'appli copie autre chose que du texte* (image sélectionnée dans Chrome ou Figma) : il est désormais remis dès qu'il a changé, quoi qu'ait copié l'appli.
2. *Presse-papiers mixte détruit* (texte avec image ou fichiers, ex. copie du Finder) : le contrôle `clipboard info` se fait maintenant à chaque ⌘C d'essai, pas seulement quand il n'y a pas de texte ; image ou fichiers, même avec du texte à côté : pas de ⌘C, phrase collée comme du texte et message dans la pastille.
3. *⌘C tardif* (appli lente, Electron) laissant la sélection dans le presse-papiers : attente portée de 150 à 400 ms pour une commande déjà reconnue (`COMMAND_COPY_WAIT`).
4. *Remise qui échoue* : elle est vérifiée ; si elle échoue, le presse-papiers est vidé (mieux vaut le perdre que laisser la sélection dedans).
5. *Copies intermédiaires non écrasées* : les chaînes de la boucle du ⌘C d'essai et les octets d'une conversion UTF-8 échouée sont écrasés (`wipe`).

**Testé** : `has_non_text_type` (texte seul, image, fichiers, texte + image) et `wipe` ; l'ensemble des cas du contrôleur (`commands_101` à `commands_110`) reste vert. `cargo clippy --all-targets -- -D warnings` propre ; `cargo test --workspace` : 359 tests passés, 0 échec. **Non testable sans Mac réel** : le comportement du presse-papiers et du ⌘C d'essai (remise, délai, Chrome) : à vérifier à l'essai de la tâche 5.

**Limites connues, non corrigées** :
- *`ConcealedType`* : c'est l'application qui écrit la sélection dans le presse-papiers lors du ⌘C, pas Scriibe ; marquer l'écriture comme « masquée » n'est donc pas possible de notre côté. Un gestionnaire de presse-papiers (Alfred, Raycast, Paste) ou le presse-papiers universel d'Apple peut voir la sélection pendant le repli ⌘C (0,2 à 0,5 s). L'accessibilité, qui ne touche pas au presse-papiers, passe en premier ; le repli ne sert qu'aux applis qui n'exposent pas leur sélection (Chrome, Electron). Un réglage pour couper ce repli reste à décider avec Christophe.
- *Mise en forme* : un presse-papiers texte avec mise en forme (RTF, HTML) revient en texte seul après le repli, comme au collage d'une dictée.
- *Mémoire* : la copie en JSON de `api/prefill` et les tampons de la page ne sont pas écrasés (inévitable : la page doit afficher le texte) ; la chaîne interne d'`arboard` non plus.
- *Page ouverte en double* : une autre fenêtre ou un onglet périmé peut prendre le texte à la place de la bonne (défaut de fonctionnement, pas une fuite).
- *Dernière précaution au collage* : si la sélection restait malgré tout dans le presse-papiers, `paste` la prendrait pour l'ancien contenu et la remettrait 1,5 s après ; l'attente prolongée réduit ce risque sans le supprimer.

## Essai réel a

Essai du 07/11/2026 sur le Mac Studio de Christophe, `Scriibe.app` compilée le jour même (commit de la tâche 4), commande « ajoute ça au dictionnaire » sur un mot sélectionné. Un texte repère (`TEMOIN-…`) était mis dans le presse-papiers avant chaque essai et relu juste après.

| Essai | Résultat | Presse-papiers |
|---|---|---|
| TextEdit, mot « Madame » | Fenêtre Scriibe ouverte, « Entendu » rempli, « Écrit » actif. Journal : `commande: dictionnaire.ajouter (sélection : 6 signes)`, sans le mot | repère intact |
| Notes, mot « Guennec » | **1er essai raté** : phrase non reconnue (aucune ligne `commande:`), collée comme texte par-dessus la sélection (⌘Z l'a rétablie). **2e essai réussi** (7 signes) | repère intact |
| TextEdit, aucune sélection (2 fois) | Phrase collée comme du texte, pas de fenêtre ; journal : `ignorée (aucune sélection)` | repère intact |
| Chrome, mot « Workflows » | Réussi (9 signes) | repère intact |

**Verdict de Christophe :** la commande fonctionne dans TextEdit, Notes et Chrome ; le cas sans sélection se comporte comme prévu ; le mot n'apparaît pas dans le journal.

**Non résolu :** l'échec du 1er essai dans Notes. Cause probable : Whisper a transcrit la phrase un peu autrement (la reconnaissance exige que la phrase soit toute la dictée), mais le journal ne garde pas la transcription (choix de vie privée) donc non vérifié. Un 1er essai dans TextEdit (3,5 s de parole, 39 signes collés) était probablement une phrase trop longue. À surveiller : si l'échec revient, ajouter des variantes de phrase ou un journal du seul nombre de mots de la dictée.

**Non distingué :** le journal ne dit pas si Chrome a répondu par l'accessibilité ou par le ⌘C d'essai ; le presse-papiers est dans les deux cas resté intact.
