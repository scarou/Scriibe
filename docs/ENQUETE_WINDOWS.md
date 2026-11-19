# Enquête Windows (chantier 7)

Découpage : `decoupage-2026-11-17-c7-enquete-windows.md` (vault IA-OS). Point de départ : `docs/COMMANDES_ETENDUES.md` § Essai lot 3 (deux défauts vus le 16/11 sur le PC Windows, jamais reproduits).

## Enquête

*(Tâche 0, 19/11/2026. Code : `276f4d6` (0.12.0 plus un correctif clippy), compilé sur le PC par `pilote maj` puis `pilote compiler --mode release`. Copie d'essai des données dans `C:\essai-enq\donnees` (mise au propre en local, aucune donnée vers un service en ligne), lancée dans la session de l'auteur par une tâche planifiée temporaire (supprimée ensuite, comme les processus ; le dossier `C:\essai-enq` reste, il contient une jonction vers les modèles : ne pas le supprimer en récursif). L'auteur seul aux gestes du clavier et de la souris, une étape à la fois ; journaux lus par l'assistant. Pas de capture d'écran : l'auteur a dit ce qu'il voyait.)*

### Bug 1 · la pastille reste bloquée sur « j'écoute » : cause prouvée

**Reproduit.** Bloc-notes devant ; Ctrl droit (la touche de dictée) tenu ; **sans la relâcher, clic sur l'icône de Scriibe dans la barre des tâches** (sa fenêtre passe devant) ; relâchement de Ctrl droit. La pastille reste sur « j'écoute ». Elle ne se termine qu'à l'appui suivant sur une touche, ici un appui bref sur Ctrl droit 87,8 s plus tard.

| Essai | Résultat |
|---|---|
| Ctrl droit tenu, clic sur **Scriibe** (barre des tâches), relâchement dedans | **bloquée** : « j'écoute » 87,8 s, jusqu'à un nouvel appui |
| Un appui bref sur Ctrl droit, fenêtre de Scriibe toujours devant | la pastille « j'écoute » disparaît, « Mise au propre » s'affiche puis disparaît |
| Ctrl droit tenu, clic sur l'**Explorateur de fichiers** (barre des tâches), relâchement dedans | **normal** : la pastille disparaît au relâchement (trois dictées de suite, 1,8 à 3,2 s de son, collées) |

Journal de l'essai bloqué (`scriibe.log` de l'instance d'essai) :

```
09:43:59.819 dictée: appui — micro ouvert (…), appli : Notepad
             (rien pendant 87 s : ni relâchement, ni annulation)
09:45:27.470 touches: relayées par la page (la fenêtre de Scriibe a le focus ; 7 événements du clavier reçus du système depuis le lancement)
09:45:27.613 dictée: relâchement — 87.8 s de son
```

**Cause** (code lu, confirmée par les trois essais) : la fenêtre de Scriibe devant, **le crochet clavier de Windows ne reçoit plus rien** (piège connu S06, `docs/PIEGES.md`, cause inconnue ; vérifié ici : même avec l'Explorateur devant le crochet voit tout). C'est la page qui relaie les touches (`ui/src/lib/keyrelay.ts`), mais :
1. côté page, `up` **ignore le relâchement d'une touche qu'elle n'a pas vue s'enfoncer** (`if (!pressed.delete(e.code)) return;`) ; or la dictée a commencé **avant** le focus (le crochet a vu l'appui) ;
2. côté Rust, `Controller::handle_page_key` (`crates/pipeline/src/controller.rs`) passe par `translate_key(page_held, …)` : pour une touche absente de `page_held`, le relâchement n'est de toute façon pas traduit en événement (ignoré), et le crochet n'est lui-même plus là pour le dire ;
3. `release_page_keys` (perte du focus) ne relâche que les touches vues par la page (`page_held`), pas celles du crochet.

Le relâchement de la touche de dictée n'arrive donc jamais ; le moniteur de touches garde la touche pour tenue (`HELD` du crochet, `HotkeyMonitor`). À la frappe suivante (autre touche), le contrôleur la prend pour « une autre touche tapée » : `dictée: annulée (une autre touche a été tapée)` (`controller.rs`, `cancel`), ce qui explique l'annulation à 21 s et 26 s du 16/11 : l'auteur avait changé de fenêtre (vers Scriibe) puis tapé.

**Tests écrits (échouent aujourd'hui, ignorés ou en attendu-échec pour garder la suite verte)** :
- `crates/pipeline/tests/dictation.rs` · `a_key_pressed_before_the_page_got_the_focus_ends_the_dictation_when_the_page_sees_it_released` (`#[ignore]`) : appui par le crochet, silence du crochet plus d'une seconde, relâchement relayé par la page ⇒ la dictée doit finir. `cargo test -p scriibe-pipeline --test dictation -- --ignored` : **FAILED** (« le relâchement vu par la page doit finir la dictée commencée par le crochet »).
- `ui/src/lib/keyrelay.test.ts` · `it.fails(« relaie le relâchement d'une touche appuyée avant la fenêtre… »)` : passe tant que le défaut existe (« 1 expected fail »), échouera dès la correction, à retourner en `it`.
- Le test voisin `ignore le relâchement d'une touche appuyée avant d'arriver dans la fenêtre` (même fichier) **fige la décision contraire** : à réécrire avec la correction.

**Non vérifié** : (a) la cause du silence du crochet (S06) reste inconnue : cet essai montre seulement que la page de Scriibe au premier plan le provoque et que l'Explorateur non ; (b) une fenêtre exécutée en administrateur au premier plan (Gestionnaire des tâches, invite UAC) rend sourd un crochet de niveau d'intégrité inférieur (comportement Windows documenté) : **non essayé**, même conséquence probable.

**Pistes de correction** (aucune appliquée, tâche suivante) : (1) au retour du focus ou au premier événement de la page, demander l'état réel des touches tenues (`GetAsyncKeyState`) et relâcher dans le contrôleur celles que le crochet croit tenues ; ou, plus simple, (2) laisser passer le relâchement d'une touche inconnue de la page quand une dictée est en cours, des deux côtés (page et `handle_page_key`) ; (3) garde-fou : une dictée sans relâchement au-delà de N secondes se termine d'elle-même avec message.

### Bug 2 · message « nécessaires pour écrire » en changeant les touches : non reproduit

**La phrase n'existe dans aucune version du code** (`git log -S` sur le dépôt, recherche dans `crates`, `ui/src`, v0.7.0 et v0.8.0 : rien). Le refus le plus proche, dans la page « Touche de dictée », est celui d'une touche qui sert à taper : v0.8.0 « *X ne peut pas servir de touche de dictée : elle sert à taper (ou à AltGr)* » ; aujourd'hui « *X ne peut pas lancer la dictée : elle sert à taper* ». L'auteur l'a décrit de mémoire le 16/11 (« nécessaires pour écrire ») : c'est probablement ce refus.

Essais faits :
- **À l'écran** (Réglages → Dictée → Touche de dictée → Changer…, AltGr) : l'auteur lit « AltGr (Alt droit) — AltGr (Alt droit) ne peut pas lancer la dictée : elle sert à taper ». C'est le refus voulu (AltGr, flèches et lettres ne lancent pas la dictée : `NOT_FOR_DICTATION`, `crates/platform/src/keys.rs`), **mais le nom de la touche est écrit deux fois** (défaut d'affichage, mineur).
- **Par l'API de capture de l'instance d'essai** (`api/keys/capture/*`, touches simulées, `cap.ps1` dans le dossier temporaire) : Ctrl droit, Maj gauche, Alt gauche, Windows gauche, F9 acceptés pour la dictée ; AltGr (seul ou avec Ctrl gauche, comme un clavier AZERTY l'envoie) et la lettre A refusés (« elle sert à taper ») ; Ctrl gauche + Maj gauche : « Ctrl gauche sert déjà à dicter » (conflit avec la touche de traduction en place) ; commande : Maj gauche, AltGr, Windows droite acceptés ; traduction : deux touches refusées (« une seule touche pour traduire »). Aucune ne produit une autre phrase.

**Non élucidé et pourquoi** : la touche pressée le 16/11 n'est pas connue, et le journal ne garde **aucune trace d'une capture refusée** (ni la touche, ni le message). La cause de mauvaises touches ce jour-là (noms `ShiftLeft` au lieu de `left_shift` dans `settings.json`, ignorés sans erreur) est déjà notée dans § Essai lot 3.

**Ce qui manque au journal pour comprendre le prochain cas** :
- une ligne par capture refusée : type (dictée, traduction, commande), identifiants des touches vues, texte du refus (jamais de frappe de lettres au-delà de l'identifiant de la touche) ;
- des événements de touche au niveau du crochet quand une dictée dure plus de 10 s sans relâchement : état de `HELD` et de `page_held`, dernier événement du crochet, fenêtre au premier plan (utile à la fois pour le bug 1 et pour une fenêtre administrateur) ;
- à la fin d'une dictée annulée par « une autre touche », la touche en cause et la durée de la dictée.

### Constats en passant

- **`settings.json` avec BOM illisible** : un `settings.json` réécrit par PowerShell 5 (`Set-Content -Encoding UTF8` ajoute un BOM) donne « migration: settings.json est illisible, rien n'est déplacé tant qu'il n'est pas réparé » et Scriibe démarre sur des réglages par défaut (modèles, touches). Un fichier retouché au Bloc-notes avec BOM ferait de même. Une tolérance au BOM (ou un message qui le nomme) serait utile ; **non traité ici**.
- **Deux `whisper-server` en vie** après l'essai : un serveur de dictée courte et un de dictée longue (« whisper court » et « whisper »), arrêtés à la main ; pas un défaut nouveau (déjà noté § Essai lot 3 pour les copies d'essai).

### Ce qui reste (non traité dans cette tâche)

Les deux autres lignes du carnet rattachées au chantier 7 (reformulation copiée seule et sa présence dans Win+V ; « Lancer à l'ouverture de session » sous Windows) ne sont pas touchées ici : tâches suivantes du découpage.

## Correctifs

*(Tâche 1, 19/11/2026, commit `16b595d`, local, non poussé. Code envoyé au PC par `git bundle` sur une branche `essai-c7` (pas de push), compilé et testé sur place par `pilote` puis par `cargo test` en SSH, branche supprimée et PC remis sur `main` ensuite.)*

### Bug 1 · pastille bloquée : corrigé

- **Page** (`ui/src/lib/keyrelay.ts`) : `up` relaie aussi le relâchement d'une touche qu'elle n'a pas vue s'enfoncer. Le test qui figeait le contraire (« ignore le relâchement d'une touche appuyée avant d'arriver dans la fenêtre ») est supprimé ; celui de la tâche 0 (`it.fails`) devient un `it` ordinaire.
- **Contrôleur** (`Controller::handle_page_key`) : un relâchement que la page n'a pas vu s'enfoncer est appliqué **seulement si une dictée est en cours** (`monitor.is_down()`) ; sinon ignoré comme avant. Tests : `a_key_pressed_before_the_page_got_the_focus_ends_the_dictation_when_the_page_sees_it_released` (le test de la tâche 0, plus ignoré, au vert) et `a_release_seen_only_by_the_page_is_ignored_without_a_dictation` (sans dictée, ou autre touche : rien ne change).
- **Crochet Windows** (`crates/platform/src/windows/keyboard.rs`) : un appui sur une touche que le crochet croit déjà tenue alors que `GetAsyncKeyState` la dit relâchée est un relâchement jamais vu : il est rattrapé (événement de relâchement envoyé avant l'appui) au lieu de prendre l'appui pour une répétition. Sans cela, après la correction, **le premier appui suivant sur la touche de dictée aurait été perdu** (le crochet gardait la touche pour tenue). Les touches retenues par le crochet (jamais enfoncées pour le système) sont exclues. **Pas de test automatique** (le crochet ne s'exécute pas sans vrai clavier) : compilé et passé à clippy sur le PC ; **à essayer à la main à la tâche 3** (appuyer sur Ctrl droit juste après avoir relâché dans la fenêtre de Scriibe, la dictée doit partir au premier appui).
- **Pas traité** : la cause du silence du crochet quand la fenêtre de Scriibe a le focus (S06) reste inconnue ; la correction l'évite sans la lever. La fenêtre en administrateur au premier plan n'a pas été essayée.

### Bug 2 · message des touches : journal ajouté (non reproduit)

- **Captures refusées** (`crates/cli/src/web/mod.rs`, `capture_view`) : une ligne par capture refusée, `capture des touches: refusée (dictation|translate|command) — touches vues : <noms> — <refus>`, une seule fois par capture (la page relit l'état plusieurs fois par seconde). Seuls les noms des touches vues et le message de refus y figurent.
- **Écoute longue** (`Controller::tick`, `note_long_dictation`) : une ligne par écoute de plus de 15 s : âge du dernier événement du crochet, touches tenues vues par la page, nombre d'événements du clavier reçus du système depuis le lancement, appli au premier plan. Ni les mots dits, ni les touches. Test `a_long_listening_leaves_one_journal_line` (une ligne, pas avant 15 s, une par dictée).
- **Annulation par une autre touche** (`Controller::cancel`) : la ligne donne la durée d'écoute (« … après 21.5 s d'écoute »), sans nommer la touche tapée. Test `a_cancelled_dictation_says_how_long_it_listened`.
- **Affichage** : le nom de la touche n'est plus écrit deux fois (« AltGr (Alt droit) — AltGr (Alt droit) ne peut pas… ») : `ui/src/lib/keyrefusal.ts` (`refusalLine`) garde le préfixe seulement quand le refus ne nomme pas la touche (« une seule touche pour traduire »). 3 tests dans `keyrefusal.test.ts`.

### Reformulation copiée seule et Win+V

Test automatisé `crates/platform/tests/windows_clipboard.rs` (un seul test, Windows seulement) :
1. **Marques** : une copie ordinaire n'a aucune des trois marques ; `copy_transient` met le texte dans le presse-papiers avec `ExcludeClipboardContentFromMonitorProcessing`, `CanIncludeInClipboardHistory` et `CanUploadToCloudClipboard` à 0.
2. **Historique** : interrogé par WinRT (`Clipboard.GetHistoryItemsAsync`, par PowerShell), le texte copié ordinairement y entre (témoin), le texte copié seul n'y entre pas. Il demande une session ouverte et l'historique activé ; sinon il le dit et ne conclut pas (jamais de réglage système modifié par le test). Sans session interactive (SSH), tout le test se saute en le disant.

**Résultat sur le PC de l'auteur** (dans sa session, par une tâche planifiée temporaire supprimée ensuite) : historique désactivé au premier passage → marques vérifiées, historique « non vérifié » ; l'auteur l'active (réglage de son PC, à lui de le laisser ou non) ; second passage : « historique Win+V vérifié : le texte ordinaire y est, le texte copié seul n'y est pas (2 éléments lus) », test vert. *(Modifié par la relecture, § Relecture I4 : le test est désormais `#[ignore]`, remet le presse-papiers et échoue au lieu de se sauter ; il se lance à la demande dans la session de l'auteur.)*

### Tests

- **Mac** : 893 tests Rust (3 ignorés), 176 tests de l'interface ; `cargo fmt --check`, `clippy --workspace --all-targets -D warnings`, `npm run check` sans erreur.
- **PC Windows** (`cargo test --workspace`, SSH) : 877 tests Rust passés, 0 échec, 1 ignoré. Un test de `scriibe-platform --lib` a échoué au premier passage (lancé par `pilote compiler --mode tests`, qui n'imprime pas le détail) puis réussi aux deux relances : échec ponctuel, test non identifié (le piège de `instance::tests::a_second_instance_is_refused_until_the_first_ends` est déjà noté au journal 0.11.0) ; à surveiller.
- Clippy pour la cible `x86_64-pc-windows-gnu` : impossible sur ce Mac (pas de `x86_64-w64-mingw32-gcc`) ; le code Windows est compilé sur le PC même.

### Reste pour la tâche 2 et la tâche 3

- Relecture du code ci-dessus par un sous-agent (tâche 2), en particulier le rattrapage du crochet (`GetAsyncKeyState` à l'appui) et le fait que le journal n'écrit rien de dicté ni de tapé.
- Essai à la main des gestes sur le PC (tâche 3) : 10 essais de « tenir Ctrl droit, cliquer sur Scriibe, relâcher » ; appui sur Ctrl droit au premier essai juste après ; AltGr dans la page des touches (nom écrit une fois, ligne de refus au journal).

## Relecture

*(Tâche 2, 19/11/2026. Sous-agent relecteur Sonnet · Élevé, qui n'a pas écrit le code, sur `0f7b325..8ad156e`, lecture seule ; détail et scénarios dans son rapport du dossier temporaire de la session. Résultat : 0 bloquant, 4 importants, 5 mineurs.)*

### Importants, tous corrigés

| # | Défaut | Correction | Test |
|---|---|---|---|
| I1 | `handle_page_key` jetait tout relâchement relayé par la page si le crochet avait parlé moins d'1 s avant : un appui court (tenir Ctrl droit, cliquer sur Scriibe, relâcher en moins d'1 s) laissait la pastille bloquée ; le test de la tâche 1 contournait le cas par une pause de 1,1 s | Dans la fenêtre d'écho, le relâchement d'une **touche de la dictée en cours** est appliqué (un doublon est inoffensif : la dictée est déjà finie) ; appuis et autres touches restent ignorés (`controller.rs`) | `a_short_press_released_in_the_page_within_a_second_ends_the_dictation` (à 300 ms, touche étrangère, appui doublon) |
| I2 | Rattrapage `GetAsyncKeyState` : une touche retenue par un autre crochet (PowerToys, AutoHotkey, bureau à distance) paraît relâchée au système pendant qu'elle est tenue ; chaque répétition automatique serait devenue « relâchement + nouvel appui » (dictée coupée et relancée en boucle). Spéculatif (pas de ces logiciels sur le PC), mais sans coût à éviter | Le rattrapage exige en plus que le dernier événement de la touche soit plus ancien que **1,2 s** (délai de répétition maximal de Windows : 1 s) : `release_was_missed` (`keys.rs`), heure `info.time` par touche tenue (`windows/keyboard.rs`) | `a_missed_release_needs_the_system_to_say_up_and_a_gap_longer_than_an_auto_repeat` (dont le passage du compteur par zéro) |
| I3 | La ligne « capture des touches: refusée » écrivait au journal les lettres, chiffres, Espace et Entrée tapés pendant la capture (`label_of`) : contraire à la règle « le journal ne dit jamais ce qui est tapé » | `keys::loggable_label` : seules les touches de la liste sont nommées, les autres deviennent « une touche hors liste » ; le message de refus n'est recopié que si toutes les touches sont de la liste ; l'écran garde les noms (`web/mod.rs`) | `the_journal_names_listed_keys_only` |
| I4 | `windows_clipboard.rs` écrasait le presse-papiers de l'auteur à chaque `cargo test --workspace` sans le remettre, pouvait finir vert en se sautant (session absente), avec une marque « absente » faute d'avoir pu ouvrir le presse-papiers, ou sans conclure sur Win+V (historique désactivé, témoin absent) | `#[ignore]` (lancé à la demande, `docs/TESTS.md`) ; texte d'avant remis par une garde `Drop`, même en cas d'échec ; `mark()` panique si le presse-papiers ne s'ouvre pas ; session absente, historique désactivé ou témoin absent : le test **échoue** (« RESULTAT WIN+V : NON VERIFIE ») au lieu de passer ; la preuve est la ligne « RESULTAT WIN+V : VERIFIE » | Le test lui-même (Windows seulement) |

### Vérifié sans défaut

Verrous (`page_held` puis `state` : jamais l'inverse, pas d'interblocage) ; `GetAsyncKeyState` appelé sans verrou ni attente dans le crochet ; `cfg(windows)` (rien n'atteint Mac et Linux) ; lignes du journal « écoute longue » et « annulation » (durées, nombres, nom de l'exécutable : rien de dicté ni de tapé) ; aucune chaîne d'interface sans traduction ; ni nom ni courriel de l'auteur dans le diff. Les tests du pipeline échouent sans les correctifs de la tâche 1.

### Mineurs restants (non corrigés)

- **M1** `ui/src/lib/keyrefusal.ts` : en anglais, pour une combinaison, le nom est traduit partie par partie alors que le message l'est en bloc : le préfixe n'est pas retiré et le nom est écrit deux fois, à moitié en français. Cosmétique.
- **M2** Rattrapage Windows : `CommandMenuMask` (Alt gauche, Windows) n'est pas informé du relâchement rattrapé ; le masquage du menu peut suivre l'ancien drapeau « utilisée » (une touche sans effet en trop ou en moins).
- **M3** Couverture du repli de `handle_page_key` : le second volet de `a_release_seen_only_by_the_page_is_ignored_without_a_dictation` (touche inconnue) ne teste pas la garde du contrôleur ; manquent le relâchement de Maj (traduction) pendant la dictée et une dictée à combinaison (Ctrl + F9) dont une seule touche est relâchée.
- **M4** Linux GNOME Wayland : la page relaie maintenant tout relâchement, même sans appui vu, vers `wayland_release_hint` ; par exemple le relâchement d'Alt après Alt+Tab vers Scriibe pendant qu'un raccourci est tenu le coupe. Voulu pour l'essentiel (dictée tenue 5 min), non documenté, à regarder à l'essai Linux (tâche 3).
- **Limite de I2** : un nouvel appui sur la touche de dictée **moins de 1,2 s** après son dernier événement, quand son relâchement a été manqué, est encore pris pour une répétition (un appui perdu) ; ce délai court ne laisse de toute façon pas de dictée utile.
- **Note** : `announce_page_keys` journalise « la page relaie les touches » aussi quand le relâchement vient du repli de la fenêtre d'écho (une fois par lancement).

### Tests

- **Mac** : `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` sans erreur après les corrections.
- **PC Windows** (code envoyé par `git bundle` sur une branche `essai-c7`, supprimée ensuite, PC remis sur `main` `276f4d6`, bundle effacé) : `cargo clippy --workspace --all-targets -- -D warnings` sans avertissement ; `cargo test --workspace` sans échec (le test Win+V est ignoré : 1 ignoré de plus). **Le test Win+V modifié et le rattrapage du crochet ne sont compilés que là ; le test n'a pas été relancé dans la session de l'auteur** (tâche 3 : `-- --ignored --nocapture`, attendre la ligne `RESULTAT WIN+V : VERIFIE`).

## Essai

*(Tâche 3, 19/11/2026. Code `666063e` (local, non poussé), envoyé par `git bundle` sur une branche `essai-c7` des deux PC, compilé en release par `pilote compiler tous --mode release`, branches supprimées et PC remis sur `main` ensuite. Windows : copie des données d'essai `C:\essai-enq\donnees`, lancée dans la session de l'auteur par une tâche planifiée temporaire (supprimée). Mac : `Scriibe.app` recompilée et réinstallée (`tools/faire_app_macos.sh`), lancée par `open`. Linux : lancée par Activités. L'auteur seul aux gestes ; les résultats viennent de ses réponses et des journaux. Aucune capture d'écran (la pastille a été jugée par l'auteur).)*

| Cas | Système | Résultat | Preuve |
|---|---|---|---|
| Bug 1 : tenir Ctrl droit, cliquer sur Scriibe, relâcher, ×10 | Windows | **10/10 bons** : pastille jamais bloquée, chaque appui suivant a lancé la dictée (rattrapage du crochet non perdu) | journal : 10 appuis, 10 relâchements, 10 collages, 0 annulation, aucune ligne « écoute longue » |
| Bug 2 : AltGr dans « Changer… » de la touche de dictée, ×10 | Windows | **10/10 bons** : « AltGr (Alt droit) ne peut pas lancer la dictée : elle sert à taper », nom écrit une seule fois, aucun autre message | journal : 10 lignes « capture des touches: refusée (dictation) … » (noms de touches seulement) |
| Test Win+V (`windows_clipboard`, `-- --ignored`) | Windows | **vérifié** : le texte ordinaire est dans l'historique, le texte copié seul n'y est pas (4 éléments lus), presse-papiers remis | ligne « RESULTAT WIN+V : VERIFIE » |
| « Lancer à l'ouverture de session » | Windows | **marche** : déconnexion puis reconnexion, Scriibe et `whisper-server` démarrés seuls à 11:38:13 | processus et entrée `HKCU\…\Run` lus par SSH ; entrée retirée ensuite |
| Dictée (Option droite) | Mac | **comme avant** : pastille, texte collé dans Notes | journal |
| Commande (Ctrl gauche : « mettre entre guillemets ») | Mac | **comme avant** : texte sélectionné entouré de guillemets | gestes de l'auteur |
| Dictée (Ctrl+Gauche, raccourci GNOME) | Linux | **comme avant** : pastille, texte collé | gestes de l'auteur |
| Commande (Ctrl+Bas : « mettre entre guillemets ») | Linux | **comme avant** : « sélection illisible », rien collé (GNOME Wayland ne lit pas la sélection : même ligne au journal du 18/11) | journal : `commande: entourer.guillemets ignorée (sélection illisible)` |

### À savoir

- **Case « Lancer à l'ouverture de session » grisée** au premier essai : voulu, Scriibe refuse le démarrage automatique quand elle est lancée depuis un dossier `target\` ou un dépôt git (`launch_location_problem`). Il a fallu la copier hors du dépôt (`C:\essai-enq\prog`, supprimée ensuite). Une fois lancée ainsi, la case se coche, `Scriibe` entre dans `Run` (`"…\scriibe.exe" lancer --sans-navigateur`) et n'est pas désactivée par Windows. Au démarrage automatique, Scriibe prend les données **normales** de l'utilisateur (pas la variable `SCRIIBE_DATA` de l'essai).
- **Non essayé** : fenêtre administrateur au premier plan ; le point M4 de la relecture (Linux Wayland, relâchement d'Alt après Alt+Tab pendant un raccourci tenu) ; Mac et Linux n'ont pas pu confirmer le rattrapage du crochet (code `cfg(windows)`), seulement l'absence de changement visible.
- **Collage vers Scriibe** : pendant les 10 essais du bug 1 l'auteur est resté sur la fenêtre de Scriibe, donc les textes sont partis dans Scriibe et non dans le Bloc-notes (sans conséquence pour la pastille ni le crochet).
- **Mac** : la recompilation redemande les trois accès au trousseau (connu).

### Ajouts

Aucune nouvelle demande de l'auteur pendant l'essai.

## Relais

*(Fin de la tâche 1, 19/11/2026, contexte à 30 % : relais décidé par la règle du `CLAUDE.md` (25 % ou plus en fin de tâche : la tâche suivante part d'une nouvelle session). Ne change ni l'ordre ni les consignes.)*

- **Fait** : v0.12.0 publiée (Release avec zips Mac, Windows, Linux, `b9806e6`) puis correctif clippy Linux poussé (`276f4d6`, CI verte) ; tâche 0 (`0f7b325`) ; tâche 1 (`16b595d` code, `e0716b7` § Correctifs). **3 commits locaux non poussés** (`0f7b325`, `16b595d`, `e0716b7`). PC Windows remis sur `main` (`276f4d6`), tâches planifiées d'essai supprimées, `C:\essai-enq` laissé (contient une jonction vers les modèles : ne pas le supprimer en récursif). L'**historique du presse-papiers de Windows (Win+V) a été activé par l'auteur** pour l'essai ; il peut le laisser ou le couper.
- **En cours** : rien ; arbre propre. `CLAUDE.md` met le chantier 7 en découpage en cours.
- **Prochain pas** : tâche 2 du découpage (relecture par un sous-agent Sonnet · Élevé qui n'a pas écrit le code, sur `16b595d` et `e0716b7` ; corriger bloquants et importants ; § Relecture ; commit local ; rapport du sous-agent de 5 lignes au plus, détail en fichier du dossier temporaire). Points à lui faire regarder : le rattrapage du crochet par `GetAsyncKeyState` à l'appui (`crates/platform/src/windows/keyboard.rs`), `Controller::handle_page_key` (verrous `page_held` puis `state`), le journal qui n'écrit rien de dicté ni de tapé (captures refusées, écoute longue, annulation), le test `windows_clipboard.rs`.
- **Pièges** : `cd ui` change le dossier courant du shell (chemins absolus) ; PowerShell 5 écrit un BOM avec `Set-Content -Encoding UTF8` (Scriibe juge alors `settings.json` illisible) ; par SSH le presse-papiers et l'historique Win+V ne sont pas joignables (le test se saute), il faut une tâche planifiée `/IT` dans la session de l'auteur (compte `DESKTOP-A0OFLLO\HP`, `-AllowStartIfOnBatteries`) ; code non poussé envoyé au PC par `git bundle` (`git fetch <bundle> HEAD:essai-c7`) ; `pilote compiler --mode tests` n'imprime pas le détail d'un échec : relancer `cargo test` par SSH ; pas de `x86_64-w64-mingw32-gcc` sur le Mac (pas de clippy Windows local).
- **Mineurs restants** : tolérance au BOM de `settings.json` (non traité) ; échec ponctuel non identifié de `scriibe-platform --lib` sous Windows ; fenêtre administrateur au premier plan non essayée ; `CLAUDE.md` à 8 400 signes (seuil R03 : 8 000).

*(Fin de la tâche 2, 19/11/2026 : relecture faite, importants corrigés, commit local « tâche 2 » (cinq commits locaux non poussés : 0f7b325, 16b595d, e0716b7, 8ad156e et celui-ci). Prochain pas : tâche 3, nouvelle session B. Piège : le test Win+V est maintenant `#[ignore]`.)*
