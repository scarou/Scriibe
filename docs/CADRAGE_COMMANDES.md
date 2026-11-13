# Cadrage des commandes vocales étendues (chantier 6)

Cette note prépare deux idées du carnet du 11/11, ainsi que deux idées de la ROADMAP :

- « Reformuler complètement la sélection en cours » (« rends ça plus amical ») ;
- « Mode macro vocal étendu » (créer un mail, ouvrir une application, sans sélection) ;
- ROADMAP n°4 des idées du 03/11 : extraits à raccourcis vocaux (« insérer mon adresse email ») ;
- ROADMAP n°7 des idées du 03/11 : commande de recherche sur internet (« recherche YouTube … »).

C'est un cadrage seulement : on ne code rien. On découpera ensuite l'implémentation à partir de cette note. Le cadrage part de la couche « verbe + objet » du 07/11 (S18, `docs/COMMANDES.md`), qui a posé deux principes : pas de modèle, et seulement sur une sélection. La reformulation a besoin d'un modèle. Les macros, les extraits et la recherche se passent de sélection. Le cadrage dit donc si l'on garde ces deux principes.

Sections :

- Constat (tâche 0)
- Cadrage (tâche 1)
- Relecture (tâche 2)

## Constat

Le constat a été fait le 13/11 par un sous-agent de recherche, qui n'a rien modifié. Il a lu le code, les tests et la doc de Scriibe (état de fin S25), ainsi que le code Swift et la ROADMAP de Scribe. Il n'a ouvert aucun vrai fichier de réglages, aucun journal réel et aucune clé. Les exemples ci-dessous sont reconstruits d'après le code. Quelques références ont été recontrôlées : `try_command` (`controller.rs:703`), `builtin` (`commands.rs:271`), `recognize` (`commands.rs:469`), `MIN_WORDS` et `open` (`open.rs:18`). Ce qui n'a pas été vérifié est signalé « supposé ».

Le résumé tient en cinq points :

- La couche « verbe + objet » marche déjà sans sélection et avec un texte libre derrière la phrase : les types sont prêts (`Needs::Nothing`, `Argument::Rest`). Mais aucune commande livrée ne s'en sert, et le chef d'orchestre n'a aucune branche pour elles.
- Aucun canal ne permet aujourd'hui d'envoyer au modèle « une consigne + un texte ». Le prompt de mise au propre refuse explicitement d'exécuter ce qu'on lui dicte. Reformuler une sélection demandera un second prompt, comme celui de la traduction.
- La sélection n'est jamais envoyée à un modèle, ni écrite dans le journal ou l'historique. Mais le moteur de mise au propre par défaut est OpenRouter, qui est en ligne.
- Linux ne sait ni lire la sélection ni envoyer un raccourci. Ouvrir une URL ou une application passe par une seule fonction, `open::open`, qui ne valide rien.
- Scribe n'a ni commande vocale, ni macro, ni extrait, ni recherche, ni reformulation de la sélection. Ces idées ne sont que dans sa ROADMAP (n°12, 12 bis, 12 ter, 13, 33).

### 1. Couche « verbe + objet »

**Doc.** `docs/COMMANDES.md` contient le constat, la conception validée (principe, 8 cas, 6 points), les tâches, la relecture et l'essai réel. Son principe annonce déjà que les extraits (n°4) et la recherche (n°7) réutiliseront la couche, et que ces deux cas « n'exigent pas de sélection ».

**Catalogue.**

- Il est codé en dur dans `commands::builtin()` (`crates/core/src/commands.rs:271`). Chaque fiche est un `CommandDef { id, needs, argument, phrases_fr, phrases_en }`.
- `needs` vaut `Selection` ou `Nothing`, et `argument` vaut `None` ou `Rest` (l.20-34).
- Commandes livrées, toutes avec `needs = Selection` :
  - `dictionnaire.ajouter` ;
  - `entourer.{parentheses, guillemets, crochets, accolades}` ;
  - `format.{gras, italique, souligne}`.
- Phrases par défaut en français et en anglais.
- Le réglage `voiceCommands` (`VoicePhrases`, rangé par langue puis par id) remplace les phrases par défaut. L'interrupteur général est `voiceCommandsEnabled`, activé par défaut (`crates/core/src/settings.rs:339-348`). Les raccourcis de mise en forme se règlent dans `formatShortcuts` (défaut `Mod+B`, `Mod+I`, `Mod+U`).
- « Mes commandes » (`customCommands`, type `CustomCommand`, ids `perso.<n>`, ajoutées par `all_defs()`, l.234) permet à la personne de créer une phrase qui appuie **un raccourci** de l'appli active, avec ou sans sélection. C'est aujourd'hui la seule forme de macro.
- Interface : carte `ui/src/components/VoiceCommands.svelte` dans l'onglet Dictionnaire. Routes `GET api/commands`, `POST api/commands/validate-phrase` et `validate-shortcut`.

**Comparaison.**

- `normalize()` (l.436), `words()` (l.397) et `fold_char()` (l.416) font quatre choses :
  - passer en minuscules ;
  - retirer les accents (œ devient oe) ;
  - couper sur tout caractère non alphanumérique ;
  - donc faire disparaître ponctuation, apostrophes et tirets.
- Aucune tolérance floue : `recognize` (l.469) compare mot à mot, exactement.
- La phrase doit être **toute** la transcription. Une fiche `Argument::Rest` accepte « phrase + reste non vide ».
- La phrase la plus longue l'emporte, et une phrase de moins de 2 mots est refusée (`MIN_WORDS`).
- Le catalogue suit la langue de dictée (`catalogues()`, l.456), ou prend les deux si la dictée est en plusieurs langues.
- Les erreurs de Whisper s'absorbent en ajoutant des phrases (« mais en gras », « mettre en souligner »).

Exemple : « Mets en gras. » devient `mets en gras` et correspond à `format.gras`. Dans le test `commands_extra_argument…`, « recherche YouTube chats noirs » correspond à une fiche fictive `Rest`, avec le reste `chats noirs`.

**Déclenchement** (`crates/pipeline/src/controller.rs`).

- `process` (l.812) appelle `try_command` (l.703) juste après le filtre des phrases inventées. L'appel se fait sur le texte brut, **avant** le dictionnaire et la mise au propre.
- `try_command` appelle `commands::recognize_for(raw, settings)` (`commands.rs:506`).
- La sélection n'est lue **qu'après une correspondance**, par `paster.selected_text()`, et seulement si `needs == Selection`. Une dictée ordinaire ne lit jamais la sélection par ce chemin (test `commands_101b`).
- Aiguillage par id :
  - `format.*` et `perso.*` appuient un raccourci ;
  - `entourer.*` colle le texte entouré ;
  - `dictionnaire.ajouter` passe par `CommandSink` ;
  - un id inconnu tombe dans la branche `other` : le journal note « inconnue du chef d'orchestre » et la phrase est collée comme du texte.
- **Aucune branche n'existe pour une commande à argument (recherche) ni pour un extrait.** Il faudra un nouveau préfixe d'id, de nouvelles branches et une nouvelle sortie, sur le modèle de `CommandSink`.

**Rien ne correspond, ou la commande échoue.**

- Rien ne correspond : la phrase suit le chemin d'une dictée ordinaire. Elle est collée et **remplace la sélection** s'il y en a une. ⌘Z restaure la sélection.
- Commande qui exige une sélection, mais il n'y en a pas : le journal note « commande: X ignorée (aucune sélection) — la phrase est collée comme du texte », sans message à la personne.
- Sélection illisible : la phrase est collée, avec une note de pastille à texte fixe (Accessibilité manquante, presse-papiers non textuel, sélection illisible).

**Garde-fous.**

- La phrase doit être toute la transcription et compter au moins 2 mots.
- Sélection exigée.
- Interrupteur général.
- La page refuse une phrase déjà prise ou trop courte (`validate_phrase`, l.515), mais seulement à la page, pas à l'enregistrement des réglages.
- Raccourcis de « Mes commandes » : ⌘ ou Ctrl exigé, et ⌘Q, ⌘⇧Q, ⌃⌘Q refusés.
- Ajout au dictionnaire : la sélection est limitée à 80 signes sur une ligne.
- Faux positif connu : quand Whisper entend mal la phrase, elle est collée à la place de la sélection (essai réel, Notes).

**Annulation.**

- Entourer : un seul collage, donc un seul ⌘Z. Exception : Chrome sur Mac, où le texte est tapé et demande plusieurs ⌘Z (essai du 10/11).
- Mise en forme : c'est le raccourci de l'appli, donc un seul ⌘Z.
- Dictionnaire : rien à annuler.

**Tests.**

- 22 tests `commands_*` dans `crates/core/src/commands.rs`. Parmi eux :
  - `commands_004`, une fiche fictive `Needs::Nothing` « extrait.insérer » ;
  - `commands_extra_argument…` ;
  - `commands_extra_the_longest_phrase_wins`.
- Dans `crates/pipeline/tests/dictation.rs` : `commands_101` à `110`, `201` à `210` et `301` à `311`, avec `FakePaster::set_selected`.
- `commands_110` et `209` injectent « SECRET-123 » et vérifient qu'il n'apparaît ni dans le journal, ni dans l'historique, ni dans la pastille, ni dans les requêtes.
- **Aucun cas dans `fixtures/`** : les commandes sont propres à Scriibe et n'ont pas de référence Scribe.
- Non couvert : un essai de bout en bout passant par l'aiguillage `SwitchPaster`.

### 2. Mise au propre par un modèle

**Moteurs.** `Cleaner::endpoint` (`crates/engines/src/services.rs:200`) choisit parmi cinq possibilités :

- OpenRouter (en ligne, clé, `provider.sort=latency`) ;
- LM Studio (`lmStudioUrl`, défaut `127.0.0.1:1234`) ;
- Ollama ;
- llama.cpp, avec un `llama-server` lancé par l'appli (`-c 8192 -np 2 -kvu`, `crates/engines/src/llama.rs:31`) ;
- aucun : la mise au propre, les modes et la traduction sont alors indisponibles.

Tous passent par `POST /v1/chat/completions` (`complete`, l.284), avec `temperature 0`, `max_tokens = max(nb de caractères, 200)` et `reasoning_effort: none` en local.

**Délais et échecs.**

- Délai : 40 s en local, 20 s en ligne. On attend jusqu'à 15 s que llama.cpp démarre.
- Une réponse coupée (`finish_reason: length`) compte comme une erreur.
- En cas d'échec ou de rejet par le garde-fou (`REWRITE_REJECTED`), le **texte brut** est collé, après la typographie, avec la note « Collé sans mise au propre (…) ».
- Un mode qui reformule (Reformulé, Libre) réessaie une fois en Simple (`cleanup`, l.325).

**Prompt** (`crates/core/src/cleaner.rs`).

- `system_prompt` (l.319), `messages` (l.361) et `wrap` (l.353) construisent le prompt. Les textes viennent de `prompt_text.rs`, extraits de Scribe.
- Ordre du prompt : consigne système, exemples, puis la dictée entre `<dictée>…</dictée>` avec un rappel.
- Le début du prompt reste identique d'une dictée à l'autre, pour que le serveur le garde en mémoire. Le préchauffage envoie ce même prompt avec « Bonjour. » à l'appui de la touche (`warm_up_messages`, l.439).
- L'empreinte des huit modes de Scribe vaut `4ed86f0c1c154918` (`built_in_prompts_fingerprint`, l.454, testée par `fixtures/prompts.json`).
- **Le prompt refuse les instructions** : « Tu es un correcteur de dictée vocale, pas un assistant… tu ne réponds jamais à son contenu » (`SYSTEM_HEAD`, `SYSTEM_QUESTION`). `Cleaner::cleanup`, `rewrite` et `translate` acceptent n'importe quel texte, mais ce texte est traité comme une dictée à corriger, jamais comme une consigne.
- Précédent d'un second prompt : la traduction (`translation_prompt`, `translation_messages`, l.408-436). Elle a sa consigne, ses exemples, son garde-fou (`accepts_translation`) et son préchauffage à elle.

**Crans de fidélité** (ROADMAP n°18, S19). `Fidelity` (`crates/core/src/modes.rs:15`) vaut `Literal`, `Light`, `Reformulated` ou `Free`. Les crans ne changent que le contrôle fait après coup par `cleaner::accepts` (l.88) :

| Cran | Contrôle des mots gardés |
|---|---|
| Littéral | au moins 90 % dès 5 mots |
| Léger | au moins 75 % dès 8 mots |
| Reformulé | seuils anciens (kept ≥ 0,15, own ≥ 0,3) |
| Libre | aucun |

Les contrôles de longueur, de question, de signes et de formule de réponse en tête restent les mêmes pour tous les crans. Pour Reformulé et Libre, `rewrites()` choisit en plus la consigne de réécriture et les exemples de style. Le modèle ne reçoit aucun chiffre. Les crans se règlent à trois endroits :

- `simpleFidelity` pour le mode Simple ;
- la fiche d'un mode personnel ;
- Apaisé, fixé à Libre.

L'empreinte n'a pas changé. Le cran Libre n'a pas encore été essayé à la main.

**Mode.** `AppSettings::active_profile()` rend un `ModeProfile` : style, `rewrites`, vouvoiement, genre, exemples. `messages` et `accepts` s'en servent. Le mode Simple de base peut se passer du modèle (3 mots ou moins, ou « sauter si déjà propre »).

### 3. Journalisation

**Journal** (`scriibe.log`, `FileLog`, `crates/platform/src/storage.rs`, rotation à 512 Ko).

- Il contient : étapes, durées, niveau du micro, moteur, modèle, mode, appli, nombre de signes collés, état de la sélection. Pour une commande, il note seulement l'id et la longueur de la sélection.
- Règle écrite dans le code : « jamais le texte dicté ni les clés ». Le sous-agent n'a trouvé aucun texte de la personne écrit au journal dans les chemins lus. Les erreurs de `ServiceError` (`crates/engines/src/http.rs:108`) sont des textes fixes.
- Non vérifié : `llama-server.log`, la sortie d'erreur de llama.cpp, qui pourrait contenir du texte selon sa verbosité.

**Historique** (`history.json`, `history-totals.json`, `FileHistory`).

- `Dictation` (`crates/core/src/history.rs:55`) garde `rawText`, `finalText`, l'appli, les durées, le mode et la traduction.
- `keepDictationText` est vrai par défaut. La conservation est illimitée par défaut (`historyKeepDays`, borné de 2 à 3 650 jours, `retention.rs`).
- **Une commande n'écrit rien dans l'historique** : `try_command` rend `Handled` avant l'enregistrement.

**Ce qui part en ligne.**

- L'audio part vers Deepgram, moteur de transcription par défaut.
- La dictée et le vocabulaire du dictionnaire partent vers OpenRouter, moteur de mise au propre par défaut.
- Les moteurs locaux ne sortent rien de la machine.
- La sélection ne part aujourd'hui vers aucun service.

### 4. Ce qui dépend du système (`crates/platform`)

| Fonction | macOS | Windows | Linux |
|---|---|---|---|
| Lire la sélection | Accessibilité (`AXSelectedText`, `macos/ffi.rs:194`), puis ⌘C d'essai (`macos/paste.rs:364`, texte remis ensuite, rien si image ou fichiers) | Ctrl+C d'essai (`windows/paste.rs`, `probe_selection`, numéro de séquence du presse-papiers, rien dans une console) | **absente** (`X11Paster` n'a que `paste` et `copy` ; le trait rend « sélection illisible ») |
| Coller | ⌘V ; frappe du texte dans Chrome et dérivés | Ctrl+V par `SendInput` | X11 : XTest Ctrl+V ; Wayland GNOME : portail `RemoteDesktop` (`linux/wayland.rs:178`, Ctrl+V seulement) |
| Envoyer un raccourci | `send_shortcut` | `press_shortcut` (`SendInput`) | **non pris en charge** (`SHORTCUT_UNSUPPORTED`) |
| Ouvrir une URL, un dossier, une appli | `open` | `cmd /C start "" <cible>` | `xdg-open` |

- `crates/platform/src/open.rs::open(target)` (l.18) est la seule fonction d'ouverture. Une adresse en `http://127.0.0.1:` s'ouvre dans la fenêtre de Scriibe. Toute autre cible passe à la commande du système, sans contrôle du schéma. Elle ne reçoit aujourd'hui que l'interface locale, les dossiers de données et le journal.
- **Rien n'est codé pour `mailto:`, une URL web externe ou une application.** Aucun accès au client de messagerie (ni AppleScript, ni MAPI).
- Supposé : `open`, `xdg-open` et `start` avec `mailto:` ouvriraient la messagerie par défaut.
- Sur Mac et Windows, `Paster::selection()` est appelé à **chaque** dictée, après la mise au propre, pour retirer la majuscule et le point final. Sous Windows, cela fait un Ctrl+C d'essai à chaque dictée.
- Wayland : on ne connaît pas l'appli au premier plan, une touche de modification seule ne marche pas (`docs/PIEGES.md`), et seul Ctrl+V passe par le portail. Supposé : le portail pourrait envoyer d'autres touches (`NotifyKeyboardKeysym`), et X11 pourrait lire la sélection « primaire ». Rien de cela n'est codé.
- ROADMAP n°3 : la lecture de la sélection est faite sur Mac et Windows. Reste Linux.

### 5. Scribe

- **Aucun code** de commande vocale, de macro, d'extrait, de recherche ou de reformulation de la sélection. `Paster.swift` ne fait que coller et copier. Il sauve et remet **tous les types** du presse-papiers (l.69-78) et le marque transitoire (`org.nspasteboard.TransientType`, l.21-22). Scriibe sur Mac ne remet que le texte.
- Les idées se trouvent seulement dans la ROADMAP de Scribe :
  - n°33 : extraits à raccourcis vocaux, sans modèle, carte « Extraits », extrait seul ou au milieu d'une phrase ;
  - n°12 : macro-commandes (ouvrir une appli, lancer un raccourci) ;
  - n°12 bis : commandes vocales ;
  - n°12 ter : mode commande sur la sélection avec un modèle qui réécrit (« rends ça plus amical »), touche dédiée, garde-fou propre, ⌘Z, moteur local ou en ligne car la sélection peut être confidentielle ;
  - n°13 : touche de recherche directe.
- S18 a écarté le modèle pour le n°12 ter. Le sous-agent n'a pas trouvé de ligne sur ce choix dans `docs/ECARTS.md` (à confirmer).

### 6. ROADMAP de Scriibe et existant

Idées du 03/11 :

- **n°4** : « Extraits à raccourcis vocaux (Scribe n°33) : "insérer mon adresse email" ; carte "Extraits", reconnaissance tolérante. »
- **n°7** : dire « recherche YouTube … » ou « recherche Google … » pour lancer la recherche dans le navigateur. Commandes réglables dans l'interface. À cadrer avec le mode commande (n°6) : même déclencheur « verbe + objet », sans modèle.
- **n°6** : mode commande sur la sélection. Fait en S18 sur Mac. Reste Windows et Linux (n°3), et le transfert vers un modèle.

Prochaines étapes :

- **n°18** : crans de fidélité par mode. Codé en S19, reste l'essai à la main de Littéral et Libre. « Le cran Libre est la Réécriture libre du n°16. »
- **n°20** : commandes sur sélection portées sous Windows, essayées le 10/11.

Ce qui existe déjà en code :

- Extraits et recherche : seulement des tests et des commentaires (`commands.rs:7`, `:23`, `:579`, `:640`), plus `Argument::Rest`. Ni fonction, ni carte, ni réglage.
- Macros, ouverture d'application, `mailto:` : rien.

### 7. Ce qui complique l'idée

1. **Décision du 07/11.** La reformulation contredit « sans modèle ». Les macros, les extraits et la recherche contredisent « seulement sur sélection ». Les fiches savent décrire ces cas, mais le chef d'orchestre ne sait pas les exécuter.
2. **Pas de canal « consigne + texte ».** Il faudra un second prompt, avec ses exemples et un garde-fou à lui, sur le modèle de la traduction. Le garde-fou actuel exige de garder des mots (sauf en Libre) et refuse les formules de réponse.
3. **Mémoire du serveur.** Un second prompt ne touche pas l'empreinte des prompts livrés, mais il n'est pas préchauffé. Supposé : le premier appel serait plus lent. llama.cpp n'a que deux emplacements (`-np 2`) et 8 192 jetons. Comme `max_tokens` suit la longueur du texte, une longue sélection peut être coupée, ce qui donne une erreur.
4. **Confidentialité de la sélection.** Le moteur par défaut est en ligne. Le ⌘C d'essai rend déjà la sélection visible quelques dixièmes de seconde aux gestionnaires de presse-papiers, et Windows peut la garder dans l'historique Win+V. Il faudra au moins un choix « local seulement » pour la reformulation.
5. **Linux.** Pas de lecture de la sélection, donc pas de reformulation : la phrase serait collée à la place de la sélection. Pas de raccourci envoyé, donc les macros fondées sur un raccourci ne font rien. Une commande personnelle sans sélection « avale la dictée » (limite connue de `docs/COMMANDES.md`).
6. **Remplacement silencieux.** Une commande mal entendue est collée à la place de la sélection. Une commande qui remplace la sélection par une reformulation aggraverait ce risque. Il faudra un aperçu, ou un ⌘Z garanti, sachant que Chrome sur Mac en demande plusieurs.
7. **Reconnaissance exacte.** Le texte qui suit une commande à argument est brut. Il faudra choisir l'encodage (`percent_encode`, `services.rs:21`, est privé et réservé à Deepgram) et une liste des sites de recherche autorisés.
8. **Ouverture non contrôlée.** Sous Windows, `open::open` passe par `cmd /C start`. Supposé : un `&` dans une URL de recherche pourrait être interprété par `cmd`. Il faudra une liste blanche pour les URL externes et pour les applications. Sur Wayland, l'appli au premier plan est inconnue.
9. **Données personnelles.** Le texte d'une requête ou d'un extrait ne doit aller ni au journal ni à l'historique. Le modèle à suivre existe déjà : id et longueur seulement, aucune entrée d'historique, messages fixes. Mais un extrait collé par le chemin ordinaire entrerait dans `rawText` et `finalText`, alors qu'il contient souvent une adresse, un IBAN ou une signature. Sous Windows, `write_text(.., transient)` exclut le collage de l'historique du presse-papiers, mais pas la sonde de sélection.
10. **Règles du dépôt.** Les commandes n'ont aucun cas dans `fixtures/` : c'est un écart voulu, faute de référence dans Scribe. Les nouvelles commandes demanderont des tests écrits à la main, une ligne dans `docs/ECARTS.md`, les traductions (`t('…')`, `en.ts`, `en-server.ts`) et une Aide en deux langues.
11. **Limites déjà connues** (relecture de la tâche 10 de `docs/COMMANDES.md`) :
    - Les éditeurs de code copient la ligne courante sans sélection, et le ⌘C d'essai la prend pour une sélection.
    - Une sélection identique au presse-papiers est vue comme « aucune sélection ».
    - `core/src/shortcut.rs` contient un `cfg!(target_os = "macos")` hors de `crates/platform`.
    - Le refus des phrases trop courtes ou en double ne se fait qu'à la page.
    - Un doublon entre les deux langues n'est pas détecté.
12. **Non vérifié.** Le détail des routes et de la carte de l'interface (repris de `docs/COMMANDES.md`), `llama-server.log`, le comportement réel d'`open` avec `mailto:` ou un `&`, la latence à froid de llama.cpp.

## Cadrage

Cadrage fait le 13/11 (tâche 1). Pour chaque question, j'ai proposé des options avec une recommandation. L'auteur a tranché ce qui touche à l'usage et à la sécurité (questions 1 à 5 et 7). J'ai tranché la question 6, qui est technique, et les détails techniques des autres. Aucun code n'est écrit ici.

### En bref

- Une **touche de commande** maintenue avec la touche de dictée fait de la dictée une commande. Elle fonctionne comme la touche de traduction. Sous cette touche, rien n'est jamais collé comme une dictée.
- Sous la touche :
  - recherche sur internet (Google, YouTube, Wikipédia, DuckDuckGo, plus les sites de la personne) ;
  - extraits ;
  - ouverture d'une application de la liste autorisée ;
  - nouveau mail ;
  - reformulation de la sélection par une consigne libre ;
  - les commandes d'aujourd'hui.
- La reformulation passe par un moteur local par défaut. Elle n'utilise un moteur en ligne que si la personne l'autorise.
- Aucune nouvelle commande n'écrit dans l'historique. Le journal ne garde que le genre de commande et des longueurs.
- On livre en quatre étapes : socle et recherches, puis extraits, puis applications et mail, puis reformulation.

### Ce qui change dans les décisions du 07/11

- **« Sans modèle »** : levé pour la **seule reformulation**. Elle a besoin de la touche de commande et d'une sélection, et passe par un moteur local par défaut. Toutes les autres commandes restent sans modèle.
- **« Uniquement sur sélection »** : levé pour les **recherches, extraits, applications et mail**, et seulement sous la touche de commande.
- **Les commandes d'aujourd'hui sans touche ne changent pas.** « Mets en gras » dit sans la touche de commande, sur une sélection, marche comme avant, avec le même interrupteur `voiceCommandsEnabled`. Les mêmes commandes marchent aussi sous la touche.

### Décisions

**1. Déclencheur : une touche dédiée** (décision de l'auteur : « touche dédiée »)

L'auteur a préféré la touche au mot d'appel. Avec la touche, aucun déclenchement par erreur n'est possible, et c'est le choix du n°12 ter de Scribe.

Fonctionnement :

- Nouveau réglage `commandKeys`. Il suit le modèle de `translateKeys` : une touche maintenue avec la touche de dictée au début de l'appui, choisie par « Changer… » dans Réglages → Dictée.
- La touche par défaut sera fixée à l'étape 1, après un essai sur les trois systèmes. Elle doit être différente des touches de dictée et de traduction, et respecter les limites connues : pas de touche de modification seule sous Wayland, et Fn invisible sous Windows.
- Si la touche de commande et la touche de traduction sont maintenues ensemble, la commande l'emporte et la traduction est ignorée.
- Sous la touche, la transcription brute passe par le filtre des phrases inventées. Elle ne passe ni par le dictionnaire ni par la mise au propre. Elle est ensuite comparée au catalogue complet (décision 4) avec la même normalisation qu'aujourd'hui (`normalize`).
- Une phrase d'un seul mot est permise sous la touche, puisqu'aucune dictée ordinaire n'y passe. `MIN_WORDS` ne vaut que hors de la touche.
- Ensuite :
  - une phrase du catalogue reconnue exécute la commande ;
  - si rien ne correspond, la sélection est lue. S'il y en a une, la phrase devient la consigne de reformulation (décision 2) ;
  - si rien ne correspond et qu'il n'y a pas de sélection, la pastille affiche « Commande inconnue : <mots entendus> ». **Rien n'est collé.** La pastille n'est pas journalisée.
- La sélection n'est lue que sous la touche, et seulement quand c'est nécessaire (commande sur sélection ou reformulation), comme dans `try_command` aujourd'hui.

**2. Reformulation de la sélection** (décisions de l'auteur : « consigne libre » ; « local, en ligne sur accord »)

- **Consigne.** Sous la touche, avec une sélection, tout ce qui n'est pas une commande du catalogue est la consigne : « rends ça plus amical », « raccourcis », « mets au vouvoiement », « traduis en anglais ».
- **Moteur** :
  - Par défaut, seulement un moteur local : llama.cpp, LM Studio ou Ollama, s'il est réglé comme moteur de mise au propre.
  - Nouveau réglage `allowSelectionOnline`, **décoché par défaut**, intitulé « Autoriser l'envoi de la sélection à un moteur en ligne ». Coché, la reformulation peut passer par OpenRouter.
  - Sans moteur autorisé : refus avec un message fixe, qui renvoie au réglage. Rien n'est envoyé.
  - Choix technique : le réglage appartient à l'ordinateur, comme les moteurs (cadrage des profils, décision 1).
- **Prompt** (choix technique) :
  - Un second prompt, à part, sur le modèle de la traduction (`translation_prompt`) : une consigne système fixe et quelques exemples, puis un message avec `<texte>…</texte>` et `<consigne>…</consigne>`.
  - Les prompts livrés et leur empreinte `4ed86f0c1c154918` ne changent pas.
  - Le début reste fixe pour la mémoire du serveur.
  - Préchauffage quand la touche de commande est appuyée avec un moteur local, comme pour la traduction.
- **Crans de fidélité (n°18) : non utilisés.** C'est la consigne qui dit à quel point changer le texte. Un seuil de mots gardés refuserait « raccourcis » ou « traduis ». Choix technique.
- **Garde-fou propre** `accepts_rewrite` (choix technique) :
  - réponse non vide ;
  - pas de formule de réponse ni de préambule (« Voici… ») ;
  - pas de balises ;
  - longueur au plus 3 fois celle de la sélection plus 300 signes ;
  - une réponse identique à la sélection donne le message « Rien à changer ».
- **Limites** (choix technique) :
  - sélection limitée à 4 000 signes : llama.cpp a 4 096 jetons par emplacement (`-c 8192 -np 2`) ;
  - `max_tokens` dimensionné sur 3 fois la sélection ;
  - délai de 40 s en local et 20 s en ligne, comme aujourd'hui.
- **Échec.** Rien n'est remplacé, la sélection reste intacte, et la pastille affiche « Reformulation impossible (…) » avec un texte fixe. Pas de nouvel essai dans un autre mode.
- **Annulation.** Le texte reformulé remplace la sélection en un seul collage. ⌘Z ou Ctrl+Z rend l'original. Il n'y a pas d'aperçu, comme dans le n°12 ter de Scribe. Exception connue : dans Chrome sur Mac, le texte est tapé et demande plusieurs ⌘Z. L'Aide le dira.

**3. Macros** (décisions de l'auteur : « liste autorisée » ; « objet dicté en option »)

- **Applications** :
  - La personne ajoute chaque application dans l'interface : une ou plusieurs phrases, et l'application choisie dans la liste des applications installées. Rien d'autre ne peut s'ouvrir.
  - Pas de confirmation. La pastille affiche « Ouverture de <nom> ».
  - **Aucune commande système, aucun script, aucun argument.**
  - Identifiant rangé (choix technique) : identifiant d'application sur Mac (bundle id), raccourci du menu Démarrer sous Windows (AppUserModelID ou chemin du `.lnk`), identifiant `.desktop` sous Linux. Le nom affiché est rangé aussi.
  - Une application devenue introuvable donne un message et ne lance rien d'autre.
- **Nouveau mail** :
  - « nouveau mail » (aussi « nouveau courriel », « new email ») ouvre `mailto:` dans la messagerie par défaut.
  - « nouveau mail réunion de lundi » préremplit l'objet (`mailto:?subject=…`, encodé).
  - Pas de destinataire, car Scriibe ne lit pas les contacts. Rien n'est envoyé.
  - Choix technique : le mail demande un argument facultatif, alors que `Argument::Rest` exige un reste non vide. On ajoutera une variante, ou deux fiches.
- **Ouverture** (choix technique). Une nouvelle fonction de `crates/platform`, distincte d'`open::open`, qui reste réservée aux usages internes :
  - elle n'accepte que `https:` et `mailto:` pour les adresses, et un identifiant de la liste pour les applications ;
  - elle **ne passe jamais par un interpréteur de commandes** : `ShellExecuteW` sous Windows au lieu de `cmd /C start`, et commande lancée avec ses arguments, sans shell, sur Mac et Linux (`open`, `xdg-open`, `gtk-launch`).

**4. Catalogue commun** (décisions de l'auteur : « touche de commande seule » pour les extraits ; les quatre sites)

Une seule carte « Commandes vocales » (onglet Dictionnaire), en plusieurs parties :

| Partie | Contenu | Réglable |
|---|---|---|
| Sur la sélection | Les commandes d'aujourd'hui (entourer, gras, italique, souligné, dictionnaire) | Phrases, comme aujourd'hui |
| Mes raccourcis | « Mes commandes » d'aujourd'hui (phrase → raccourci de l'appli active) | Oui |
| Recherches | Google (aussi « cherche … » seul), YouTube, Wikipédia (fr ou en selon la langue de dictée), DuckDuckGo, plus les sites de la personne | Phrases, et sites ajoutés |
| Extraits | Nom, phrases, texte (plusieurs lignes, au plus 5 000 signes) | Oui |
| Applications | Phrases → application de la liste | Oui |
| Mail | « nouveau mail » | Phrases |

- **Extraits.** Seulement sous la touche de commande. Le texte est collé tel quel, sans modèle, sans dictionnaire et sans typographie. L'extrait au milieu d'une dictée ordinaire (n°33 de Scribe) est écarté pour l'instant ; ce serait une étape à part, plus tard.
- **Recherches.** « recherche <site> <requête> » ouvre l'adresse du site dans le navigateur par défaut. `{q}` y est remplacé par la requête encodée (RFC 3986). Un site ajouté par la personne doit avoir une adresse `https://` contenant `{q}`. La règle est contrôlée à la page **et** à l'enregistrement des réglages : la relecture de la tâche 10 avait relevé que le refus des phrases ne se faisait qu'à la page.
- **Langues.** Les commandes livrées ont leurs phrases en français et en anglais, comme aujourd'hui. Les phrases que la personne écrit (extraits, applications, sites) sont essayées quelle que soit la langue de dictée : ce sont ses mots. Choix technique.
- **Rangement** (choix technique, d'après le cadrage des profils) :
  - nouveaux champs `snippets` et `webSearches` **à l'utilisateur**, groupe « Commandes » ;
  - `voiceApps`, `commandKeys` et `allowSelectionOnline` **à l'ordinateur** : les identifiants d'applications et les moteurs dépendent de la machine ;
  - le test de classement des champs les couvrira.

**5. Confidentialité** (décisions de l'auteur : « rien, comme aujourd'hui » pour l'historique ; extraits « décochés par défaut » à l'export)

- **Historique** : aucune entrée pour une nouvelle commande, ni texte, ni fiche chiffrée. C'est le cas de `try_command` aujourd'hui.
- **Journal** : seulement le genre de commande (`recherche.youtube`, `extrait.<n>`, `appli.<n>`, `mail`, `reformulation`), des longueurs, le moteur et les durées. Jamais la sélection, la consigne, la requête, l'objet du mail ni le texte d'un extrait.
- **Envois** :
  - la sélection et la consigne vont seulement au moteur autorisé (décision 2) ;
  - la requête va au site de recherche par le navigateur ;
  - l'objet va à la messagerie ;
  - rien d'autre ne sort.
- **Presse-papiers.** Un extrait ou une reformulation est collé comme texte transitoire : drapeaux d'exclusion de l'historique sous Windows, marque `org.nspasteboard.TransientType` sur Mac comme dans Scribe. Choix technique.
- **Export d'un utilisateur** (cadrage des profils, décision 5) : les extraits sont dans l'aperçu mais **décochés par défaut**, comme le prénom. Les sites de recherche sont cochés. Les applications ne sont pas exportées, car elles appartiennent à l'ordinateur.
- **Tests.** Chaque nouvelle commande a un test « SECRET » sur le modèle de `commands_110` et `209` : le texte confidentiel n'apparaît ni dans le journal, ni dans l'historique, ni dans les requêtes non autorisées.

**6. Windows et Linux** (choix technique)

| | macOS | Windows | Linux (X11 et Wayland) |
|---|---|---|---|
| Touche de commande | oui | oui | oui, dans les limites des touches de Wayland (à essayer à l'étape 1) |
| Recherches, mail | oui | oui (`ShellExecuteW`) | oui (`xdg-open`) |
| Extraits | oui | oui | oui (collage) |
| Applications | oui | oui | oui (`gtk-launch`) |
| Reformulation | oui | oui | **non**, tant que la lecture de la sélection n'existe pas (ROADMAP n°3). La partie est grisée avec « Non disponible sous Linux » |
| Mes raccourcis | oui | oui | non (envoi de raccourci non pris en charge, inchangé) |

Pourquoi : tout ce qui ne lit pas la sélection et n'envoie pas de raccourci marche déjà sur les trois systèmes (collage, ouverture). La lecture de la sélection sous Linux est un chantier à part (n°3), qui ne doit pas bloquer celui-ci.

**7. Ordre de livraison** (décision de l'auteur : « reformulation en dernier »)

Du plus simple au plus risqué. Chaque étape est livrable seule, avec ses tests, son Aide en français et en anglais, ses traductions et sa ligne dans `docs/ECARTS.md`.

| Étape | Contenu | Systèmes | Livrable seule parce que |
|---|---|---|---|
| 1. Socle et recherches | Réglage et capture de `commandKeys` ; aiguillage sous la touche ; « Commande inconnue » sans collage ; commandes d'aujourd'hui sous la touche ; recherches (4 sites et sites de la personne) ; fonction d'ouverture contrôlée | Les trois | Sans modèle ni sélection, utile dès le premier jour |
| 2. Extraits | Partie « Extraits », rangement `snippets`, collage transitoire, export décoché | Les trois | Ne dépend que du socle |
| 3. Applications et mail | Liste autorisée (`voiceApps`, choix dans les applications installées), « nouveau mail » avec objet facultatif | Les trois | Ne dépend que du socle et de la fonction d'ouverture |
| 4. Reformulation | Second prompt, `accepts_rewrite`, `allowSelectionOnline`, limites, préchauffage | Mac, Windows | Seule étape avec un modèle ; elle profite d'un socle déjà essayé |

Hors de ce chantier : l'extrait au milieu d'une dictée (plus tard), la lecture de la sélection sous Linux (ROADMAP n°3) et l'envoi de raccourcis sous Linux.

### Conséquences pour les autres chantiers

- **Profils (chantier 4).** Les nouveaux champs se classent comme dit en décision 4. L'enveloppe d'export recevra `snippets` (décochés) et `webSearches`.
- **ROADMAP.** Les n°4 et n°7 des idées du 03/11, ainsi que le transfert vers un modèle du n°6, renvoient à cette note (tâche 3).
- **Scribe.** Écarts à noter dans `docs/ECARTS.md` à chaque étape. Scribe n'a rien de tout cela ; les n°12, 12 bis, 12 ter, 13 et 33 de sa ROADMAP en sont la source.

## Relecture

À remplir (tâche 2).
