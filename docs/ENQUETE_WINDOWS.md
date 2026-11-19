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

**Résultat sur le PC de l'auteur** (dans sa session, par une tâche planifiée temporaire supprimée ensuite) : historique désactivé au premier passage → marques vérifiées, historique « non vérifié » ; l'auteur l'active (réglage de son PC, à lui de le laisser ou non) ; second passage : « historique Win+V vérifié : le texte ordinaire y est, le texte copié seul n'y est pas (2 éléments lus) », test vert. Sur l'intégration continue ou en SSH, l'essai se saute (pas de session) : il se lance dans la session de l'auteur.

### Tests

- **Mac** : 893 tests Rust (3 ignorés), 176 tests de l'interface ; `cargo fmt --check`, `clippy --workspace --all-targets -D warnings`, `npm run check` sans erreur.
- **PC Windows** (`cargo test --workspace`, SSH) : 877 tests Rust passés, 0 échec, 1 ignoré. Un test de `scriibe-platform --lib` a échoué au premier passage (lancé par `pilote compiler --mode tests`, qui n'imprime pas le détail) puis réussi aux deux relances : échec ponctuel, test non identifié (le piège de `instance::tests::a_second_instance_is_refused_until_the_first_ends` est déjà noté au journal 0.11.0) ; à surveiller.
- Clippy pour la cible `x86_64-pc-windows-gnu` : impossible sur ce Mac (pas de `x86_64-w64-mingw32-gcc`) ; le code Windows est compilé sur le PC même.

### Reste pour la tâche 2 et la tâche 3

- Relecture du code ci-dessus par un sous-agent (tâche 2), en particulier le rattrapage du crochet (`GetAsyncKeyState` à l'appui) et le fait que le journal n'écrit rien de dicté ni de tapé.
- Essai à la main des gestes sur le PC (tâche 3) : 10 essais de « tenir Ctrl droit, cliquer sur Scriibe, relâcher » ; appui sur Ctrl droit au premier essai juste après ; AltGr dans la page des touches (nom écrit une fois, ligne de refus au journal).
