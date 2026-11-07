# Cas de référence pas encore repris

Tout ce qui est dans `Checks.swift` mais pas dans `fixtures/`, avec la raison. À reprendre à l'étape indiquée de la [feuille de route](../ROADMAP.md).

## Cas présents mais pas encore rejoués par `cargo test`

| Fichier | Cas | Raison |
|---|---|---|
| `ui_logic.json` | 14 | **Rejoués en S04 par `ui/src/lib/logic.test.ts`** (Vitest) : les 11 cas avec paire entrée → sortie (position de l'indicateur, cartes repliables, zone du banc), le vumètre (`ui_logic-012`, test écrit à la main) et l'Aide (`ui_logic-014`, `tabs.test.ts`) ; `ui_logic-013` (Réglages en cinq rubriques) est écarté (`docs/ECARTS.md`). Le test échoue si un cas n'est ni rejoué ni écarté. |
| `playful_schtroumpf.json` | 9 | **Mode Amusant abandonné (S05)** : ces cas ne seront jamais rejoués (le fichier reste tel quel, ils sont déclarés « reporté » par le banc de test). |

## À reprendre avec l'interface complète (étape 6) — fait en S04 sauf mention

| Lignes de `Checks.swift` | Sujet | Pourquoi pas maintenant |
|---|---|---|
| 542–563, 573–586, 1000–1125 (sauf contrôles) | **Banc de dictée** : déroulement (`ReadingBench` : phrase suivante, « Passer », « Relire »), rapport avec temps à la française, archive des passages, comparaison, différences mot à mot, réglages d'un passage | **Repris en S04** : déroulement, archive, comparaison, différences mot à mot (`bench_archive.rs`, `bench_store.rs`, `benchflow.ts`) ; **Repris en S07** : la comparaison « avec / sans modèle » (`RunEntry`, `versions`, `clean_but_changed`, `benchflow.ts`) et les réglages du banc selon les modes personnels (déjà couverts, test ajouté) |
| 786–839 | **Test de frappe** (`TypingTest`) : vitesse nette, seuils (90 %, 12 s–10 min, 200 mots/min), moyenne | **Repris en S04** : `ui/src/lib/typing.ts` + `typing.test.ts` (textes extraits par `tools/extraire_frappe.py`) ; reste la zone de saisie sans correction automatique propre à AppKit (une zone de texte web sans correction ni collage la remplace) |
| 247–252 | **Phrases types** de `--cleantest` (12 phrases, contrôles associés) | **Repris en S06** : `crates/core/src/clean_test.rs` (phrases extraites par `tools/extraire_phrases_types.py`, tests `clean_test::tests`) ; reste la ligne de commande `--cleantest` (non portée : les essais se font depuis les pages) |
| 242–246 | Réglages du banc selon les modes personnels | **Repris en S07** (`bench_view::tests`) |
| 604, 608–625 | Vumètre et aperçu de l'indicateur (minuteries) | courbe du vumètre reprise (`MeterHistory`) ; l'aperçu et les minuteries attendent la fenêtre d'indicateur |
| 748–788, 825–839 | Zone du banc et zone de frappe (`NSTextView`) | propre à AppKit |

## Impossibles sans le Swift sur un Mac

| Sujet | À faire |
|---|---|
| **Messages complets des prompts** (consigne, exemples, dictionnaire) pour les 8 modes, la traduction et le préchauffage | Un oracle texte exact demande d'exécuter `Cleaner.messages(...)` ; seul l'empreinte (`prompts.json`) est connue aujourd'hui. Au retour sur le Mac : exporter les messages dans un fichier JSON **sans modifier le dépôt Scribe** (branche locale jetable, jamais fusionnée), puis les ajouter à `fixtures/prompts.json`. |
| Mesures avec un modèle (`--cleantest`, `--selftest`) | Pas des tests automatiques : des mesures, à refaire sur chaque plateforme. |

## Noté en S03 (aucun cas de `Checks.swift` en plus)

- La **logique de minuteries de la pause des médias** (`MediaPause.swift`, hors de la partie pure déjà reprise) n'a pas de cas dans `Checks.swift` : elle est vérifiée par des tests écrits à la main (`crates/platform/src/media.rs`, `media::tests`, avec une fausse source audio) et à la main sur un PC.
- Rien d'autre n'a été ajouté ni retiré : les cas en attente (`ui_logic.json`, mode Amusant, prompts complets, banc, test de frappe) sont inchangés.

## Noté en S08 (aucun cas de `Checks.swift` en plus)

- Les phrases du **banc anglais** (17) et les **phrases types anglaises** (12) n'existent pas dans Scribe : écrites pour Scriibe (`crates/core/data/bench_phrases_en.json`, `clean_samples_en.json`), vérifiées par des tests écrits à la main (`bench::tests`, `clean_test::tests`), écart noté dans `docs/ECARTS.md`. Les cas de `fixtures/` ne les concernent pas.

## Sans objet pour le cœur

Sélecteur de fichiers, `llama-server --help` (cas `invariant` : à vérifier à l'exécution), cartes et rubriques de l'interface (reprises dans `ui_logic.json` quand elles ont une logique testable).

- **Commandes sur sélection (S18)** : limites connues de la relecture du 07/11/2026 listées dans `docs/COMMANDES.md` (« Relecture de la livraison ») : VS Code (copie de ligne sans sélection), sélection identique au presse-papiers, faux positifs de signes, `dictionary::apply` encore appelé par l'essai de modèle et le banc.
