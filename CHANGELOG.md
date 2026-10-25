# Journal

Une section par session de travail, la plus récente en haut. Scriibe n'a pas encore de version publiée.

## S01 — 2026-10-25 · Fondations, cas de référence, cœur Rust et moteurs

Session cloud (Linux). Le dépôt `scarou/scribe` (Swift) n'a pas été modifié : il reste la référence.

- **Cadrage** : tester la faisabilité d'un portage Windows de Scribe a conduit à une réécriture en **Rust + Tauri 2**, dans un dépôt séparé (`scarou/scriibe`, privé, licence MIT, nom Scriibe). Scribe reste inchangé et sert d'oracle. Analyse critique du conseil reçu d'un autre assistant dans `docs/ARCHITECTURE.md` (pas de streaming ; la mise au propre par un modèle est le cœur ; les vraies difficultés sont la touche globale, le collage, Wayland).
- **Étape 1 — fondations** : `LICENSE` (MIT), `README.md`, `CLAUDE.md`, `ROADMAP.md`, `docs/ARCHITECTURE.md`.
- **Étape 2 — cas de référence** : 365 cas tirés des vérifications de `Checks.swift` (16 fichiers JSON dans `fixtures/`), plus `fixtures/README.md` (format), `fixtures/A_FAIRE.md` (ce qui reste) et deux outils : `tools/verifier_fixtures.py` (structure) et `tools/verifier_litteraux.py` (chaque texte existe tel quel dans `Checks.swift`, près de la ligne citée).
- **Étape 3 — cœur Rust** (`crates/core`) : typographie, signes dictés, listes, « à la ligne », dictionnaire, texte déjà propre, filtre des phrases inventées par Whisper, garde-fous de la mise au propre et de la traduction, 8 modes et modes personnels, réglages (lecture tolérante, même format que Scribe), prompts, touche de dictée (machine à états), pause des médias, micro, historique et conservation. Les prompts sont extraits du Swift par script ; leur empreinte vaut `4ed86f0c1c154918`, comme Scribe 1.12.0.
- **Étape 4 — moteurs** (`crates/engines`) : cycle de vie de `whisper-server` et `llama-server` (démarrage partagé, attente de « prêt », libération au repos, reprise sans assistant MTP), clients Deepgram, OpenRouter, LM Studio, Ollama, llama.cpp, transcription Whisper. HTTP, processus, horloge, journal et clés derrière des traits ; monde simulé `FakeWorld` pour les tests.
- **Intégration continue** : GitHub Actions sur Linux, Windows et macOS (`.github/workflows/ci.yml`), avec le détail des échecs publié en annotation.
- **Résultat** : 342 des 365 cas de référence rejoués par `cargo test` (303 dans le cœur, 39 dans les moteurs) ; 14 cas d'interface et 9 du mode Amusant attendent. 72 fonctions de test au total. CI verte sur les trois systèmes au commit `c603001`.
- **Corrections en route** : un test de chemins mélangeait `/` et `\` sous Windows (le code était juste) ; deux messages de commit annonçaient des chiffres non mesurés (183 et 295 cas) — les vrais étaient 146 et 234, corrigés dans le message suivant.
- **Rien d'essayé sur une vraie machine** : la touche globale, le micro, le collage, l'icône et les vrais `whisper-server` / `llama-server` ne sont pas testés (conteneur Linux sans écran ni son).
