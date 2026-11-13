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
