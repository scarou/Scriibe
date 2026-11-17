# Reprise — 17/11/2026 (version 0.10.0 publiée : refonte de l'interface faite, découpage soldé en 14 tâches)

Prompt à coller dans une nouvelle session lancée depuis `~/Documents/Mes projets IA/Scriibe-dev/Scriibe` (groupe « Scriibe · Windows »). Plus de découpage en cours : l'auteur dit ce qu'il veut faire, ou « /carnet » pour reprendre les lignes en attente (23 au 17/11).

```text
Projet Scriibe (dictée vocale multiplateforme, Rust + Tauri 2 + Svelte 5). Session sur le MAC de l'auteur, dossier ~/Documents/Mes projets IA/Scriibe-dev/Scriibe. Lis CLAUDE.md, REPRISE.md et la dernière section de CHANGELOG.md (0.10.0), puis résume l'état en 5 lignes avant d'agir.

ÉTAT : dépôt public scarou/Scriibe, branche main. Release v0.10.0 publiée le 17/11/2026 (lot 2 de la refonte de l'interface : carte « Sauvegarder et transférer », textes harmonisés, recherche dans l'Aide, « À propos » ; commit d1ed332, CI des trois systèmes verte, zips Mac, Windows et Linux sur la Release ; essai Mac, Linux et Windows : docs/REFONTE_INTERFACE.md § Essai lot 2), après la v0.9.0 (lot 1 : menu en 4 groupes, pages par thème ; commit 3c7c95a), la v0.8.0 (reformulation de la sélection) et les précédentes. Cargo.toml en 0.10.0. Le découpage « Refonte de l'interface par thématiques » est soldé (14 tâches) ; le chantier 4 (profils) et le chantier 6 (commandes vocales étendues) sont faits.

POINTS EN SUSPENS (carnet ~/Library/Mobile Documents/com~apple~CloudDocs/IA-OS/1 - Projets/scriibe/carnet-idees-scriibe.md, lignes `[ ]`) :
- Relecture de la 0.10.0, mineurs non corrigés : « Exporter… » muet quand il n'y a rien à exporter ; minuteur de 4 s des demandes de transfert ; pas d'« Annuler » à la saisie du chemin (Linux) ; « + » en trop de la touche de traduction sous GNOME. Textes de la carte d'export qui parlent de Linux sur tous les systèmes.
- Défaut ancien : ce que la fenêtre garde dans son navigateur (case du banc, cartes repliées) est perdu à chaque lancement (port local différent à chaque démarrage).
- À vérifier : Mac (message « Tout est en place » de l'assistant), PC Windows et Linux (carnets scriibe-windows et scriibe-linux : anglais à l'écran, autres imports et exports de parties, surlignage de l'Aide dans une fenêtre réelle).
- Des commits de documentation suivent la v0.10.0 : ligne « Publier la version suivante » au carnet.
- Restes d'essai (copies de données, hors données réelles) : Mac `~/Library/Application Support/Scriibe-essai-lot2` et `~/Applications/Scriibe-essai-lot2.app` ; Linux `~/essai-lot2` ; Windows `C:\essai-refonte-lot2`. Aucune Scriibe d'essai ne tourne.

RÈGLES : aperçu et accord de l'auteur avant toute Release ou tout push public ; cargo fmt, cargo clippy --all-targets -- -D warnings et cargo test --workspace sans erreur avant tout commit ; ne jamais modifier scarou/scribe ni lire de .env ou de clé ; ne jamais écrire le nom ni l'adresse de l'auteur dans le dépôt ; Scriibe.app se lance par open, jamais par Bash (copie d'essai : SCRIIBE_INSTALL=… et open -n --env SCRIIBE_DATA=…) ; peu de sollicitations, une étape à la fois pour les essais de l'auteur ; ne jamais annoncer un chiffre avant de l'avoir mesuré.
```
