# Logo animé (chantier 8)

Idée du carnet du 11/11 : le curseur du logo clignote lentement ; animé par défaut, fixe au choix dans les réglages. Choix retenu (13/11) : SVG animé, pas de GIF.

## Constat

*Relevé le 13/11/2026 (tâche 0), sans modification du code.*

### Où le logo s'affiche dans l'appli

- **Un seul endroit : la barre latérale** de la fenêtre (`ui/src/App.svelte`, ligne `<div class="brand"><img src={dark ? logo.dark : logo.light} …/></div>`). C'est le **logotype** (le mot complet) en `<img>`, pas inline. Masqué en affichage étroit (`.brand { display: none }`).
- `ui/src/lib/logotype.ts` choisit le fichier : `logo/scriibe-logotype-{windows,macos,linux}[-sombre].svg` selon `status.os` et le mode sombre (importés par `?url`, donc embarqués par Vite).
- **Pas d'autre emplacement** : ni Tableau de bord, ni À propos, ni Aide, ni fenêtre de pastille n'affichent le logo (aucune autre occurrence de `logotype`, `<img` ou `<svg` dans `ui/src` hors `Logo.svelte`).
- `ui/src/components/Logo.svelte` (symbole seul, `currentColor`, tracé de `ui/src/lib/logo.ts`) **n'est utilisé nulle part** : à ignorer pour l'animation.
- Hors page : icône de l'exécutable et de Scriibe.app, icône près de l'horloge (plume sans curseur) : images fixes, hors périmètre. Le `README.md` garde le logotype fixe (décidé).

### Comment le curseur est dessiné

- Dans chacun des 7 logotypes (`logo/scriibe-logotype-*.svg`, 6 par système + `-mono`) : **un élément `<rect>` à part**, le dernier du fichier, `x≈477.80 y=58 width=7 height=94 rx=2`, rempli d'une couleur unie (turquoise `#00B8AD` pour macOS, pas du dégradé `url(#g)` des lettres). Exemple : `<rect x="477.80" y="58" width="7" height="94" rx="2" fill="#00B8AD"/>`.
- Il est produit par `tools/faire_logotype.py` (ligne `<rect x="{cursor:.2f}" …>`), qui régénère les 7 fichiers ; **c'est là qu'il faut ajouter l'animation**, pas à la main dans les SVG.
- Les icônes (`scriibe-icone-*.svg`) et le symbole n'ont pas de curseur.

### Conséquences pour la tâche 1 (recommandations)

1. **Animer par CSS (`@keyframes` sur l'opacité) dans le `<rect>` du SVG**, via `faire_logotype.py`. Les animations CSS d'un SVG chargé en `<img>` tournent dans les trois webviews, et le fichier reste fixe pour le README, qui garde les fichiers actuels : publier les versions animées sous un autre nom (ex. `logo/anime/…`) pour ne pas animer le README.
2. **Réglage « animé / fixe » : choisir entre deux fichiers** (`logotype.ts` renvoie l'URL animée ou la fixe). Un `<img>` ne laisse pas la page piloter son CSS interne ; deux fichiers évitent d'inliner le SVG.
3. **Réduction des animations** : `@media (prefers-reduced-motion: reduce)` dans le SVG lui-même (fonctionne en `<img>`), en plus du réglage ; à vérifier sur WebView2 et WebKitGTK.
4. **Mode sombre** : déjà géré par le choix du fichier (`-sombre`) ; il faut donc 6 fichiers animés (3 systèmes × clair/sombre).
5. Le réglage est à ajouter dans la configuration (comme les autres cases d'apparence) ; `i18n.test.ts` exigera sa traduction EN.

## Logo animé

*Tâche 1, 13/11/2026.*

- **Logos** : `tools/faire_logotype.py` fabrique aussi `logo/anime/scriibe-logotype-{windows,macos,linux}[-sombre].svg` : le `<rect>` du curseur reçoit la classe `c`, animée par CSS (`opacity`, cycle de 2,4 s, plein puis éteint en douceur). `@media (prefers-reduced-motion: reduce)` dans le SVG coupe l'animation. Les 7 logos fixes sont inchangés (régénérés, aucune différence) ; le README garde les fixes.
- **Réglage** : `animatedLogo` (`AppSettings.animated_logo`, vrai par défaut, gardé dans les réglages), case « Logo animé » dans Réglages → Apparence. `logotype(os, animate)` choisit le fichier animé ou fixe ; clair et sombre par les fichiers `-sombre`.
- **Aide** FR et EN (Réglages → Apparence), traductions EN ajoutées.
- **Tests** : `settings_scriibe.rs` (défaut vrai, valeur gardée), `logotype.test.ts`. `cargo fmt`, `clippy -D warnings`, `cargo test --workspace`, `npm run check`, `npm test` (97) et `npm run build` : sans erreur.
- **Pas vérifié** (tâche 2 sur le Mac, puis PC) : le clignotement à l'écran, le rendu en WebView2 et WebKitGTK, la réduction des animations.

## Essai réel

*Tâche 2, Mac de l'auteur, 13/11/2026 ; Scriibe.app compilée en release, lancée par `open`.*

| Étape | Résultat |
|---|---|
| Curseur du logo (clair) | Clignote lentement : vu. |
| Case « Logo animé » décochée | Logo fixe : vu. |
| Après redémarrage (réglage mis à « faux » par moi via le serveur local, pour l'essai) | Logo fixe : vu. |
| Case recochée | Clignote de nouveau ; `settings.json` passe à `true` (12:30:03) : vu. |
| Mode sombre du Mac | Logo sombre lisible, curseur clignote : vu. |
| « Réduire les animations » (Accessibilité) | Logo fixe : vu. |

**Écart noté, clos** : juste après le premier décochage, `settings.json` contenait encore `animatedLogo: true` alors que l'écran montrait le logo fixe (cause non identifiée). Contrôle refait : un décochage par clic écrit bien `false` dans le fichier (12:34:33), comme le cochage écrit `true`. Les deux chemins marchent ; l'écart n'est pas reproduit.

## Ajouts

Aucune demande nouvelle pendant l'essai.

## Essai réel : PC Linux (GNOME 50, Wayland, WebKitGTK 2.52)

*13/11/2026, programme compilé sur le PC au commit `6bcda46`.*

- Curseur du logo : clignote. Case décochée : fixe, gardé après relance. Recochée : clignote. Mode sombre : lisible, clignote. Tous vus.
- **Réduction des animations : défaut trouvé.** Le curseur clignotait encore, avec GNOME (`org.gnome.desktop.a11y.interface reduced-motion = reduce`) et le portail (`org.freedesktop.appearance reduced-motion = 1`) corrects, Scriibe relancé. WebKitGTK ne le transmet pas à la page.
- **Correction** : `crates/platform/src/motion.rs` lit le portail (ashpd, `Settings::reduced_motion`, gardé 5 s) ; l'état l'envoie (`reduced_motion`) ; `shouldAnimate` (`ui/src/lib/logotype.ts`) garde le logo fixe. Sur macOS et Windows la fonction rend faux : la webview transmet déjà le réglage (Mac essayé ; Windows à essayer).
- **Deuxième défaut, trouvé à l'essai** : la première version de la correction ne marchait pas dans le programme (état : `reduced_motion` faux) alors qu'une lecture isolée rendait « réduit ». Cause probable : `Settings::new()` d'ashpd partage une connexion D-Bus tenue par la boucle des portails (`linux/wayland.rs`). La lecture ouvre maintenant sa propre connexion (`Settings::with_connection`). Essai à la main : `cargo test -p scriibe-platform --lib motion -- --ignored --nocapture`.
- **Verdict Linux (programme `04bc842`)** : l'état dit `reduced_motion` vrai ; avec « Réduire les animations » activé, logo **fixe** (vu). Reste à essayer sur le PC Windows.
