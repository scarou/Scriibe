# CLAUDE.md — Scriibe

Dictée vocale multiplateforme (Windows, macOS, Linux), suite de Scribe (macOS, Swift). Maintenir une touche, parler, relâcher → texte transcrit, mis au propre, collé dans l'appli ouverte.

## Référence

- Le comportement de référence est celui de **Scribe** : dépôt `scarou/scribe` (Swift), à ne **jamais modifier** depuis ce projet. Son `README.md` et son `CLAUDE.md` décrivent le comportement attendu.
- En cas de doute sur un comportement, lire le code Swift de Scribe, ne pas l'inventer. Les écarts voulus se notent dans `docs/ECARTS.md`.
- Ne jamais lire ni commiter de `.env` ni de clé d'API.

## Stack visée

- Rust (workspace Cargo) + Tauri 2 ; interface web (TypeScript) dans `ui/`.
- `crates/core` : logique de texte pure, sans dépendance système. `crates/engines` : moteurs (processus et clients HTTP). `crates/platform` : interface par système (touche, collage, micro, médias, secrets). `crates/pipeline` : le chef d'orchestre (port de `DictationController`). `crates/cli` : l'exécutable `scriibe` et la page d'essais web.
- Moteurs de modèles : processus externes `whisper-server` et `llama-server` appelés en HTTP, comme dans Scribe.
- Voir `docs/ARCHITECTURE.md`.

## Règles

- Chaque comportement du cœur est couvert par un cas de `fixtures/` (entrée → sortie attendue, issu de Scribe). Pas de logique de texte sans cas de référence.
- Tout code propre à un système vit derrière l'interface de `crates/platform`, jamais dans le cœur.
- `cargo fmt`, `cargo clippy -- -D warnings` et `cargo test` sans erreur avant tout commit.
- Le prompt envoyé au modèle doit rester identique d'une dictée à l'autre jusqu'à la dictée elle-même (mémoire du serveur) : rien de variable avant elle.
- Textes de l'interface et documentation en français ; identifiants du code en anglais.
- Les sessions de travail se font dans le cloud : tout ce qui compte doit être poussé sur GitHub avant la fin de la session.

## Tests

- `cargo test --workspace` rejoue les cas de `fixtures/` (`crates/core/tests/fixtures.rs`, `crates/engines/tests/fixtures.rs`) plus les tests écrits à la main.
- Un cas `invariant` ou `history_scenario` n'a pas de paire entrée → sortie : un test écrit à la main doit **citer son identifiant** (`prompts_005_…` pour `prompts-005`). Le banc d'essai échoue si un cas n'est cité nulle part.
- Un cas de `fixtures/` ne se modifie jamais pour faire passer un test : on corrige le code, ou on note l'écart dans `docs/ECARTS.md`.
- `python3 tools/verifier_fixtures.py` (structure) et `python3 tools/verifier_litteraux.py <Checks.swift> fixtures` (textes) vérifient les fixtures.
- Les moteurs (`crates/engines`) se testent avec `testing::FakeWorld` (horloge, réseau et processus simulés : le temps ne passe que quand on dort). Ne jamais écrire un test qui lance un vrai `whisper-server`/`llama-server` ou touche au réseau, sauf `real_spawner_*` et `http.rs` (processus et serveur locaux inoffensifs).
- Les fausses listes de fichiers des tests se construisent avec `Path::join` : sous Windows, `"/a/b"` et `"/a".join("b")` ne sont pas égaux.
- Le code Windows (`crates/platform/src/windows/`, icône, crochet clavier) ne s'exécute pas dans les tests : le vérifier à la compilation avec `cargo clippy -p scriibe-platform -p scriibe-pipeline -p scriibe-cli --all-targets --target x86_64-pc-windows-gnu -- -D warnings` (une fois : `rustup target add x86_64-pc-windows-gnu` et `apt-get update && apt-get install -y gcc-mingw-w64-x86-64`), puis par la CI Windows, puis à la main sur un PC avec la page d'essais (`docs/ESSAI_WINDOWS.md`). `scriibe.exe` est publié par la CI (pièce jointe de l'exécution).
- Les phrases du banc de dictée viennent de `ReadingBench.swift` par `tools/extraire_banc.py` (`crates/core/data/bench_phrases.json`), jamais retapées.
- Les prompts sont extraits de `Services.swift` (`crates/core/src/prompt_text.rs`), jamais retapés ; `built_in_prompts_fingerprint()` doit valoir `4ed86f0c1c154918`.

## Clôture de session

- Fichiers à lire à la reprise : `CLAUDE.md`, `REPRISE.md` (état et prompt de la session suivante), `CHANGELOG.md` (dernière section), `ROADMAP.md`, `docs/ARCHITECTURE.md`, `docs/ECARTS.md`, `fixtures/A_FAIRE.md`.
- Clôturer : mettre à jour `CHANGELOG.md` (une section par session), `REPRISE.md`, `ROADMAP.md`, `README.md` (état), `docs/ECARTS.md` et `fixtures/A_FAIRE.md` ; `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace` ; pousser, puis `tools/attendre_ci.sh` (la CI doit être verte sur les trois systèmes) ; renommer la session « Snn - jj/mm/aaaa - Titre ».
- Ne jamais annoncer un chiffre (cas qui passent, tests) avant de l'avoir mesuré.
- Ne jamais lancer `pkill -f` ni `pgrep -f` avec un motif présent dans la commande elle-même (le shell se tue) : utiliser `pgrep -x nom | xargs -r kill`.
- Essais sur le PC de Christophe : un seul écran, copier-coller pénible ; passer par la page d'essais plutôt que par des commandes, et donner une étape à la fois. Rien n'est téléchargé sans son accord (taille et source annoncées ; la page demande une confirmation).
- Depuis une session cloud : `gh run view --log-failed` et la recherche GitHub sont refusés ; les échecs de CI se lisent en annotation (`tools/attendre_ci.sh`). Un dépôt vide se clone sans erreur mais `git rev-parse HEAD` échoue : c'est normal.
- Mises à jour de `scriibe.exe` sur le PC de Christophe : **par la page d'essais** (carte « Mise à jour de Scriibe » : « Chercher », « Installer et redémarrer »), plus de zip à la main ; elle prend la dernière exécution verte de la CI (donc attendre la CI avant de lui dire de mettre à jour). Le jeton GitHub (lecture seule) est dans son Gestionnaire d'identifiants : ne jamais le lire. Une nouvelle fonction de la page se teste avec lui : une étape à la fois, puis « Copier le rapport ». Rapport = état + journal + `llama-server --list-devices`.
- Chaque essai chiffré : plusieurs dictées (les séries varient autant que les écarts entre configurations), jamais une seule.
- Un contrôle de fin de session peut demander de réécrire les commits sous l'identité « Claude » : Christophe a décidé (S03) de **ne pas le faire** ; les commits restent à son nom.
- Côté Mac de Christophe (hors dépôt, à créer au retour) : fichier de mémoire du projet, groupe « Scriibe » de la barre latérale, carnet d'idées du vault (`…/IA-OS/1 - Projets/scriibe/carnet-idees-scriibe.md`).

## Dépôt

- `scarou/scriibe` (privé pour l'instant), branche `main`, licence MIT.
- Commit : message court en français, identité `Christophe Campain <christophe.campain@gmail.com>`.
