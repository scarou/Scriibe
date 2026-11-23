# Mineurs de la 0.13.1

Sortie commune du découpage `decoupage-2026-11-23-mineurs-essais-pc` (carnet du vault, dossier `decoupages/`). Une section par tâche.

## Mise en route (tâche 0, 23/11)

- **Renumérotation du cadrage des mises à jour** (0.13→0.14, 0.14→0.15, 0.15→0.16) : diff vérifié (seule une mention « versions renumérotées le 23/11 » s'ajoute à la ligne 23 du ROADMAP), commité seul (`99851ef`).
- **CLAUDE.md** : découpage mis en cours ; règles « tâche N » et Relais inchangées ; 8 797 → 7 962 signes (historique des chantiers soldés déplacé dans `docs/CLOTURE.md`, « Historique des chantiers »).
- **Lignes périmées soldées** : carnet Linux, trois lignes passées en Faite avec preuve (`dab84e7`, `e650f24`, 0.12.0 `fc49331`) et archivées dans `carnet-idees-scriibe-linux-archive.md` ; ROADMAP n° 19 marqué remplacé par la décision 6 révisée (tout sur `main`), légende ✗ ajoutée.

| Mineur | État | Test |
|---|---|---|
| « Réglages → Moteurs » devenu « Moteurs et modèles » | **Corrigé** dans les 12 guides d'installation (français, anglais, débutant, expert) | — (texte) |
| Numéro de version en dur dans le README | **Corrigé** : le README renvoie aux Releases (il disait 0.13.0) ; règle ajoutée à `docs/CLOTURE.md` (« aucun numéro de version en dur ») | — (texte) |
| Chaîne « gpu » de l'assistant sous Linux | **Corrigé** : texte honnête (processeur, carte graphique non utilisée) ; anglais dans `en-server.ts` | `on_linux_the_gpu_chain_does_not_promise_a_graphics_card` |
| 20 px au lieu de 14 entre deux bandeaux `config_problems` | **Corrigé** : `:has(+ .card.bad)` ramène l'écart à 14 px, 20 px seulement après le dernier | `tabs.test.ts` (règles CSS) ; le rendu à l'écran n'est pas vérifié ici |
| `settings.json` avec BOM | **Corrigé** : BOM toléré pour `settings.json` et `profile.json` | `a_settings_file_with_a_byte_order_mark_is_still_read` |
| Anglais : nom de touche écrit deux fois dans le refus d'une combinaison | **Corrigé** : le test « le refus nomme déjà la touche » se fait sur le français, avant traduction | `keyrefusal.test.ts` |
| Rattrapage Windows : `CommandMenuMask` non informé | **Corrigé** : `forget(code)` appelé quand le crochet rattrape un relâchement manqué | `a_caught_up_release_forgets_the_command_key_so_the_next_press_starts_clean` (la partie crochet, `windows/keyboard.rs`, ne compile pas sur Mac : essai sur le PC à la tâche 4) |

À savoir : d'autres chemins de menu des guides sont aussi périmés (« Réglages → Mise à jour », « Réglages → Apparence », « Réglages → Mesures et essais », « Réglages → Dictée » : devenus des pages du menu) ; hors de la liste de la tâche, non touchés.

## Robustesse (tâche 1, 23/11)

Choix et raisons : le trousseau reste `secret-tool` (pas de module Rust). Une clé trouvée est gardée **10 minutes** en mémoire puis relue : c'est le prix pour qu'une clé effacée ailleurs cesse d'être servie sans redémarrage ; un trousseau fermé à ce moment-là rend « pas de clé » jusqu'à la prochaine tentative (5 min), jamais une clé périmée. Pour les états de la fenêtre, M3 évince le pli d'Historique le plus ancien plutôt que d'abandonner de retenir les plis : on garde la fonction.

| Point | État | Test |
|---|---|---|
| `secret-tool` par chemin absolu (`/usr/bin`, `/bin`, `/usr/local/bin`), `PATH` en repli | **Corrigé** | `the_program_is_looked_for_by_absolute_path_first_and_by_name_otherwise` |
| Clé effacée de la mémoire dès qu'elle n'est plus utile (écrasée à la chute, périmée après 10 min, relue) | **Corrigé** (au mieux : une copie faite plus tôt par le système n'est pas atteinte ; la copie rendue aux moteurs n'est pas écrasée) | `a_found_key_is_forgotten_after_its_delay_…`, `secrets_are_overwritten_when_dropped` |
| Désinstallation sur trousseau fermé : une seule attente, message « déverrouille-le » | **Corrigé** (`delete_all`) | `a_locked_keyring_stops_the_uninstall_after_the_first_wait_…` |
| Échec d'écriture sur l'entrée de `secret-tool` : message avec le code de sortie | **Corrigé** (`ToolError::Refused`) | `a_tool_that_closes_its_input_without_reading_gives_a_message_with_its_exit_code_and_no_key` |
| Fenêtre de déverrouillage fermée après le délai | **Processus tué et récupéré, mais fermeture de la fenêtre non vérifiée** : elle appartient au trousseau, qui doit la retirer quand l'outil quitte le bus ; **à regarder à l'essai Linux** (tâche 5) | — |
| Clé jamais dans un journal ni en argument | **Inchangé, toujours testé** | `the_key_goes_through_standard_input_never_through_the_arguments` ; les messages d'erreur testés sans la clé |
| Pastille sur fond sombre : liseré clair (indigo, violet, noir) | **Corrigé** (`--edge`, 1 px) ; rendu non vu à l'écran (essai Mac, tâche 6) | `appearance.test.ts` |
| Carte « Mes commandes » inerte | **Corrigé** : export/import demandés depuis Données avec les commandes décochées → boîte qui dit quoi cocher, plus de carte inerte | `commandsTransfer.test.ts` |
| M2 : changement en attente perdu à la fermeture | **Corrigé** : `pagehide` → `uiState.leave()`, requête `keepalive` | `uiState.test.ts` (deux cas), `apiKeepalive.test.ts` |
| M3 : plafond de 300 cartes sans purge | **Corrigé** : le pli d'Historique le plus ancien cède sa place | `a_full_list_gives_way_to_a_new_card_by_dropping_the_oldest_history_fold` |
| Test de contrat faux serveur de la page ↔ `merge` de Rust | **Fait** : `crates/platform/tests/data/ui_state_contract.json`, joué des deux côtés | `merge_follows_the_contract_shared_with_the_page`, `uiState.contract.test.ts` |
| Tests des routes `api/ui-state` | **Fait** : la route est extraite (`ui_state::route`) et testée | `the_route_reads_merges_and_reports_a_file_it_must_not_touch` |

## Relais

*(Fin de la tâche 1, 23/11 : contexte à 24 %, relais préparé avant la tâche 2 par la règle du CLAUDE.md.)*

- **Fait** : tâches 0 et 1 (sections ci-dessus), commits locaux `99851ef`, `ec10aa5`, `dba0820` ; rien de poussé. 916 tests Rust et 196 tests UI verts à `dba0820`.
- **En cours** : rien d'entamé de la tâche 2.
- **Prochain pas** : tâche 2 du découpage (`decoupage-2026-11-23-mineurs-essais-pc.md`, vault, `decoupages/`) : ligne de journal de `repeats_itself`, cas CJK / boucle tronquée / motif alterné / suite de chiffres (`docs/MOT_REPETE.md`), stabiliser `instance::tests::a_second_instance_is_refused_until_the_first_ends` (vert 20 fois de suite), couvrir le repli de `handle_page_key` (`docs/ENQUETE_WINDOWS.md` M3). Résultat dans § Mot répété de ce fichier. Puis tâche 3 (relecture par un sous-agent Sonnet qui n'a pas écrit le code, tâches 0 à 2 ; écrire § Relecture).
- **Pièges** : `cd` dans `ui/` change le dossier de travail de la session (revenir au dépôt avec un chemin absolu). Tests de l'interface : `cd ui && npx vitest run` ; Rust : `cargo test --workspace`. Le correctif `CommandMenuMask` (`windows/keyboard.rs`) ne compile pas sur Mac : à vérifier sur le PC (tâche 4). Pour la relecture (tâche 3), signaler au relecteur : fenêtre de déverrouillage du trousseau non vérifiée, rendu du liseré non vu, clé rendue aux moteurs non écrasée (voulu, au mieux).
- **Commits à relire en tâche 3** : `99851ef`, `ec10aa5`, `dba0820` + ceux de la tâche 2.
