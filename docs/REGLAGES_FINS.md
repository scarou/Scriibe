# Réglages fins (chantier 6)

## Veille et couleurs (tâche 0, 18/11/2026)

**Durées de veille** : Whisper et llama.cpp proposent les mêmes huit choix : 1, 2, 5, 10, 15, 30, 60 min et « Jamais » (`0`, le moteur reste en mémoire tant que Scriibe tourne). Liste unique : `IDLE_CHOICES` (`crates/core/src/settings.rs`) et `ui/src/lib/idle.ts`.

- Anciens réglages lus tels quels : Whisper 0, 5, 10, 30, 60 ; llama.cpp 30, 120 (2 h), 480 (8 h), 0. Un ancien réglage hors liste (120, 480) reste dans le menu de llama.cpp, avant « Jamais », et s'affiche « 2 h » / « 8 h ». Défauts inchangés (Whisper 10, llama.cpp 30).
- Une valeur de Whisper écrite à la main hors liste (14) se ramène à la plus proche des durées de Scribe (cas `settings-020`, non modifié) ; llama.cpp la garde (plafond 24 h).
- « Jamais » : `idle_limit(0)` = `None`, le moteur n'est jamais libéré (tests `llama_with_never_is_not_released`, `no_idle_limit_means_never`, `idle_limit`).

**Couleurs de l'indicateur** : 12 couleurs, sans couleur libre : les cinq de Scribe (Scriibe, gris, rouge, bleu, noir) et sept ajouts (orange, vert, turquoise, violet, rose, indigo, marron). Choix : teintes soutenues, dégradé clair → foncé comme les autres, contraste d'au moins 4,5 : 1 avec le texte blanc de la pastille aux deux bouts du dégradé (test `appearance.test.ts`, teintes corrigées à la relecture) ; pas de jaune (illisible avec du blanc). Une couleur déjà choisie est relue telle quelle ; une couleur inconnue redonne celle de Scriibe.

**Tests** : `settings_scriibe.rs` (`idle_choices_…`, `old_idle_settings_…`, `old_and_new_indicator_colors_…`), `engines/tests/server.rs`, `app.rs`, `idle.test.ts`, `appearance.test.ts`. Écart noté dans `docs/ECARTS.md`. Cartes mesurées : `cargo test --workspace`, clippy `-D warnings`, `vitest` (172) verts.

**Pas fait ici** : aperçu à l'écran des sept couleurs (essai visuel sur machine) (l'Aide, elle, est à jour pour les durées et les douze couleurs).

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

**Tests** : `secret_service::tests` (10 après la relecture : clé relue après « redémarrage », entrée standard seule, sans trousseau × 3 modes avec les deux variables, clé refusée, lecture non répétée, effacement, deux courses lecture/écriture, deux essais avec un faux `secret-tool` réel). `cargo test --workspace`, clippy, vitest (172) verts sur le Mac.

**Pas vérifié ici (preuve à faire sur le PC Linux, tâche 3)** : compilation du code propre à Linux (`linux/mod.rs`, branche Linux de `delete_credentials` et de `save_secret` : aucune cible Linux sur le Mac) ; comportement réel de `secret-tool` (déverrouillage, trousseau fermé, session sans bureau) ; aucune trace de la clé dans les journaux réels.

## Relecture (tâche 2, 18/11/2026)

Relecteur : sous-agent Sonnet (effort élevé), qui n'a pas écrit le code, sur `27a9798` et `fc49331`. Résultat : **0 bloquant, 3 importants, 8 mineurs**. Aucune fuite de clé, aucun repli en clair, aucun interblocage de tube, anciens réglages bien relus. Détail complet du relecteur : fichier de session `relecture-c6.md` (scratchpad, non conservé).

**Importants, corrigés (un test chacun)** :
1. *Trousseau fermé : lecture bloquante pendant la dictée, clé « absente » 30 s après.* Un trousseau qui n'a pas répondu est un état à part (`Unreachable`), redemandé au bout de 5 min seulement (et non plus 30 s, pas de fenêtre de déverrouillage en pleine dictée) ; première lecture au lancement dans un fil à part, hors du chemin d'une dictée (`app.rs`, Linux seulement) ; une clé enregistrée par la page apparaît tout de suite. Test `a_failed_reading_is_not_repeated_…`.
2. *Contraste du texte blanc* : cinq couleurs (orange, vert, turquoise, rose, marron) étaient sous 4,5 : 1 à leur bout clair (texte de 13 px). Teintes corrigées ; **test de contraste** sur les sept ajouts aux deux bouts (`appearance.test.ts`). Gris, rouge et bleu (hérités de Scribe, 3,3 à 3,9) sont laissés tels quels : écart de Scribe, non touché.
3. *Course lecture/écriture* : une lecture lancée avant l'enregistrement pouvait écraser la clé par « absente » pendant 30 s. Compteur de génération par clé : une lecture périmée ne range rien (aussi pour l'effacement). Tests `a_key_stored_while_a_reading_waits_…` et `an_erased_key_is_not_brought_back_…`.

**Mineurs corrigés** : `scriibe cle` accepte Linux ; Aide « Désinstaller » (fr, en) et commentaire de `uninstall.rs` à jour ; doc de la tâche 0 reformulée ; `CommandTool` testé pour de vrai avec un faux `secret-tool` (entrée standard seule, entrée fermée, sortie d'erreur non lue, délai qui tue le processus, programme absent).

**Mineurs restants** (à reprendre au carnet) :
- `secret-tool` est cherché dans le `PATH` du processus : un `PATH` modifié ferait lire la clé par un autre programme (même utilisateur, impact faible) ; chemin absolu d'abord, `PATH` en repli.
- La clé trouvée reste en mémoire jusqu'à la sortie de Scriibe (comme le trousseau du Mac) : une clé effacée ailleurs n'est vue qu'au redémarrage.
- Désinstallation sur trousseau fermé : jusqu'à 3 × 8 s d'attente, et le message ne dit pas « déverrouille le trousseau ».
- Échec d'écriture sur l'entrée de `secret-tool` : message générique sans le code de sortie ; fenêtre de déverrouillage éventuellement laissée ouverte après le délai de 60 s.
- Pastille en fond sombre : indigo et violet y perdent leur forme (ombre quasi invisible) ; « Noir » est invisible sur fond noir (hérité de Scribe). Une fine bordure claire règlerait les deux.

## Essai (tâche 3, 18/11/2026)

*Code au commit `2034c90`, **poussé sur `main` pour l'occasion** (accord de l'auteur : les PC ne tirent que de GitHub), puis `pilote maj tous` et `pilote compiler --mode release` (Linux et Windows : OK ; Windows a d'abord échoué, un ancien `scriibe.exe` verrouillait la compilation, arrêté puis recompilé). Mac : copie d'essai `Scriibe-essai-c6.app` sur une copie des données (`Scriibe-essai-c6`), lancée par `open`. Windows : lancé dans la session de l'auteur par une tâche planifiée temporaire (supprimée), sur la copie `C:\essai-refonte-lot2`. Linux : lancé par l'auteur depuis Activités, sur ses vraies données (réglages remis ensuite : veille 30/30, couleur Scriibe, moteur llama.cpp). Réglages faits par l'auteur ; journaux lus par l'assistant. Pas de capture : l'auteur a vu l'indicateur lui-même.*

| Cas | Mac | Windows | Linux |
|---|---|---|---|
| Whisper à 1 min : moteur déchargé | OK (« serveur arrêté après 1 min sans dictée ») | OK | OK |
| llama.cpp à 1 min : moteur déchargé | OK | non essayé (pas de llama.cpp sur ce PC) | OK |
| « Jamais » : dictée puis 2 min 30 à 2 min 45, aucun arrêt | OK (Whisper et llama.cpp) | OK (Whisper) | OK (Whisper et llama.cpp) |
| 3 nouvelles couleurs (orange, violet, marron), thème clair | lisibles | lisibles | lisibles |
| Mêmes couleurs, thème sombre | lisibles | lisibles | lisibles |
| Clé OpenRouter : enregistrement, relance, « présente » | sans objet | sans objet | OK (voir ci-dessous) |
| Reformulation avec la clé gardée | sans objet | sans objet | OK (OpenRouter `anthropic/claude-haiku-4.5`, 1,15 s) |
| Clé absente des journaux et de `ps` | OK (rien trouvé) | OK | OK (recherche de la valeur exacte des deux clés dans `scriibe.log`, `llama-server.log`, journaux de l'essai, `journalctl`, `ps` : 0) |

**Linux, clé** : `secret-tool` était **absent** du PC. Premier essai (clé Deepgram collée) : message attendu et exact (« l'outil secret-tool est absent : installe le paquet libsecret-tools … Rien n'a été enregistré. »), rien d'écrit. Après `sudo apt install libsecret-tools` : les deux clés enregistrées sans question de GNOME (trousseau déjà ouvert à la session), journal « clés: openrouter enregistrée dans le trousseau du bureau », entrées vérifiées (`secret-tool lookup`, code de sortie seul), « présente » encore affichée après relance de Scriibe. **Le paquet manque donc sur une installation courante** : la décision de § Clé Linux (remplacer `secret-tool` par un module Rust si le paquet manque souvent) reste à prendre : un seul PC vu, le message guide bien ; à reprendre au carnet.

**Observations**
- Un moteur à 1 min s'arrête 1 à 2 min après la dernière activité (contrôle toutes les minutes, ex. Mac : réglé à 20:59, arrêt à 21:00:26).
- Sur le Mac, le Whisper chargé à 20:58 a été déchargé avant la première dictée : le premier appui recharge le moteur (≈ 1 s), sans défaut.
- Windows : un `scriibe.exe` oublié bloque `pilote compiler` (déjà dans `docs/PIEGES.md`).
- Non essayé : indigo, rose, vert, turquoise, orange sur sombre au-delà des trois choisies ; Linux sans trousseau ouvert (trousseau verrouillé : couvert par test).

## Ajouts

Aucune nouvelle demande de l'auteur pendant l'essai.
