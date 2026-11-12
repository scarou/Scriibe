# Reprise — 12/11/2026 (clôture S25, gestion des modes, Mac)

Prompt à coller dans une nouvelle session lancée depuis `~/Documents/Mes projets IA/Scriibe-dev/Scriibe` (groupe « Scriibe · Windows », celui des sessions du tri du 12/11).

```text
Projet Scriibe (dictée vocale multiplateforme, Rust + Tauri 2 + Svelte 5). Session sur le MAC de l'auteur, dossier ~/Documents/Mes projets IA/Scriibe-dev/Scriibe. Lis CLAUDE.md, REPRISE.md, la dernière section de CHANGELOG.md (S25) et la mémoire (projet-scriibe), puis résume l'état en 5 lignes avant d'agir.

ÉTAT : dépôt public scarou/Scriibe, branche main, dernier commit 1f2b23d (CI verte sur Linux, macOS, Windows et scriibe.exe). Release v0.2.4 publiée le 06/11/2026 ; Cargo.toml en 0.3.0 ; RIEN n'est publié depuis (ROADMAP n°14 à terminer). Tri du 12/11 en 8 chantiers (~/Library/Mobile Documents/com~apple~CloudDocs/IA-OS/1 - Projets/scriibe/decoupages/tri-2026-11-12-scriibe.md) : chantiers 1 à 5 faits ; 6 découpé ; 7 et 8 à découper.

FAIT EN S25 (chantier 5, gestion des modes, sans version) :
- Réglages → Modes : une case « Afficher dans le menu de l'icône » par mode (champ hiddenModes) ; décocher retire le mode du menu, même le mode actif ; au moins un mode reste affiché ; gardé après redémarrage.
- Export d'un mode personnel (.scriibe.json, enveloppe scriibe-export v1, avertissement, boîte « Enregistrer sous » sous macOS et Windows, dossier Documents sous Linux) et import contrôlé (1 Mo, 50 modes, 20 000 caractères par texte, aperçu, nom pris : ajout numéroté ou remplacement au choix).
- Relu par un agent distinct (aucun bloquant ; 3 importants et mineurs corrigés), essayé à la main sur le Mac. Détail : docs/GESTION_MODES.md ; écarts : docs/ECARTS.md.
- Carnets du vault à jour (deux lignes du 11/11 en Faite ; « à vérifier sur le PC » ajoutée aux carnets Windows et Linux).

POINTS EN SUSPENS :
- Le settings.json de l'auteur garde des modes masqués laissés par l'essai (Simple, Sophrologie, Soutenu, un mode personnel) : lui dire de recocher les cases dans Réglages → Modes (il le fait lui-même).
- À vérifier sur PC : boîtes « Ouvrir » et « Enregistrer sous », champ de chemin Linux, cases du menu de l'icône (carnets Windows et Linux ; compilation Windows possible par pilote-pc, la CI Windows est verte).
- Non essayés à l'écran : l'ajout numéroté d'un import (le remplacement l'a été).
- Publication de la 0.3.0 : essais ggml-large-v3-turbo sur Windows et Linux, relecture de l'Aide, CI complète, aperçu et « oui » de l'auteur.
- L'application Scriibe.app installée sur le Mac est celle du dernier essai (modes de S25) ; rien d'autre ne tourne.

PROCHAINE TÂCHE : chantier 6, cadrage des commandes vocales étendues (reformuler la sélection, macros, ouvrir une application, recherche internet), note seulement, sans code. L'auteur dit « tâche 0 » ; lis la consigne de la tâche 0 dans ~/Library/Mobile Documents/com~apple~CloudDocs/IA-OS/1 - Projets/scriibe/decoupages/decoupage-2026-11-12-c6-cadrage-commandes.md et suis-la telle quelle (⚙ Opus · Moyen : dis-le si tu ne tournes pas sur Opus). Premier pas : git status propre, chantier 5 clos et poussé, mise à jour de la ligne « découpage en cours » du CLAUDE.md, puis sous-agent de recherche.

RÈGLES : aperçu et accord de l'auteur avant toute Release ou tout push public ; cargo fmt, cargo clippy --all-targets -- -D warnings et cargo test --workspace sans erreur avant tout commit ; ne jamais modifier scarou/scribe ni lire de .env ou de clé ; ne jamais écrire le nom ni l'adresse de l'auteur dans le dépôt ; Scriibe.app se lance par open, jamais par Bash ; peu de sollicitations, une étape à la fois pour les essais de l'auteur ; ne jamais annoncer un chiffre avant de l'avoir mesuré.
```
