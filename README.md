# Scriibe

Dictée vocale pour Windows, macOS et Linux. Maintenir une touche, parler, relâcher : le texte est transcrit, mis au propre et collé dans l'appli ouverte.

Scriibe est la suite multiplateforme de [Scribe](https://github.com/scarou/scribe) (macOS, Swift), qui reste la **version de référence** du comportement. Le but est la même expérience sur les trois systèmes, avec un code unique (Rust + Tauri).

> **État (S02) : première version utilisable sous Windows, en cours d'essai.** Le cœur, les moteurs, la couche système Windows (touche globale, micro, collage, icône), le chef d'orchestre et une **page d'essais** web sont écrits ; la chaîne a été essayée sur un vrai PC (micro, Whisper, llama.cpp, téléchargements) mais **la touche globale et le collage dans une vraie appli ne sont pas encore vérifiés**. Pas de fenêtre de réglages complète ni de macOS/Linux pour l'instant. Voir [docs/ESSAI_WINDOWS.md](docs/ESSAI_WINDOWS.md) pour l'essayer, [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) pour le plan, [ROADMAP.md](ROADMAP.md) pour les étapes et [CHANGELOG.md](CHANGELOG.md) pour l'historique.

## Principes

- **Local d'abord** : transcription par `whisper-server` (whisper.cpp) et mise au propre par `llama-server`, LM Studio ou Ollama ; le cloud (Deepgram, OpenRouter) reste un choix, jamais une obligation.
- **Le comportement est mesuré** : la logique de texte est vérifiée par des cas de référence (`fixtures/`) tirés de Scribe.
- **Ouvert** : licence MIT, développement en français, anglais à venir.

## Licence

[MIT](LICENSE)
