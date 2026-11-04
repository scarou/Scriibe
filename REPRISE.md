# Reprise — 04/11/2026 (fin de S15)

Prompt à coller dans une nouvelle session lancée depuis `~/Documents/Mes projets IA/Scriibe-dev/Scriibe` (groupe « Scriibe-Dev MacOS »).

```text
Projet Scriibe (dictée vocale multiplateforme, Rust + Tauri 2 + Svelte 5). Session sur le MAC de Christophe (Mac Studio), dossier ~/Documents/Mes projets IA/Scriibe-dev/Scriibe. Lis CLAUDE.md, REPRISE.md, la dernière section de CHANGELOG.md (S15) et la mémoire (projet-scriibe.md), puis résume l'état en 5 lignes avant de commencer.

ÉTAT : dépôt public scarou/Scriibe, branche main, Release v0.2.2 publiée (zip Mac + Windows), Cargo.toml en 0.2.2. Dernier commit de code : 19742c9 (pause des vidéos de Chrome par JavaScript, vrai ⌘V pour le collage), suivi du commit de clôture S15. 313 tests Rust et 82 tests d'interface mesurés, 0 échec.

FAIT EN S15 (détail dans CHANGELOG.md) : releases 0.2.1 et 0.2.2 ; mise à jour intégrée du Mac écrite (jamais essayée) ; version Linux menée par la session « Version Linux » (workflow release-linux.yml, guides, moteurs téléchargés) ; double installation de Scriibe.app réglée (seule /Applications/Scriibe.app reste) ; une vidéo de Chrome arrêtée ne repart plus quand on dicte (pause par JavaScript, il faut avoir coché « Autoriser JavaScript provenant d'Apple Events » dans Chrome) ; collage dans le champ de recherche Google corrigé (⌘ pressé puis relâché, 5 essais sur 5).

EN SUSPENS : la mise à jour intégrée du Mac n'a jamais été essayée sur le vrai Mac ; le ⌘V en vraie frappe n'a été essayé que dans Chrome (pas encore dans Notes, Claude, etc.) ; l'Aide mise à jour n'a pas été relue par Christophe ; Scriibe n'est pas dans gestion-des-projets/projets.json ; la session « Version Linux » est en attente d'une phrase de Christophe sur le PC Linux.

PROCHAINE TÂCHE : publier la v0.2.3. Premier pas : (la CI complète du commit 19742c9 est verte sur les trois systèmes) montrer à Christophe l'aperçu de la Release (ce qu'elle contient, version, zip Mac/Windows/Linux) et attendre son « oui » ; puis Cargo.toml en 0.2.3, cargo build --release, commit, push, tools/publier_macos.sh 0.2.3 (aperçu puis « oui »), attendre les workflows release-windows.yml et release-linux.yml. Ensuite essayer pour la première fois la mise à jour intégrée du Mac : Scriibe.app (0.2.2) → onglet « Mise à jour » → « Chercher une nouvelle version » → « Installer et redémarrer » ; lire ~/Library/Application Support/Scriibe/scriibe.log soi-même.

RÈGLES : lire CLAUDE.md. Christophe ne comprend pas « fichier caché », les chemins abrégés ni « … » : une étape à la fois, commandes en blocs copiables, une seule question courte par message. Aperçu avant toute action publique (Release, push) et attendre son accord. Les autorisations macOS vont à Scriibe.app lancée par open, jamais à un programme lancé par Bash ; un script osascript lancé par Bash se bloque sur des demandes invisibles. Compiler : cd ui && npm ci && npm run build, cargo build --release, puis SCRIIBE_INSTALL=/Applications/Scriibe.app tools/faire_app_macos.sh --lancer (3 demandes de trousseau à chaque compilation : ne compiler que si utile). Ne jamais lancer pkill -f ni pgrep -f avec un motif présent dans la commande. Ne jamais annoncer un chiffre avant de l'avoir mesuré. Commits à l'identité Christophe Campain, message court en français.
```
