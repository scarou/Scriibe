# Banc des modèles de mise au propre (25/11/2026)

Outil : `scriibe messages-mise-au-propre` (le vrai prompt, mode Simple, cran Léger, produit par `cleaner::messages`) + `tools/banc_mise_au_propre.py` (rejeu sur un `llama-server` local, mêmes options que Scriibe, température 0). Corpus (hors dépôt, texte personnel) : 192 dictées à écart de l'analyse précédente, 60 dictées sans écart tirées de l'historique, 12 phrases témoins d'accords à corriger (écrites pour le banc). Écart = comparaison mot à mot, casse, accents et ponctuation ignorés, hors signes dictés et mots du dictionnaire.

| Modèle (GGUF Q4) | Écart sur les 192 | Écart sur 60 saines | Accords corrigés (12) | Médiane / dictée | Échecs |
|---|---|---|---|---|---|
| chocolatine-3b (actuel) | 167 | 18 | 10 | 0,56 s | 0 |
| **gemma-4-E4B** | **82** | **10** | **12** | 0,51 s | 0 |
| Ministral-3-3B | 159 | 41 | 12 | 0,36 s | 0 |
| gemma-4-12B | non valable | non valable | — | 35 s | 110 |
| gemma-4-26B-A4B | non valable | non valable | — | 11 s | 58 |

## Lecture

- **gemma-4-E4B est le meilleur sur tout** : deux fois moins d'écarts que chocolatine sur les dictées à écart, moins sur les saines, tous les accords corrigés, aussi rapide. Nature de ses écarts (82) : 56 remplacements, 48 suppressions, 6 ajouts.
- Ministral-3-3B corrige les accords mais réécrit beaucoup (41 dictées saines abîmées sur 60) : écarté.
- gemma-4-12B et gemma-4-26B-A4B : de nombreuses requêtes ont expiré (délai de 120 s, Mac de 32 Go, modèles trop lourds pour le rejeu) ; une réponse vide compte comme suppression, les chiffres sont donc faux. Trop lents (11 à 35 s) pour une dictée de toute façon.
- Le rejeu de chocolatine laisse 18 dictées saines sur 60 avec un écart : l'historique a été produit avec d'autres versions du prompt ou du dictionnaire ; les comparaisons entre modèles, elles, sont faites dans les mêmes conditions.
- Aucun modèle n'est à zéro écart : le contrôle déterministe reste nécessaire.

## Suite proposée

1. Modèle : **gemma-4-E4B** (déjà le modèle LM Studio des réglages ; déjà dans le dossier des modèles de Scriibe).
2. Contrôle déterministe après le modèle : aligner dictée et sortie mot à mot, n'accepter que ponctuation, casse, accents, nombres, accords de genre et de nombre (même racine, terminaison différente), dictionnaire, signes dictés, hésitations ; rétablir le mot dicté pour tout autre remplacement, suppression ou ajout. À mesurer ensuite sur ce même corpus.

## Contrôle mot à mot (écrit le 25/11/2026, `crates/core/src/word_guard.rs`)

`restore_dictated_words(dictée, sortie, dictionnaire)` : alignement mot à mot (plus longue sous-suite commune). Passent la ponctuation, la casse, les accents, les traits d'union, les nombres (chiffres et lettres), les accords (terminaisons de genre et de nombre, verbes être, avoir, aller, pouvoir, déterminants), les mots du dictionnaire, les signes dictés (au moins un vrai mot de commande) et les hésitations. Tout autre remplacement, suppression ou ajout est défait : le mot dicté revient. Pas encore branché dans la chaîne de dictée ; essayé sur le banc par `scriibe controler-mot-a-mot` et `banc_mise_au_propre.py rapport controle`.

Même corpus, après contrôle. Les restes sont des changements permis (accords, nombres, dictionnaire, signes dictés) ; liste relue à la main pour gemma-4-E4B : aucun changement de mot non permis.

| Modèle | Dictées à changement permis (sur 192) | Sur 60 saines | Accords corrigés (12) |
|---|---|---|---|
| chocolatine-3b | 17 | 1 | 10 |
| **gemma-4-E4B** | 28 | 3 | **12** |
| Ministral-3-3B | 35 | 3 | 12 |

Les chiffres des deux gros modèles ne valent rien (requêtes expirées rétablies en dictée brute).

Lecture : avec le contrôle, aucun modèle ne trahit plus les mots ; ce qui les distingue est ce qu'ils corrigent. gemma-4-E4B corrige tous les accords témoins, chocolatine en laisse deux. Limites : un accord faux mais plausible passe (la liste des formes irrégulières est courte et s'étend au besoin) ; le contrôle n'est qu'un filtre, il ne corrige rien lui-même.
