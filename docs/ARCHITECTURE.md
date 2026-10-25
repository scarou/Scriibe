# Architecture de Scriibe

Ce document fixe l'architecture visée. Le comportement fonctionnel est celui de [Scribe](https://github.com/scarou/scribe) (macOS) ; ce document ne le redécrit pas, il dit comment le reproduire sur trois systèmes avec un seul code.

## 1. Le parcours d'une dictée

```
touche appuyée ─► micro (mono 16 kHz) ─► touche relâchée
      ─► transcription (Whisper local ou Deepgram)
      ─► corrections de texte (signes dictés, listes, dictionnaire)
      ─► mise au propre (modèle + mode, garde-fou)  [ou texte brut si échec]
      ─► typographie française
      ─► collage dans l'appli active, presse-papiers remis
```

Ce n'est **pas** du streaming : la transcription part au relâchement. La mise au propre par un modèle de langage est le cœur de la valeur (modes, garde-fou, traduction à la volée).

## 2. Découpage

```
scriibe/
├─ crates/
│  ├─ core/        logique de texte pure : aucune dépendance système, aucun réseau
│  ├─ engines/     whisper-server, llama-server, clients Deepgram / OpenRouter / LM Studio / Ollama
│  └─ platform/    interface par système + une implémentation par système
├─ src-tauri/      application Tauri : assemble core + engines + platform, expose les commandes à l'interface
├─ ui/             interface web (TypeScript) : Réglages, Dictionnaire, Modes, Historique, Tableau de bord, Aide
├─ fixtures/       cas de référence JSON tirés de Scribe
└─ docs/
```

### `crates/core` (pur, testable partout)

Signes dictés et listes, remplacements du dictionnaire, tolérance aux variantes de Whisper, typographie française, garde-fous (`acceptsCleanup`, `acceptsTranslation`), modes et construction des prompts, historique et purge (avec les totaux par jour). Aucune entrée-sortie : des fonctions de texte et des structures de données. C'est la partie qu'on porte avec le plus de soin et qu'on teste le plus.

### `crates/engines`

Gère les processus `whisper-server` et `llama-server` (lancement, port, préchauffage, déchargement au repos, reprise sans assistant MTP si l'option est refusée) et les clients HTTP. Même principe que Scribe : les moteurs de modèles restent des processus séparés. Cela donne le même code sur les trois systèmes, les binaires CUDA, Vulkan et Metal déjà disponibles, et le déchargement de la mémoire.

### `crates/platform`

Une interface (trait) par fonction dépendant du système, et une implémentation par système, choisie à la compilation :

| Fonction | Windows | macOS | Linux |
|---|---|---|---|
| Touche globale (appui, relâchement, autre touche, modificateur) | hook clavier bas niveau | moniteur d'événements | X11 ; Wayland : portail ou repli |
| Collage + presse-papiers remis | `SendInput` Ctrl+V | `CGEvent` ⌘V | X11 : XTest ; Wayland : repli copie seule |
| Micro | WASAPI (`cpal`) | CoreAudio (`cpal`) | PipeWire / PulseAudio (`cpal`) |
| Pause des médias | sessions audio + touche média | processus audio CoreAudio | MPRIS (D-Bus) |
| Secrets (clés d'API) | Gestionnaire d'identifiants | Trousseau | Secret Service |
| Icône + indicateur | zone de notification, fenêtre toujours au premier plan sans focus | barre des menus | zone de notification (si disponible) |
| Dossier de données | `%APPDATA%\Scriibe` | `~/Library/Application Support/Scriibe` | `~/.local/share/scriibe` |

Le cœur ne connaît pas cette couche : il reçoit du texte et rend du texte.

## 3. Cas de référence (`fixtures/`)

La confiance dans le portage vient de là. Les cas de `Checks.swift` (typographie, filtre Whisper, garde-fous, signes dictés, touches, fichiers) sont extraits en JSON :

```json
{ "id": "typography-007", "name": "heure « 19h » → « 19 h »", "line": 34, "fn": "french_typography",
  "args": { "text": "Je serai là vers 19h, on dîne." }, "expected": "Je serai là vers 19\u00a0h, on dîne." }
```

Le format complet, la liste des fonctions et les outils de vérification sont décrits dans [`fixtures/README.md`](../fixtures/README.md) ; ce qui reste à reprendre est dans [`fixtures/A_FAIRE.md`](../fixtures/A_FAIRE.md).

Le cœur Rust doit les passer **à l'identique** (365 cas dans 16 fichiers). Le Swift sert d'oracle ; un écart voulu se documente dans `docs/ECARTS.md`, jamais en modifiant un cas en silence. Les cas qui dépendent d'un modèle (`--cleantest`) ne sont pas des tests automatiques : ils servent de mesures, comme dans Scribe.

## 4. Interface

Tauri 2 + interface web. Le Rust garde la logique et l'état ; l'interface affiche et envoie des commandes. Parité visée avec Scribe : onglets de Réglages, Dictionnaire, Modes personnalisables, Historique, Tableau de bord, Aide, bancs d'essai. L'indicateur de dictée est une fenêtre à part, transparente, sans focus.

## 5. Points durs connus

- **Touche de dictée** : Option droite (macOS) est AltGr sur un clavier AZERTY sous Windows. Le choix de la touche par défaut est propre à chaque système, et les touches de modification seules demandent un hook maison.
- **Collage** : bloqué dans les fenêtres lancées en administrateur sous Windows ; Wayland interdit l'injection de touches sur la plupart des compositeurs.
- **Performances** : les temps de Scribe (0,3 à 0,6 s) viennent d'un M1 Max. Sans carte graphique, `large-v3-turbo` est lent : il faudra proposer des modèles par niveau de machine.
- **Mode Amusant (schtroumpf)** : dépend de `NaturalLanguage` (Apple). Hors périmètre du premier jalon.
- **WebView** : WebKitGTK (Linux) se comporte différemment de WebView2 (Windows) et de WKWebView (macOS) ; tester l'interface sur les trois.

## 6. Décisions

| Décision | Choix | Date |
|---|---|---|
| Langage et cadre | Rust + Tauri 2, interface web | 2026-10-25 |
| Moteurs de modèles | processus externes appelés en HTTP (pas de `whisper-rs`) | 2026-10-25 |
| Version Mac actuelle | gardée inchangée, version de référence | 2026-10-25 |
| Dépôt | séparé de Scribe, privé puis public, licence MIT | 2026-10-25 |
| Ordre des systèmes | Windows → macOS → Linux X11 → Linux Wayland (bêta) | 2026-10-25 |

À trancher : la touche par défaut par système ; la gestion de Wayland ; la langue de l'interface au-delà du français ; le cadre de l'interface web (Svelte ou React).
