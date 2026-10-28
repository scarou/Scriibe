# Feuille de route

Légende : ✔ fait · ▶ prochaine étape · ○ à venir.

1. ✔ **Fondations** : dépôt, licence, architecture, CLAUDE.md. *(S01)*
2. ✔ **Cas de référence** : 399 cas tirés de `Checks.swift` dans `fixtures/` (365 en S01, 34 contrôles du banc en S02). Restent le déroulement du banc, le test de frappe et les messages complets des prompts (`fixtures/A_FAIRE.md`). *(S01, S02)*
3. ✔ **Cœur Rust** (`crates/core`) : 337 cas rejoués (303 en S01 + 34 du banc) ; avec les 39 des moteurs, 376 des 399 passent ; 14 cas d'interface (rejoués depuis S04) ; les 9 du mode Amusant sont abandonnés avec lui. Prompts identiques à Scribe 1.12.0 (empreinte). *(S01, S02)*
4. ✔ **Moteurs** (`crates/engines`) : `whisper-server` et `llama-server`, clients Deepgram, OpenRouter, LM Studio, Ollama. Testés avec un monde simulé ; les vrais serveurs restent à essayer sur une machine. *(S01)*
5. ✔ **MVP Windows** — essayé de bout en bout sur un vrai PC (S03) : touche, collage, icône, bips, annulation, traduction, pause des médias, cloud en dépannage, mise à jour intégrée. Restes : voir « Étape 5 en détail ». *(S02, S03)*
6. ✔ **Interface complète** (parité avec Scribe) écrite en S04 : fenêtre Tauri 2, interface Svelte 5, Réglages, Dictionnaire, Modes, Historique, Tableau de bord, Aide, banc comparable, test de frappe, assistant de premier lancement, pastille (bêta) ; cas `ui_logic.json` rejoués. Essayés sur le PC en fin de S04 : la fenêtre, l'assistant, une dictée avec la pastille. **Reste à essayer** : la pastille sur plusieurs dictées et dans ses trois styles, la reprise d'un téléchargement (voir « Étape 6 en détail »). *(S04)*
7. ○ **macOS**, puis **Linux X11**, puis **Linux Wayland** (bêta).
8. ○ **Ouverture** : README anglais, CONTRIBUTING, vérification du nom (INPI/EUIPO, recherche web), passage du dépôt en public.
9. ✘ **Mode Amusant** (schtroumpf) : **abandonné** (décision de Christophe, S05).

## Étape 5 en détail

Objectif : dicter sous Windows, de bout en bout (touche → micro → transcription → mise au propre → collage), sur le PC Windows de Christophe (HP, AZERTY, Radeon RX 540 2 Go + Intel UHD 620).

**Fait** : S02 : `crates/platform`, `crates/pipeline`, `crates/cli`, page d'essais, CI qui publie `scriibe.exe`, micro, Whisper, llama.cpp. S03 : touche globale, collage, icône, bips, annulation, traduction (essayés), attente du préchauffage, préchargement au lancement, mise à jour depuis la page, pause des médias, clés et moteurs cloud, llama-server Vulkan et choix de la carte, durée de conservation du modèle ; mesures et décision par niveau de machine (voir `CHANGELOG.md`, S03). S04 : reprise automatique des téléchargements, assistant de premier lancement par niveau de machine (décision confirmée), réglages sortis de `moteurs.json`.

**Reste à faire (sur le PC de Christophe)** :

1. **Pause des médias** : essayer un appel (Teams, Meet), un lecteur local (VLC), une sortie autre que l'appareil par défaut ; ignorer d'autres programmes système si besoin (`IGNORED` dans `audio_sessions.rs`).
2. **Précision de Whisper `small` contre `base`** : deux passages du banc (onglet Banc d'essai) puis la comparaison, jugée sur les textes et non sur les temps (`small` prend 9 à 10 s ici) ; **Qwen 0,5 B** (maintenant dans l'onglet Installation) pour une mise au propre locale plus rapide.
3. **Rester chargé** : décider si le modèle local de mise au propre reste chargé tant que Scriibe tourne (le réglage existe : « tant que Scriibe tourne »).

## Étape 6 en détail

**Fait (S04, vérifié par les tests, la CI et des captures depuis le cloud)** : fenêtre Tauri 2 + interface Svelte 5 embarquée ; Tableau de bord (chiffres, test de frappe, assistant), Historique, Modes (livrés et personnels), Dictionnaire, Réglages, Installation (reprise des téléchargements), Banc d'essai (passages gardés, comparaison, différences mot à mot), Mise à jour, Journal et rapport, Aide (8 sections) ; pastille d'écoute à l'écran (bêta, coupée par défaut) ; cas `ui_logic.json` rejoués.

**Essayé sur le PC (fin de S04)** : la fenêtre s'ouvre (aussi après une mise à jour), l'assistant applique une chaîne, les cartes graphiques sont vues, la pastille ne vole pas le focus (une dictée). **Reste à essayer** : fermeture puis réouverture de la fenêtre par l'icône, repli navigateur, boîtes de dialogue, copie du rapport, la pastille sur plusieurs dictées (trois styles, position, traduction) puis la rendre active par défaut si elle est bonne, une reprise de téléchargement en vrai.

**Reste à faire (code)** : logo définitif (l'icône est provisoire) ; comparaison « avec / sans modèle » du banc (écart noté) ; « réglages du banc selon les modes personnels » ; zone de capture des essais de frappe plus stricte si besoin ; langue de l'interface au-delà du français ; commandes Tauri (IPC) si une fonction l'exige (choix de fichier).

## Plus tard, sans date

- Mode par application, transcription en continu pour l'indicateur « défilant » (idées du carnet de Scribe, à reprendre une fois la parité atteinte).
- Windows : signature du binaire (Authenticode) et SmartScreen ; installateur.
- Linux : Wayland (portails, `libei`, repli « copie seule »).
- Moteur **Kyutai STT** (`stt-1b-en_fr`, en continu, 0,5 s de délai) comme option pour Mac (MLX) et PC NVIDIA ; non viable sur processeur d'après la documentation lue (S02, taux d'erreur en français non vérifié : relire la fiche du modèle quand les pages seront accessibles). Serveur WebSocket à part.
- Début de prise : garder le micro ouvert un court instant entre deux dictées si un pilote met du temps à s'adapter (à envisager seulement si le problème revient après réglage de Windows).
- Windows : démarrage avec la session, installateur, signature (déjà noté), vérification SmartScreen.
