# Feuille de route

1. **Fondations** : dépôt, licence, architecture, CLAUDE.md. *(en cours)*
2. **Cas de référence** : extraire de `Checks.swift` (Scribe) les cas entrée → sortie en JSON dans `fixtures/`.
3. **Cœur Rust** (`crates/core`) : signes dictés, listes, dictionnaire, typographie, garde-fous, modes, prompts, historique ; tous les cas de `fixtures/` passent.
4. **Moteurs** (`crates/engines`) : lancement de `whisper-server` et `llama-server`, clients Deepgram, OpenRouter, LM Studio, Ollama.
5. **MVP Windows** : touche globale, micro, transcription, mise au propre, collage, icône de zone de notification.
6. **Interface complète** (parité avec Scribe) : Réglages, Dictionnaire, Modes, Historique, Tableau de bord, Aide, bancs d'essai.
7. **macOS**, puis **Linux X11**, puis **Linux Wayland** (bêta).
8. **Ouverture** : README anglais, CONTRIBUTING, CI sur les trois systèmes, passage du dépôt en public.
