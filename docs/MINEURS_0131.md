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
