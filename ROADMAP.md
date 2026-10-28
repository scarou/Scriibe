# Feuille de route

Légende : ✔ fait · ▶ prochaine étape · ○ à venir.

1. ✔ **Fondations** : dépôt, licence, architecture, CLAUDE.md. *(S01)*
2. ✔ **Cas de référence** : 399 cas tirés de `Checks.swift` dans `fixtures/` (365 en S01, 34 contrôles du banc en S02). Restent le déroulement du banc, le test de frappe et les messages complets des prompts (`fixtures/A_FAIRE.md`). *(S01, S02)*
3. ✔ **Cœur Rust** (`crates/core`) : 337 cas rejoués (303 en S01 + 34 du banc) ; avec les 39 des moteurs, 376 des 399 passent ; 14 cas d'interface (rejoués depuis S04) ; les 9 du mode Amusant sont abandonnés avec lui. Prompts identiques à Scribe 1.12.0 (empreinte). *(S01, S02)*
4. ✔ **Moteurs** (`crates/engines`) : `whisper-server` et `llama-server`, clients Deepgram, OpenRouter, LM Studio, Ollama. Testés avec un monde simulé ; les vrais serveurs restent à essayer sur une machine. *(S01)*
5. ✔ **MVP Windows** — essayé de bout en bout sur un vrai PC (S03) : touche, collage, icône, bips, annulation, traduction, pause des médias, cloud en dépannage, mise à jour intégrée. Restes : voir « Étape 5 en détail ». *(S02, S03)*
6. ✔ **Interface complète** (parité avec Scribe) écrite en S04 : fenêtre Tauri 2, interface Svelte 5, Réglages, Dictionnaire, Modes, Historique, Tableau de bord, Aide, banc comparable, test de frappe, assistant de premier lancement, pastille (bêta) ; cas `ui_logic.json` rejoués. Essayée sur le PC (S04, S05) : la fenêtre (fermeture et réouverture), l'assistant, la pastille (trois styles, traduction, active par défaut), les boîtes de confirmation, la mise à jour, le banc (trois passages). S05 : touches réglables, Aide complète, logo, pause des médias visée, cartes « Dernières dictées » et « Temps des dictées ». **Reste** : essayer le lot de touches sur le PC, la parité avec Scribe (voir « Parité avec Scribe »). *(S04, S05)*
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

**Fait et essayé sur le PC (S04, S05)** : fenêtre Tauri 2 + interface Svelte 5 embarquée ; Tableau de bord (dont « Dernières dictées »), Historique, Modes (sept livrés, personnels), Dictionnaire, Réglages, Installation (reprise des téléchargements), Banc d'essai (passages gardés, comparaison, « Temps des dictées »), Mise à jour, Journal et rapport ; pastille d'écoute (active par défaut) ; fermeture et réouverture de la fenêtre ; boîtes de confirmation ; mise à jour par la fenêtre ; logo (symbole et icône de la zone de notification) ; pause des médias avec VLC et Chrome.

**Écrit, vérifié par les tests et des captures, PAS ENCORE ESSAYÉ sur le PC (S05)** : touches de dictée et de traduction réglables par capture directe (touche Windows, Maj gauche, « < > », combinaisons ; Alt et Windows sans menu ; attente de 0,3 s pour les touches de modification seules) ; Aide complète ; cartes « Dernières dictées » et « Temps des dictées ».

**Reste à essayer sur le PC** : la pause des médias pendant un appel (Meet, Teams) ; la reprise d'un téléchargement coupé ; la capture de **Fn** (probablement invisible pour Windows) ; le banc avec un **micro correct** (USB ou casque : comparer au passage fait avec le micro du portable) ; surveiller le collage perdu (la ligne « collage: » du journal).

**Reste à faire (code)** : voir « Parité avec Scribe » ; langue de l'interface au-delà du français ; comparaison « avec / sans modèle » du banc (écart noté) ; « réglages du banc selon les modes personnels » ; zone de capture des essais de frappe plus stricte si besoin ; commandes Tauri (IPC) si une fonction l'exige.

## Parité avec Scribe : ce qui manque (inventaire de S05, d'après la lecture des vues Swift)

1. **LM Studio et Ollama** dans l'interface (le code des moteurs existe : liste des modèles `local_models`, adresses `lmStudioURL` et `ollamaURL` ; aucune route web ni menu).
2. **Sons de début et de fin** : choix (dont « Aucun ») et volume (aujourd'hui deux bips Windows fixes ; réglages `startSoundName`, `endSoundName`, `soundVolume` ignorés).
3. **Menu de l'icône** : rappeler les touches et changer de mode en un clic (aujourd'hui : Ouvrir Scriibe, dossier des données, journal, Quitter).
4. **Phrases types** (`CleanTest.swift`, 12 phrases, fixtures 247–252) et « Essayer sur les phrases types » dans un mode ; **banc d'essai de la mise au propre** (`TrialBench.swift` : la même phrase donnée à plusieurs modèles).
5. **Carte « Signes dictés »** : choix du caractère des listes (tiret, pastille, étoile : réglage `listBullet`).
6. **Moteurs** : modèle Deepgram (`transcriptionModel`), choix d'un fichier modèle Whisper (« Choisir… »), libération de Whisper au repos (`whisperIdleMinutes`, 10 min fixes dans l'interface).
7. **Logotype** (le mot « Scriibe » en tracés, comme celui de Scribe) ; icônes macOS (`.icns`) et Linux (étape 7).
8. Amélioration notée : relancer le **préchargement** quand on change de modèle ou de moteur (aujourd'hui seulement au lancement) ; titre des boîtes de confirmation (« 127.0.0.1:… indique », boîte standard du navigateur).
9. Écartés : autorisations macOS (sans objet), mode Amusant (abandonné), mode par application (idée de Scribe, après la parité).

## Plus tard, sans date

- Mode par application, transcription en continu pour l'indicateur « défilant » (idées du carnet de Scribe, à reprendre une fois la parité atteinte).
- Windows : signature du binaire (Authenticode) et SmartScreen ; installateur.
- Linux : Wayland (portails, `libei`, repli « copie seule »).
- Moteur **Kyutai STT** (`stt-1b-en_fr`, en continu, 0,5 s de délai) comme option pour Mac (MLX) et PC NVIDIA ; non viable sur processeur d'après la documentation lue (S02, taux d'erreur en français non vérifié : relire la fiche du modèle quand les pages seront accessibles). Serveur WebSocket à part.
- Début de prise : garder le micro ouvert un court instant entre deux dictées si un pilote met du temps à s'adapter (à envisager seulement si le problème revient après réglage de Windows).
- Windows : démarrage avec la session, installateur, signature (déjà noté), vérification SmartScreen.
