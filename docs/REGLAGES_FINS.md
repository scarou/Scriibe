# Réglages fins (chantier 6)

## Veille et couleurs (tâche 0, 18/11/2026)

**Durées de veille** : Whisper et llama.cpp proposent les mêmes huit choix : 1, 2, 5, 10, 15, 30, 60 min et « Jamais » (`0`, le moteur reste en mémoire tant que Scriibe tourne). Liste unique : `IDLE_CHOICES` (`crates/core/src/settings.rs`) et `ui/src/lib/idle.ts`.

- Anciens réglages lus tels quels : Whisper 0, 5, 10, 30, 60 ; llama.cpp 30, 120 (2 h), 480 (8 h), 0. Un ancien réglage hors liste (120, 480) reste dans le menu de llama.cpp, avant « Jamais », et s'affiche « 2 h » / « 8 h ». Défauts inchangés (Whisper 10, llama.cpp 30).
- Une valeur de Whisper écrite à la main hors liste (14) se ramène à la plus proche des durées de Scribe (cas `settings-020`, non modifié) ; llama.cpp la garde (plafond 24 h).
- « Jamais » : `idle_limit(0)` = `None`, le moteur n'est jamais libéré (tests `llama_with_never_is_not_released`, `no_idle_limit_means_never`, `idle_limit`).

**Couleurs de l'indicateur** : 12 couleurs, sans couleur libre : les cinq de Scribe (Scriibe, gris, rouge, bleu, noir) et sept ajouts (orange, vert, turquoise, violet, rose, indigo, marron). Choix : teintes soutenues, dégradé clair → foncé comme les autres, pour que le texte blanc de la pastille reste lisible sur fond clair comme sombre ; pas de jaune (illisible avec du blanc). Une couleur déjà choisie est relue telle quelle ; une couleur inconnue redonne celle de Scriibe.

**Tests** : `settings_scriibe.rs` (`idle_choices_…`, `old_idle_settings_…`, `old_and_new_indicator_colors_…`), `engines/tests/server.rs`, `app.rs`, `idle.test.ts`, `appearance.test.ts`. Écart noté dans `docs/ECARTS.md`. Cartes mesurées : `cargo test --workspace`, clippy `-D warnings`, `vitest` (172) verts.

**Pas fait ici** : aperçu à l'écran des sept couleurs (essai visuel sur machine) ; Aide (`Help.svelte`, `HelpEn.svelte`) mise à jour pour les durées et les douze couleurs.

## Clé Linux (tâche 1, 18/11/2026)

Sous Linux, la page Profil (Réglages → Comptes en ligne) range les clés Deepgram et OpenRouter dans le **trousseau du bureau** (Secret Service : Trousseau de GNOME, KWallet), comme le Trousseau du Mac et le Gestionnaire d'identifiants de Windows. Code : `crates/platform/src/secret_service.rs` ; lecture en premier dans la chaîne des sources, l'environnement (`OPENROUTER_API_KEY`, `DEEPGRAM_API_KEY`) ensuite, comme sur les deux autres systèmes. Entrées : service « Scriibe », compte « openrouter » / « deepgram » (mêmes noms que sur Mac).

**Choix technique : l'outil `secret-tool` (paquet `libsecret-tools`) plutôt qu'un module Rust.** Pourquoi : aucune dépendance de compilation de plus (un module Secret Service pur Rust tire un lot de dépendances à auditer pour un secret, et un lien C demanderait `libdbus-dev` sur chaque machine de construction) ; le code se teste en entier sur le Mac avec un trousseau factice. Prix : le paquet doit être installé ; s'il manque, le message le dit (« installe le paquet libsecret-tools, ou garde la variable d'environnement … »). Si l'essai sur le PC Linux (tâche 3) montre que le paquet manque souvent, le remplacer par un module Rust ne change que ce fichier (le trait `SecretTool`).

**Garde-fous (un test chacun, factice)** :
- la clé passe par l'**entrée standard** de `secret-tool`, jamais par ses arguments (visibles dans `ps`) ; sans retour à la ligne (il serait gardé avec la clé) ;
- la sortie d'erreur de l'outil part à `/dev/null` et n'est **jamais lue** : les messages d'erreur ne portent que le code de sortie ; le journal ne note que « clés: openrouter enregistrée dans le trousseau du bureau » ;
- sans trousseau (outil absent, bus absent, trousseau fermé et sans réponse en 60 s) : **rien n'est écrit** nulle part, message qui nomme la variable d'environnement ; pas de repli en clair ;
- clé vide, de plus de 4096 signes ou avec un signe de commande : refusée avant tout appel ;
- une lecture sans résultat n'est pas refaite avant 30 s (une demande de déverrouillage ne revient pas à chaque rafraîchissement de la page) ; une clé enregistrée apparaît tout de suite ;
- désinstallation : les clés sont aussi effacées du trousseau (`secret-tool clear`, après vérification qu'elles y sont) ; texte de l'assistant, de `scriibe desinstaller` et de la fenêtre mis à jour (`plan.keys` = `secretservice`).

**Tests** : `secret_service::tests` (6 : clé relue après « redémarrage », entrée standard seule, sans trousseau × 3 modes avec les deux variables, clé refusée, lecture non répétée, effacement). `cargo test --workspace`, clippy, vitest (172) verts sur le Mac.

**Pas vérifié ici (preuve à faire sur le PC Linux, tâche 3)** : compilation du code propre à Linux (`linux/mod.rs`, branche Linux de `delete_credentials` et de `save_secret` : aucune cible Linux sur le Mac) ; comportement réel de `secret-tool` (déverrouillage, trousseau fermé, session sans bureau) ; aucune trace de la clé dans les journaux réels.
