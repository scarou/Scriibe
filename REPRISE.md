# Reprise — 26/10/2026 (fin de S02)

Le texte ci-dessous est le prompt à coller au début de la session suivante.

```text
Projet Scriibe : dictée vocale multiplateforme (Windows, macOS, Linux), suite de Scribe (macOS, Swift, dépôt scarou/scribe), réécrite en Rust + Tauri 2. Dépôt scarou/scriibe (privé, branche main, licence MIT ; GitHub le nomme maintenant scarou/Scriibe, la redirection marche). Christophe Campain est le seul développeur ; tu écris tout le code, il décide et valide.

DÉMARRAGE (session cloud Linux, aucun dépôt n'est présent au départ)
1. Attache les deux dépôts avec add_repo, puis clone-les un par un (shallow, timeout généreux, jamais en parallèle) puis register_repo_root : scarou/scriibe en accès « push » (c'est là que tu travailles) et scarou/scribe en lecture seule (la référence Swift : à lire, JAMAIS à modifier).
2. Lis d'abord CLAUDE.md, REPRISE.md, la dernière section de CHANGELOG.md (S02), ROADMAP.md (surtout « Étape 5 en détail »), docs/ARCHITECTURE.md, docs/ECARTS.md, docs/ESSAI_WINDOWS.md et fixtures/A_FAIRE.md, puis résume l'état en 5 lignes.
3. Le Mac de Christophe n'est pas disponible (il est en déplacement) : tout ce qui compte est dans GitHub. Il essaie le code sur son PC Windows (HP, AZERTY, Radeon RX 540 2 Go + Intel UHD 620, donc moteurs sur processeur) en téléchargeant scriibe.exe depuis GitHub (onglet Actions, dernière exécution verte de « CI », pièce jointe `scriibe-windows-x64`), en double-cliquant dessus (la page d'essais s'ouvre dans le navigateur) et en te collant le « rapport » de la page. Il n'a qu'un seul écran : évite les commandes à copier, donne une étape à la fois.

ÉTAT (fin de S02, 26/10/2026) — dernier commit de code 15090e0, CI verte sur Linux, Windows et macOS, plus le commit de clôture
- Étapes 1 à 4 faites (S01) ; étape 5 (MVP Windows) bien avancée : crates/platform (traits, faux, stockage, implémentation Windows : crochet clavier, SendInput, presse-papiers, cpal, Gestionnaire d'identifiants, icône de notification), crates/pipeline (port de DictationController, 35 tests de bout en bout), crates/cli (`scriibe` : lancer = touche + icône + page d'essais web, verifier, micro, essai, cle). Ctrl droit dicte, Maj en plus traduit.
- Mesuré : 376 des 399 cas de référence rejoués par cargo test (337 cœur, 39 moteurs) ; 14 cas d'interface et 9 du mode Amusant attendent ; 144 fonctions de test. Le code Windows est vérifié à la compilation croisée (cible x86_64-pc-windows-gnu) et par la CI Windows, jamais par un test exécuté.
- ESSAYÉ sur le PC de Christophe : micro (bang du pilote Conexant à l'ouverture et casque très faible : corrigés par un niveau qui ignore 0,45 s et un gain ≤ ×10 ; réglages Windows : volume d'entrée 40-50 %, « améliorations audio » désactivées), Whisper `base` (2 à 4 s pour 4 à 8 s de parole), llama.cpp avec Qwen 2.5 1,5 B (mise au propre 3,7 à 4,2 s, 30 s à froid, qualité insuffisante : « bon ben » non retiré, un mot changé à tort), téléchargements confirmés depuis la page. Whisper `small`, Vulkan, cloud : non essayés.
- PAS ENCORE ESSAYÉ : la touche globale réelle (Ctrl droit dans le Bloc-notes, Maj, annulation), le collage SendInput dans une vraie appli, l'icône (couleurs), les bips, la traduction réelle, l'effet des fils sur les cœurs physiques (Whisper et llama-server, nouvelle version), la pause des médias (non portée).
- Kyutai STT étudié (voir CHANGELOG S02) : non viable sur ce PC (CUDA/Metal/MLX seulement) ; idée de moteur optionnel plus tard.

PROCHAINE TÂCHE : finir l'étape 5 (ROADMAP « Reste à faire »), dans cet ordre
 1. Demande à Christophe de télécharger la dernière version de scriibe.exe, de lancer la dictée avec la touche (Ctrl droit) dans le Bloc-notes avec la mise au propre sur « Aucune » (rapide) et de décrire : texte collé ? icône (gris repos, rouge écoute, orange traitement, vert collé, bleu message) ? bips ? annulation par une autre touche ? Maj pour traduire (mise au propre « llama.cpp » ou cloud nécessaire) ? Corrige ce qui ne marche pas.
 2. Mesure les fils sur les cœurs physiques (journal : « , N fils »), puis les options de vitesse : Whisper `small`, version « vulkan » de llama-server sur la Radeon (2 Go), Qwen 0,5 B, ou le cloud si Christophe a une clé OpenRouter (`scriibe cle openrouter`, jamais la lire ni l'afficher).
 3. Pause des médias sous Windows (sessions audio WASAPI), puis comparaison des temps au tableau « Temps mesurés » du README de Scribe, et décision sur les modèles proposés par niveau de machine.
 4. Ensuite l'étape 6 : fenêtre Tauri et Réglages (la page d'essais sert de base).

RÈGLES (apprises en S01 et S02)
- Ne jamais lire ni commiter de .env ni de clé d'API. Ne jamais modifier scarou/scribe.
- Le comportement de référence est celui de Scribe : en cas de doute, lis le Swift, n'invente pas. Tout écart voulu va dans docs/ECARTS.md. Un cas de fixtures/ ne se modifie jamais pour faire passer un test : on corrige le code ou on note l'écart.
- Tout code propre à un système vit derrière crates/platform ; le cœur reste pur. Les moteurs se testent avec FakeWorld ; aucun test ne lance de vrai whisper-server / llama-server ni ne touche au réseau (sauf real_spawner_* et http.rs, inoffensifs).
- Avant chaque commit : cargo fmt --all, cargo clippy --all-targets -- -D warnings, cargo test --workspace ; pour le code Windows aussi cargo clippy -p scriibe-platform -p scriibe-pipeline -p scriibe-cli --all-targets --target x86_64-pc-windows-gnu -- -D warnings (une fois par session : rustup target add x86_64-pc-windows-gnu ; apt-get update && apt-get install -y gcc-mingw-w64-x86-64). Après chaque push : tools/attendre_ci.sh (en arrière-plan) ; la CI doit être verte sur les TROIS systèmes, et le job « scriibe.exe (Windows) » produit l'exécutable. Les échecs de CI se lisent en annotation (gh run view --log-failed est refusé depuis le cloud, la recherche GitHub aussi ; gh api sur d'autres dépôts, kyutai.org et Hugging Face ne sont pas joignables depuis le cloud).
- Pièges déjà rencontrés : « /a/b » et Path::new("/a").join("b") ne sont pas égaux sous Windows (join dans les tests) ; programmes en .exe ; pas de pkill (sysinfo) ni de `pkill -f`/`pgrep -f` avec un motif présent dans la commande (le shell se tue : pgrep -x nom | xargs -r kill) ; fancy-regex sans assertions arrière de longueur variable ; AppSettings::default() change d'identifiants à chaque appel (ne pas comparer deux défauts) ; windows-sys 0.59 (poignées = pointeurs).
- Ne jamais annoncer un chiffre (cas qui passent, nombre de tests, temps) avant de l'avoir mesuré.
- Le prompt envoyé au modèle doit rester identique d'une dictée à l'autre jusqu'à la dictée elle-même ; les prompts viennent de Services.swift par script, les phrases du banc de ReadingBench.swift par tools/extraire_banc.py, jamais retapés.
- Textes de l'interface et documentation en français ; identifiants du code en anglais. Commits courts en français, identité Christophe Campain <christophe.campain@gmail.com> (git config user.name/email dans le dépôt), avec les lignes d'attribution demandées par la session.
- Rien de public sans l'accord de Christophe. Aucun téléchargement sans son accord (la page affiche nom, taille réelle et source, puis demande confirmation).

EN SUSPENS (pas pour cette session sauf demande)
- Au retour de Christophe sur son Mac : exporter les messages complets des prompts des 8 modes, de la traduction et du préchauffage depuis le Swift, sur une branche locale jetable de Scribe jamais fusionnée, dans fixtures/prompts.json (voir fixtures/A_FAIRE.md) ; créer la mémoire du projet, le groupe « Scriibe » de la barre latérale et le carnet d'idées du vault.
- Vérifier le nom Scriibe (INPI/EUIPO, recherche web) avant l'ouverture ; README anglais et CONTRIBUTING.
- Choix à faire plus tard : Wayland, interface web (Svelte ou React), langue de l'interface au-delà du français, équivalent portable du mode Amusant, moteur optionnel Kyutai (Mac MLX / PC NVIDIA), démarrage avec Windows, signature Authenticode.
- Reprendre le déroulement du banc de dictée (archive, comparaison), le test de frappe et ui_logic.json avec l'interface (étape 6) ; les 17 phrases et leurs contrôles sont déjà repris.

CLÔTURE DE SESSION : applique la section « Clôture de session » de CLAUDE.md (CHANGELOG, REPRISE, ROADMAP, README, ECARTS, A_FAIRE à jour ; fmt, clippy, tests, CI verte), puis renomme la session « S03 - jj/mm/aaaa - Titre résumant la session ».
```
