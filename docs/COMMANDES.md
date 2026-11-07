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
- **Manquent** (pour l'inventaire de la tâche 6) : aucun signe anglais ; virgule, point d'exclamation, point d'interrogation, deux-points, point-virgule, arobase, barre oblique, dièse, etc.
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
