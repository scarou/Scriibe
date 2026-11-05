<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="logo/scriibe-logotype-windows-sombre.svg">
    <img src="logo/scriibe-logotype-windows.svg" alt="Scriibe" width="420">
  </picture>
</p>

# Scriibe

🇬🇧 [English version](README.en.md)

Dictée vocale pour Windows, macOS et Linux. Maintenir une touche, parler, relâcher : le texte est transcrit, mis au propre et collé dans l'appli ouverte.

Scriibe est la suite multiplateforme de [Scribe](https://github.com/scarou/scribe) (macOS, Swift), qui reste la **version de référence** du comportement. Le but est la même expérience sur les trois systèmes, avec un code unique (Rust + Tauri).

> **État (S16, clôture du 04/11/2026) : version 0.2.3 publiée (Mac, Windows, Linux) ; mise à jour intégrée du Mac essayée avec succès (0.2.2 → 0.2.3) ; d'autres changements (assistant « local d'abord », Aide adaptée au système) attendent la prochaine version.** **État (S15, clôture du 04/11/2026) : version 0.2.2 publiée (Mac, Windows) ; mise à jour intégrée sur Mac écrite (pas encore essayée) ; version Linux en essai ; sur Mac, la pause des vidéos de Chrome passe par JavaScript et le collage dans les pages web est plus fiable.** **État (S14, clôture du 03/11/2026) : interface refaite à la manière de Scribe (Réglages à onglets, cartes repliables, apparence claire/sombre réglable), dictée sur une sélection (Mac), documentation en anglais, version 0.2.0 publiée (Mac et Windows).** **État (S14, début) : dépôt public ; alerte de nouvelle version et mise à jour Windows par les Releases GitHub, sans jeton (la CI ajoute `scriibe.exe` à chaque Release).** **État (S13) : touches de dictée et de traduction réglables sur Mac (Option, ⌘, Fn, Ctrl, Maj, F1 à F20, Début, Fin, pages), essayées sur le Mac ; CI non relancée depuis le 01/11/2026 (commits `[skip ci]`).** **État (S12) : alerte de nouvelle version (Mac et Windows), filtres de l'historique, explications de l'interface, ligne d'état dans la barre latérale ; CI verte sur les trois systèmes le 01/11/2026.** **État (S11) : l'appli Mac existe** (Scriibe.app : fenêtre, icône de la barre des menus, pastille, une seule instance ; dictée essayée sur le Mac, [docs/ESSAI_MACOS.md](docs/ESSAI_MACOS.md)). **État (S08) : utilisable sous Windows, parité avec Scribe presque atteinte, essayée sur le PC ; interface en français et en anglais ; premier jalon Linux (X11) écrit et essayé sur écran virtuel.** S08 ajoute : une Aide pour novices, l'assistant de premier lancement relançable, la dictée **dans la fenêtre de Scriibe**, l'**interface en français et en anglais** (réglage « Langue de l'interface », phrases de test anglaises). Rappel S07 : comparaison « avec / sans modèle » du banc, boîtes de la page, onglet Installation refondu (recherche sur Hugging Face, modèles installés), repli sur le processeur quand la carte graphique manque de mémoire, compilation sur le PC de Christophe, et la couche Linux X11 (touche, collage, micro, sons, pause des médias ; [docs/ESSAI_LINUX.md](docs/ESSAI_LINUX.md)). Rappel S06 : Le cœur, les moteurs, la couche système Windows (touche, micro, collage, icône, sons, pause des médias), le chef d'orchestre, la fenêtre (Tauri 2, WebView2), l'interface complète (Svelte 5 : tableau de bord, historique avec le mode de chaque dictée, modes et phrases types, dictionnaire et signes dictés, réglages, installation, banc d'essai de la dictée et de la mise au propre, assistant de premier lancement, Aide) et la **mise à jour intégrée** sont essayés sur un vrai PC. S06 ajoute : **touches de dictée et de traduction réglables** (essayées), **LM Studio et Ollama**, **sons** de début et de fin, **menu de l'icône** (mode en un clic), banc d'essai de la mise au propre, préchargement relancé quand on change de modèle, le **logotype**. Mesuré sur un portable sans vraie carte graphique : Whisper `base` local + mise au propre OpenRouter ≈ 3 à 4 s après le relâchement (Scribe sur M1 Max : 0,8 à 2 s) ; les petits modèles locaux (Qwen 0,5 et 1,5 B) reformulent mal les modes de style (essai des 12 phrases types). Installer sur un Mac, pas à pas : [docs/INSTALLATION_MACOS.md](docs/INSTALLATION_MACOS.md). Voir [docs/ESSAI_WINDOWS.md](docs/ESSAI_WINDOWS.md) pour l'essayer, [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) pour le plan, [ROADMAP.md](ROADMAP.md) pour les étapes et [CHANGELOG.md](CHANGELOG.md) pour l'historique.

## Installer

Pas à pas, pour une personne qui n'a jamais compilé de programme :

- **Mac** (puce Apple, macOS 14 ou plus) : [docs/INSTALLATION_MACOS.md](docs/INSTALLATION_MACOS.md) ([English](docs/INSTALLATION_MACOS.en.md)).
- **Windows** (10 ou 11, 64 bits) : [docs/INSTALLATION_WINDOWS.md](docs/INSTALLATION_WINDOWS.md) ([English](docs/INSTALLATION_WINDOWS.en.md)).
- **Linux** (X11 ; Wayland n'est pas pris en charge ; expérimental) : [docs/INSTALLATION_LINUX.md](docs/INSTALLATION_LINUX.md) ([English](docs/INSTALLATION_LINUX.en.md)). Le fichier `scriibe-linux-x64.zip` est ajouté aux Releases à partir de la prochaine version publiée ; l'état de l'essai est dans [docs/ESSAI_LINUX.md](docs/ESSAI_LINUX.md), et on peut aussi compiler soi-même (voir ci-dessous).

Les versions à télécharger sont dans les [Releases](https://github.com/scarou/Scriibe/releases) du dépôt. Les moteurs locaux (Whisper, llama.cpp) sont installés ou téléchargés depuis l'application, avec l'accord de l'utilisateur à chaque téléchargement ; les moteurs en ligne (Deepgram, OpenRouter) demandent une clé personnelle.

### Compiler soi-même

Prérequis : [Rust](https://rustup.rs) et [Node.js](https://nodejs.org). Ordre à respecter (l'interface est embarquée dans l'exécutable) :

```bash
cd ui && npm ci && npm run build && cd ..
cargo build --release
```

L'exécutable est `target/release/scriibe` (`scriibe.exe` sous Windows). Sur Mac, `tools/faire_app_macos.sh` en fait une `Scriibe.app`. Sous Linux, la compilation demande au moins `libasound2-dev` (voir `docs/ESSAI_LINUX.md` pour l'état).

## Principes

- **Local d'abord** : transcription par `whisper-server` (whisper.cpp) et mise au propre par `llama-server`, LM Studio ou Ollama ; le cloud (Deepgram, OpenRouter) reste un choix, jamais une obligation.
- **Le comportement est mesuré** : la logique de texte est vérifiée par des cas de référence (`fixtures/`) tirés de Scribe.
- **Ouvert** : licence MIT, développement en français, interface, Aide et procédures d'installation en français et en anglais ([README en anglais](README.en.md)).

## Licence

[MIT](LICENSE)
