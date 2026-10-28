# Logo de Scriibe

Le symbole est celui de Scribe : **une plume de stylo percée d'une onde sonore** (la voix entre dans la plume, l'encre en sort). Scriibe en garde le dessin à l'identique ; ce qui change d'un système à l'autre, c'est la couleur de la marque.

| Système | Couleur | Dégradé de l'icône (clair, milieu, foncé) |
|---|---|---|
| macOS | turquoise (celle de Scribe) | `#3FD6CB`, `#00B8AD`, `#009C93` |
| Windows | bleu | `#5B97F7`, `#2F6FED`, `#2257C7` |
| Linux | violet | `#9B7BF2`, `#7B52E0`, `#5E3CC0` |

| Fichier | Usage |
|---|---|
| `scriibe-symbole.svg` | Symbole seul, un ton (`currentColor`) : petites tailles, barre latérale |
| `scriibe-icone-windows.svg`, `-macos.svg`, `-linux.svg` | Icône de l'appli (grille macOS : carré de 824 px dans 1024), une par système |

## Où la couleur est utilisée

- **L'interface** : variables CSS `--brand`, `--brand-deep`, `--brand-bright` et `--accent` dans `ui/src/app.css`, choisies par `data-os` (que le serveur local renseigne : `windows`, `macos` ou `linux`). Un seul jeu de fichiers pour les trois systèmes : rien n'alourdit l'exécutable.
- **La pastille à l'écran** : la couleur « Couleur de Scriibe » suit la marque du système.
- **L'icône de l'exécutable Windows** : `crates/cli/icons/icon.ico` et `icon.png` (bleu). Les icônes macOS et Linux seront fabriquées à l'étape 7, avec le même outil.
- **L'icône près de l'horloge** (Windows) : un disque de la couleur de l'état (gris, rouge, orange, vert, bleu) avec la plume en blanc (`crates/platform/src/windows/plume32.alpha`, un masque de 32×32 octets).

## Fabriquer les icônes

```bash
node tools/faire_icones.js <dossier-temporaire>     # trace les SVG en PNG (Playwright)
python3 tools/faire_icones.py <dossier-temporaire>  # assemble icon.ico, icon.png et le masque de la plume
```

Si le dessin du symbole change, changer ensemble : les SVG de ce dossier, `ui/src/lib/logo.ts` (même tracé) et, chez Scribe, `ScribeMark.swift`.

## À faire

- **Logotype** (le mot « Scriibe » en tracés, comme celui de Scribe avec Dancing Script et Courier Prime) : pas encore dessiné ; la barre latérale écrit le nom en texte à côté du symbole.
- Icônes macOS (`.icns`) et Linux (PNG de plusieurs tailles) à l'étape 7.
