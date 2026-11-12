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

### Suite

Le découpage réel (créer les trois fichiers, réduire le `CLAUDE.md`, mesurer la taille) est la tâche 1.
