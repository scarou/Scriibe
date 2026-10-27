# Scriibe

Dictée vocale pour Windows, macOS et Linux. Maintenir une touche, parler, relâcher : le texte est transcrit, mis au propre et collé dans l'appli ouverte.

Scriibe est la suite multiplateforme de [Scribe](https://github.com/scarou/scribe) (macOS, Swift), qui reste la **version de référence** du comportement. Le but est la même expérience sur les trois systèmes, avec un code unique (Rust + Tauri).

> **État (S03) : utilisable sous Windows, essayé de bout en bout sur un vrai PC.** Le cœur, les moteurs, la couche système Windows (touche Ctrl droit, micro, collage, icône, bips, pause des médias), le chef d'orchestre, une **page d'essais** web (installation des moteurs, réglages, clés cloud, banc de dictée, rapport) et la **mise à jour intégrée** sont écrits et essayés. Mesuré sur un portable sans vraie carte graphique : Whisper `base` local + mise au propre OpenRouter ≈ 3,2 à 3,9 s après le relâchement (Scribe sur M1 Max : 0,8 à 2 s) ; tout en local ≈ 4 à 6 s. Pas de fenêtre de réglages complète ni de macOS/Linux pour l'instant. Voir [docs/ESSAI_WINDOWS.md](docs/ESSAI_WINDOWS.md) pour l'essayer, [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) pour le plan, [ROADMAP.md](ROADMAP.md) pour les étapes et [CHANGELOG.md](CHANGELOG.md) pour l'historique.

## Principes

- **Local d'abord** : transcription par `whisper-server` (whisper.cpp) et mise au propre par `llama-server`, LM Studio ou Ollama ; le cloud (Deepgram, OpenRouter) reste un choix, jamais une obligation.
- **Le comportement est mesuré** : la logique de texte est vérifiée par des cas de référence (`fixtures/`) tirés de Scribe.
- **Ouvert** : licence MIT, développement en français, anglais à venir.

## Licence

[MIT](LICENSE)
