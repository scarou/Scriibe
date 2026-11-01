<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="logo/scriibe-logotype-windows-sombre.svg">
    <img src="logo/scriibe-logotype-windows.svg" alt="Scriibe" width="420">
  </picture>
</p>

# Scriibe

Dictée vocale pour Windows, macOS et Linux. Maintenir une touche, parler, relâcher : le texte est transcrit, mis au propre et collé dans l'appli ouverte.

Scriibe est la suite multiplateforme de [Scribe](https://github.com/scarou/scribe) (macOS, Swift), qui reste la **version de référence** du comportement. Le but est la même expérience sur les trois systèmes, avec un code unique (Rust + Tauri).

> **État (S12) : alerte de nouvelle version (Mac et Windows, jeton GitHub en lecture seule), filtres de l'historique, explications de l'interface, ligne d'état dans la barre latérale ; CI verte sur les trois systèmes le 01/11/2026.** **État (S11) : l'appli Mac existe** (Scriibe.app : fenêtre, icône de la barre des menus, pastille, une seule instance ; dictée essayée sur le Mac, [docs/ESSAI_MACOS.md](docs/ESSAI_MACOS.md)). **État (S08) : utilisable sous Windows, parité avec Scribe presque atteinte, essayée sur le PC ; interface en français et en anglais ; premier jalon Linux (X11) écrit et essayé sur écran virtuel.** S08 ajoute : une Aide pour novices, l'assistant de premier lancement relançable, la dictée **dans la fenêtre de Scriibe**, l'**interface en français et en anglais** (réglage « Langue de l'interface », phrases de test anglaises). Rappel S07 : comparaison « avec / sans modèle » du banc, boîtes de la page, onglet Installation refondu (recherche sur Hugging Face, modèles installés), repli sur le processeur quand la carte graphique manque de mémoire, compilation sur le PC de Christophe, et la couche Linux X11 (touche, collage, micro, sons, pause des médias ; [docs/ESSAI_LINUX.md](docs/ESSAI_LINUX.md)). Rappel S06 : Le cœur, les moteurs, la couche système Windows (touche, micro, collage, icône, sons, pause des médias), le chef d'orchestre, la fenêtre (Tauri 2, WebView2), l'interface complète (Svelte 5 : tableau de bord, historique avec le mode de chaque dictée, modes et phrases types, dictionnaire et signes dictés, réglages, installation, banc d'essai de la dictée et de la mise au propre, assistant de premier lancement, Aide) et la **mise à jour intégrée** sont essayés sur un vrai PC. S06 ajoute : **touches de dictée et de traduction réglables** (essayées), **LM Studio et Ollama**, **sons** de début et de fin, **menu de l'icône** (mode en un clic), banc d'essai de la mise au propre, préchargement relancé quand on change de modèle, le **logotype**. Mesuré sur un portable sans vraie carte graphique : Whisper `base` local + mise au propre OpenRouter ≈ 3 à 4 s après le relâchement (Scribe sur M1 Max : 0,8 à 2 s) ; les petits modèles locaux (Qwen 0,5 et 1,5 B) reformulent mal les modes de style (essai des 12 phrases types). Voir [docs/ESSAI_WINDOWS.md](docs/ESSAI_WINDOWS.md) pour l'essayer, [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) pour le plan, [ROADMAP.md](ROADMAP.md) pour les étapes et [CHANGELOG.md](CHANGELOG.md) pour l'historique.

## Principes

- **Local d'abord** : transcription par `whisper-server` (whisper.cpp) et mise au propre par `llama-server`, LM Studio ou Ollama ; le cloud (Deepgram, OpenRouter) reste un choix, jamais une obligation.
- **Le comportement est mesuré** : la logique de texte est vérifiée par des cas de référence (`fixtures/`) tirés de Scribe.
- **Ouvert** : licence MIT, développement en français, interface en français et en anglais (README anglais à venir).

## Licence

[MIT](LICENSE)
