# Divergences entre dictée et mise au propre (analyse du 25/11/2026)

Source : historiques locaux des deux profils (mode Simple, cran Léger, modèle chocolatine-3b local). Mesure par comparaison mot à mot de `rawText` et `finalText`, sur les dictées passées par le modèle (`outcome: model`). Les corpus (texte personnel) restent hors du dépôt : `~/Library/Application Support/Scriibe/bancs/divergences-simple.json` (192 dictées).

## Chiffres mesurés

- Profil de l'auteur : 867 dictées Simple passées par le modèle ; **192 (22 %) ont au moins un écart réel**, une fois retirés les signes dictés (« ouvrez les guillemets »…) et les mots du dictionnaire. Écarts : 150 remplacements, 107 suppressions, 30 ajouts.
- Part des mots changés (tous écarts compris) : 11 % chez l'auteur, 7 % sur l'autre profil (158 dictées).

## Familles d'écarts

- **Remplacement de mots courts** (le plus fréquent) : « ça → cela » (11), « des → ces », « c → cela », « qui → il », « correspondant → ce produit correspond », « 5 → cinq », singulier/pluriel (« dictées → dictée »).
- **Suppression** : mots de liaison et de confort (« et », « mais », « bien », « là », « en fait », « tu »), « peux » (5), « est-il » (la question « Aïku est-il adapté ? » devient « Haïku adapté ? »).
- **Ajout** : « peux », « ne », « également », « donc », « puis » : le modèle complète ou réordonne.
- **Légitimes** (à garder) : « a → à », accents, « scrib → scriibe » (dictionnaire).

## Pistes

1. **Contrôle déterministe après le modèle** : aligner dictée et sortie mot à mot, n'accepter que les changements de ponctuation, de casse, d'accents, de traits d'union, de nombres, les mots du dictionnaire, les signes dictés et les hésitations, et rétablir le mot dicté pour tout autre remplacement, suppression ou ajout. Indépendant du modèle.
2. **Banc comparatif** sur les 192 dictées (+ un échantillon de dictées sans écart pour vérifier qu'on ne dégrade pas) : chocolatine-3b (actuel), gemma-4-E4B, Ministral-3-3B, gemma-4-12B, gemma-4-26B-A4B. Mesures : part de dictées avec écart réel, mots supprimés ou remplacés, durée par dictée. Il faut un outil qui rejoue le vrai prompt de Scriibe (aucune sous-commande ne le fait aujourd'hui).
