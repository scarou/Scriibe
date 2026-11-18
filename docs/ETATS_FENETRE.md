# États de la fenêtre perdus à chaque lancement (chantier 3)

Découpage : `decoupage-2026-11-17-c3-etats-fenetre.md` (vault IA-OS). Sans version propre : part dans la 0.11.0.

## Constat et choix

*(Tâche 0, 18/11/2026, Mac.)*

### Cause

`Server::http("127.0.0.1:0")` (`crates/cli/src/web/mod.rs:97`) prend un port libre différent à chaque démarrage. `localStorage` est rangé **par origine** (adresse + port) : à chaque lancement la page a une nouvelle origine, donc un `localStorage` vide. `sessionStorage` n'est pas concerné de la même façon : il ne dure de toute façon que le temps de la fenêtre.

### Usages relevés dans `ui/src` (6 clés, 14 lignes `getItem`/`setItem`/`removeItem`)

Le découpage en comptait 9 ; le relevé complet en trouve 14 lignes pour 6 clés (hors tests). Les usages sont tous entourés d'un `try/catch` : un stockage refusé ne casse rien.

| Clé | Stockage | Où | Ce qu'elle garde | Devenir |
|---|---|---|---|---|
| `fold:<id de carte>` | `localStorage` | `lib/folds.ts`, lu par `components/Card.svelte` (toutes les pages) | Carte repliée (`1`) ou dépliée (`0`) ; absente = jamais touchée, la carte prend son état initial | **Déménage** → fichier, entrée `folds` (id → booléen) |
| `scriibe.bench.compare` | `localStorage` | `tabs/Bench.svelte` | Case « Comparer avec le modèle » (`on`/`off`, défaut coché) | **Déménage** → fichier, entrée `benchCompare` |
| `dictionary:height` | `localStorage` | `tabs/Dictionary.svelte` | Hauteur commune des deux cartes (px, 180 à 1400) ; absente = hauteur naturelle | **Déménage** → fichier, entrée `dictionaryHeight` |
| `uiLanguage` | `localStorage` | `lib/i18n.svelte.ts` | Copie du réglage « Langue de l'interface » (`fr`/`en`/`auto`) pour éviter un instant de français au démarrage. La vérité reste `settings.json` | **Déménage** → fichier, entrée `uiLanguage` (copie, comme aujourd'hui) |
| `tab` | `sessionStorage` | `lib/nav.svelte.ts` (lecture l. 9, écriture l. 43) | Onglet affiché, retrouvé après un rechargement de la page | **Reste** : ne doit durer que le temps de la fenêtre ; à chaque lancement on arrive sur l'Accueil, c'est voulu |
| `STALE_FLAG` | `sessionStorage` | `lib/api.ts` (écriture), `App.svelte` (lecture, effacement) | Drapeau d'un seul rechargement : « l'utilisateur actif a changé », pour afficher le message | **Reste** : vit moins d'une seconde, dans la même fenêtre |

Donc **4 clés déménagent, 2 restent**. Les 3 états de la ligne du carnet sont `fold:*`, `scriibe.bench.compare`, `dictionary:height` ; la langue est un quatrième cas trouvé au relevé : aujourd'hui sa copie est elle aussi vide à chaque lancement, donc l'instant de français au démarrage a lieu à tout lancement (le corriger ici ne coûte rien de plus).

### Choix : un fichier d'états d'interface (confirmé)

**Confirmé** : un fichier `ui-state.json` dans le dossier de données de Scriibe, à côté de `settings.json` (`storage.rs`, `folder()`), plutôt qu'un port fixe.

Raisons :
1. **Un port fixe peut être déjà pris** (autre appli, port réservé par le système, deuxième exemplaire de Scriibe). Il faudrait alors retomber sur un port libre : le bug reviendrait, par intermittence, sans qu'on le voie, pire qu'aujourd'hui.
2. **Moins facile à viser** : un port connu d'avance livre à toute autre page ou appli de la machine l'adresse du serveur local ; aujourd'hui seul le jeton (dans le chemin) la protège, avec un port qui change en plus. Le fichier ne change rien à cette surface.
3. **Indépendant de l'afficheur** : le stockage du navigateur diffère sur les 3 systèmes (WebView2, WKWebView, WebKitGTK) et selon que la page est vue dans la fenêtre ou dans un navigateur ouvert par adresse. Un fichier se comporte pareil partout et survit aux mises à jour.
4. **Coût faible** : un fichier de quelques lignes, lu une fois au démarrage.

Écartés : fixer le port (raisons 1 et 2) ; tout mettre dans `settings.json` (ce sont des préférences d'affichage, pas des réglages : elles ne doivent ni partir avec la sauvegarde/restauration de l'utilisateur, ni être partagées entre utilisateurs, ni entrer dans `backup.rs`).

### Conception retenue pour la tâche 1

- **Format** : `{ "version": 1, "folds": { "<id>": true }, "benchCompare": true, "dictionaryHeight": 420, "uiLanguage": "auto" }`. Entrées toutes optionnelles.
- **Portée** : par ordinateur (pas par utilisateur) : à la racine du dossier de données, jamais dans le dossier d'un profil, ni dans le vault ni dans `~/Documents`.
- **Rust** : lecture et fusion d'un petit objet, écriture par `storage::write_atomic` (fichier temporaire puis renommage), sous un verrou (`Mutex`) pour l'enchaînement lire-fusionner-écrire. Deux routes : lecture de tout, écriture d'un fragment (« patch »).
- **Validation côté serveur** : types attendus, hauteur ramenée entre 180 et 1400, ids de cartes limités en nombre et en longueur ; le reste est ignoré.
- **Fichier absent, vide, abîmé ou d'une version inconnue** : valeurs par défaut, sans message ; la prochaine écriture le remplace.
- **Interface** : l'état est lu **une fois avant d'afficher la page** (`main.ts`, avant `mount`) et gardé en mémoire : toutes les lectures de `folds.ts`, `Bench.svelte`, `Dictionary.svelte`, `i18n.svelte.ts` restent synchrones, comme aujourd'hui. Les écritures partent en arrière-plan ; pour la hauteur du Dictionnaire (glissement de la poignée) on n'envoie qu'à la fin du geste ou après un court délai. Réponse du serveur en échec : on garde l'état en mémoire, sans erreur affichée.
- **Fenêtre de l'indicateur** (`#overlay`) : lit ces états (sa langue), ne les écrit jamais.
- **Vigilances pour la tâche 1** : `backup.rs` ne doit pas emporter ni écraser `ui-state.json` ; `api.test.ts` simule `sessionStorage` (qui reste), pas `localStorage` ; ajouter la fenêtre sous Tauri comme sous navigateur (même route).
- **Tests prévus** : aller-retour Rust (écrire, relire sur une nouvelle instance) pour les 3 états ; fichier abîmé, vide, absent, version inconnue ; valeurs hors bornes ; deux écritures de fragments différents ne s'écrasent pas ; côté interface, le cache lit l'état chargé et ne plante pas si la route échoue.

## Fichier d'interface

*(Tâche 1, 18/11/2026, Mac.)*

Fait comme prévu au § Constat et choix ; écarts notés plus bas.

- **Rust** : `crates/platform/src/ui_state.rs` (`UiStateStore`). Fichier `ui-state.json` à la racine du dossier de données (`app.config.folder()`), version 1, écrit par `storage::write_atomic` (temporaire à nom unique puis renommage : une interruption laisse l'ancien fichier), sous un verrou pour lire-fusionner-écrire. Seules les entrées connues aux valeurs permises sont gardées (identifiants de carte de 80 caractères au plus, `[A-Za-z0-9_.:-]`, 300 cartes au plus ; hauteur entière de 180 à 1400 ; `uiLanguage` `fr`/`en`/`auto`) ; le reste, dont la clé `profile` que la page ajoute à ses envois, est ignoré.
- **Fichier absent, vide, abîmé, sans version ou d'une version 0** : valeurs par défaut sans erreur, remplacé par la prochaine écriture. **Version plus récente** (retour à une ancienne Scriibe) : lu comme vide et **jamais écrasé** (l'écriture est refusée, la page garde son état en mémoire).
- **Serveur** (`crates/cli/src/web/mod.rs`) : `GET api/ui-state` rend l'état ; `POST api/ui-state` fusionne un fragment (`null` retire une carte ou la hauteur) et rend l'état obtenu, ou 500 avec un texte. Hors `guarded` : pas de 409 « utilisateur changé » (propre à l'ordinateur).
- **Interface** : `ui/src/lib/uiState.ts` (`UiStateCache`). `main.ts` appelle `restoreUiState()` (`lib/boot.ts`) **avant** `mount`, pour la page comme pour la pastille `#overlay` (qui ne fait que lire) : lecture des états, bornée à 2 s, puis `i18n.restore()` reprend la copie de la langue (le module est chargé avant la lecture). *(Corrigé à la relecture : la version de la tâche 1 ne le faisait pas pour la pastille et n'avait pas de délai.)* Lectures synchrones ; écritures appliquées en mémoire tout de suite, envoyées une à une dans l'ordre, erreurs avalées. `folds.ts`, `Bench.svelte`, `Dictionary.svelte` (envoi regroupé 300 ms après le dernier cran de la poignée) et `i18n.svelte.ts` n'utilisent plus `localStorage`. `sessionStorage` (onglet, drapeau de rechargement) inchangé.
- **Désinstallation, sauvegarde, restauration** : rien à changer. La liste des fichiers de la sauvegarde (`restore::FILES`) ne contient pas `ui-state.json`, qui n'est donc ni exporté ni écrasé ; la désinstallation emporte le dossier de données entier.
- **Tests** : 10 en Rust (`ui_state::tests` : les 4 états survivent à une nouvelle instance ; fichier absent, vide, tableau, sans version, version 0, tronqué, abîmé ; version plus récente jamais écrasée ; valeurs hors bornes et clés inconnues ignorées ; identifiants et nombre de cartes bornés ; fragments qui ne s'écrasent pas, `null` ; 8 écritures simultanées toutes retenues ; aucun fichier temporaire laissé) et 7 en Vitest (`uiState.test.ts` : survie à un redémarrage, sans réponse du programme, `null`, regroupement du geste continu, `flush`, un envoi raté n'empêche pas le suivant, réponse illisible). `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm run check`, `npm test`, `npm run build` : sans erreur.
- **Non fait ici** : pas de lancement réel de Scriibe sur le Mac (la règle du CLAUDE.md veut un lancement par `open`, jamais par Bash) ; les routes ne sont pas couvertes par un test de serveur (elles se réduisent à `read`/`patch`). L'essai réel, relance comprise, est la tâche 3. Pas de contrôle clippy Windows : le code est le même sur tous les systèmes.
- **Mineur connu** : un changement de hauteur fait dans les 300 ms avant la fermeture de la fenêtre n'est pas envoyé (pas de vidage à la fermeture) ; les cartes, la case et la langue partent tout de suite.
- **À savoir** : au premier lancement avec cette version, la copie de la langue s'écrit une fois dans le fichier (`i18n.use` la compare à l'état lu) ; les états gardés jusqu'ici dans le navigateur sont perdus une dernière fois (pas de reprise : ils l'étaient déjà à chaque lancement).

## Relecture

*(Tâche 2, 18/11/2026, Mac. Relecteur : sous-agent Sonnet, effort Élevé, qui n'avait pas écrit le code, lecture seule, sur le commit a581bbb. Détail brut de son rapport : fichier temporaire de la session, repris ici.)*

Bilan : **0 bloquant, 2 importants, 8 mineurs**. Les 2 importants sont corrigés, avec un test chacun ; 5 mineurs corrigés, 3 notés.

### Importants (corrigés)

- **I1 · `await uiState.load()` sans délai (`main.ts`)** : une requête qui traîne sans échouer laissait la fenêtre blanche. Corrigé : `load()` abandonne au bout de 2 s et affiche avec les valeurs par défaut ; une réponse tardive est ignorée (elle n'écrase pas ce qui a changé entre-temps). Tests : `uiState.test.ts` « n'attend pas un programme qui ne répond pas… » et « une réponse rapide annule le délai ».
- **I2 · la pastille `#overlay` ne reprenait plus la langue choisie** : avant, elle partageait le `localStorage` de la page (même adresse) ; sans lecture des états, elle suivait la langue du système. Corrigé : `restoreUiState()` (`lib/boot.ts`) lit pour les deux fenêtres, sans jamais écrire depuis la pastille. Test : `boot.test.ts` (la langue gardée est reprise après la lecture ; sans état, `auto`).

### Mineurs corrigés

- **M1 · une erreur de lecture prise pour « fichier absent »** (`ui_state.rs`) : un fichier momentanément verrouillé (antivirus Windows) ou illisible faisait écraser tout le reste à la prochaine écriture. Corrigé : seuls « absent » et « pas du texte » valent vide ; toute autre erreur laisse le fichier intact (`patch` refuse). Test : `an_unreadable_file_is_left_alone_and_a_non_text_file_is_replaced`.
- **M4 · écritures sans effet** : un fragment refusé ou de même valeur réécrivait le fichier (ou le créait avec seulement la version). Corrigé : pas d'écriture si l'état ne change pas. Test : `a_patch_that_changes_nothing_writes_nothing`.
- **M7 · `savedFold('constructor')`** rendait une fonction héritée de l'objet. Corrigé par `Object.hasOwn`. Test : `folds.test.ts`.
- **M8 · tests trop faibles** : ordre des envois testé avec une réponse différée (`les envois partent à la suite…`) ; liste pleine : carte neuve refusée, carte connue retirée par `null`, place libérée réutilisée ; test des « quatre états » renommé ; `i18n.restore()` couvert par `boot.test.ts`. Non fait : un test de contrat entre le faux serveur de la page et `merge` de Rust (les deux côtés sont testés chacun de leur côté), des tests des routes.
- **M6 · verrou propre au processus** : commentaire corrigé (« dans ce processus ») ; la seconde instance de Scriibe est refusée.
- **M5 · `write_atomic` muet pendant une désinstallation** : commentaire ajouté, comportement gardé (la page garde ses états en mémoire, le dossier part de toute façon).

### Mineurs restants (à porter au carnet par la tâche 4)

- **M2 · un changement en attente est perdu à la fermeture ou au rechargement automatique** (409 « utilisateur changé »). Seule la hauteur du Dictionnaire attend (300 ms) ; piste : vider (`flush`) sur `pagehide` avec `fetch(..., {keepalive: true})`, ce qui demande de changer `api()`.
- **M3 · plafond de 300 cartes sans purge** : les cartes de l'Historique (une par jour, `historique-<jour>`) finiront, après environ 240 jours de plis, par ne plus être retenues, sans signal. Pistes : ne pas retenir les plis de l'Historique, ou évincer les plus anciens.
- **Le test de contrat entre les deux côtés et les tests de route** (voir M8).

### Contrôles après corrections

`cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (845 réussis, 0 échec ; 12 tests de `ui_state`), `npm run check` (0 erreur), `npm test` (170, dont 15 pour les états : 7 de la tâche 1 et 8 de la relecture), `npm run build` : sans erreur.

## Essai

*(Tâche 3, 18/11/2026. Version : 33d5226 (tâche 2), compilée en release sur les 3 machines.)*

**Résultat : les trois états sont retrouvés après avoir quitté et relancé Scriibe, sur Mac, Windows et Linux.** `ui-state.json` a été relu après chaque geste (écriture immédiate constatée), puis la page relue après relance par l'auteur.

| Machine | Fichier | Case « Comparer » | Plis | Hauteur du Dictionnaire | Après relance |
|---|---|---|---|---|---|
| Mac | `~/Library/Application Support/Scriibe/ui-state.json` | décochée | 17 et 18/11 repliées | 742 px | tout retrouvé |
| Windows | `%APPDATA%\Scriibe\ui-state.json` | décochée | 17/11 repliée, 15/11 dépliée explicitement | 731 px | tout retrouvé |
| Linux | `~/.local/share/scriibe/ui-state.json` | décochée | 16 et 17/11 repliées | 541 px | tout retrouvé |

### Déroulé

- **Mise à jour des PC** : les 14 commits du chantier 3 n'étaient pas poussés (`pilote maj` passe par GitHub). Sur l'accord de l'auteur, un paquet `git bundle` (d731136..HEAD) a été copié sur chaque PC et appliqué par `git pull --ff-only` ; rien publié, aucun push.
- **Compilation** : `pilote compiler tous --mode release`. Windows a d'abord échoué (`scriibe.exe` en cours d'exécution, accès refusé) : arrêt du processus par `taskkill`, nouvelle compilation réussie. Mac : `npm run build`, `cargo build --release`, `tools/faire_app_macos.sh`, lancement par `open`.
- **Lancement** : Mac par `open`, Windows par le raccourci `scriibe.exe` du bureau (l'autre raccourci, `scriibe`, vise `C:\scriibe` : ancienne version), Linux par Activités (l'ancienne version tournait encore, binaire remplacé : fermée et relancée par l'auteur).
- Aucune capture n'a été nécessaire : le fichier d'états suffit comme preuve.

### Constats

- **Pli « dépliée » sur Windows** : l'auteur a cru avoir replié 2 cartes, une seule l'était. Le fichier montre `historique-2026-11-15: false` : une carte repliée par défaut (carte ancienne) a été dépliée par le clic, et l'état « déplié » est bien retenu. Comportement conforme, pas un défaut.
- **Linux** : le fichier contenait déjà les trois états avant l'étape annoncée, l'auteur ayant avancé ses gestes ; sans incidence sur le résultat.
- Piège de procédure : sur un PC, une compilation échoue si l'ancien exécutable tourne ; fermer Scriibe d'abord.

## Ajouts

Aucune nouvelle demande pendant l'essai.
