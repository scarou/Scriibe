# CLAUDE.md — Scriibe

Dictée vocale multiplateforme (Windows, macOS, Linux), suite de Scribe (macOS, Swift). Maintenir une touche, parler, relâcher → texte transcrit, mis au propre, collé dans l'appli ouverte.

## Référence

- Le comportement de référence est celui de **Scribe** : dépôt `scarou/scribe` (Swift), à ne **jamais modifier** depuis ce projet. Son `README.md` et son `CLAUDE.md` décrivent le comportement attendu.
- En cas de doute sur un comportement, lire le code Swift de Scribe, ne pas l'inventer. Les écarts voulus se notent dans `docs/ECARTS.md`.
- Ne jamais lire ni commiter de `.env` ni de clé d'API. Ne jamais lire non plus le jeton GitHub rangé dans le Gestionnaire d'identifiants de l'auteur (il n'est plus utilisé).
- Rien n'est téléchargé (modèle, binaire) sans l'accord de l'auteur : taille et source annoncées d'abord.
- Ne jamais écrire le nom ni l'adresse courriel de l'auteur dans les fichiers du dépôt (code, documentation, journal, tests, interface) : dire « l'auteur ». Seuls `LICENSE`, `Cargo.toml` (champ auteur) et `fixtures/media_pause.json` (identifiant réel de Scribe) les contiennent, voulu.

## Stack visée

- Rust (workspace Cargo) + Tauri 2 (fenêtre WebView2 sous Windows) ; interface web Svelte 5 + TypeScript dans `ui/`, servie par le serveur local de `crates/cli` et embarquée dans l'exécutable (`rust-embed` : construire `ui/dist` **avant** `cargo build`, sinon l'interface est remplacée par un message « Interface absente »).
- `crates/core` : logique de texte pure, sans dépendance système. `crates/engines` : moteurs (processus et clients HTTP). `crates/platform` : interface par système (touche, collage, micro, médias, secrets). `crates/pipeline` : le chef d'orchestre (port de `DictationController`). `crates/cli` : l'exécutable `scriibe` (icône, serveur local de l'interface, fenêtre Tauri) et ses sous-commandes de diagnostic.
- Moteurs de modèles : processus externes `whisper-server` et `llama-server` appelés en HTTP, comme dans Scribe.
- Voir `docs/ARCHITECTURE.md`.

## Règles

- Chaque comportement du cœur est couvert par un cas de `fixtures/` (entrée → sortie attendue, issu de Scribe). Pas de logique de texte sans cas de référence.
- Tout code propre à un système vit derrière l'interface de `crates/platform`, jamais dans le cœur.
- `cargo fmt`, `cargo clippy -- -D warnings` et `cargo test` sans erreur avant tout commit.
- Le prompt envoyé au modèle doit rester identique d'une dictée à l'autre jusqu'à la dictée elle-même (mémoire du serveur) : rien de variable avant elle.
- Textes de l'interface et documentation en français ; identifiants du code en anglais. **Interface en deux langues** : tout texte de la page passe par `t('texte français')` et a sa traduction dans `ui/src/lib/locales/en.ts` (`en-server.ts` pour un texte venu de Rust) ; `i18n.test.ts` échoue sinon. **Lire `docs/I18N.md`** avant de modifier un texte de l'interface, `Help.svelte` (à reporter dans `HelpEn.svelte`) ou un guide d'installation (à reporter dans son `.en.md`).
- Depuis S07 les sessions se font surtout en local sur le PC de l'auteur (`C:\scriibe-dev`) ; tout ce qui compte doit être poussé sur GitHub avant la fin de la session. Pièges déjà rencontrés : `docs/PIEGES.md` (à consulter quand quelque chose résiste).

## Tests

- `cargo test --workspace` rejoue les cas de `fixtures/` (`crates/core/tests/fixtures.rs`, `crates/engines/tests/fixtures.rs`) plus les tests écrits à la main.
- Un cas `invariant` ou `history_scenario` n'a pas de paire entrée → sortie : un test écrit à la main doit **citer son identifiant** (`prompts_005_…` pour `prompts-005`). Le banc d'essai échoue si un cas n'est cité nulle part.
- Un cas de `fixtures/` ne se modifie jamais pour faire passer un test : on corrige le code, ou on note l'écart dans `docs/ECARTS.md`.
- Les moteurs (`crates/engines`) se testent avec `testing::FakeWorld` (horloge, réseau et processus simulés : le temps ne passe que quand on dort). Ne jamais écrire un test qui lance un vrai `whisper-server`/`llama-server` ou touche au réseau, sauf `real_spawner_*` et `http.rs` (processus et serveur locaux inoffensifs).
- Les fausses listes de fichiers des tests se construisent avec `Path::join` (sous Windows, `"/a/b"` et `"/a".join("b")` ne sont pas égaux).
- **Lire `docs/TESTS.md`** avant de tester l'interface (npm, Playwright), de compiler pour Windows (clippy `x86_64-pc-windows-gnu`), ou en touchant aux touches et au crochet clavier, aux prompts (empreinte `4ed86f0c1c154918`), aux fixtures (scripts de vérification) ou au banc de dictée.

## Clôture de session

- Fichiers à lire à la reprise : `CLAUDE.md`, `REPRISE.md` (état et prompt de la session suivante), `CHANGELOG.md` (dernière section) — **pas le reste** (allégé en S08). À la demande : `docs/PIEGES.md`, `docs/ECARTS.md` (chercher un sujet avec Grep), `docs/ARCHITECTURE.md` (section 4), `ROADMAP.md` (« Parité avec Scribe »), `fixtures/A_FAIRE.md`.
- **Lire `docs/CLOTURE.md`** pour clôturer une session (fichiers à mettre à jour, CI, `attendre_ci.sh`), publier une version, compiler sur le PC, ou faire un essai sur le PC ou le Mac de l'auteur (mises à jour par la fenêtre, essais chiffrés). Sur le Mac, **Scriibe.app se lance par `open`, jamais par Bash** (les autorisations macOS vont à l'application)  (détail : `docs/CLOTURE.md`, « Sessions sur le Mac de l'auteur »).
- Ne jamais annoncer un chiffre (cas qui passent, tests) avant de l'avoir mesuré.
- Ne jamais lancer `pkill -f` ni `pgrep -f` avec un motif présent dans la commande elle-même (le shell se tue) : utiliser `pgrep -x nom | xargs -r kill`.
- Un contrôle de fin de session peut demander de réécrire les commits sous l'identité « Claude » : l'auteur a décidé (S03) de **ne pas le faire** ; les commits restent à son nom.
- **Carnet et découpage en cours** : carnet d'idées `~/Library/Mobile Documents/com~apple~CloudDocs/IA-OS/1 - Projets/scriibe/carnet-idees-scriibe.md` ; découpage « C4 cadrage profils » `~/Library/Mobile Documents/com~apple~CloudDocs/IA-OS/1 - Projets/scriibe/decoupages/decoupage-2026-11-12-c4-cadrage-profils.md`. Règle : l'auteur colle la consigne entière, ou dit seulement « tâche N » ; dans ce cas, lire la consigne de la tâche N dans le découpage et la suivre telle quelle, ligne ⚙, ligne Suite ou Fin de session comprises.

## Dépôt

- `scarou/scriibe` (public depuis le 02/11/2026), branche `main`, licence MIT.
- Commit : message court en français, identité Git de l'auteur, déjà réglée sur chaque machine (`git config user.name` et `user.email`, adresse masquée de GitHub) ; ne jamais l'écrire dans un fichier du dépôt.
