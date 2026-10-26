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
│  ├─ platform/    interface par système + une implémentation par système (+ faux pour les tests)
│  ├─ pipeline/    le chef d'orchestre : touche → micro → transcription → mise au propre → collage → historique
│  └─ cli/         `scriibe` : exécutable en ligne de commande avec icône de notification (jalon de l'étape 5)
├─ src-tauri/      application Tauri : assemble core + engines + platform, expose les commandes à l'interface
├─ ui/             interface web (TypeScript) : Réglages, Dictionnaire, Modes, Historique, Tableau de bord, Aide
├─ fixtures/       cas de référence JSON tirés de Scribe
└─ docs/
```

### `crates/core` (pur, testable partout)

Signes dictés et listes, remplacements du dictionnaire, tolérance aux variantes de Whisper, typographie française, garde-fous (`acceptsCleanup`, `acceptsTranslation`), modes et construction des prompts, historique et purge (avec les totaux par jour). Aucune entrée-sortie : des fonctions de texte et des structures de données. C'est la partie qu'on porte avec le plus de soin et qu'on teste le plus.

### `crates/engines`

Gère les processus `whisper-server` et `llama-server` (`EngineServer` : démarrage partagé entre les appels, attente de « prêt », arrêt, libération au repos, reprise sans assistant MTP si l'option est refusée) et les clients (`Deepgram`, `Transcriber`, `Cleaner` pour OpenRouter, LM Studio, Ollama et llama.cpp : requêtes, préchauffage, repli en Simple, garde-fous). Même principe que Scribe : les moteurs de modèles restent des processus séparés. Cela donne le même code sur les trois systèmes, les binaires CUDA, Vulkan et Metal déjà disponibles, et le déchargement de la mémoire.

### `crates/pipeline`

Port de `DictationController.swift` : appui → préchauffage (Whisper, modèle de mise au propre) → enregistrement → relâchement → transcription → filtre des phrases inventées → corrections (dictionnaire, signes dictés, listes) → traduction ou mise au propre (ou repli sur le texte brut) → typographie → collage → historique ; autre touche = annulation ; Maj ajoutée = traduction. Il ne connaît le système que par les traits de `platform` et les moteurs par `engines`. `Controller::handle_key` reçoit les événements du clavier, `tick` est appelé à intervalle régulier (alerte de micro muet, préchauffage différé de la traduction).

### `crates/platform`

Une interface (trait) par fonction dépendant du système, et une implémentation par système, choisie à la compilation (`platform::system()`). Les systèmes pas encore pris en charge ont des implémentations « pas encore pris en charge » qui compilent partout ; `platform::testing` fournit des faux (micro, collage, indicateur…) pour tester le chef d'orchestre. Le traitement du son (mélange des canaux, passage à 16 kHz, niveau, WAV), la traduction des codes de touche de Windows et le stockage (dossier de données, réglages relus quand le fichier change, historique, journal) sont communs et testés sur les trois systèmes. Le code Windows (`windows/`) est vérifié à la compilation par la CI et à la main sur un PC (`docs/ESSAI_WINDOWS.md`).

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

Le cœur Rust doit les passer **à l'identique** (399 cas dans 17 fichiers : 376 sont rejoués par `cargo test`, 14 attendent l'interface, 9 le mode Amusant). Le Swift sert d'oracle ; un écart voulu se documente dans `docs/ECARTS.md`, jamais en modifiant un cas en silence. Les cas qui dépendent d'un modèle (`--cleantest`) ne sont pas des tests automatiques : ils servent de mesures, comme dans Scribe.

## 4. Interface

**Jalon actuel (étape 5)** : `crates/cli` sert une **page d'essais** web locale (serveur `tiny_http` sur 127.0.0.1, port libre, adresse à jeton secret) : état de l'installation, téléchargements confirmés, banc de dictée (les 17 phrases de Scribe), rapport. Elle prépare l'interface complète ci-dessous et sert de banc d'essai sur les vraies machines ; ses essais passent par le chef d'orchestre avec des adaptateurs qui détournent le collage et l'historique (`web/sinks.rs`).

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
| Expressions régulières | `fancy-regex` (assertions arrière et groupes atomiques, que le moteur `regex` n'a pas) ; pas d'assertion arrière de longueur variable : on l'écrit en plusieurs assertions de longueur fixe | 2026-10-25 |
| Reconnaissance de la langue (garde-fou de la traduction) | trait `LanguageGuesser` ; implémentation `lingua` (français + 5 langues), fonction `language-detection` du cœur, activée par défaut (≈ 25 Mo de modèles) | 2026-10-25 |
| Calendrier | passé à chaque appel (`Calendar<Tz>`) : le cœur ne lit pas l'horloge du système | 2026-10-25 |
| Intégration continue | GitHub Actions sur Linux, Windows et macOS à chaque envoi | 2026-10-25 |
| Réseau et processus des moteurs | appels **bloquants** (`ureq`, pas de moteur asynchrone) sur des fils de travail ; HTTP, processus, horloge, journal et clés derrière des traits (`Http`, `Spawner`, `Clock`, `Log`, `Secrets`) pour tout tester avec un monde simulé (`FakeWorld`) | 2026-10-25 |
| Moteurs optionnels (idée) | Kyutai STT (`stt-1b-en_fr`, en continu) pour Mac (MLX) et PC NVIDIA, Whisper gardé pour les machines sans carte graphique ; serveur WebSocket à part | 2026-10-26 |
| Touche de dictée sous Windows et Linux | Ctrl droit, Maj en plus pour traduire (AZERTY : AltGr est Ctrl gauche + Alt droite pour Windows, jamais confondu avec Ctrl droit) | 2026-10-25 |
| Premier jalon de l'étape 5 | exécutable en ligne de commande avec icône de notification, avant la fenêtre Tauri | 2026-10-25 |
| Tâches de fond du chef d'orchestre | trait `Executor` (fils en réel, exécution immédiate ou manuelle dans les tests) | 2026-10-25 |
| Retrouver un serveur resté d'une session précédente | `sysinfo` (lignes de commande des processus), à la place de `pkill`, qui n'existe pas sous Windows | 2026-10-25 |

À trancher : la gestion de Wayland ; la langue de l'interface au-delà du français ; le cadre de l'interface web (Svelte ou React).
