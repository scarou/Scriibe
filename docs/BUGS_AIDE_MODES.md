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
