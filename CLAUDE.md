# CLAUDE.md — Scriibe

Dictée vocale multiplateforme (Windows, macOS, Linux), suite de Scribe (macOS, Swift). Maintenir une touche, parler, relâcher → texte transcrit, mis au propre, collé dans l'appli ouverte.

## Référence

- Le comportement de référence est celui de **Scribe** : dépôt `scarou/scribe` (Swift), à ne **jamais modifier** depuis ce projet. Son `README.md` et son `CLAUDE.md` décrivent le comportement attendu.
- En cas de doute sur un comportement, lire le code Swift de Scribe, ne pas l'inventer. Les écarts voulus se notent dans `docs/ECARTS.md`.
- Ne jamais lire ni commiter de `.env` ni de clé d'API.

## Stack visée

- Rust (workspace Cargo) + Tauri 2 ; interface web (TypeScript) dans `ui/`.
- `crates/core` : logique de texte pure, sans dépendance système. `crates/engines` : moteurs (processus et clients HTTP). `crates/platform` : interface par système (touche, collage, micro, médias, secrets).
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
- Les prompts sont extraits de `Services.swift` (`crates/core/src/prompt_text.rs`), jamais retapés ; `built_in_prompts_fingerprint()` doit valoir `4ed86f0c1c154918`.

## Dépôt

- `scarou/scriibe` (privé pour l'instant), branche `main`, licence MIT.
- Commit : message court en français, identité `Christophe Campain <christophe.campain@gmail.com>`.
