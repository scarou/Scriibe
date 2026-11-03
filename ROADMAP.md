# Feuille de route

Légende : ✔ fait · ▶ prochaine étape · ○ à venir.

1. ✔ **Fondations** : dépôt, licence, architecture, CLAUDE.md. *(S01)*
2. ✔ **Cas de référence** : 399 cas tirés de `Checks.swift` dans `fixtures/` (365 en S01, 34 contrôles du banc en S02). Restent le déroulement du banc, le test de frappe et les messages complets des prompts (`fixtures/A_FAIRE.md`). *(S01, S02)*
3. ✔ **Cœur Rust** (`crates/core`) : 337 cas rejoués (303 en S01 + 34 du banc) ; avec les 39 des moteurs, 376 des 399 passent ; 14 cas d'interface (rejoués depuis S04) ; les 9 du mode Amusant sont abandonnés avec lui. Prompts identiques à Scribe 1.12.0 (empreinte). *(S01, S02)*
4. ✔ **Moteurs** (`crates/engines`) : `whisper-server` et `llama-server`, clients Deepgram, OpenRouter, LM Studio, Ollama. Testés avec un monde simulé ; les vrais serveurs restent à essayer sur une machine. *(S01)*
5. ✔ **MVP Windows** — essayé de bout en bout sur un vrai PC (S03) : touche, collage, icône, bips, annulation, traduction, pause des médias, cloud en dépannage, mise à jour intégrée. Restes : voir « Étape 5 en détail ». *(S02, S03)*
6. ✔ **Interface complète** (parité avec Scribe, presque terminée en S06) écrite en S04 : fenêtre Tauri 2, interface Svelte 5, Réglages, Dictionnaire, Modes, Historique, Tableau de bord, Aide, banc comparable, test de frappe, assistant de premier lancement, pastille (bêta) ; cas `ui_logic.json` rejoués. Essayée sur le PC (S04, S05) : la fenêtre (fermeture et réouverture), l'assistant, la pastille (trois styles, traduction, active par défaut), les boîtes de confirmation, la mise à jour, le banc (trois passages). S05 : touches réglables, Aide complète, logo, pause des médias visée, cartes « Dernières dictées » et « Temps des dictées ». S06 : touches essayées sur le PC, LM Studio et Ollama, sons, menu de l'icône, mode dans l'historique, signes dictés, réglages de moteurs, phrases types, banc de la mise au propre, préchargement relancé, logotype. **Reste** : les essais listés plus bas. S07 : comparaison « avec / sans modèle » du banc, boîtes de la page, Installation refondue. S08 : Aide pour novices, assistant relançable, dicter dans la fenêtre de Scriibe, **interface en français et en anglais** (Aide anglaise, menu de l'icône, phrases de test anglaises). *(S04, S05, S06, S07, S08)*
7. ▶ **macOS** : premier jalon de la couche système écrit en S09 sans Mac, essayé en S10 sur le Mac (touche, traduction, annulation, Whisper local : marchent ; pause des médias : marche avec Chrome ; **S11 : Scriibe.app (fenêtre Tauri, icône de la barre des menus, pastille, verrou d'instance) faite et essayée sur le Mac ; reste touches réglables sur Mac, carte graphique, démarrage avec la session, Musique/Spotify/appel, mise à jour intégrée, CI complète**) (`docs/ESSAI_MACOS.md`). **Linux X11** (premier jalon fait en S07 sur écran virtuel : touche globale, collage, fenêtre active, micro, sons, pause des médias ; voir `docs/ESSAI_LINUX.md`), puis **macOS**, puis **Linux Wayland** (bêta). Reste pour Linux : essai sur un vrai bureau, retenue des touches ordinaires (`XGrabKey`), icône (AppIndicator), fenêtre, trousseau, téléchargement des moteurs.
7b. ▷ **Mise à jour sur tous les systèmes** : cadrage dans `docs/MISE_A_JOUR.md`. **Chantier A fait en S12 et essayé sur le Mac** (alerte de nouvelle version, jeton GitHub en lecture seule) ; reste l'entrée du menu de l'icône et l'affichage du titre/de la date sur Mac. Chantiers B (la CI publie `.app` Mac et binaire Linux) et C (installation sur Mac et Linux) : décisions de Christophe en attente. **Trousseau macOS** : trois demandes d'accès à chaque recompilation, non résolu sans compte développeur Apple (voir `CHANGELOG.md`, S12).
8. ○ **Ouverture** : README anglais, CONTRIBUTING, vérification du nom (INPI/EUIPO, recherche web), passage du dépôt en public.
9. ✘ **Mode Amusant** (schtroumpf) : **abandonné** (décision de Christophe, S05).

## Prochaines étapes

1. ▶ **Publier une v0.2.1** (interface refaite à la manière de Scribe, apparence réglable, dictée sur une sélection sur Mac, documentation anglaise) : numéro dans `Cargo.toml`, compiler, commiter, pousser, `tools/publier_macos.sh 0.2.1` (aperçu puis « oui » de Christophe) ; attendre `release-windows.yml`. Puis **essayer sur le PC la mise à jour par la fenêtre** (0.2.0 → 0.2.1, jamais essayée de bout en bout) et l'alerte de nouvelle version sur le Mac (Scriibe.app à recompiler en 0.2.x).
2. Voir sur le PC la nouvelle interface (Réglages à onglets, cartes repliables, Historique groupé par jour).
3. Détection de la sélection sous Windows et Linux (aujourd'hui Mac seulement) ; cartes de groupes de Modes repliables.
4. Chantier C de `docs/MISE_A_JOUR.md` (installation intégrée sur Mac), release Linux ; essais Mac restants (combinaison de touches, Fn, traduction personnalisée).

## Étape 5 en détail

Objectif : dicter sous Windows, de bout en bout (touche → micro → transcription → mise au propre → collage), sur le PC Windows de Christophe (HP, AZERTY, Radeon RX 540 2 Go + Intel UHD 620).

**Fait** : S02 : `crates/platform`, `crates/pipeline`, `crates/cli`, page d'essais, CI qui publie `scriibe.exe`, micro, Whisper, llama.cpp. S03 : touche globale, collage, icône, bips, annulation, traduction (essayés), attente du préchauffage, préchargement au lancement, mise à jour depuis la page, pause des médias, clés et moteurs cloud, llama-server Vulkan et choix de la carte, durée de conservation du modèle ; mesures et décision par niveau de machine (voir `CHANGELOG.md`, S03). S04 : reprise automatique des téléchargements, assistant de premier lancement par niveau de machine (décision confirmée), réglages sortis de `moteurs.json`.

**Reste à faire (sur le PC de Christophe)** :

1. **Pause des médias** : appel (Meet) et VLC essayés en S07 ; reste une sortie autre que l'appareil par défaut ; ignorer d'autres programmes système si besoin (`IGNORED` dans `audio_sessions.rs`).
2. **Précision de Whisper `small` contre `base`** : deux passages du banc (onglet Banc d'essai) puis la comparaison, jugée sur les textes et non sur les temps (`small` prend 9 à 10 s ici) ; **Qwen 0,5 B** (maintenant dans l'onglet Installation) pour une mise au propre locale plus rapide.
3. **Rester chargé** : décider si le modèle local de mise au propre reste chargé tant que Scriibe tourne (le réglage existe : « tant que Scriibe tourne »).

## Étape 6 en détail

**Fait et essayé sur le PC (S04, S05)** : fenêtre Tauri 2 + interface Svelte 5 embarquée ; Tableau de bord (dont « Dernières dictées »), Historique, Modes (sept livrés, personnels), Dictionnaire, Réglages, Installation (reprise des téléchargements), Banc d'essai (passages gardés, comparaison, « Temps des dictées »), Mise à jour, Journal et rapport ; pastille d'écoute (active par défaut) ; fermeture et réouverture de la fenêtre ; boîtes de confirmation ; mise à jour par la fenêtre ; logo (symbole et icône de la zone de notification) ; pause des médias avec VLC et Chrome.

**Essayé en S06** : touches de dictée et de traduction réglables par capture directe (touche Windows, Maj gauche, « < > », Ctrl gauche + F10, Alt gauche ; Alt et Windows sans menu) ; le crochet ne reçoit rien dans la fenêtre de Scriibe : la capture passe par la page ; Aide complète et cartes « Dernières dictées » / « Temps des dictées » (relues par Christophe : pas de retour négatif).

**Reste à essayer sur le PC** : la reprise d'un téléchargement coupé ; le banc avec un **micro correct** (USB ou casque) et la lenteur de Whisper `base` vue en S06 (4 à 5 s) ; LM Studio et Ollama avec un vrai serveur ; la suppression d'un modèle depuis « Modèles installés » (seulement testée automatiquement) ; surveiller le collage perdu (ligne « collage: » du journal) et le raté de pause des médias avec Chrome vu une fois en S07 au soir (non reproduit). **Essayé en S07** : pause pendant un appel, Purr, « Tester ma touche », « Rétablir », Fn (non reconnue : normal), repli sur le processeur, comparaison du banc, nouvelles boîtes, recherche et téléchargement Hugging Face. **Essayé en S06** : touches réglables (voir `CHANGELOG.md`), menu de l'icône, sons, signes dictés, boîte d'ouverture de fichier, phrases types et banc.

**Reste à faire (code)** : voir « Parité avec Scribe » ; messages d'erreur des moteurs à traduire dans l'interface anglaise ; zone de capture des essais de frappe plus stricte si besoin ; commandes Tauri (IPC) si une fonction l'exige.

## Parité avec Scribe : ce qui reste (inventaire de S05, mis à jour en S06)

**Fait en S06** (écarts dans `docs/ECARTS.md`) : LM Studio et Ollama dans l'interface ; sons de début et de fin (choix, « Aucun », volume) ; menu de l'icône (touches, mode en un clic, copier la dernière dictée) ; phrases types et banc d'essai de la mise au propre ; carte « Signes dictés » (`listBullet`) ; modèle Deepgram, fichier modèle Whisper, libération de Whisper ; logotype ; préchargement relancé au changement de modèle, de moteur, de mode ou de dictionnaire.

**Reste** :
0. ✔ Onglet Installation refondu (S07) : modèles installés avec suppression, recherche et choix de fichiers sur Hugging Face (la liste n'est plus figée). Recherche et listes **non essayées avec le vrai Hugging Face** (injoignable depuis le cloud) : à essayer sur le PC.
1. ✔ **Langue de l'interface** : français et anglais (S08, choix de Christophe). Une troisième langue = un fichier de plus dans `ui/src/lib/locales/` (et une Aide).
2. ✔ **Comparaison « avec / sans modèle »** du banc et réglages du banc selon les modes personnels (S07 : le nom et l'empreinte de la consigne d'un mode personnel étaient déjà dans les réglages d'un passage ; test ajouté).
3. ✔ Boîtes de confirmation et d'avertissement dessinées par la page (titre « Scriibe », bouton qui dit ce qu'il fait ; S07 ; à essayer sur le PC).
4. Menu de l'icône, sons, choix de fichier et capture des touches hors de Windows (étape 7). ✔ **Touches réglables sur Mac** (S13, essayées sur le Mac ; combinaisons et Fn non essayées). ✔ Entrée « nouvelle version » du menu de l'icône et titre/date de la dernière version sur Mac (suite de S13 ; entrée non vue à l'écran).
5. Écartés : autorisations macOS (sans objet), mode Amusant (abandonné), mode par application (idée de Scribe, après la parité).

## Plus tard, sans date

- Mode par application, transcription en continu pour l'indicateur « défilant » (idées du carnet de Scribe, à reprendre une fois la parité atteinte).
- Windows : signature du binaire (Authenticode) et SmartScreen ; installateur.
- Linux : Wayland (portails, `libei`, repli « copie seule »).
- Moteur **Kyutai STT** (`stt-1b-en_fr`, en continu, 0,5 s de délai) comme option pour Mac (MLX) et PC NVIDIA ; non viable sur processeur d'après la documentation lue (S02, taux d'erreur en français non vérifié : relire la fiche du modèle quand les pages seront accessibles). Serveur WebSocket à part.
- Début de prise : garder le micro ouvert un court instant entre deux dictées si un pilote met du temps à s'adapter (à envisager seulement si le problème revient après réglage de Windows).
- Windows : démarrage avec la session, installateur, signature (déjà noté), vérification SmartScreen.
