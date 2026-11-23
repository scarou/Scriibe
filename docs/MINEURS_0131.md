# Mineurs de la 0.13.1

Sortie commune du découpage `decoupage-2026-11-23-mineurs-essais-pc` (carnet du vault, dossier `decoupages/`). Une section par tâche.

## Mise en route (tâche 0, 23/11)

- **Renumérotation du cadrage des mises à jour** (0.13→0.14, 0.14→0.15, 0.15→0.16) : diff vérifié (seule une mention « versions renumérotées le 23/11 » s'ajoute à la ligne 23 du ROADMAP), commité seul (`99851ef`).
- **CLAUDE.md** : découpage mis en cours ; règles « tâche N » et Relais inchangées ; 8 797 → 7 962 signes (historique des chantiers soldés déplacé dans `docs/CLOTURE.md`, « Historique des chantiers »).
- **Lignes périmées soldées** : carnet Linux, trois lignes passées en Faite avec preuve (`dab84e7`, `e650f24`, 0.12.0 `fc49331`) et archivées dans `carnet-idees-scriibe-linux-archive.md` ; ROADMAP n° 19 marqué remplacé par la décision 6 révisée (tout sur `main`), légende ✗ ajoutée.

| Mineur | État | Test |
|---|---|---|
| « Réglages → Moteurs » devenu « Moteurs et modèles » | **Corrigé** dans les 12 guides d'installation (français, anglais, débutant, expert) | — (texte) |
| Numéro de version en dur dans le README | **Corrigé** : le README renvoie aux Releases (il disait 0.13.0) ; règle ajoutée à `docs/CLOTURE.md` (« aucun numéro de version en dur ») | — (texte) |
| Chaîne « gpu » de l'assistant sous Linux | **Corrigé** : texte honnête (processeur, carte graphique non utilisée) ; anglais dans `en-server.ts` | `on_linux_the_gpu_chain_does_not_promise_a_graphics_card` |
| 20 px au lieu de 14 entre deux bandeaux `config_problems` | **Corrigé** : `:has(+ .card.bad)` ramène l'écart à 14 px, 20 px seulement après le dernier | `tabs.test.ts` (règles CSS) ; le rendu à l'écran n'est pas vérifié ici |
| `settings.json` avec BOM | **Corrigé** : BOM toléré pour `settings.json` et `profile.json` | `a_settings_file_with_a_byte_order_mark_is_still_read` |
| Anglais : nom de touche écrit deux fois dans le refus d'une combinaison | **Corrigé** : le test « le refus nomme déjà la touche » se fait sur le français, avant traduction | `keyrefusal.test.ts` |
| Rattrapage Windows : `CommandMenuMask` non informé | **Corrigé** : `forget(code)` appelé quand le crochet rattrape un relâchement manqué | `a_caught_up_release_forgets_the_command_key_so_the_next_press_starts_clean` (la partie crochet, `windows/keyboard.rs`, ne compile pas sur Mac : essai sur le PC à la tâche 4) |

À savoir : d'autres chemins de menu des guides sont aussi périmés (« Réglages → Mise à jour », « Réglages → Apparence », « Réglages → Mesures et essais », « Réglages → Dictée » : devenus des pages du menu) ; hors de la liste de la tâche, non touchés.

## Robustesse (tâche 1, 23/11)

Choix et raisons : le trousseau reste `secret-tool` (pas de module Rust). Une clé trouvée est gardée **10 minutes** en mémoire puis relue : c'est le prix pour qu'une clé effacée ailleurs cesse d'être servie sans redémarrage ; un trousseau fermé à ce moment-là rend « pas de clé » jusqu'à la prochaine tentative (5 min), jamais une clé périmée. Pour les états de la fenêtre, M3 évince le pli d'Historique le plus ancien plutôt que d'abandonner de retenir les plis : on garde la fonction.

| Point | État | Test |
|---|---|---|
| `secret-tool` par chemin absolu (`/usr/bin`, `/bin`, `/usr/local/bin`), `PATH` en repli | **Corrigé** | `the_program_is_looked_for_by_absolute_path_first_and_by_name_otherwise` |
| Clé effacée de la mémoire dès qu'elle n'est plus utile (écrasée à la chute, périmée après 10 min, relue) | **Corrigé** (au mieux : une copie faite plus tôt par le système n'est pas atteinte ; la copie rendue aux moteurs n'est pas écrasée) | `a_found_key_is_forgotten_after_its_delay_…`, `secrets_are_overwritten_when_dropped` |
| Désinstallation sur trousseau fermé : une seule attente, message « déverrouille-le » | **Corrigé** (`delete_all`) | `a_locked_keyring_stops_the_uninstall_after_the_first_wait_…` |
| Échec d'écriture sur l'entrée de `secret-tool` : message avec le code de sortie | **Corrigé** (`ToolError::Refused`) | `a_tool_that_closes_its_input_without_reading_gives_a_message_with_its_exit_code_and_no_key` |
| Fenêtre de déverrouillage fermée après le délai | **Processus tué et récupéré, mais fermeture de la fenêtre non vérifiée** : elle appartient au trousseau, qui doit la retirer quand l'outil quitte le bus ; **à regarder à l'essai Linux** (tâche 5) | — |
| Clé jamais dans un journal ni en argument | **Inchangé, toujours testé** | `the_key_goes_through_standard_input_never_through_the_arguments` ; les messages d'erreur testés sans la clé |
| Pastille sur fond sombre : liseré clair (indigo, violet, noir) | **Corrigé** (`--edge`, 1 px) ; rendu non vu à l'écran (essai Mac, tâche 6) | `appearance.test.ts` |
| Carte « Mes commandes » inerte | **Corrigé** : export/import demandés depuis Données avec les commandes décochées → boîte qui dit quoi cocher, plus de carte inerte | `commandsTransfer.test.ts` |
| M2 : changement en attente perdu à la fermeture | **Corrigé** : `pagehide` → `uiState.leave()`, requête `keepalive` | `uiState.test.ts` (deux cas), `apiKeepalive.test.ts` |
| M3 : plafond de 300 cartes sans purge | **Corrigé** : le pli d'Historique le plus ancien cède sa place | `a_full_list_gives_way_to_a_new_card_by_dropping_the_oldest_history_fold` |
| Test de contrat faux serveur de la page ↔ `merge` de Rust | **Fait** : `crates/platform/tests/data/ui_state_contract.json`, joué des deux côtés | `merge_follows_the_contract_shared_with_the_page`, `uiState.contract.test.ts` |
| Tests des routes `api/ui-state` | **Fait** : la route est extraite (`ui_state::route`) et testée | `the_route_reads_merges_and_reports_a_file_it_must_not_touch` |

## Mot répété (tâche 2, 23/11)

Choix et raisons : la règle de `repeats_itself` reste « refaire sur le serveur normal, dont la réponse remplace l'autre » ; elle ne touche jamais au texte elle-même. Une fausse alerte coûte une seconde transcription, pas une altération : les trois exemplaires dits exprès reviennent intacts quand le serveur normal les entend de même.

| Point | État | Test |
|---|---|---|
| Ligne de journal quand la prise est refaite | **Fait** : `whisper: boucle vue (N caractères), seconde transcription sur le serveur normal` ; la raison et la taille seulement, jamais le texte | `a_redone_take_leaves_one_log_line_without_the_dictated_text` (le mot dit ne se retrouve pas dans la ligne ; rien quand la réponse est propre) |
| M1 : boucle CJK sans séparateur | **Corrigé** : chaque caractère han, hiragana ou katakana compte pour un demi-mot (deux caractères = un mot) ; « ありがとう » ×3, « 正 » ×6, « ありがとうございます » ×2 sont des boucles ; « はいはい », « ありがとうありがとう » (deux fois seulement) restent du texte | `the_edges_of_the_loop_rule` |
| M3 : boucle coupée en son milieu | **Corrigé** : un passage court (2 ou 3 mots) dit deux fois puis repris sans être fini (« C'est fait. C'est fait. C'est ») est une boucle | `the_edges_of_the_loop_rule` |
| M3 : motif alterné deux fois | **Limite assumée, testée** : « Fé. Fait. Fé. Fait. » reste du texte (indiscernable d'une répétition voulue, comme « Lance yogurt. Lance yogurt. ») ; trois fois, ou deux fois puis « Fé » : boucle | `the_edges_of_the_loop_rule` |
| Ajout 1 : suite de chiffres sur une prise courte | **Corrigé** : un texte fait de 6 nombres ou plus (avec « et ») sur une prise de 2 s ou moins est refait sur le serveur normal ; pas au-delà de 2 s, pas pour 3 nombres, pas dans une phrase | `a_run_of_numbers_on_a_very_short_take_is_redone_but_not_on_a_longer_one` |
| M4 : triples voulus (« très très très bien », « ha ha ha ») | **Intacts** : refaits, mais le serveur normal rend le même texte | `a_wanted_triple_comes_back_intact_when_the_normal_server_confirms_it` |
| Ajout 3 : « Très très bien » → « Très bien » à la mise au propre (Linux, `qwen2.5-1.5b`) | **Note seulement** : l'essai Linux (tâche 5) le reverra, à comparer avec Scribe avant d'en faire un écart | — |
| `instance::tests::a_second_instance_is_refused_until_the_first_ends` | **Stabilisé** (il est dans le crate `platform`, non `cli`) : cause trouvée, un test voisin qui lance un programme (`open`, `installed_apps`) garde le verrou `flock` quelques millisecondes entre `fork` et `exec` ; échec 23 fois sur 25 avec le filtre `instance:: open`, 4 sur 25 avec `installed_apps`, 0 sur 25 après. Le test attend maintenant que le verrou tombe (5 s au plus) ; le code de production n'est pas modifié | 25 lancements avec les voisins fautifs, puis 20 lancements du crate entier : 0 échec |
| M3 de l'enquête Windows : repli de `handle_page_key` | **Couvert** : Maj relâchée dans la page pendant une dictée du crochet (fenêtre d'écho ou après) ne finit ni ne traduit rien ; Maj enfoncée puis relâchée dans la page traduit une fois ; dictée à combinaison (Ctrl gauche + F9) : relâcher l'une ou l'autre finit la dictée, une touche étrangère non. Le garde de `controller.rs` retiré : les 3 tests échouent | `a_shift_released_in_the_page_during_a_dictation_neither_ends_it_nor_translates`, `a_shift_pressed_and_released_in_the_page_translates_the_dictation_the_hook_started`, `releasing_one_key_of_a_combination_in_the_page_ends_the_dictation` (celui-ci Mac et Windows seulement ; sa version Windows ne compile que sur le PC, tâche 4) |

À savoir : la règle « deux fois puis un début de troisième » fait refaire aussi « Je veux, je veux, je » dit sans y penser ; coût : une seconde, texte inchangé. Mesure sur les 2 287 textes des cas de référence : 0 texte signalé à tort. L'historique de la machine (19 entrées) est trop petit pour mesurer un taux réel.

## Relecture (tâche 3, 23/11)

Sous-agent relecteur Sonnet (effort élevé, n'a pas écrit le code) sur `99851ef`, `ec10aa5`, `dba0820`, `20a1571`, `d8531aa` : **0 bloquant, 2 importants, 9 mineurs**. Aucune fuite de clé (messages d'erreur, arguments, journal), aucune panique de `repeats_itself` (fuzz de 300 000 textes), `CommandMenuMask::forget` relu à la main sans défaut. Détail du relecteur : dossier temporaire de la session (`relecture-0131.md`), repris ici.

**Importants, corrigés (un test chacun)**
- I1 `secret_service.rs` : à l'expiration des 10 min, un trousseau reverrouillé ou lent faisait oublier une clé valide (« pas de clé » 5 min, fenêtre de déverrouillage au début d'une dictée). Maintenant seule une réponse « absente » la fait oublier ; un trousseau muet la laisse servie et repousse la relecture du délai des trousseaux muets. Test : `a_keyring_that_does_not_answer_after_the_delay_keeps_the_key_it_knew`.
- I2 `secrets_are_overwritten_when_dropped` ne prouvait rien (`mem::take` vidait déjà la chaîne). `wipe_bytes` met les octets à zéro par écriture volatile, `wipe` écrase le tampon **sur place** ; le test relit le tampon par son pointeur et un compteur par fil vérifie que `Secret` et `ToolOutput` appellent `wipe` en tombant.

**Mineurs corrigés**
- M1 `ui_state.rs` : un jour d'Historique plus ancien que tous ceux qu'on garde évinçait un jour plus récent (« Tout replier » les envoie du plus récent au plus ancien, au-delà de 300 cartes) ; il est maintenant refusé. Test ajouté à `a_full_list_gives_way_to_a_new_card_by_dropping_the_oldest_history_fold`.
- M8 `whisper.rs` : le début de troisième copie ne compte plus qu'en fin de texte (« Il y a il y a il pleut » n'est plus refait). Cas ajouté à `the_edges_of_the_loop_rule`.
- M10 : « dans le même page » → « la même page » (3 guides), phrase fausse de `ROADMAP.md` n° 23 (la note porte déjà les nouveaux numéros).

**Mineurs restants (non corrigés, notés)**
- M2 : le contrat page ↔ programme ne couvre ni le plafond ni l'éviction (la page ne plafonne pas : elle garde les cartes évincées jusqu'au rechargement ; voulu).
- M3 `uiState.ts` : `leave()` ne reprend pas un changement déjà parti dans la file derrière un envoi lent ; un pli enregistré sans délai n'est pas en `keepalive`. Risque faible en local.
- M4 : `uiState.test.ts` (« la fermeture n'attend pas un envoi en cours ») a une assertion toujours vraie ; `apiKeepalive.test.ts` lit le source de `main.ts` au lieu du comportement.
- M5 : `CommandTool::new()` (branchement de `resolve_program`) et le message « a refusé la clé » de `store` (`ToolError::Refused`) ne sont pas exercés par un test.
- M6 : si `secret-tool` n'est dans aucun des trois dossiers (NixOS, Guix, `~/.local`), le nom seul passe par le `PATH` : la protection annoncée ne joue pas ; un fichier non exécutable dans `/usr/bin` n'est pas écarté (`is_file`).
- M9 : `is_number_run` compte « 2,5 » comme deux nombres ; effet : une seconde transcription de plus sur une prise de 2 s ou moins.
- Test CSS des bandeaux (`tabs.test.ts`) : une regex sur le source ; `:has()` demande WebKitGTK ≥ 2.40.

## Essai Windows (tâche 4, 23/11)

PC Windows (`C:\scriibe-dev`) : branche `essai-mineurs` au commit `3e39efa` envoyée par `git bundle` (`276f4d6..main`, 130 Ko), `main` du PC intact et remise en place à la fin. Compilation release réussie (2 min 07, aucun avertissement Rust). Essai à l'écran sur une **copie** des données (`SCRIIBE_DATA=C:\essai-mineurs\donnees`, modèles par jonction), lancée dans la session de l'auteur par tâche planifiée ; Scriibe habituel (`C:\scriibe`) arrêté puis relancé ; copie, tâche et bundle supprimés. Prérequis vérifiés avant : Whisper (`ggml-small`, `large-v3-turbo`, `base`), Qwen 0,5 et 1,5 Md, Rust, Node, Git ; 267 Go libres.

| Ligne du carnet | Verdict |
|---|---|
| Bug « nécessaires pour écrire » (16/11), geste ×10 | **Absent**. L'auteur : 11 refus d'AltGr dans l'interface, message identique (« AltGr (Alt droit) ne peut pas lancer la dictée : elle sert à taper », nom écrit **une fois**) ; 10 captures de plus par l'API (AltGr, Ctrl gauche + AltGr, lettre A, deux Maj pour traduire) ; une ligne de journal par refus. La phrase n'existe dans aucun code. À classer **non reproduit (21 essais)**. |
| Avertissement « le type généré ne définit aucune méthode ou propriété publique » (23/11) | **Source trouvée, rien à corriger dans Scriibe** : `pilote-pc/pilote/relais/fenetre.ps1` ligne 97, `Add-Type -TypeDefinition` de la classe `PiloteForm` (n'a que des membres protégés et un constructeur). Reproduit à l'identique sur le PC avec ce seul fragment. Sans danger. Correctif possible côté pilote-pc : `-WarningAction SilentlyContinue` ou un membre public. |
| Gestion des modes (S25, ROADMAP n°21), 12/11 | **Vu à l'écran sous Windows** : décocher « Afficher dans le menu de l'icône » (mode actif compris) les retire du menu, rétabli en recochant ; choix page → coche du menu et menu → page sans recharger ; export (« Enregistrer sous », nom `scriibe-mode-essai-a-2026-11-23.scriibe.json`) ; import : fichier abîmé refusé, nom pris → « Essai A 2 » ; mode gardé après 2 redémarrages. Une seule réserve : après un choix de mode par le menu (étape 3), l'API a répondu « Soutenu » alors que l'auteur voyait « Littéral » ; **non reproduit** à l'essai suivant (profil et API d'accord, mode gardé au redémarrage). À surveiller. |
| Aide (S23), 12/11, et lot 2 du 17/11 | **Vus** : sommaire fixe, remontée au clic, recherche surlignée, bande sous 860 px, interface en anglais (aucun texte resté en français), export de la partie utilisateur (fichier créé, deux modes dedans). Import utilisateur et commandes non repris : même mécanique que l'import du dictionnaire essayé le 17/11, couverte par les tests ; **aucune commande vocale personnalisée à exporter** sur ce PC. |
| Fenêtre administrateur au premier plan (19/11) | **Vue** : Gestionnaire des tâches élevé devant, Ctrl droit tenu 3 s : ni pastille ni ligne de dictée dans le journal, le crochet est sourd (comportement documenté de Windows pour un niveau d'intégrité inférieur). Même touche avec le Bloc-notes devant : dictée normale, collage de 6 signes. Pas de correction possible sans lancer Scriibe en administrateur ; à noter dans l'Aide ou `PIEGES.md` si l'auteur le souhaite. |
| Échec ponctuel de `scriibe-platform --lib` (19/11) | **1 échec sur 49 passages** (226/227), au tout premier passage après compilation ; le nom du test n'a pas été relevé (sortie filtrée). 0 échec sur les 48 suivants, dont 18 passages simultanés sous charge. `cargo test --workspace` : 34 suites, 905 réussis, 0 échec, 2 ignorés. **Non identifié** : à relever (sortie complète) s'il revient. |
| Correctifs Windows de la 0.13.1 | **Vérifiés** : `releasing_one_key_of_a_combination_in_the_page_ends_the_dictation` et les tests de `CommandMenuMask` (`keys::tests`) verts sur le PC ; BOM de `settings.json` toléré (`scriibe verifier` lit les réglages, touche relue) ; nom de touche non répété dans le refus (français ; l'anglais n'a pas été refait à l'écran, couvert par un test). |
| Reformulation copiée seule et Win+V, accord lié (16/11) | **Non essayés** : pas de clé OpenRouter sur ce PC ; reste ouvert. |
| Désinstallation sans garder les modèles, pastille en démarrage discret, démarrage depuis `C:\scriibe`, clés effacées (13/11) | **Non essayés** : destructifs (5,8 Go à la Corbeille, clés du Gestionnaire d'identifiants), à n'essayer qu'avec l'accord explicite de l'auteur ; restent ouverts. |
| Pastille bloquée sur « j'écoute » (16/11) | **Non revu** : aucune pastille bloquée pendant cet essai. |

**Ajouts** : aucun. Retours d'erreur (`RETOURS_ERREUR.md`) : aucun mineur apparu.

**Retour de l'auteur** : plusieurs de ces essais lui ont paru répétés (essayés ailleurs, ou évidents : « la pastille s'est évidemment affichée avec Ctrl droit »). Pour la tâche 5 (Linux) : relire d'abord le carnet Linux et `docs/` pour ne garder que ce qui n'a **jamais** été vu sur ce PC, et ne pas demander de témoin évident.

À savoir : restes d'anciens essais sur le PC (`C:\essai-c6`, `essai-c7`, `essai-enq`, `essai-lot2`, `essai-lot3`, `essai-refonte-lot2`, `scriibe-essai`), non touchés, à ranger si l'auteur le veut.

## Essai Linux (tâche 5, 23/11)

PC Linux (GNOME 50, Wayland) : `main` du PC (`2034c90`) intact ; branche `essai-mineurs` (`e55a805`) envoyée par `git bundle` (`2034c90..main`), compilée dans un worktree à cible séparée. Prérequis vérifiés en lecture seule : `secret-tool`, Whisper (`base`, `small`, `large-v3-turbo-q8_0`), `qwen2.5-1.5b`. **Compilation release : 4 min 25, 0 avertissement ; `cargo test --workspace` : 34 suites, 919 réussis, 0 échec, 3 ignorés.** Essai par l'API d'une **instance isolée** (accord de l'auteur) lancée par SSH avec les variables Wayland du Scriibe en cours, données copiées (`models` en lien), sans touches ; Scriibe habituel jamais touché. Piège : `DISPLAY`/`XAUTHORITY` ne se lisent **pas** sur `gnome-shell` (vides) mais sur le Scriibe déjà lancé, sinon GTK échoue (panique `tao`) alors que le serveur démarre quand même. Tout est retiré à la fin (instance, copie, worktree, branche, cible, fichier exporté).

| Ligne | Verdict |
|---|---|
| Modes, export sous Linux (ROADMAP n°21, 12/11) | **Vu** : pas de boîte, fichier écrit dans `~/Documents/scriibe-mode-essai-l-2026-11-23.scriibe.json`, réponse avec le chemin |
| Modes, import (champ de chemin) | **Vu** : nom pris → « Essai L 2 », appliqué avec le jeton de l'aperçu ; fichier abîmé et fichier absent refusés ; sans chemin → `ask_path` ; mode masqué gardé dans `hiddenModes`, groupes du menu inchangés |
| Modes, coche du menu de l'icône (barre du haut GNOME) | **Non vu à l'écran** : seul l'état (réglage) est vérifié ; même code qu'à l'essai Windows |
| Chaîne « gpu » de l'assistant | **Vu** : texte honnête (portable sans carte graphique, mesures 2,3 à 3,1 s) |
| « Très très bien » à la mise au propre (`qwen2.5-1.5b`, Simple) | « Très très bien. » et « euh c'est très très bien merci » : **reformulation écartée** par le garde-fou, donc texte gardé tel quel ; « Oui, oui… » → « Oui, » ; « Ha ha ha, très très très bien » → « …très bien » (1,5 s). Le modèle supprime donc une répétition sur les phrases longues, le garde-fou protège les courtes. **Note seulement**, non comparé à Scribe (même consigne, même modèle). |
| Trousseau, enregistrer puis relire une clé | **Non refait** : fait le 18/11 et couvert par tests ; les deux clés existantes présentes (lecture seule, aucune écrasée) |
| Trousseau fermé : fenêtre de déverrouillage retirée après le délai | **Non vérifié** : demanderait de verrouiller le trousseau de l'auteur puis qu'il le déverrouille (mot de passe) ; couvert par `a_locked_keyring_stops_the_uninstall_…` côté processus ; à n'essayer qu'avec son accord |
| Alt+Tab pendant un raccourci tenu (Wayland) | **Vu** (geste de l'auteur, une fois) : Ctrl+← tenu 18,9 s avec Alt+Tab au milieu ; le relâchement a été reçu (« relâchement — 18.9 s de son »), transcription 5,6 s, mise au propre 2,0 s, collage de 18 signes, aucune dictée bloquée. Le message « écoute depuis plus de 15 s » du journal est l'avertissement normal d'une prise longue. Détail : c'est le Scriibe habituel (code `2034c90`) qui a répondu, pas l'instance d'essai (aucune touche) ; le code des portails n'a pas changé depuis. Pastille non vérifiée à l'écran. |
| Lignes 12–17/11 non reprises (anglais à l'écran, imports et exports d'autres parties, surlignage de l'Aide dans WebKitGTK ; reformulation sur sélection ; restauration sans sélecteur de fichier ; démarrage X11 ; autres) | **Non essayées** : vues sous Windows (Aide, anglais) ou chantiers de code (lecture de la sélection, wl-clipboard, sélecteur), pas des vérifications ; restent au carnet |

**Ajouts** : aucun. Retours d'erreur : aucun mineur apparu.

## Relais

*(Fin de la tâche 4, 23/11.)*

- **Fait** : tâche 4 (§ Essai Windows) ; rien d'autre n'a changé dans le code ; commits locaux de la tâche 3 et celui de cette note ; rien de poussé. PC Windows : `main` en place, Scriibe habituel relancé, copie d'essai supprimée.
- **Prochain pas** : tâche 5 (essai Linux, branche `essai-mineurs` par `git bundle`, nouvelle session Sonnet · Moyen). Le bundle de la tâche 4 était `276f4d6..main` ; pour le PC Linux vérifier son commit avec `pilote etat linux` avant.
- **Pièges** : `pilote` n'a pas de commande d'envoi de bundle (scp + `git fetch <bundle> main:essai-mineurs`) ; `robocopy` sans `/R:0 /W:0` reste bloqué sur un fichier verrouillé (`instance.lock`) ; une copie de données par jonction `models` se supprime par `rmdir` de la jonction d'abord.
- **Consigne de l'auteur (23/11, pour la tâche 5)** : ne rien lui demander de ce que tu peux faire toi-même (API par tunnel SSH, journaux, captures `pilote`, tests) et ne tester que ce qui est critique ; un geste physique seulement pour ce qui n'a jamais été vu et ne peut pas se faire à distance, regroupé en une étape ; pas de témoin évident.

*(Fin de la tâche 5, 23/11.)*

- **Fait** : tâche 5 (§ Essai Linux) ; commits locaux, rien de poussé. PC Linux : `main` et Scriibe habituel intacts, instance d'essai et branche retirées.
- **Prochain pas** : tâche 6 (essai Mac) en nouvelle session Sonnet · Moyen.
- **Pièges** : voir § Essai Linux (variables d'affichage lues sur le Scriibe en cours, pas sur `gnome-shell`) ; la session refuse un `pgrep | kill` ou un `rm -rf` groupé sur le PC : tuer par numéro de processus, supprimer chemin par chemin.
