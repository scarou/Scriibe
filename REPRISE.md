# Reprise — 10/11/2026 (clôture S18-S20, Mac)

Prompt à coller dans une nouvelle session lancée depuis `~/Documents/Mes projets IA/Scriibe-dev/Scriibe` (groupe « Scriibe · macOS »).

```text
Projet Scriibe (dictée vocale multiplateforme, Rust + Tauri 2 + Svelte 5). Session sur le MAC de Christophe (Mac Studio), dossier ~/Documents/Mes projets IA/Scriibe-dev/Scriibe. Lis CLAUDE.md, REPRISE.md, la dernière section de CHANGELOG.md (S20) et la mémoire, puis résume l'état en 5 lignes avant de commencer.

ÉTAT : dépôt public scarou/Scriibe, branche main (voir `git log --oneline -5`). Release v0.2.4 publiée le 06/11/2026 ; Cargo.toml en 0.2.4 ; RIEN n'est publié depuis (ROADMAP n°14 : commandes vocales sur sélection, signes dictés, crans de fidélité, mode Apaisé (CNV), espace après chaque dictée, Chrome, guides d'installation, etc.). Numéro de la prochaine version à décider (0.2.5 ou 0.3.0).

FAIT EN S19-S20 (09-10/11) :
- Mode Simple : garde-fou à 75 % de mots gardés sur les dictées de 8 mots ou plus (mots de commande dictés à part) ; crans de fidélité Littéral, Léger, Reformulé, Libre (Simple et modes personnels au choix) ; mode livré Apaisé (CNV) au cran Libre ; Aide complétée ; phrases d'essai d'Apaisé en plus.
- Règle d'espacement remplacée (e650f24) : chaque dictée est suivie d'une espace ; essayée sur Mac (TextEdit, Notes, saut de ligne, sélection), Linux et Windows.
- Chrome : Windows 11/11 et Linux 11/11 collés sans correction (10/11) ; sur Mac les collages se perdaient (1 sur 9) ; le texte est maintenant tapé au clavier dans Chrome et ses dérivés (10 sur 10 dans Google et YouTube) ; ⌘C d'essai en vraie frappe ; détail du collage dans le journal. Voir docs/PIEGES.md.

POINTS EN SUSPENS :
- Essais à la main restants : crans Littéral et Libre ; Apaisé (CNV) sur Gemma 4 E4B ; Chrome : Google Docs, commandes vocales sur sélection (« mettre entre parenthèses » : un seul ⌘Z ?) ; Safari et Firefox (collage inchangé, sur les trois systèmes) ; (Chrome sous Windows et sous Linux mesurés le 10/11 par des sessions séparées : 11 sur 11 chacun, rien à corriger.)
- Une dictée d'au plus trois mots saute le modèle : pas de majuscule ni de point si Whisper n'en met pas (règle de Scribe). Amélioration possible, non décidée.
- ROADMAP n°15 (`/api/status` attend le trousseau) : non commencé, prudence (trousseau).
- Le mode personnel « Apaisé (CNV) » créé pour l'essai du 07/11 fait doublon avec le mode livré : à supprimer par Christophe.
- Après chaque compilation, macOS redemande 3 fois l'accès au trousseau : « Toujours autoriser » (pas Entrée).

PROCHAINE TÂCHE : décider avec Christophe de la publication d'une version (numéro, notes de version, statut d'essai par système), après relecture de l'Aide et CI complète ; aperçu et accord avant toute Release ou tout push public.

RÈGLES : aperçu et accord de Christophe avant toute Release ou tout push public ; `cargo fmt`, `cargo clippy --all-targets -- -D warnings` et `cargo test --workspace` sans erreur avant tout commit ; ne jamais modifier scarou/scribe ni lire de .env ou de clé ; ne jamais lire ni modifier le trousseau ; un cas de fixtures ne se modifie jamais pour faire passer un test ; ne jamais annoncer un chiffre avant de l'avoir mesuré ; lui seul tient la touche et parle ; une étape à la fois pour lui.
```
