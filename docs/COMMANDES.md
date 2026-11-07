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
