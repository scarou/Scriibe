# Feuille de route

Légende : ✔ fait · ▶ prochaine étape · ○ à venir.

1. ✔ **Fondations** : dépôt, licence, architecture, CLAUDE.md. *(S01)*
2. ✔ **Cas de référence** : 399 cas tirés de `Checks.swift` dans `fixtures/` (365 en S01, 34 contrôles du banc en S02). Restent le déroulement du banc, le test de frappe et les messages complets des prompts (`fixtures/A_FAIRE.md`). *(S01, S02)*
3. ✔ **Cœur Rust** (`crates/core`) : 337 cas rejoués (303 en S01 + 34 du banc) ; avec les 39 des moteurs, 376 des 399 passent ; 14 cas d'interface et 9 du mode Amusant attendent. Prompts identiques à Scribe 1.12.0 (empreinte). *(S01, S02)*
4. ✔ **Moteurs** (`crates/engines`) : `whisper-server` et `llama-server`, clients Deepgram, OpenRouter, LM Studio, Ollama. Testés avec un monde simulé ; les vrais serveurs restent à essayer sur une machine. *(S01)*
5. ▶ **MVP Windows** — code écrit et essayé en partie sur un vrai PC (S02) ; il reste la touche globale réelle, le collage et l'icône. Voir « Étape 5 en détail ».
6. ○ **Interface complète** (parité avec Scribe) : Réglages, Dictionnaire, Modes, Historique, Tableau de bord, Aide, bancs d'essai, test de frappe ; rejouer `fixtures/ui_logic.json` côté interface.
7. ○ **macOS**, puis **Linux X11**, puis **Linux Wayland** (bêta).
8. ○ **Ouverture** : README anglais, CONTRIBUTING, vérification du nom (INPI/EUIPO, recherche web), passage du dépôt en public.
9. ○ **Mode Amusant** (schtroumpf) : dépend de l'analyseur grammatical d'Apple ; trouver un équivalent portable ou l'abandonner.

## Étape 5 en détail

Objectif : dicter sous Windows, de bout en bout (touche → micro → Whisper → mise au propre → collage), sur le PC Windows de Christophe (HP, AZERTY, Radeon RX 540 2 Go : moteurs sur processeur).

**Fait en S02** : `crates/platform` (traits, faux, stockage, implémentation Windows vérifiée à la compilation), `crates/pipeline` (port de `DictationController`, 35 tests), `crates/cli` (`scriibe` : page d'essais web, `verifier`, `micro`, `essai`, `cle`), CI qui publie `scriibe.exe`. **Essayé sur le PC** : micro (pilote Conexant : « bang » à l'ouverture, niveau faible d'un casque → corrigés), transcription Whisper `base` (2 à 4 s), llama.cpp avec Qwen 2,5 1,5 B (3,7 à 4,2 s, qualité insuffisante sur CPU), téléchargements confirmés depuis la page.

**Reste à faire pour clore l'étape** (dans cet ordre) :

1. **Touche globale et collage réels** (jamais essayés) : Ctrl droit dans le Bloc-notes, Maj (traduction), annulation par une autre touche, icône (gris/rouge/orange/vert/bleu), bips, collage avec presse-papiers remis. Corriger ce qui ne marche pas (fenêtre en administrateur : collage bloqué).
2. **Mesurer les fils sur les cœurs physiques** (Whisper et llama-server) avec la dernière version, et la mise au propre comparée : Whisper `small` (plus précis, plus lent ?), version « vulkan » de `llama-server` sur la Radeon (2 Go de mémoire vidéo), modèle de mise au propre plus petit (Qwen 0,5 B) ou **cloud** (OpenRouter : clé dans le Gestionnaire d'identifiants avec `scriibe cle`).
3. **Pause des médias** (sessions audio WASAPI) : pas encore portée ; réglage `pauseMedia` sans effet.
4. **Comparer** les temps relevés au tableau « Temps mesurés » du README de Scribe (M1 Max : 0,3 à 0,6 s) et décider de ce qui est acceptable sur un PC sans carte graphique (modèles proposés par niveau de machine).
5. Ensuite seulement : la fenêtre Tauri avec les Réglages de base (étape 6, qui reprendra la page d'essais).

## Plus tard, sans date

- Mode par application, transcription en continu pour l'indicateur « défilant » (idées du carnet de Scribe, à reprendre une fois la parité atteinte).
- Windows : signature du binaire (Authenticode) et SmartScreen ; installateur.
- Linux : Wayland (portails, `libei`, repli « copie seule »).
- Moteur **Kyutai STT** (`stt-1b-en_fr`, en continu, 0,5 s de délai) comme option pour Mac (MLX) et PC NVIDIA ; non viable sur processeur d'après la documentation lue (S02, taux d'erreur en français non vérifié : relire la fiche du modèle quand les pages seront accessibles). Serveur WebSocket à part.
- Début de prise : garder le micro ouvert un court instant entre deux dictées si un pilote met du temps à s'adapter (à envisager seulement si le problème revient après réglage de Windows).
- Windows : démarrage avec la session, installateur, signature (déjà noté), vérification SmartScreen.
