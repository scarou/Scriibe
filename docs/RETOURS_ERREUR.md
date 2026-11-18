# Retours d'erreur et commandes vocales (chantier 5)

Découpage : `decoupage-2026-11-17-c5-retours-erreur.md` (vault IA-OS). Trois lignes du carnet : pastille d'erreur qui reste affichée, son d'erreur pour une commande, interrupteur par commande. Sans version propre : part dans la 0.12.0 avec le chantier 6.

Décisions de l'auteur (18/11/2026) : (1) la pastille d'erreur reste jusqu'au clic sur sa croix ou jusqu'à la dictée suivante, les autres pastilles gardent leur durée ; (2) un seul son d'erreur pour « non reconnue » et « a échoué », la pastille dit laquelle ; (3) un interrupteur par commande sur la page Commandes vocales, gardé par profil, sous l'interrupteur général.

## Conception

*(Tâche 0, 18/11/2026, Mac. Aucun code. Les numéros de ligne sont ceux de `crates/pipeline/src/controller.rs` au commit `1d10a99`.)*

### 1. Où naissent les erreurs, la pastille, les sons, les commandes

| Quoi | Où |
|---|---|
| Toute erreur de dictée ou de commande | `Controller` (`crates/pipeline/src/controller.rs`) : `deps.indicator.flash(IndicatorState::Message(texte), secondes)`, 4 s au plus. `Message(String)` sert **aussi** aux succès (« Ouverture de l'application », « Ajouter au dictionnaire… », « Collé — micro faible ») : rien ne distingue une erreur d'une information. |
| Le modèle de la pastille | `crates/cli/src/overlay.rs` : `OverlayModel` (état, `until` pour les affichages temporaires, `generation`), `Snapshot::json()` lu par la page toutes les 45 ms (`GET api/overlay`). |
| La fenêtre de la pastille | `crates/cli/src/window.rs` : `build_overlay` (440 × 110, sans cadre, transparente, toujours devant, `focusable(false)`, **insensible à la souris** : `set_ignore_cursor_events(true)` sur les trois systèmes), `start_overlay` (fil qui l'affiche et la cache), `reveal` (affichage sans activation : `ShowWindow(SW_SHOWNA)` sous Windows, `orderFrontRegardless` sous macOS, `show()` sous Linux). |
| Le dessin | `ui/src/overlay/OverlayView.svelte` : l'état `message` est une capsule « ! » pleine largeur (440 − 2 × 12 px), posée en bas de la fenêtre, alignée à gauche, au centre ou à droite selon la position choisie. |
| Les relais d'indicateur | `OverlayIndicator` (`overlay.rs`), `TeeIndicator` (`web/sinks.rs` : garde aussi les messages d'un essai de mode), `TrayIndicator` (Windows, macOS/Linux), `ConsoleIndicator`, `FakeIndicator` (tests). |
| Les sons | `crates/platform/src/sounds.rs` : trait `Sounds::play(nom, volume)`, 11 sons synthétisés (`NAMES`), joués par `PlaySoundW` (Windows), `afplay` (Mac), `paplay`/`pw-play`/`aplay` (Linux). Le contrôleur joue le son de début à l'appui (l. 586) et celui de fin au relâchement (l. 730) ; aucun son pour une erreur. |
| La reconnaissance des commandes | `crates/core/src/commands.rs` : `all_defs(settings)` (catalogue du programme, commandes perso, sites, extraits, mail, applications), `recognize` / `recognize_for` (sans la touche : seul `voiceCommandsEnabled` ouvre ; sous la touche : toujours), `route_under_key` → `Route::{Command, Close, Instruction, TooLong, EmptySearch, UnknownSearch}`. |
| L'exécution | `controller.rs` : `try_command` (sans la touche : une phrase non reconnue devient du **texte**, jamais une erreur), `process_command` (sous la touche), `run_command` (une branche par famille d'identifiant), `rewrite_selection`. |
| Les réglages des commandes | `settings.rs` : `voiceCommandsEnabled`, `voiceCommands` (phrases), `customCommands`, `webSearches`, `snippets`, `voiceApps` ; tous dans `USER_KEYS` (= `profile.json` de l'utilisateur ; sans utilisateur rangé à part, `settings.json`). |
| La page | `ui/src/tabs/Commands.svelte` : carte d'activation (interrupteur général), puis six blocs (livrées, sites, extraits, applications, perso, reformulation) dans un `<div inert>` quand l'interrupteur général est coupé. Vue : `commands_view` (`crates/cli/src/web/mod.rs`). |

### 2. Les cas d'erreur concernés

Trois sortes, par ce que la personne doit comprendre. **Rien dans la pastille ni le journal ne contient les mots dits** (règle déjà tenue : messages fixes).

**A. Commande non reconnue** (sous la touche seulement) : pastille persistante, **son d'erreur**, titre « Commande non reconnue ».

| Cas | Source | Détail affiché |
|---|---|---|
| Phrase proche d'une commande | `Route::Close` (l. 802) | — |
| Phrase de plus de 25 mots | `Route::TooLong` (l. 808) | « Commande trop longue » |
| Site de recherche inconnu | `Route::UnknownSearch` (l. 819) | « Site de recherche inconnu » |
| Commande désactivée (nouveau, tâche 2) | `Route::Disabled` | « Commande désactivée » |

**B. Commande non faite** (reconnue, mais elle n'aboutit pas) : pastille persistante, **son d'erreur**, titre « Commande non faite » et la raison en dessous.

| Cas | Source (l.) | Raison affichée |
|---|---|---|
| Pas de sélection, sous la touche | `run_command` 950, 978, 1004, 1030 | « Aucune sélection » |
| Sélection illisible | 958 | « Sélection illisible » |
| Sélection trop longue pour le dictionnaire / refus du dictionnaire | 983, 995 (branche `Err` seulement) | texte existant |
| Raccourci invalide ou non envoyé | 1016, 1023 | texte existant |
| Rien à entourer | 1033 | texte existant |
| Rien à chercher, recherche impossible ou non ouverte | `Route::EmptySearch` (l. 815), 1051, 1054, 1061 | texte existant |
| Extrait disparu ou vide | 1072 | « Extrait impossible » |
| Mail non ouvert | 1097 | « Mail non ouvert » |
| Application introuvable ou non ouverte | 1108, 1118 | texte existant |
| Commande inconnue du chef d'orchestre | 1127 | « Commande inconnue » |
| Reformulation : non disponible sous Linux, application changée, moteur sans accord, sélection trop longue, rien à changer, réponse écartée, service en échec, copie impossible | 835, 842, 875–890, 902 | texte existant |
| **Sans la touche** : commande reconnue dont la sélection est illisible (Accessibilité, presse-papiers, échec) ; la phrase est collée comme du texte | 961–970, puis note finale l. 1324 | « Commande non faite : … » existant (la dictée a réussi, la commande non) |

**C. Dictée en erreur** : pastille persistante, **pas de son d'erreur** (la décision 2 ne vise que les commandes), sans titre : le message dit déjà tout.

| Cas | Source | Message |
|---|---|---|
| Micro impossible à ouvrir | l. 559 | message du micro |
| Aucun son capté | l. 735 | « Aucun son capté — vérifie le micro (…) » |
| Transcription en échec | l. 1143 | message du moteur |

**Qui reste comme avant** (durée inchangée, pas de son) : tous les succès (`Done`, « Ouverture de l'application », « Ajouter au dictionnaire — complète « Écrit » », « Collé — micro faible ») ; les réussites **partielles** où le texte est dans le presse-papiers (« Copié, à coller avec ⌘V — … », « Reformulation copiée… » : l. 915, 1043, 1080, 1322) ; les remarques de traduction (« écartée », « échouée » : le français est collé) ; « Rien entendu » (appui à vide fréquent : rester jusqu'au clic serait pénible).

*Pourquoi ce partage.* Une erreur reste affichée quand la personne doit agir ou comprendre (micro, commande ratée) ; elle ne reste pas quand le texte est tout de même arrivé ou que l'appui était sans doute accidentel. Si l'auteur préfère « Rien entendu » ou les remarques de traduction persistantes, c'est une ligne de plus dans la liste de la tâche 1.

### 3. Le son d'erreur

- Un son **hors de la liste choisissable** : `ERROR_SOUND = "Erreur"` dans `sounds.rs`, synthétisé comme les autres (`notes()`), absent de `NAMES` (l'auteur ne peut pas le prendre pour début ou fin). Deux notes graves qui descendent (330 → 220 Hz, 0,35 s), à l'opposé du « Funk » montant. Aucun changement du trait `Sounds`, aucune nouvelle dépendance sur les trois systèmes.
- Volume : `soundVolume` de l'ordinateur. **Pas de son d'erreur** si le volume est à 0 ou si le son de début **et** celui de fin sont « Aucun » : la personne qui a coupé les sons veut le silence (pas de réglage de plus : décision 2).
- Joué au moment où la pastille d'erreur s'affiche, depuis le contrôleur (un seul endroit : `fail_command`, voir 4), pour les cas A et B seulement.

### 4. La pastille persistante

Le trait `Indicator` gagne **une méthode** `fail(&self, error: Failure)`, avec une implémentation **par défaut** qui appelle `flash(Message(texte), 4.0)` : les implémentations qui l'ignorent (`ConsoleIndicator`, icônes de la zone de notification) gardent leur comportement. `Failure` = un genre (`NotRecognised`, `NotDone`, `Dictation`) et le détail éventuel.

- `OverlayIndicator::fail` : le modèle passe en état « erreur » **sans `until`** ; l'icône de la zone de notification, elle, reçoit `flash(Message, 4.0)` comme aujourd'hui. `TeeIndicator::fail` doit **aussi** être redéfini (il garde les messages d'un essai de mode : sans lui, l'erreur passerait par la méthode par défaut et perdrait son genre). `FakeIndicator` enregistre les `fail`.
- Fin de l'erreur : **n'importe quel `show` ou `flash` suivant** la remplace (donc la dictée suivante : `show(Listening)` à l'appui, déjà là), ou `OverlayModel::dismiss()` au clic sur la croix. Aucun délai maximal (décision 1). Si « Afficher la pastille » est décoché, l'erreur n'est pas affichée et le fil de `window.rs` l'efface (`dismiss`), pour qu'elle ne surgisse pas plus tard si le réglage est recoché.
- `Snapshot::json()` : champ `error` = `null` ou `{kind, message}`. La page (`OverlayView.svelte`) dessine la capsule d'erreur : titre (selon le genre : « Commande non reconnue », « Commande non faite »), raison sur une seconde ligne, **croix** (bouton de 24 × 24 px, `aria-label` traduit). Icône « ? » pour « non reconnue », « ! » pour le reste. Textes dans `en.ts` / `en-server.ts` (`i18n.test.ts`).
- La croix appelle `POST api/overlay/dismiss`. Un clic ailleurs sur la capsule ne fait rien (pas de fermeture par mégarde ; la zone cliquable reste celle de la capsule seulement).

### 5. Rendre la pastille cliquable sans voler le focus

**Le problème.** La fenêtre fait 440 × 110 px, transparente, et tout son contour est insensible à la souris. Si on la rendait cliquable en entier pour une erreur qui reste affichée, une zone de 440 × 110 px au bas de l'écran ne répondrait plus à l'appli en dessous, pendant tout le temps de l'erreur. Et un clic sur une fenêtre active normalement l'appli, ce qui ferait perdre le focus (et le collage) à l'appli où l'on dicte.

**Solution retenue (la même sur les trois systèmes pour la forme, propre à chacun pour le focus).** La fenêtre **se réduit à la capsule d'erreur** et devient cliquable **seulement** pendant l'erreur ; elle redevient 440 × 110 et insensible à la souris à la fin.

1. La page mesure sa capsule (`ResizeObserver`) et envoie sa taille (`POST api/overlay/shape`, largeur et hauteur arrondies au pixel supérieur) ; le modèle la garde (`Snapshot.clickable: Option<(f64, f64)>`). La capsule se met en `padding: 0` autour d'elle pendant l'erreur (le rond d'ombre est rogné : accepté).
2. Le fil de `start_overlay` (40 ms) compare l'état voulu à l'état affiché (visible, taille cliquable) et, au changement : `set_size`, `place_overlay` avec **cette** taille (`origin_x` prend déjà la largeur ; le bas reste à `BOTTOM_GAP`, la capsule ne bouge donc pas à l'écran), `set_ignore_cursor_events(false)`, puis `reveal` (déjà sans activation). Le retour à la normale fait l'inverse.
3. Avant la première mesure (quelques dizaines de ms), la fenêtre est à sa taille normale et reste insensible à la souris : aucun clic ne peut y arriver.

*Pourquoi pas la détection du curseur* (garder la fenêtre insensible et la rendre cliquable quand la souris passe sur la capsule) : elle demande la position de la souris à chaque tour, qui n'est pas fiable sous GNOME Wayland (les fenêtres sont par XWayland : la position n'est mise à jour que sur les fenêtres X11). Réduire la fenêtre n'en a pas besoin.

**Ne pas prendre le focus, par système :**

| Système | Mécanisme | Ce qui est sûr / à vérifier |
|---|---|---|
| **Windows** | `focusable(false)` (tao : style `WS_EX_NOACTIVATE`) : un clic n'active pas la fenêtre. | À vérifier à l'essai : après un clic sur la croix, la frappe suivante arrive dans l'appli d'origine (le contrôle WebView2 peut vouloir le focus clavier). **Repli prévu** : sous-classer la fenêtre pour répondre `MA_NOACTIVATE` à `WM_MOUSEACTIVATE` (dans `crates/platform/src/windows/`). |
| **macOS** | `focusable(false)` (la fenêtre ne peut pas devenir fenêtre clé). Scriibe est une appli « accessoire » : un clic sur une fenêtre peut tout de même activer l'application. | **Le plus risqué des trois.** À essayer **en premier à la tâche 1**, sur ce Mac, par un essai de 10 minutes (clic réel sur la croix, puis on vérifie l'appli au premier plan). **Repli 1** : donner à la fenêtre la classe d'un `NSPanel` à style « non activant » (`NSWindowStyleMaskNonactivatingPanel`, via `objc2`, dans `crates/platform/src/macos/`), comme le fait la bibliothèque `tauri-nspanel`. **Repli 2** (si rien ne tient) : pas de clic sur Mac ; la croix est remplacée par l'entrée « Fermer l'avis » du menu de l'icône, plus la dictée suivante. À dire à l'auteur avant de le prendre. |
| **Linux** (X11 et XWayland) | `focusable(false)` (GTK `set_accept_focus(false)`, indice d'entrée X11 faux) : le gestionnaire de fenêtres ne donne pas le focus au clic. | À vérifier à l'essai sur le PC GNOME. `reveal` rappelle aujourd'hui `set_ignore_cursor_events(true)` à chaque affichage : à adapter (il ne doit le faire que hors erreur). |

Ces trois comportements ne se testent pas par un test automatique (ils dépendent de la pile graphique) : la tâche 1 vérifie le Mac, la tâche 4 (essai) les trois systèmes, en tapant du texte dans l'appli d'origine après le clic.

### 6. L'interrupteur par commande

**Où il agit.** Une commande désactivée n'est plus reconnue **dans les deux situations** (sans la touche : sa phrase redevient du texte ; sous la touche : la pastille dit « Commande désactivée », cas A). L'interrupteur général reste au-dessus et inchangé : il garde son rôle (sans la touche seulement) ; l'état des interrupteurs par commande est conservé quand il est coupé (la page les grise, comme aujourd'hui, sans les effacer).

**Format.** Une nouvelle clé de réglage **`disabledCommands`** : une liste d'identifiants de commandes **désactivées** (`["format.gras", "recherche.youtube", "perso.2"]`).

- Dans `USER_KEYS` (suit l'utilisateur : `profile.json`, ou `settings.json` tant qu'aucun utilisateur n'est rangé à part), avec les phrases (`voiceCommands`) et l'interrupteur général. Champ `AppSettings::disabled_commands: Vec<String>`, `#[serde(rename = "disabledCommands", default)]`.
- **Liste des désactivées, pas des actives** : un profil qui n'a pas la clé, ou une clé vide, a donc **toutes ses commandes actives**. Une nouvelle commande ajoutée par une version future naît active. Les commandes de la personne (`perso.*`, `recherche.N`, `extrait.N`, `appli.N`) suivent la même liste.
- **Pas de migration, pas de nouveau numéro de schéma** (`SETTINGS_SCHEMA` reste à 2) : une clé facultative ajoutée ne change rien pour un fichier qui ne l'a pas. Le fichier d'un ancien profil n'est **pas réécrit** à la lecture ; la clé (`[]` tant qu'on n'a rien désactivé) n'apparaît qu'à la première sauvegarde de réglages, comme tout champ d'`AppSettings` (la sérialisation écrit tous les champs). Une ancienne version du programme qui relirait un profil avec la clé l'ignore (lecture clé par clé : `get(o, "…")`, `settings.rs` l. 650 et suivantes) et la perdrait à sa prochaine sauvegarde : accepté.
- **Lecture tolérante** (comme les autres champs, `settings.rs` l. 701–717) : une valeur qui n'est pas une liste de textes → liste vide (tout actif), jamais une erreur de lecture ; espaces retirés, doublons et vides écartés, 500 identifiants au plus, 100 caractères chacun. Les identifiants **inconnus sont gardés à la lecture** (un profil recopié d'une version plus récente garde ses interrupteurs) et **retirés à l'écriture** quand l'identifiant n'est plus dans `all_defs` : supprimer une commande de la personne retire aussi son interrupteur, et une commande créée plus tard avec le même identifiant ne naît pas désactivée.
- **Export et import d'un profil** (`profile_file.rs`) : `disabledCommands` part avec les « réglages de personne » (`PREFERENCE_KEYS`) **réduit aux commandes du programme** (les identifiants des commandes de la personne sont renumérotés à l'import : ils ne se retrouveraient pas). L'import écarte tout identifiant qui n'existe pas dans le catalogue du programme. La sauvegarde complète (`backup`) recopie `profile.json` tel quel : rien à faire.

**Reconnaissance** (`commands.rs`). `all_defs` reste **entier** (la vérification des phrases déjà prises et `is_close` doivent encore voir les commandes désactivées : sinon réactiver une commande créerait un doublon de phrase). C'est `recognize_for` qui écarte les fiches dont l'identifiant est dans `disabledCommands`. `route_under_key` ajoute `Route::Disabled` : si la phrase correspondrait à une commande désactivée, c'est ce cas, **avant** `is_close`. Le contrôleur affiche « Commande désactivée » (genre A).

**Page** (tâche 2). Un interrupteur sur chaque ligne de commande des six blocs (livrées, sites, extraits, applications, perso ; la reformulation de la sélection n'est pas une commande du catalogue : elle a déjà son accord à part et ne change pas). Un nouveau point d'accès `POST api/commands/enabled` (`{id, enabled}`) écrit sous le verrou de réglages comme les autres ; `commands_view` rend aussi la liste. Une ligne désactivée est grisée mais **reste modifiable** (phrases, suppression).

### 7. Ce qui est pour la tâche 1 et pour la tâche 2

- **Tâche 1** (pastille et son) : `Failure` et `Indicator::fail` (défaut, `OverlayIndicator`, `TeeIndicator`, `FakeIndicator`) ; `fail_command` / `fail_dictation` dans le contrôleur et remplacement des appels du tableau du § 2 ; `ERROR_SOUND` ; `OverlayModel` (état d'erreur, `dismiss`, forme) ; `GET api/overlay` (+ `error`), `POST api/overlay/dismiss` et `shape` ; `window.rs` (taille et cliquable) ; `OverlayView.svelte` ; textes anglais ; Aide (`Help.svelte` et `HelpEn.svelte`). **Commencer par l'essai de 10 minutes de clic sur le Mac** (§ 5). Tests : une erreur de commande donne `fail` avec le bon genre et joue le son ; un succès ne le joue pas ; une erreur reste (`snapshot` après 10 s) ; la dictée suivante, `dismiss` ou un autre `flash` la remplacent ; la dictée en erreur est persistante sans son ; pas de son d'erreur avec volume 0 ou sons « Aucun » ; `TeeIndicator` garde le message ; l'anglais.
- **Tâche 2** (désactiver) : § 6. Tests : commande désactivée ignorée sans la touche (phrase = texte) et sous la touche (`Route::Disabled`) ; ancien profil sans la clé = tout actif, **fichier non réécrit** à la lecture ; clé malformée = tout actif ; identifiant inconnu gardé à la lecture et retiré à l'écriture ; changement d'utilisateur = autre jeu de commandes ; export/import d'un profil (commandes du programme seulement) ; l'interrupteur général inchangé.

### 8. Incertitudes

- Le comportement du clic sans focus (macOS surtout, Windows ensuite) est une hypothèse de lecture de la documentation de Tauri et des systèmes, **non essayée ici** (pas de code dans cette tâche). Les replis sont décrits pour chaque système.
- Les messages à l'écran de plusieurs lignes : la hauteur de la capsule d'erreur est mesurée par la page, jamais calculée côté Rust.
- Un `Failure` oublié au moment de remplacer les appels retomberait en pastille de 4 s (comportement actuel) : le test de la tâche 1 parcourt les cas du § 2.

## Pastille et son

*(Tâche 1, 18/11/2026, Mac. Code selon § Conception ; écarts notés ci-dessous.)*

**Fait**
- `crates/platform/src/indicator.rs` : `FailureKind` (`NotRecognised`, `NotDone`, `Dictation`), `Failure { kind, message }`, méthode `Indicator::fail` (par défaut : `flash(Message, 4 s)`). `FakeIndicator` garde les erreurs (`failures()`), `TeeIndicator` (essai de mode) et `OverlayIndicator` la redéfinissent ; l'icône de la zone de notification reçoit toujours le message temporaire.
- `crates/platform/src/sounds.rs` : `ERROR_SOUND = "Erreur"` (deux notes graves 330 → 220 Hz), hors de `NAMES` (test).
- `controller.rs` : `fail_command` (pastille + son, sauf volume 0 ou « Aucun » au début **et** à la fin), `fail_dictation` (pastille seule) ; `refuse` prend le genre. Remplacés : tous les cas des tableaux A, B et C du § Conception. `CommandOutcome::TextAfterFailure` porte le cas « sans la touche, sélection illisible » (la dictée est collée, la remarque est une erreur qui reste). Le dictionnaire distingue enfin son succès (message temporaire) de son refus (erreur).
- `overlay.rs` : `OverlayModel::fail` (aucun délai), `dismiss`, `set_shape` (bornée à la fenêtre normale, ignorée sans erreur) ; `Snapshot.failure` et `shape` ; `GET api/overlay` gagne `error`. Routes `POST api/overlay/dismiss` et `shape` (`web/mod.rs`).
- `window.rs` : la fenêtre se réduit à la capsule mesurée, au même endroit à l'écran qu'une pastille ordinaire, et n'accepte la souris que pendant l'erreur ; pastille décochée : l'erreur est effacée (`dismiss`) plutôt que gardée.
- Page : `OverlayView.svelte` (capsule d'erreur, titre selon le genre, raison dessous, croix de 24 px, mesure par `ResizeObserver`), `en-server.ts` (« Commande non faite »), Aide française et anglaise (puce « Erreur »).

**Tests ajoutés** (13) : `dictation.rs` (erreur non reconnue, non faite, succès sans erreur ni son, « sans la touche » avec sélection illisible, dictée en échec sans son, réussite partielle temporaire, sons coupés) ; `overlay.rs` (reste jusqu'à `dismiss` même une heure après, remplacée par l'écoute ou un autre état, `dismiss` tardif sans effet sur une dictée repartie, forme bornée, l'icône garde le message temporaire) ; `sounds.rs` (son d'erreur hors liste).

**Essai du clic sur le Mac** (copie d'essai `Scriibe-essai-c5.app` lancée par `open`, erreur déclenchée par un point d'accès temporaire retiré ensuite) : la pastille d'erreur s'affiche, un clic réel sur la croix l'efface, et **l'appli au premier plan (TextEdit) est restée la même** d'après un suivi toutes les 0,2 s pendant le clic. `focusable(false)` suffit donc : ni `NSPanel` ni repli nécessaires pour l'instant. **Limite** : un changement de focus plus court que 0,2 s échapperait au suivi, et la frappe dans l'appli après le clic n'a pas été essayée ; à confirmer à l'essai (tâche 4), avec Windows et Linux.

**Écarts avec le § Conception** : aucun sur le fond. `shape` est posée sur la capsule +1 px ; la fenêtre normale reste à `BOTTOM_GAP`, la fenêtre réduite se place 12 px plus haut (`STAGE_PADDING`) pour que la capsule ne bouge pas à l'écran.

**Non fait ici** : rien vu à l'écran de la capsule d'erreur elle-même (pas de capture) ; Windows et Linux non essayés.
