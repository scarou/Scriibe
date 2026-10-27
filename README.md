# Scriibe

Dictée vocale pour Windows, macOS et Linux. Maintenir une touche, parler, relâcher : le texte est transcrit, mis au propre et collé dans l'appli ouverte.

Scriibe est la suite multiplateforme de [Scribe](https://github.com/scarou/scribe) (macOS, Swift), qui reste la **version de référence** du comportement. Le but est la même expérience sur les trois systèmes, avec un code unique (Rust + Tauri).

> **État (S04) : utilisable sous Windows, interface complète écrite ; la fenêtre reste à essayer sur le PC.** Le cœur, les moteurs, la couche système Windows (touche Ctrl droit, micro, collage, icône, bips, pause des médias), le chef d'orchestre et la **mise à jour intégrée** sont essayés sur un vrai PC (S03). S04 ajoute la **fenêtre** (Tauri 2, WebView2) et l'**interface** (Svelte 5) : tableau de bord, historique, modes, dictionnaire, réglages, installation des moteurs (téléchargements qui reprennent), banc d'essai comparable, test de frappe, assistant de premier lancement par niveau de machine, aide, et une pastille d'écoute à l'écran (bêta). Mesuré sur un portable sans vraie carte graphique : Whisper `base` local + mise au propre OpenRouter ≈ 3,2 à 3,9 s après le relâchement (Scribe sur M1 Max : 0,8 à 2 s) ; tout en local ≈ 4 à 6 s. Pas de macOS ni de Linux pour l'instant. Voir [docs/ESSAI_WINDOWS.md](docs/ESSAI_WINDOWS.md) pour l'essayer, [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) pour le plan, [ROADMAP.md](ROADMAP.md) pour les étapes et [CHANGELOG.md](CHANGELOG.md) pour l'historique.

## Principes

- **Local d'abord** : transcription par `whisper-server` (whisper.cpp) et mise au propre par `llama-server`, LM Studio ou Ollama ; le cloud (Deepgram, OpenRouter) reste un choix, jamais une obligation.
- **Le comportement est mesuré** : la logique de texte est vérifiée par des cas de référence (`fixtures/`) tirés de Scribe.
- **Ouvert** : licence MIT, développement en français, anglais à venir.

## Licence

[MIT](LICENSE)
