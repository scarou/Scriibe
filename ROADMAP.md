# Feuille de route

1. **Fondations** : dépôt, licence, architecture, CLAUDE.md. *(en cours)*
2. **Cas de référence** : extraire de `Checks.swift` (Scribe) les cas entrée → sortie en JSON dans `fixtures/`. *(fait pour le cœur : 365 cas ; reste le banc, le test de frappe et les prompts complets, voir `fixtures/A_FAIRE.md`)*
3. **Cœur Rust** (`crates/core`) : signes dictés, listes, dictionnaire, typographie, garde-fous, modes, prompts, historique ; tous les cas de `fixtures/` passent. *(fait : 342 des 365 cas passent ; restent 14 cas d'interface et 9 du mode Amusant. Les prompts sont identiques à ceux de Scribe 1.12.0, empreinte comprise.)*
4. **Moteurs** (`crates/engines`) : lancement de `whisper-server` et `llama-server`, clients Deepgram, OpenRouter, LM Studio, Ollama. *(fait, testé avec un monde simulé et un vrai mini serveur HTTP ; les vrais `whisper-server` et `llama-server` restent à essayer sur une machine, à l'étape 5.)*
5. **MVP Windows** : touche globale, micro, transcription, mise au propre, collage, icône de zone de notification.
6. **Interface complète** (parité avec Scribe) : Réglages, Dictionnaire, Modes, Historique, Tableau de bord, Aide, bancs d'essai.
7. **macOS**, puis **Linux X11**, puis **Linux Wayland** (bêta).
8. **Ouverture** : README anglais, CONTRIBUTING, CI sur les trois systèmes, passage du dépôt en public.
