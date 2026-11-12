# Alléger le CLAUDE.md (règle R03)

Chantier 1 du tri (découpage du 12/11/2026). Le `CLAUDE.md` est chargé à chaque session : il fait 12 308 caractères, la cible est ≤ 8 000.

## Plan

Principe : reste dans le `CLAUDE.md` ce qui sert à chaque session ou ce qu'on viole sans s'en apercevoir ; part dans `docs/` ce qui sert à la demande, avec une ligne de renvoi qui dit quand le lire.

### Reste dans le CLAUDE.md (environ 6 500 caractères)

- En-tête et description.
- **Référence** : bloc entier (comportement de Scribe, `.env`, règle sur le nom de l'auteur).
- **Stack visée** : bloc entier.
- **Règles**, version courte :
  - cas de `fixtures/` pour chaque comportement du cœur ;
  - code propre à un système derrière `crates/platform` ;
  - `cargo fmt`, `clippy -D warnings`, `test` avant tout commit ;
  - prompt identique d'une dictée à l'autre jusqu'à la dictée ;
  - sessions locales sur le PC de l'auteur, tout pousser avant la fin ;
  - une ligne : interface en deux langues, tout texte passe par `t('…')` et a sa traduction ; détail dans `docs/I18N.md`.
- **Tests**, version courte :
  - `cargo test --workspace` ;
  - un cas de `fixtures/` ne se modifie jamais pour faire passer un test ;
  - un cas `invariant` ou `history_scenario` doit être cité par un test ;
  - moteurs testés avec `FakeWorld`, jamais de vrai `whisper-server`/`llama-server` ni de réseau ;
  - une ligne de renvoi vers `docs/TESTS.md`.
- **Clôture**, version courte :
  - fichiers à lire à la reprise ;
  - ne jamais annoncer un chiffre avant de l'avoir mesuré ;
  - piège de `pkill -f` ;
  - ne pas réécrire les commits sous l'identité « Claude » ;
  - ligne « Carnet et découpage en cours » ;
  - une ligne de renvoi vers `docs/CLOTURE.md`.
- **Dépôt** : bloc entier.

### Part dans `docs/` (à la demande)

| Contenu | Fichier | Quand le lire |
|---|---|---|
| Règle i18n complète : `t()`, `tx()`, `en.ts`, `en-server.ts`, `i18n.test.ts`, `HelpEn.svelte`, menu de l'icône (`tray_source.rs`), `README.en.md` et guides `INSTALLATION_*.en.md` | `docs/I18N.md` | en modifiant un texte de l'interface, `Help.svelte` ou un guide d'installation |
| Tests d'interface (npm, Playwright, captures) ; `Path::join` ; clippy Windows (cible `x86_64-pc-windows-gnu`) ; crochet clavier Windows et relais des touches ; machine à états des touches ; extraction du banc et des textes de frappe ; empreinte des prompts `4ed86f0c1c154918` ; `verifier_fixtures.py` et `verifier_litteraux.py` | `docs/TESTS.md` | avant de tester ou de compiler, ou en touchant aux touches, au code Windows, aux prompts ou aux fixtures |
| Minutes GitHub Actions et CI ; compilation sur le PC ; publier une version ; détail de la clôture ; mises à jour de `scriibe.exe` par la fenêtre ; essais sur le PC de l'auteur ; essais chiffrés ; sessions sur le Mac (`Scriibe.app`, journal, autorisations) ; `attendre_ci.sh` | `docs/CLOTURE.md` | en clôturant une session, en publiant une version, ou en faisant un essai sur le PC ou le Mac de l'auteur |

### Choix

Trois fichiers regroupés par moment d'usage : une ligne de renvoi dans le `CLAUDE.md` dit quand ouvrir chacun. Les règles que l'on viole sans s'en apercevoir (nom de l'auteur, fixtures intouchables, prompt stable, `pkill -f`, chiffre non mesuré) restent dans le `CLAUDE.md`.

### Résultat (tâche 1, 12/11/2026)

- `CLAUDE.md` : 12 308 → **6 231** octets (6 084 caractères), cible ≤ 8 000 atteinte.
- Créés : `docs/I18N.md` (1 186), `docs/TESTS.md` (2 985), `docs/CLOTURE.md` (3 835).
- Vérification par script : chaque puce de l'ancien `CLAUDE.md` se retrouve mot pour mot dans le nouveau ou dans un des trois fichiers, sauf la règle i18n, volontairement réécrite (résumé dans le `CLAUDE.md`, détail dans `docs/I18N.md`).

### Suite

Le découpage réel (créer les trois fichiers, réduire le `CLAUDE.md`, mesurer la taille) est la tâche 1.

## Relecture (tâche 2, 12/11/2026)

Relecteur : sous-agent Sonnet, qui a comparé `git show e4eac8a:CLAUDE.md` au nouveau `CLAUDE.md` et aux trois fichiers de `docs/`. Tous les chemins cités existent ; aucune contradiction.

| # | Défaut | Correction |
|---|---|---|
| 1 | Interdiction de lire le jeton GitHub du Gestionnaire d'identifiants restée dans `docs/CLOTURE.md` : elle vaut pour toute session | Remontée dans `CLAUDE.md` (Référence), à côté de la règle sur `.env` |
| 2 | « Rien n'est téléchargé sans l'accord de l'auteur » dans `docs/CLOTURE.md` : vaut dès qu'on télécharge | Remontée dans `CLAUDE.md` (Référence), une ligne |
| 3 | Lancement de `Scriibe.app` par `open` (jamais par Bash) lisible seulement en ouvrant `docs/CLOTURE.md` | Une phrase dans le renvoi de clôture du `CLAUDE.md`, avec le renvoi à « Sessions sur le Mac de l'auteur » |
| 4 | `Path::join` pour les fausses listes de fichiers, piège qu'on viole en écrivant n'importe quel test | Remonté dans le bloc Tests du `CLAUDE.md` (copie dans `docs/TESTS.md` conservée) |
| 5 | Repères de la règle i18n réécrite (`tx`, `i18n.svelte.ts`, menu de l'icône) | Rien : voulu, tout est dans `docs/I18N.md` |
| 6 | `C:scriibe-dev` au lieu de `C:\scriibe-dev` (antérieur à l'allègement) | Corrigé |

## Essai à froid (tâche 3, 12/11/2026)

Modèle : Haiku 5.5, effort faible, session B, contexte vide. Lectures autorisées : `CLAUDE.md` (déjà chargé) et les fichiers qu'il cite. Aucune recherche libre.

**1) Lancer les tests, dont les cas de référence.**
`cargo test --workspace` rejoue les cas de `fixtures/` (`crates/core/tests/fixtures.rs`, `crates/engines/tests/fixtures.rs`) plus les tests écrits à la main. Avant tout commit : `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test` sans erreur. Interface : `cd ui && npm ci && npm run check && npm test && npm run build` (détail dans `docs/TESTS.md`). Vérification des fixtures : `python3 tools/verifier_fixtures.py`.
Fichiers lus : `CLAUDE.md`, `docs/TESTS.md`.

**2) Que faire à la clôture d'une session.**
Mettre à jour `CHANGELOG.md` (une section par session), `REPRISE.md`, `ROADMAP.md`, `README.md` (état), `docs/ECARTS.md` et `fixtures/A_FAIRE.md`. Lancer `cargo fmt`, `clippy` et `test`, pousser, puis `tools/attendre_ci.sh` (CI verte). Renommer la session « Snn - jj/mm/aaaa - Titre ».
Fichiers lus : `CLAUDE.md`, `docs/CLOTURE.md`.

**3) Où est le carnet d'idées.**
`~/Library/Mobile Documents/com~apple~CloudDocs/IA-OS/1 - Projets/scriibe/carnet-idees-scriibe.md` (chemin donné dans le bloc « Carnet et découpage en cours » du `CLAUDE.md`).
Fichiers lus : `CLAUDE.md`.

**4) Règles qui s'appliquent avant toute modification.**
- Ne jamais modifier le dépôt de Scribe (`scarou/scribe`) depuis ce projet ; en cas de doute sur un comportement, lire son code Swift.
- Ne jamais lire `.env`, ni une clé d'API, ni le jeton GitHub du Gestionnaire d'identifiants.
- Ne rien télécharger (modèle, binaire) sans l'accord de l'auteur ; annoncer taille et source d'abord.
- Chaque comportement du cœur a son cas dans `fixtures/` ; un cas ne se modifie jamais pour faire passer un test.
- Le code propre à un système reste derrière `crates/platform`.
- Le prompt reste identique d'une dictée à l'autre ; son empreinte doit valoir `4ed86f0c1c154918`.
- Un texte d'interface passe par `t('…')` avec sa traduction : lire `docs/I18N.md` avant de le toucher.
- Ne jamais écrire le nom ni l'adresse de l'auteur dans les fichiers du dépôt.
Fichiers lus : `CLAUDE.md`, et `docs/TESTS.md` pour la règle sur le prompt et les fixtures.

**5) Comment livrer.**
Dépôt `scarou/scriibe` (public), branche `main`, licence MIT. Commit au message court en français, avec l'identité Git déjà réglée sur la machine. Pousser, puis `tools/attendre_ci.sh` (CI verte sur les trois systèmes, lancement manuel seulement si la session a touché au code propre à un système, aux tests ou aux dépendances). Une version ne se publie qu'avec `tools/publier_macos.sh x.y.z`, après un aperçu et le « oui » de l'auteur.
Fichiers lus : `CLAUDE.md`, `docs/CLOTURE.md`.

**Bilan.** Les cinq réponses sont justes, chacune avec au plus deux lectures. Point à noter : `docs/CLAUDE_MD.md` n'est cité par aucun fichier de renvoi ; je l'ai lu pour y écrire cette section, et le carnet a été lu en entier, ce qui n'était pas nécessaire (la ligne qui le désigne suffisait). Pour un essai strictement à froid, le chemin du carnet et l'ordre de lecture gagneraient à figurer dans le `CLAUDE.md` lui-même.
