# Reprise — 06/11/2026 (fin de S17)

Prompt à coller dans une nouvelle session lancée depuis `~/Documents/Mes projets IA/Scriibe-dev/Scriibe` (groupe « Scriibe · macOS »).

```text
Projet Scriibe (dictée vocale multiplateforme, Rust + Tauri 2 + Svelte 5). Session sur le MAC de Christophe (Mac Studio), dossier ~/Documents/Mes projets IA/Scriibe-dev/Scriibe. Lis CLAUDE.md, REPRISE.md, la dernière section de CHANGELOG.md (S17), ROADMAP.md (« Prochaines étapes ») et la mémoire, puis résume l'état en 5 lignes avant de commencer.

ÉTAT : dépôt public scarou/Scriibe, branche main, dernier commit poussé à la clôture de S17 (voir `git log --oneline -3`). Release v0.2.4 publiée le 06/11/2026 (Mac, Windows, Linux, moteurs Linux), Cargo.toml en 0.2.4 ; mises à jour intégrées essayées avec succès par Christophe sur Mac, Windows et Linux. `linux-pc` est fusionné dans main (dab84e7) ; la branche reste. Tests mesurés : 335 Rust (0 échec), 83 d'interface, 399 cas de référence ; CI verte sur les trois systèmes après la fusion.

FAIT EN S17 (05-06/11) :
- Espace entre deux dictées : règle codée (crates/core/src/spacing.rs, mémoire du dernier collage, 60 s, même appli), livrée dans la 0.2.4 ; puis ÉLARGIE le 06/11 (espace même sans point final, car Whisper `ggml-small` n'écrit pas de point ; `large-v3-turbo-q8_0` oui) : commits 21dbdca et 8841dd6, NON PUBLIÉE. Détails dans docs/ESPACEMENT.md (Constat, Règle, Tests, Essai réel, Relecture, Règle élargie).
- Relecture de l'Aide par Christophe : Aide et cartes à pleine largeur, bouton « ? » sur toutes les cartes (toujours à droite), question « pastille bloquée » ajoutée, toutes les cartes repliables (dont jours de l'Historique et cadres de modes), guide Mac refait pour débutants (docs/INSTALLATION_MACOS.md) et séparé du guide connaisseurs (docs/INSTALLATION_MACOS_EXPERT.md), versions anglaises faites.
- Logo, couleur et textes du système dès l'ouverture (`status.os`, `guessOs`) : la page restait bleue (Windows) tant que macOS attendait le trousseau.
- Constats : demandes de trousseau en boucle = Entrée au lieu de « Toujours autoriser » ; Chrome perd parfois une dictée (⌘V), noté, non traité ; 500 passager de GitHub au téléchargement de la mise à jour Windows.

POINTS EN SUSPENS :
- Christophe essaie `ggml-large-v3-turbo-q8_0.bin` sur Windows et Linux ; l'essai de la règle d'espacement élargie sous Windows et Linux se fera avec la prochaine version.
- Rien n'est publié depuis la 0.2.4 : règle élargie, cartes repliables, logo/couleur et guide Mac attendent la prochaine version (ROADMAP n°14, numéro à décider).
- Le guide Mac n'a jamais été suivi par un débutant sur un Mac neuf (Homebrew non essayé).
- Relecture R18 (agent distinct) non faite pour le dernier lot d'interface (cartes repliables, status.os) : svelte-check et 83 tests verts seulement.
- État de la machine : l'application de développement ~/Applications/Scriibe.app tourne (elle porte les derniers changements) ; aucun serveur de développement lancé par la session.

PROCHAINE TÂCHE (ROADMAP n°13, suite) : guide d'installation pas à pas pour débutants sous WINDOWS, sur le modèle du guide Mac (docs/INSTALLATION_MACOS.md : étapes numérotées, « C'est réussi quand… » à chaque étape, tableau « Si ça ne marche pas »), en séparant la partie connaisseurs dans docs/INSTALLATION_WINDOWS_EXPERT.md ; versions anglaises identiques en structure (.en.md) ; liens des README ; puis Linux de la même façon. Premier pas : lire docs/INSTALLATION_WINDOWS.md et ESSAI_WINDOWS.md, lister ce qui est expert (compilation, moteurs en ligne, mise à jour à la main), proposer le plan à Christophe avant d'écrire. Autre option si Christophe préfère : publier la version suivante (n°14) après son essai de large-v3-turbo.

RÈGLES : aperçu et accord de Christophe avant toute Release ou tout push public ; `cargo fmt`, `cargo clippy -- -D warnings` et `cargo test --workspace` sans erreur avant tout commit ; ne jamais modifier scarou/scribe ni lire de .env ou de clé ; ne jamais lire ni modifier le trousseau ; modifier un .md par script : ancrer sur « \n## » et relire `grep -n "^## "` avant de commiter (S17 : deux sections perdues) ; pour tout doc français d'installation, reporter dans la version anglaise (et inversement) ; une étape à la fois pour Christophe sur son PC ; lancer Scriibe.app par `open`, jamais le binaire nu ; ne jamais annoncer un chiffre avant de l'avoir mesuré.
```
