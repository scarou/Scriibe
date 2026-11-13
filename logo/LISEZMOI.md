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
| `scriibe-logotype-windows.svg`, `-macos.svg`, `-linux.svg` | Mot complet, fond clair (dégradé de la couleur du système vers l'encre `#1C2427`) |
| `scriibe-logotype-windows-sombre.svg`, `-macos-sombre.svg`, `-linux-sombre.svg` | Mot complet, fond sombre (dégradé vers `#F4F7F7`) |
| `scriibe-logotype-mono.svg` | Mot complet, un ton (`currentColor`) |

`anime/` : les six logotypes (par système, clair et sombre) dont le curseur clignote lentement (animation CSS, immobile si le système réduit les animations) ; ils servent à la barre latérale de l'appli quand « Logo animé » est coché, le README garde les fixes. Fabriqués par le même `tools/faire_logotype.py`.

## Où la couleur est utilisée

- **L'interface** : variables CSS `--brand`, `--brand-deep`, `--brand-bright` et `--accent` dans `ui/src/app.css`, choisies par `data-os` (que le serveur local renseigne : `windows`, `macos` ou `linux`). Un seul jeu de fichiers pour les trois systèmes : rien n'alourdit l'exécutable.
- **La pastille à l'écran** : la couleur « Couleur de Scriibe » suit la marque du système.
- **L'icône de l'exécutable Windows** : `crates/cli/icons/icon.ico` et `icon.png` (bleu). L'icône de **Scriibe.app** (macOS) : `crates/cli/icons/icon-macos.png` (1024 px, tiré de `scriibe-icone-macos.svg` par Chrome sans fenêtre : `--headless=new --screenshot --default-background-color=00000000 --window-size=1024,1024`), converti en `.icns` par `tools/faire_app_macos.sh`. Celle de Linux viendra avec son étape.
- **L'icône près de l'horloge** (Windows) : un disque de la couleur de l'état (gris, rouge, orange, vert, bleu) avec la plume en blanc (`crates/platform/src/windows/plume32.alpha`, un masque de 32×32 octets).

## Le logotype

La plume qui écoute, « scrii » à la plume (**Dancing Script** 700) puis « be » à la machine (**Courier Prime** Bold), le curseur. Les lettres sont **celles de Scribe**, déjà converties en tracés (aucune police n'est livrée ni à installer) : `tools/faire_logotype.py` les relit dans `scribe-logotype.svg` (dépôt de Scribe, jamais modifié), double le « i » de « scri » et décale « be » et le curseur d'autant. Polices sous licence SIL Open Font License 1.1 (textes dans le dépôt de Scribe : `OFL-DancingScript.txt`, `OFL-CourierPrime.txt`).

```bash
python3 tools/faire_logotype.py <chemin de scribe/logo/scribe-logotype.svg> [décalage du second i, 25 par défaut]
```

## Fabriquer les icônes

```bash
node tools/faire_icones.js <dossier-temporaire>     # trace les SVG en PNG (Playwright)
python3 tools/faire_icones.py <dossier-temporaire>  # assemble icon.ico, icon.png et le masque de la plume
```

Si le dessin du symbole change, changer ensemble : les SVG de ce dossier, `ui/src/lib/logo.ts` (même tracé) et, chez Scribe, `ScribeMark.swift`.

## À faire

- Icônes macOS (`.icns`) et Linux (PNG de plusieurs tailles) à l'étape 7.
