# Feuille de route

Légende : ✔ fait · ▶ prochaine étape · ○ à venir.

1. ✔ **Fondations** : dépôt, licence, architecture, CLAUDE.md. *(S01)*
2. ✔ **Cas de référence** : 365 cas tirés de `Checks.swift` dans `fixtures/`. Restent le banc, le test de frappe et les messages complets des prompts (`fixtures/A_FAIRE.md`). *(S01)*
3. ✔ **Cœur Rust** (`crates/core`) : 342 des 365 cas passent ; 14 cas d'interface et 9 du mode Amusant attendent. Prompts identiques à Scribe 1.12.0 (empreinte). *(S01)*
4. ✔ **Moteurs** (`crates/engines`) : `whisper-server` et `llama-server`, clients Deepgram, OpenRouter, LM Studio, Ollama. Testés avec un monde simulé ; les vrais serveurs restent à essayer sur une machine. *(S01)*
5. ▶ **MVP Windows** — voir « Étape 5 en détail » ci-dessous.
6. ○ **Interface complète** (parité avec Scribe) : Réglages, Dictionnaire, Modes, Historique, Tableau de bord, Aide, bancs d'essai, test de frappe ; rejouer `fixtures/ui_logic.json` côté interface.
7. ○ **macOS**, puis **Linux X11**, puis **Linux Wayland** (bêta).
8. ○ **Ouverture** : README anglais, CONTRIBUTING, vérification du nom (INPI/EUIPO, recherche web), passage du dépôt en public.
9. ○ **Mode Amusant** (schtroumpf) : dépend de l'analyseur grammatical d'Apple ; trouver un équivalent portable ou l'abandonner.

## Étape 5 en détail

Objectif : dicter sous Windows, de bout en bout (touche → micro → Whisper → mise au propre → collage), sur le PC Windows de Christophe.

Décisions à prendre avec Christophe **avant de coder** :

1. **Touche de dictée par défaut sous Windows.** Option droite n'existe pas ; sur un clavier AZERTY, la touche de droite équivalente est AltGr (Windows l'émet comme Ctrl+Alt). Proposition : **Ctrl droit**, avec **Maj** en plus pour traduire. À confirmer, ainsi que le clavier (AZERTY/QWERTY).
2. **Forme du MVP.** Proposition : d'abord un exécutable en ligne de commande avec icône dans la zone de notification (tester la chaîne sans interface), puis la fenêtre Tauri avec les Réglages de base.
3. **Carte graphique** du PC (NVIDIA, AMD, aucune) : décide du binaire `llama-server` / `whisper-server` à installer (CUDA, Vulkan ou CPU) et des modèles proposés.

Travail prévu :

- `crates/platform` : un trait par fonction (touche globale, collage + presse-papiers remis, enregistrement du micro, pause des médias, clés d'API, icône/indicateur, dossier de données), une implémentation Windows (crochet clavier bas niveau, `SendInput`, `cpal`, session audio WASAPI, Gestionnaire d'identifiants, `tray-icon`), des implémentations vides ou simulées pour que le projet compile partout.
- `crates/pipeline` (nom provisoire) : le chef d'orchestre, port de `DictationController.swift` — appui → préchauffage → enregistrement → relâchement → transcription → corrections → mise au propre (ou repli sur le texte brut) → collage → historique ; annulation par une autre touche ; traduction avec la touche en plus ; testé avec des faux de plateforme et `FakeWorld`.
- Essai sur le PC Windows : installer `whisper-server` et `llama-server` (téléchargement seulement avec l'accord de Christophe, taille et source annoncées), un modèle, puis une vraie dictée ; relever les temps et les comparer à Scribe (voir `README.md` de Scribe, « Temps mesurés »).

## Plus tard, sans date

- Mode par application, transcription en continu pour l'indicateur « défilant » (idées du carnet de Scribe, à reprendre une fois la parité atteinte).
- Windows : signature du binaire (Authenticode) et SmartScreen ; installateur.
- Linux : Wayland (portails, `libei`, repli « copie seule »).
