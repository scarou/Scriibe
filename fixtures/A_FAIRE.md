# Cas de référence pas encore repris

Tout ce qui est dans `Checks.swift` mais pas dans `fixtures/`, avec la raison. À reprendre à l'étape indiquée de la [feuille de route](../ROADMAP.md).

## Cas présents mais pas encore rejoués par `cargo test`

| Fichier | Cas | Raison |
|---|---|---|
| `ui_logic.json` | 14 | Logique d'interface (position de l'indicateur, cartes repliables, zone du banc, vumètre) : sera rejouée par les tests de l'interface web (étape 6). |
| `playful_schtroumpf.json` | 9 | Mode Amusant reporté (dépend de l'analyseur grammatical d'Apple). |

## À reprendre avec l'interface complète (étape 6)

| Lignes de `Checks.swift` | Sujet | Pourquoi pas maintenant |
|---|---|---|
| 542–563, 573–586, 1000–1125 (sauf contrôles) | **Banc de dictée** : déroulement (`ReadingBench` : phrase suivante, « Passer », « Relire »), rapport avec temps à la française, archive des passages, comparaison, différences mot à mot, réglages d'un passage | fonctionnalité d'interface ; **déjà repris en S02** : les 17 phrases et les contrôles `BenchCheck` (`bench_checks.json`, utilisés par la page d'essais) |
| 786–839 | **Test de frappe** (`TypingTest`) : vitesse nette, seuils (90 %, 12 s–10 min, 200 mots/min), moyenne | fonctionnalité d'interface ; les valeurs sont dans le test, à extraire avec le port de `TypingTest.swift` |
| 247–252 | **Phrases types** de `--cleantest` (12 phrases, contrôles associés) | à reprendre avec le port de `CleanTest.swift` |
| 242–246 | Réglages du banc selon les modes personnels | dépend du banc |
| 604, 608–625 | Vumètre et aperçu de l'indicateur (minuteries) | dépend de l'interface de l'indicateur |
| 748–788, 825–839 | Zone du banc et zone de frappe (`NSTextView`) | propre à AppKit |

## Impossibles sans le Swift sur un Mac

| Sujet | À faire |
|---|---|
| **Messages complets des prompts** (consigne, exemples, dictionnaire) pour les 8 modes, la traduction et le préchauffage | Un oracle texte exact demande d'exécuter `Cleaner.messages(...)` ; seul l'empreinte (`prompts.json`) est connue aujourd'hui. Au retour sur le Mac : exporter les messages dans un fichier JSON **sans modifier le dépôt Scribe** (branche locale jetable, jamais fusionnée), puis les ajouter à `fixtures/prompts.json`. |
| Mesures avec un modèle (`--cleantest`, `--selftest`) | Pas des tests automatiques : des mesures, à refaire sur chaque plateforme. |

## Noté en S03 (aucun cas de `Checks.swift` en plus)

- La **logique de minuteries de la pause des médias** (`MediaPause.swift`, hors de la partie pure déjà reprise) n'a pas de cas dans `Checks.swift` : elle est vérifiée par des tests écrits à la main (`crates/platform/src/media.rs`, `media::tests`, avec une fausse source audio) et à la main sur un PC.
- Rien d'autre n'a été ajouté ni retiré : les cas en attente (`ui_logic.json`, mode Amusant, prompts complets, banc, test de frappe) sont inchangés.

## Sans objet pour le cœur

Sélecteur de fichiers, `llama-server --help` (cas `invariant` : à vérifier à l'exécution), cartes et rubriques de l'interface (reprises dans `ui_logic.json` quand elles ont une logique testable).
