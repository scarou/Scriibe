# Reprise — 25/11/2026 (mise au propre : banc comparatif des modèles locaux)

Prompt à coller dans une nouvelle session lancée depuis `~/Documents/Mes projets IA/Scriibe-dev/Scriibe` (groupe « Scriibe · macOS »).

```text
⚙ Sonnet 5.5 · effort moyen — Renomme cette session « Mise au propre · B · banc des modèles, genre et nombre corrigés sans trahir les mots dictés ». Si ton dossier de travail n'est pas ~/Documents/Mes projets IA/Scriibe-dev/Scriibe, arrête-toi et dis-le.

Projet Scriibe (dictée vocale multiplateforme, Rust + Tauri 2 + Svelte 5), session sur le MAC de l'auteur. Lis CLAUDE.md, REPRISE.md, la dernière section de CHANGELOG.md, docs/DIVERGENCES_MISE_AU_PROPRE.md et la mémoire (projet-scriibe.md), puis résume l'état en 5 lignes avant d'agir.

ÉTAT : dépôt public scarou/Scriibe, branche main. Dernier commit : d6c1a4b (analyse des divergences), poussé à la clôture. Dernière version publiée : 0.13.1 ; la section « Non publié — 2026-11-23 » du CHANGELOG (commit 710a508) n'est pas encore dans une version.

FAIT À LA SESSION PRÉCÉDENTE :
- Garde-fou des dictées courtes (6 mots ou moins : aucun mot perdu), commande « entre guillemets » sur la dernière dictée collée (⌘Z puis collage), dictionnaire éditable sur place (filtre, tri, × visible) avec export et reprise en CSV (aperçu avant remplacement). 905 tests Rust et 176 tests d'interface passés, clippy propre.
- Analyse des historiques (mode Simple, modèle local chocolatine-3b, comparaison mot à mot) : 192 dictées sur 867 (22 %) ont un écart réel avec la dictée. Familles : remplacements de mots courts (« ça → cela », « des → ces », « qui → il »), suppressions de mots de liaison et de « est-il » / « peux », ajouts (« également », « donc »). Corpus de ces 192 dictées : ~/Library/Application Support/Scriibe/bancs/divergences-simple.json (hors dépôt, texte personnel ; ne jamais le copier dans le dépôt).

POINTS EN SUSPENS :
- À essayer à l'écran sur le Mac (non vu) : page Dictionnaire (modifier sur place, ×, filtre, export CSV, reprise du CSV) et « entre guillemets » sans sélection (Scriibe.app se lance par open, jamais par Bash).
- ROADMAP.md et docs/CLOTURE.md ont des modifications non commitées qui ne viennent pas de cette session : ne pas les commiter sans demander à l'auteur.
- Deux corrections du dictionnaire de l'auteur (« Est-Aiku », « Est-Aïku » → « Haïku ») sont à supprimer par lui (elles figeaient l'erreur « Haïku adapté ? »).
- Cause non élucidée : après un ⌘Z dans l'appli Claude, la lecture de la sélection ne rend rien (journal : « sélection : aucune (⌘C d'essai) »).

PROCHAINE TÂCHE (décidée par l'auteur) : **le banc de test des modèles de mise au propre**. Objectif : trouver un réglage (modèle + cran de fidélité + éventuel contrôle) qui corrige le genre et le nombre de la transcription tout en restant fidèle aux termes dictés.
Premier pas concret :
1. Écrire l'outil du banc : il doit rejouer le VRAI prompt de Scriibe (crates/core/src/cleaner.rs::system_prompt pour le mode Simple, cran Léger, et la construction de la requête dans crates/engines) sur chaque dictée brute (rawText) du corpus, avec le modèle choisi, via le serveur local (llama-server de Scriibe ou LM Studio sur localhost:1234). Aucune sous-commande ne le fait aujourd'hui : lire le code avant d'inventer le format de la requête.
2. Modèles candidats déjà sur le Mac (annoncer nom, source et taille avant tout téléchargement ; ne rien télécharger sans accord) : chocolatine-3b (actuel, ~/Library/Application Support/Scriibe/models), gemma-4-E4B, Ministral-3-3B, gemma-4-12B, gemma-4-26B-A4B (~/.lmstudio/models). Local uniquement, pas de cloud.
3. Mesures : part de dictées avec écart réel (même méthode que docs/DIVERGENCES_MISE_AU_PROPRE.md : comparaison mot à mot, hors signes dictés et mots du dictionnaire), mots supprimés ou remplacés, correction effective du genre et du nombre (fournir quelques cas témoins : accords à corriger), durée par dictée. Inclure un échantillon de dictées sans écart pour vérifier qu'on ne dégrade rien.
4. Rapport court dans docs/ (chiffres, pas de texte personnel), puis proposer : modèle retenu, et le contrôle déterministe mot à mot (n'accepter que ponctuation, casse, accents, nombres, accords de genre et de nombre, dictionnaire, signes dictés, hésitations).

RÈGLES : aperçu et accord de l'auteur avant toute Release ou push public ; cargo fmt, cargo clippy --workspace --all-targets -- -D warnings et cargo test --workspace sans erreur avant tout commit ; le prompt envoyé au modèle reste identique d'une dictée à l'autre jusqu'à la dictée (mémoire du serveur) ; ne jamais modifier scarou/scribe ni lire de .env ou de clé ; ne jamais écrire le nom ni l'adresse de l'auteur dans le dépôt ; ne jamais annoncer un chiffre avant de l'avoir mesuré ; ne décharger dans LM Studio que ce que tu y as chargé ; contexte : à 25 % prévenir, à 40 % s'arrêter à un point stable.
```
