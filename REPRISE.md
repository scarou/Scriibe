# Reprise — 25/10/2026 (fin de S01)

Le texte ci-dessous est le prompt à coller au début de la session suivante.

```text
Projet Scriibe : dictée vocale multiplateforme (Windows, macOS, Linux), suite de Scribe (macOS, Swift, dépôt scarou/scribe), réécrite en Rust + Tauri 2. Dépôt scarou/scriibe (privé, branche main, licence MIT). Christophe Campain est le seul développeur ; tu écris tout le code, il décide et valide.

DÉMARRAGE (session cloud Linux, aucun dépôt n'est présent au départ)
1. Attache les deux dépôts avec add_repo, puis clone-les un par un (shallow, timeout généreux, jamais en parallèle) puis register_repo_root : scarou/scriibe en accès « push » (c'est là que tu travailles) et scarou/scribe en lecture seule (la référence Swift : à lire, JAMAIS à modifier).
2. Lis d'abord CLAUDE.md, REPRISE.md, la dernière section de CHANGELOG.md, ROADMAP.md (surtout « Étape 5 en détail »), docs/ARCHITECTURE.md, docs/ECARTS.md et fixtures/A_FAIRE.md, puis résume l'état en 5 lignes.
3. Le Mac de Christophe n'est pas disponible (il est en déplacement) : pas de fichier iCloud, de mémoire ni de carnet d'idées ; tout ce qui compte est dans GitHub. Il peut en revanche ouvrir un dossier local sur son PC Windows pour que tu y essaies le code (à vérifier au moment voulu avec la compétence « computer use » / les outils de l'appareil lié, sinon lui demander de lancer des commandes et de te renvoyer les sorties).

ÉTAT (fin de S01, 25/10/2026) — dernier commit de code c603001, CI verte sur Linux, Windows et macOS, plus le commit de clôture
- Étapes 1 à 4 de la feuille de route faites : fondations ; 365 cas de référence tirés de Checks.swift dans fixtures/ ; cœur Rust (crates/core) ; moteurs (crates/engines).
- 342 des 365 cas passent dans cargo test (303 core, 39 engines) ; 14 cas d'interface (fixtures/ui_logic.json) attendent l'interface web ; 9 du mode Amusant (schtroumpf) sont reportés. 72 fonctions de test. Les prompts sont extraits du Swift et leur empreinte vaut 4ed86f0c1c154918 (identique à Scribe 1.12.0).
- crates/core : typographie, signes dictés, listes, « à la ligne », dictionnaire, texte déjà propre, garde-fous (mise au propre, traduction, avec le trait LanguageGuesser et lingua), modes et modes personnels, réglages (même format JSON que Scribe, lecture tolérante), prompts, hotkey (machine à états), media_pause, microphone, history (Store, conservation, tableau de bord).
- crates/engines : EngineServer (cycle de vie de whisper-server / llama-server), Transcriber (Deepgram ou Whisper), Cleaner (OpenRouter, LM Studio, Ollama, llama.cpp), HTTP / processus / horloge / journal / clés derrière des traits, FakeWorld pour les tests.
- RIEN n'a été essayé sur une vraie machine : ni touche globale, ni micro, ni collage, ni icône, ni vrais whisper-server / llama-server. Aucun crate crates/platform, crates/pipeline, src-tauri ni ui/ n'existe encore.
- Le mode Amusant est mis au propre en Simple sans la transformation schtroumpf (écart noté dans docs/ECARTS.md).

PROCHAINE TÂCHE : étape 5, MVP Windows (détail dans ROADMAP.md). Avant de coder, pose ces trois questions à Christophe et attends ses réponses :
 1. Touche de dictée par défaut sous Windows : je propose Ctrl droit, avec Maj en plus pour traduire (Option droite n'existe pas ; sur AZERTY la touche de droite équivalente est AltGr, vue par Windows comme Ctrl+Alt). Quel clavier ?
 2. Forme du MVP : d'abord un exécutable en ligne de commande avec icône dans la zone de notification pour tester la chaîne, puis la fenêtre Tauri avec les Réglages de base ? (ma recommandation)
 3. Quelle carte graphique (NVIDIA, AMD, aucune) ? Elle décide des binaires et des modèles à installer.
Puis : (a) crates/platform (un trait par fonction système + implémentation Windows : crochet clavier bas niveau, SendInput pour Ctrl+V, cpal pour le micro, Gestionnaire d'identifiants, tray-icon ; implémentations vides ou simulées ailleurs pour que tout compile) ; (b) crates/pipeline, port de DictationController.swift (appui → préchauffage → enregistrement → relâchement → transcription → corrections → mise au propre ou repli sur le texte brut → collage → historique ; annulation par une autre touche ; traduction), testé avec des faux de plateforme et FakeWorld ; (c) essai réel sur le PC Windows de Christophe : installer whisper-server et llama-server et un modèle (jamais de téléchargement sans son accord : annonce la taille et la source), une vraie dictée, relever les temps et les comparer au tableau « Temps mesurés » du README de Scribe.

RÈGLES (apprises en S01)
- Ne jamais lire ni commiter de .env ni de clé d'API. Ne jamais modifier scarou/scribe.
- Le comportement de référence est celui de Scribe : en cas de doute, lis le Swift, n'invente pas. Tout écart voulu va dans docs/ECARTS.md. Un cas de fixtures/ ne se modifie jamais pour faire passer un test : on corrige le code ou on note l'écart.
- Tout code propre à un système vit derrière crates/platform ; le cœur reste pur (aucune lecture de l'horloge, du disque ni du réseau). Les moteurs se testent avec FakeWorld ; aucun test ne lance de vrai whisper-server / llama-server ni ne touche au réseau (sauf real_spawner_* et http.rs, inoffensifs).
- Avant chaque commit : cargo fmt --all, cargo clippy --all-targets -- -D warnings, cargo test --workspace. Après chaque push : tools/attendre_ci.sh (en arrière-plan) ; la CI doit être verte sur les TROIS systèmes. Les échecs de CI se lisent en annotation (gh run view --log-failed est refusé depuis le cloud, la recherche GitHub aussi).
- Pièges Windows déjà rencontrés : « /a/b » et Path::new("/a").join("b") ne sont pas égaux ; les fausses listes de fichiers des tests se construisent avec join ; les programmes ont le suffixe .exe ; pas de pkill (on utilise sysinfo).
- fancy-regex n'accepte pas les assertions arrière de longueur variable (on les écrit en plusieurs assertions de longueur fixe).
- Ne jamais annoncer un chiffre (cas qui passent, nombre de tests) avant de l'avoir mesuré : deux messages de commit de S01 en contenaient de faux.
- Le prompt envoyé au modèle doit rester identique d'une dictée à l'autre jusqu'à la dictée elle-même ; les textes des prompts viennent de Services.swift par script, jamais retapés.
- Textes de l'interface et documentation en français ; identifiants du code en anglais. Commits courts en français, identité Christophe Campain <christophe.campain@gmail.com>, avec les lignes d'attribution demandées par la session.
- Rien de public sans l'accord de Christophe (passage du dépôt en public, publication). Aucun téléchargement de modèle sans son accord.

EN SUSPENS (pas pour cette session sauf demande)
- Au retour de Christophe sur son Mac : exporter les messages complets des prompts (consigne, exemples) des 8 modes, de la traduction et du préchauffage depuis le Swift, sur une branche locale jetable de Scribe jamais fusionnée, dans fixtures/prompts.json (voir fixtures/A_FAIRE.md) ; créer la mémoire du projet, le groupe « Scriibe » de la barre latérale et le carnet d'idées du vault.
- Vérifier le nom Scriibe (recherche web, base de marques INPI/EUIPO) avant l'ouverture ; README anglais et CONTRIBUTING.
- Choix à faire plus tard : Wayland (collage et raccourcis globaux restreints), interface web (Svelte ou React), langue de l'interface au-delà du français, équivalent portable du mode Amusant.
- Reprendre le banc de dictée, le test de frappe et ui_logic.json avec l'interface (étape 6).

CLÔTURE DE SESSION : applique la section « Clôture de session » de CLAUDE.md (CHANGELOG, REPRISE, ROADMAP, README, ECARTS, A_FAIRE à jour ; fmt, clippy, tests, CI verte), puis renomme la session « S02 - jj/mm/aaaa - Titre résumant la session ».
```
