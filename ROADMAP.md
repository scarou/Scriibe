# Feuille de route

Légende : ✔ fait · ▶ prochaine étape · ○ à venir.

1. ✔ **Fondations** : dépôt, licence, architecture, CLAUDE.md. *(S01)*
2. ✔ **Cas de référence** : 399 cas tirés de `Checks.swift` dans `fixtures/` (365 en S01, 34 contrôles du banc en S02). Restent le déroulement du banc, le test de frappe et les messages complets des prompts (`fixtures/A_FAIRE.md`). *(S01, S02)*
3. ✔ **Cœur Rust** (`crates/core`) : 337 cas rejoués (303 en S01 + 34 du banc) ; avec les 39 des moteurs, 376 des 399 passent ; 14 cas d'interface et 9 du mode Amusant attendent. Prompts identiques à Scribe 1.12.0 (empreinte). *(S01, S02)*
4. ✔ **Moteurs** (`crates/engines`) : `whisper-server` et `llama-server`, clients Deepgram, OpenRouter, LM Studio, Ollama. Testés avec un monde simulé ; les vrais serveurs restent à essayer sur une machine. *(S01)*
5. ✔ **MVP Windows** — essayé de bout en bout sur un vrai PC (S03) : touche, collage, icône, bips, annulation, traduction, pause des médias, cloud en dépannage, mise à jour intégrée. Restes : voir « Étape 5 en détail ». *(S02, S03)*
6. ▶ **Interface complète** (parité avec Scribe) : Réglages, Dictionnaire, Modes, Historique, Tableau de bord, Aide, bancs d'essai, test de frappe ; rejouer `fixtures/ui_logic.json` côté interface.
7. ○ **macOS**, puis **Linux X11**, puis **Linux Wayland** (bêta).
8. ○ **Ouverture** : README anglais, CONTRIBUTING, vérification du nom (INPI/EUIPO, recherche web), passage du dépôt en public.
9. ○ **Mode Amusant** (schtroumpf) : dépend de l'analyseur grammatical d'Apple ; trouver un équivalent portable ou l'abandonner.

## Étape 5 en détail

Objectif : dicter sous Windows, de bout en bout (touche → micro → transcription → mise au propre → collage), sur le PC Windows de Christophe (HP, AZERTY, Radeon RX 540 2 Go + Intel UHD 620).

**Fait** : S02 : `crates/platform`, `crates/pipeline`, `crates/cli`, page d'essais, CI qui publie `scriibe.exe`, micro, Whisper, llama.cpp. S03 : touche globale, collage, icône, bips, annulation, traduction (essayés), attente du préchauffage, préchargement au lancement, mise à jour depuis la page, pause des médias, clés et moteurs cloud, llama-server Vulkan et choix de la carte, durée de conservation du modèle ; mesures et décision provisoire par niveau de machine (voir `CHANGELOG.md`, S03).

**Reste à faire (petit, peut se faire avec l'étape 6)** :

1. **Reprise automatique d'un téléchargement interrompu** (en-tête `Range`) et nouvel essai : un modèle de 465 Mo a échoué une fois à 6 Mo.
2. **Assistant de premier lancement par niveau de machine** : mesurer la machine (cœurs, mémoire, cartes) et proposer la chaîne (local, local + mise au propre cloud, tout cloud) ; décision provisoire dans le journal S03, à confirmer avec Christophe.
3. **Pause des médias** : essayer un appel (Teams, Meet), un lecteur local (VLC), une sortie autre que l'appareil par défaut ; ignorer d'autres programmes système si besoin (`IGNORED` dans `audio_sessions.rs`).
4. **Précision de Whisper `small` contre `base`** (jugée sur les textes, pas sur les temps : `small` prend 9 à 10 s ici) et Qwen 0,5 B pour une mise au propre locale plus rapide.
5. **Rester chargé** : décider si le modèle local de mise au propre reste chargé tant que Scriibe tourne (réglage actuel : 30 min par défaut, comme Scribe).
6. Ensuite : la **fenêtre Tauri** avec les Réglages de base (étape 6, qui reprend la page d'essais et `moteurs.json`).

## Plus tard, sans date

- Mode par application, transcription en continu pour l'indicateur « défilant » (idées du carnet de Scribe, à reprendre une fois la parité atteinte).
- Windows : signature du binaire (Authenticode) et SmartScreen ; installateur.
- Linux : Wayland (portails, `libei`, repli « copie seule »).
- Moteur **Kyutai STT** (`stt-1b-en_fr`, en continu, 0,5 s de délai) comme option pour Mac (MLX) et PC NVIDIA ; non viable sur processeur d'après la documentation lue (S02, taux d'erreur en français non vérifié : relire la fiche du modèle quand les pages seront accessibles). Serveur WebSocket à part.
- Début de prise : garder le micro ouvert un court instant entre deux dictées si un pilote met du temps à s'adapter (à envisager seulement si le problème revient après réglage de Windows).
- Windows : démarrage avec la session, installateur, signature (déjà noté), vérification SmartScreen.
