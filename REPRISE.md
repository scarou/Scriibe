# Reprise — 13/11/2026 (chantier 8 clos, logo animé ; tri du 12/11 soldé)

Prompt à coller dans une nouvelle session lancée depuis `~/Documents/Mes projets IA/Scriibe-dev/Scriibe` (groupe « Scriibe · Windows », celui des sessions du tri du 12/11).

```text
Projet Scriibe (dictée vocale multiplateforme, Rust + Tauri 2 + Svelte 5). Session sur le MAC de l'auteur, dossier ~/Documents/Mes projets IA/Scriibe-dev/Scriibe. Lis CLAUDE.md, REPRISE.md, la dernière section de CHANGELOG.md (C6 et C7) et la mémoire (projet-scriibe), puis résume l'état en 5 lignes avant d'agir.

ÉTAT : dépôt public scarou/Scriibe, branche main, derniers commits : voir `git log` (livraison du chantier 7 poussée le 13/11, puis chantier 8 (logo animé, case « Logo animé », `docs/LOGO_ANIME.md`) poussé le 13/11, non publié ; CI à vérifier avec tools/attendre_ci.sh). Release v0.2.4 publiée le 06/11/2026 ; Cargo.toml en 0.3.0 ; RIEN n'est publié depuis (ROADMAP n°14 à terminer). Tri du 12/11 en 8 chantiers (~/Library/Mobile Documents/com~apple~CloudDocs/IA-OS/1 - Projets/scriibe/decoupages/tri-2026-11-12-scriibe.md) : chantiers 1 à 5 faits ; 6 cadré (docs/CADRAGE_COMMANDES.md, note seulement) ; 7 fait (non publié) ; 8 à découper ou à finir (voir le découpage).

FAIT EN C7 (chantier 7, système, sans version ; détail dans docs/SYSTEME.md) :
- Réglages → Système (nouvel onglet) : lancer au démarrage (Mac LaunchAgent, Windows clé Run, Linux autostart XDG ; démarrage discret), sauvegarde complète (.scriibe.json kind backup, relue) et restauration par redémarrage, désinstallation (liste exacte, tout à la Corbeille, clés effacées, assistant après la sortie, commande scriibe desinstaller [--liste]).
- Essayé de bout en bout sur le Mac, le PC Windows et le PC Linux (code transmis aux PC par paquet git, sans rien publier) ; défauts trouvés et corrigés ; relu par un agent distinct (aucun bloquant, importants corrigés). Non essayés : gros volumes à la Corbeille sous Windows et Mac, dictée après réinstallation sous Linux, pastille en démarrage discret sous Windows (carnet).

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

PROCHAINE TÂCHE : le tri du 12/11 est soldé (8 chantiers faits ou cadrés : 4 et 6 restent à découper pour l'implémentation). Reste à décider avec l'auteur : publier la 0.3.0 (ROADMAP n°14), vérifier le logo animé sur les PC (WebView2, WebKitGTK ; lignes dans les carnets Windows et Linux), ou découper les chantiers 4 et 6.

RÈGLES : aperçu et accord de l'auteur avant toute Release ou tout push public ; cargo fmt, cargo clippy --all-targets -- -D warnings et cargo test --workspace sans erreur avant tout commit ; ne jamais modifier scarou/scribe ni lire de .env ou de clé ; ne jamais écrire le nom ni l'adresse de l'auteur dans le dépôt ; Scriibe.app se lance par open, jamais par Bash ; peu de sollicitations, une étape à la fois pour les essais de l'auteur ; ne jamais annoncer un chiffre avant de l'avoir mesuré.
```
