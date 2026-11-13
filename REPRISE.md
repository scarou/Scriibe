# Reprise — 13/11/2026 (version 0.4.0 publiée ; chantier 4, lot 1 livré, lot 2 à faire)

Prompt à coller dans une nouvelle session lancée depuis `~/Documents/Mes projets IA/Scriibe-dev/Scriibe` (groupe « Scriibe · Windows »). L'auteur dit seulement **« tâche 6 »** (ou colle la consigne entière) : lire alors la consigne de la tâche 6 dans le découpage et la suivre telle quelle.

```text
Projet Scriibe (dictée vocale multiplateforme, Rust + Tauri 2 + Svelte 5). Session sur le MAC de l'auteur, dossier ~/Documents/Mes projets IA/Scriibe-dev/Scriibe. Lis CLAUDE.md, REPRISE.md, la dernière section de CHANGELOG.md (C4 lot 1) et la mémoire (projet-scriibe), puis résume l'état en 5 lignes avant d'agir.

ÉTAT : dépôt public scarou/Scriibe, branche main. Release v0.4.0 publiée le 13/11/2026 (chantiers 1, 2, 3, 5, 7, 8 du tri du 12/11). Cargo.toml en 0.4.0. Depuis : chantier 4 (profils et dictionnaires), lot 1 livré sans version (part dans la 0.5.0 avec le lot 2 et le chantier 6).

CHANTIER 4 : découpage ~/Library/Mobile Documents/com~apple~CloudDocs/IA-OS/1 - Projets/scriibe/decoupages/decoupage-2026-11-12-c4-implementation-profils.md (déjà cité dans CLAUDE.md). Tâches 0 à 5 faites (lot 1) : étape 1 fondations (schemaVersion, fichier illisible ou plus récent refusé avec bandeau, copie .bak, fichiers temporaires uniques, corrections dédoublonnées), étape 2 (import/export du dictionnaire), étape 3 (import/export des commandes perso), essai sur Mac, Linux et Windows, relecture par un agent distinct (importants corrigés). Détail, choix et tests : docs/PROFILS.md ; cadrage : docs/CADRAGE_PROFILS.md.
PROCHAINE TÂCHE : la tâche 6 (étape 4a : vue des réglages, sans changement visible), nouvelle session. Lot 2 = tâches 6 à 12 (4a, 4b migration, relecture de la migration, 4c utilisateurs visibles, étape 5 utilisateur entier, essai, livraison), puis tâche 13 (clore l'idée).

POINTS EN SUSPENS :
- Notés à la relecture du lot 1, non corrigés (voir docs/PROFILS.md § Relecture) : patch_settings change llama avant l'écriture ; fichier vide après coupure de courant (pas de fsync) ; copie .bak sur erreur de lecture passagère ; menu de l'icône muet si fichier refusé ; copies .bak sans plafond. À mettre au carnet.
- Non vu à l'écran : le bandeau « fichier refusé » sur Linux et Windows ; les corrections de la relecture ; un import avec conflit.
- Le PC Linux fait tourner une ancienne Scriibe (programme remplacé par la compilation d'essai) ; à relancer à l'occasion. docs/GESTION_MODES.md ligne 105 dit encore que les étapes 1 et 2 ne sont pas faites.
- Deux fichiers d'export d'essai restent dans Téléchargements sur le Mac (à supprimer par l'auteur).
- Chantier 6 (commandes vocales étendues) cadré dans docs/CADRAGE_COMMANDES.md, à découper pour l'implémentation.

RÈGLES : aperçu et accord de l'auteur avant toute Release ou tout push public ; cargo fmt, cargo clippy --all-targets -- -D warnings et cargo test --workspace sans erreur avant tout commit ; ne jamais modifier scarou/scribe ni lire de .env ou de clé ; ne jamais écrire le nom ni l'adresse de l'auteur dans le dépôt ; Scriibe.app se lance par open, jamais par Bash (copie d'essai : SCRIIBE_INSTALL=… et open -n --env SCRIIBE_DATA=…) ; peu de sollicitations, une étape à la fois pour les essais de l'auteur ; ne jamais annoncer un chiffre avant de l'avoir mesuré.
```
