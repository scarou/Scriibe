# Bugs Aide et modes (chantier 2 du tri du 12/11)

Deux lignes du carnet du 11/11 : « Bug · Aide : le menu de gauche disparaît » et « Bug · Le mode choisi dans les réglages n'est pas synchronisé avec celui de l'icône du menu ». Découpage : `decoupage-2026-11-12-bugs-aide-modes.md` (vault IA-OS).

## Constat

Source : recherche en lecture seule (sous-agent Sonnet, 12/11), puis relecture des points clés dans le code. **Rien n'a été lancé à l'écran** : les certitudes sont celles de la lecture du code. Chemins relatifs à la racine du dépôt.

### 1. Aide : le sommaire défile avec la page

**Cause (certitude ~90 %).** Le sommaire (`.toc`) n'a **jamais eu de `position: sticky`** : ce n'est pas une régression. Le commit 8841dd6 (06/11) n'est pas en cause : il n'a touché dans l'Aide qu'une ligne (`let os = $derived(status.os)`), et dans `Card.svelte` que la prop `accent`. Le CSS de `.layout` et `.toc` est identique depuis bb43028 (27/10) ; les cartes repliables ne sont pas utilisées dans l'Aide.

- Le défilement est celui du **document** : aucun ancêtre n'a d'`overflow` ni de hauteur fixe (`html, body { height: 100% }`, `.shell { min-height: 100vh }`, `main { flex: 1 }`). Le `.toc` est un bloc normal du flux, il monte avec la page.
- Fichiers : `ui/src/tabs/Help.svelte:93-94` (structure), `:305-306` (CSS `.layout`, `.toc`) ; idem `ui/src/tabs/HelpEn.svelte:106-107`, `:318-319`. Contexte : `ui/src/App.svelte` (`.shell`, `nav`, `main`, media query 640 px).
- Piste secondaire : si c'est la **barre de gauche principale** qui disparaît, seule la media query `@media (max-width: 640px)` de `App.svelte` la passe en `position: static` ; la fenêtre Tauri a `min_inner_size(640, 480)` (`crates/cli/src/window.rs:55`), donc cette largeur est atteignable. À confirmer avec l'auteur lequel des deux menus il voit disparaître.
- **Scribe** : même structure (`HelpView.swift:57-66`, sommaire de 228 pt dans un `HStack` à l'intérieur du `ScrollView` de `RootView.swift:12`) : le sommaire défile aussi. Seule la barre latérale principale est fixe. Ce n'est donc pas un écart à corriger vers Scribe : c'est une amélioration voulue.

**Piste de correction.** `position: sticky; top: 20px` (le padding de `main`) sur `.toc` dans les deux fichiers, `max-height` + `overflow-y: auto` pour les petites fenêtres, sticky désactivé par media query quand `flex-wrap` met le sommaire au-dessus du corps. Option : remonter en haut au changement de section.

### 2. Modes : réglages et icône désynchronisés

**Stockage : une source unique.** `settings.json` (clés `rewriteMode`, `customModeID`, `cleanupEnabled`) ; modèle `crates/core/src/settings.rs` (`mode_choice()` :626, `set_mode_choice()` :632, `rewriting_on()` :552 = `cleanup_enabled && engine != None`) ; fichier `crates/platform/src/storage.rs` (`FileConfig`, cache `Watched` invalidé par `mtime`, une seule instance partagée par le serveur web et `TraySource`, `crates/cli/src/commands.rs:141-146`). Pas de cache divergent. **Après redémarrage, tout relit le fichier : rien ne diverge.**

**Qui écrit.**
- Page : `ui/src/tabs/Modes.svelte:41-44` → `POST api/modes/choose` → `choose_mode()` (`crates/cli/src/web/mod.rs:745-753`) : `set_mode_choice` + `save_settings`, **sans** `cleanup_enabled = true` (vérifié).
- Icône : `TraySource::choose_mode` (`crates/cli/src/tray_source.rs:155-167`) : `set_mode_choice` + `cleanup_enabled = true` + `save_settings` (vérifié), même logique que Scribe.

**Qui lit.**
- Menu de l'icône : sondage toutes les 500 ms (`crates/cli/src/mac_tray.rs:312-327`, fil `scriibe-menu`) ; si la `MenuView` change, `Items::refresh` (:191-246) met à jour les cases. Pas de reconstruction à l'ouverture (`tray-icon` n'a pas d'évènement d'ouverture, `docs/ECARTS.md:29`). Le coché vient de `settings.rewriting_on().then(|| settings.mode_choice())` (`tray_source.rs:47`). Windows : même mécanisme, `crates/platform/src/windows/tray.rs`.
- Page : **une seule fois**, dans `onMount` (`Modes.svelte:75-78` : `load()` + `settings.load()`). La boucle de `ui/src/lib/status.svelte.ts` (8 s et 1 s) ne relit ni les modes ni les réglages.

**Sens des échecs.**

- **A. Icône → page (certitude ~90 %).** Clic sur un mode dans le menu → `settings.json` change, mais la page ouverte sur Réglages → Modes garde l'ancien radio coché et « Mode en cours : … » (`modes` et `settings.value` ne sont relus qu'au montage ou après une action de la page) jusqu'à un changement d'onglet ou un rechargement. `TrialBench.svelte:102` lit aussi ce store périmé.
- **B. Icône → icône, macOS et Linux (certitude ~75 %, non vérifiée à l'écran).** `muda` 0.20 bascule lui-même une `CheckMenuItem` au clic (`muda-0.20.0/src/platform_impl/macos/mod.rs:830-835`). Un clic sur le mode **déjà coché**, ou que `choose_mode` ignore (moteur absent, mode personnel supprimé), décoche la case native alors que `settings.json` ne change pas ; la `MenuView` reste identique, donc `refresh` n'est pas rappelé (`mac_tray.rs:318`) et le menu n'affiche plus aucun mode coché. **Windows corrige explicitement ce cas** (`windows/tray.rs` ~:270-274 : `shown = None` + rafraîchissement forcé) ; `mac_tray.rs` n'a pas d'équivalent. Cohérent avec `CHANGELOG.md:178` (« clics sur les modes non essayés » sur Mac).
- **C. Page → icône (cas limites).** Normalement correct sous 0,5 s. Mais la page ne force pas `cleanup_enabled`, alors que le menu masque le coché si `!rewriting_on()` : choix fait dans la page avec le nettoyage désactivé (ou moteur `none`, ou `settings.json` venu de Scribe avec `cleanupEnabled:false`) → page cochée, icône sans coché. Un mode `playful` n'est plus listé dans le menu alors que la page le ramène à Simple (`crates/cli/src/web/data.rs:35`). Non prouvé : l'affichage d'un sous-menu **déjà ouvert** mis à jour depuis un fil.
- Course possible entre les deux écritures (lecture-modification-écriture sans verrou commun) : fenêtre étroite, hors sujet ici.

**Scribe (référence, non modifié).** `Store.settings` est `@Published` (`Store.swift:12`), source unique ; `AppDelegate.menuWillOpen` (:178-204) recalcule **à chaque ouverture** coché et titres depuis `store.settings.rewritingOn ? modeChoice : nil`, donc ni sondage ni état natif à resynchroniser ; `selectMode` (:206-212) et le bouton « Utiliser ce mode » de `ModesView.swift:260-264` posent `modeChoice` **et** `cleanupEnabled = true` ; la page se redessine toute seule quand le menu change le mode (`ObservableObject`).

**Pistes de correction.**
- A : relire `modes` et `settings` dans la page (retour de focus et/ou boucle de `status.svelte.ts` toutes les 2 à 8 s, ou inclure `rewriteMode`, `customModeID`, `cleanupEnabled` dans `api/activity` lu chaque seconde, avec rechargement si ça change).
- B : dans `mac_tray.rs`, après tout clic sur un mode (`MODE_PREFIX`), forcer un `refresh` même sans changement de vue (vider `shown`), comme Windows.
- C : `choose_mode` web fait aussi `cleanup_enabled = true`, comme le menu et comme Scribe (à confirmer : écart voulu ou non dans `docs/ECARTS.md`).

### À confirmer par un essai sur Mac (tâches suivantes)
1. Cocher un mode dans le menu, regarder la page (A).
2. Recliquer le mode déjà coché, regarder le menu (B).
3. Changer le mode dans la page, regarder le menu, avec et sans nettoyage activé (C).
4. Aide : descendre dans la page, regarder quel menu disparaît, et à quelle largeur de fenêtre.

## Correctif Aide

Fait le 12/11 (tâche 1). Le « menu de gauche » traité est le sommaire de l'Aide ; la barre principale était déjà fixe.

- `ui/src/tabs/Help.svelte` et `HelpEn.svelte` : `.toc` en `position: sticky; top: 20px; max-height: calc(100vh - 40px); overflow-y: auto`, largeur `clamp(140px, 24%, 220px)` ; `.layout` en `flex-wrap: nowrap` ; `.body` en `flex: 1 1 0; min-width: 0`.
- **Pourquoi pas de retour à la ligne** : avec l'ancien `flex-wrap: wrap` et une base de 360 px pour la carte, le sommaire passait **au-dessus** du texte dans toute fenêtre de moins d'environ 1050 px (la plus petite, 640 px, comprise) ; un sticky l'aurait alors fait recouvrir le texte. Sans retour à la ligne, le sommaire reste à gauche (140 px dans la plus petite fenêtre) et seule la carte défile. Sous 640 px, la barre principale passe en haut et l'Aide occupe toute la largeur.
- Test : `ui/src/lib/tabs.test.ts` (« Aide : sommaire fixe »), pour les deux fichiers : sticky, `max-height` en `vh`, `overflow-y: auto`, `nowrap`, `min-width: 0`. Il lit le CSS des sources (`?raw`) : il garde la structure, pas le rendu.
- Résultat : `npm test` 88 passés (16 fichiers), `npm run check` 0 erreur, `npm run build` ok, `cargo test --workspace` sans échec.
- **Non vérifié à l'écran** : rendu réel en petite et en grande fenêtre, à voir à l'essai réel (tâche 3).

## Correctif modes

Fait le 12/11 (tâche 2). Source unique : `settings.json` (déjà le cas) ; ce qui manquait, c'est que chaque côté le **lise** et l'**écrive** de la même façon.

**Tests écrits d'abord et vus en échec (R16)** : `crates/cli/src/mode_sync.rs` (choix de la page → coche du menu avec mise au propre coupée ; mode Amusant retiré → coché Simple dans le menu comme dans la page) et `mac_tray::tests` (clic sur un mode → remise d'accord forcée). Trois échecs pour la bonne raison avant correction. Les tests « menu → page » et « après redémarrage » passaient déjà (source unique) : gardés comme filet. Page (relecture) : `ui/src/lib/modesSync.test.ts`, écrit avant le module (échec : module absent). Pour tester sans fenêtre, deux extractions sans changement de comportement : `web::pick_mode` et `mac_tray::needs_refresh`.

**Corrections**
- **Page → icône (C)** : `AppSettings::pick_mode` (`crates/core/src/settings.rs`), partagée par la page (`web::pick_mode`) et le menu (`TraySource::choose_mode`) : change le mode **et** rallume la mise au propre, comme `selectMode` et « Utiliser ce mode » de Scribe. Avant, un choix fait dans la page avec la reformulation coupée n'était coché nulle part dans le menu.
- **Mode Amusant retiré** : `menu_view_in` le coche comme Simple, comme la page.
- **Icône → page (A)** : `ui/src/lib/modesSync.ts` (`watchModes`) ; `Modes.svelte` relit `api/modes` toutes les 1,5 s tant que la page est ouverte et se met à jour si le choix affiché diffère (le comparatif se fait avec ce que la page affiche, pas avec la dernière lecture).
- **Icône → icône sur Mac et Linux (B)** : `mac_tray.rs`, `MODE_CLICKED` levé après chaque clic sur un mode ; le fil du menu force alors la remise d'accord des cases (`needs_refresh(.., forced)`), comme Windows. **Non vérifié à l'écran** (Mac : tâche 3).
- **Redémarrage** : couvert par `the_same_mode_is_found_by_the_page_and_the_menu_after_a_restart` (nouvelle lecture du dossier).
- **Pendant une dictée** : aucun changement de code, comportement fixé par un test (`a_mode_changed_while_listening_…`, `crates/pipeline/tests/dictation.rs`). Les réglages sont photographiés une fois, **au relâchement de la touche** : un mode changé pendant l'écoute vaut pour cette dictée ; changé pendant le traitement, il vaut pour la suivante (mode du prompt et mode gardé dans l'historique restent d'accord). Choix recommandé : une dictée en cours ne change jamais de mode en route.

**Résultat** : `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings` sans avertissement ; `cargo test --workspace` sans échec (dont les cas de référence de `fixtures/`) ; `npm test` 91 passés (17 fichiers), `npm run check` 0 erreur, `npm run build` ok.

**Reste à vérifier à la main (tâche 3)** : le sous-menu de l'icône affiche-t-il la mise à jour des cases pendant qu'il est ouvert (appels depuis un fil, non prouvé) ; le rendu Mac de B ; Windows et Linux non essayés ici.

## Essai réel

Fait le 12/11 (tâche 3), sur le Mac Studio de l'auteur, avec `Scriibe.app` compilée (`cargo build --release`, `tools/faire_app_macos.sh`, signée « Scribe Dev ») et lancée par `open`. Journal : fenêtre prête, icône installée, moteurs prêts.

Parcours en six étapes, une par message ; l'auteur a répondu « ok » à chacune, sans détail :
1. Aide, grande fenêtre : ok.
2. Aide, petite fenêtre : ok.
3. Aide en anglais (petite puis grande fenêtre), retour au français : ok.
4. Modes, page → menu de l'icône : ok. Au premier passage, le fichier de réglages disait encore « simple » alors que l'auteur avait fait d'autres essais entre-temps ; relu, il disait « formal », d'accord avec la page.
5. Modes, menu → page, puis clic sur le mode déjà coché : ok. Vérifié dans le fichier : `rewriteMode` = `calm` après le clic dans le menu.
6. Quitter puis relancer : ok.

**Verdict de l'auteur : « ok » à chaque étape, donc les deux correctifs (Aide et modes) sont acceptés.**

Limites : les réponses ne disent pas si la coche d'un sous-menu **resté ouvert** se met à jour toute seule (question posée à l'étape 4, sans réponse précise) ; Windows et Linux ne sont pas essayés ici ; seuls les passages « menu → fichier » et « redémarrage » sont confirmés par la lecture du fichier de réglages.

## Ajouts

Aucune nouvelle demande pendant l'essai.

## Relecture avant livraison (12/11, agent distinct)

Aucun point bloquant ; source unique, `pick_mode` atomique, remise d'accord du menu Mac sans course dangereuse, fixtures et tests existants intacts. **Rien n'a été corrigé** (accord de l'auteur requis) ; restent :

**À corriger**
1. **Aide en fenêtre étroite** (`Help.svelte:306-307`, `HelpEn.svelte:319-320`) : de 641 à environ 800 px de large, la carte de texte ne fait que 236 à 300 px (sommaire ≥ 140 px, barre principale 210 px). De plus, le clic sur une section du sommaire ne remonte pas la page : après avoir descendu une section longue, une section plus courte peut s'ouvrir avec son titre hors de vue (option déjà notée au Constat : remonter en haut au changement de section). La largeur essayée à l'essai réel n'a pas été notée.
2. **Page et menu pas « vus pareil » si la reformulation est coupée ou sans moteur** : moteur `none` → menu grisé sans coche, page avec radio Simple coché ; `settings.json` venu de Scribe avec `cleanupEnabled:false` → page cochée, menu non. Aucun réglage de l'interface ne touche `cleanupEnabled` (seuls `TrialBench` et `web/trial.rs`). À afficher dans la page ou à noter dans `docs/ECARTS.md`.
3. **Test de `MODE_CLICKED`** : seul le prédicat `needs_refresh` est testé ; supprimer le `store(true)` ou la lecture de `forced` laisserait le test vert. Le comportement réel n'est couvert que par l'essai manuel.
4. **Ordre au montage de `Modes.svelte`** : `settings.load()` ne part plus qu'après la réussite de `load()` (avant : en parallèle) ; si `api/modes` échoue, boutons « Essayer » grisés jusqu'au sondage suivant (1,5 s).

**Mineurs** : réponse périmée du sondage pouvant écraser un choix de la page pendant 1,5 s ; `watchModes` ne contrôle pas la forme de la réponse (latent) ; Amusant : coche Simple mais titre « Amusant » (page et menu) ; `pick_mode` rallume `cleanup_enabled` même sans moteur (voulu, aligné sur Scribe, **pas encore noté dans `docs/ECARTS.md`**) ; le test « pendant une dictée » ne compare que le libellé gardé, pas le prompt ; drapeau `MODE_CLICKED` jamais levé si `choose_mode` paniquait ou si la source n'est pas prête ; section « Essai réel » mince (réponses « ok » sans détail).
