# Finitions de l'interface — chantier 2

Suivi du chantier 2 (découpage `decoupage-2026-11-17-c2-finitions-interface.md` du vault). Sections : Constat (tâche 0), Mise en page (tâche 1), Textes (tâche 2), puis les suivantes.

## Constat

### Décision sur « (Windows) » (18/11/2026)

**Cacher, ne pas renommer.** Ces libellés désignent de vrais programmes propres à Windows (zips `win-x64`, `win-cpu`, `win-vulkan`) ; ils ne sont pas mal nommés. Le défaut est que `offered()` (`crates/cli/src/web/downloads.rs:85`) les propose aussi sur Mac, où le clic ne peut que répondre « proposé seulement sous Windows ».

Raison : un programme d'un autre système ne peut produire qu'une erreur ; la règle du carnet est « aucun texte propre à un autre système visible sur Mac ». Mise en œuvre (tâche 2) :
1. `offered()` : `cfg!(windows)` pour les trois ids Windows (Mac : aucun téléchargement) ;
2. Mac : encart « Installer avec Homebrew » (`brew install whisper-cpp`, `brew install llama.cpp`) à la place de la carte des programmes ;
3. retirer « Windows, » des libellés, puisque la page ne montre plus que ceux du système courant (traductions `en-server.ts:120-122`) ;
4. chaîne « gpu » de l'assistant : Metal sur Mac, binaire processeur seulement sur Linux (dit dans le titre).

### Points relevés (rapport du sous-agent, 18/11/2026)

Chemins relatifs au dépôt Scriibe. 16 points (8 lignes du carnet) + 2 questions.

#### Tâche 1 — Mise en page et accessibilité

1. **Recherche de l'Aide fixe au défilement** — `ui/src/components/HelpSearchBox.svelte:~100` (`.search` dans le flux, pas sticky) ; inséré par `ui/src/tabs/Help.svelte:108` et `HelpEn.svelte:125`, sous `<PageHeader>`.
   Cause : seul `.page-header` (`ui/src/components/PageHeader.svelte:25`, sticky top:0) et le sommaire `.toc` (`Help.svelte:419`, sticky top = `--page-head + 8px`) restent ; la recherche défile.
   Correctif : `.search { position: sticky; top: var(--page-head, 0px); z-index: 5; background: var(--bg); }` ; publier sa hauteur (`bind:clientHeight` → `--help-search` sur `document.documentElement`, comme PageHeader l.9-12) et la compter dans `.toc { top: calc(var(--page-head) + var(--help-search) + 8px); max-height }` (Help.svelte:419 et 427, HelpEn.svelte:436 et 444) et dans `[id] { scroll-margin-top }` de l'Aide. `ui/src/lib/tabs.test.ts:83-92` teste le sticky du sommaire : ajouter un cas pour la recherche.

2. **`scroll-padding-top` sous l'en-tête fixe** — `ui/src/app.css:58` n'a que `scroll-margin-top` sur `[id]` (valable pour les ancres, pas pour le défilement clavier : PageUp, Espace, Tab).
   Correctif : `html { scroll-padding-top: calc(var(--page-head, 0px) + 8px); }` (+ la hauteur de la recherche de l'Aide, point 1).

3. **Bandeau d'erreur rogné de 6 px** — `ui/src/App.svelte:113-122` (`<div class="card bad">` dans `<main>`, juste avant la page) et `PageHeader.svelte:25` (`margin: -20px -20px 8px`).
   Cause : `.card` a `margin-bottom: 14px` (`app.css:36`), l'en-tête remonte de 20 px : il mord 20 − 14 = 6 px sur le bandeau.
   Correctif : `main > .card.bad { margin-bottom: 20px }` (ou conteneur `.banners` avec `padding-bottom: 6px`) ; vérifier avec bandeau « Scriibe ne répond plus » et `config_problems`.

4. **Tuiles de l'Accueil de même hauteur** — `ui/src/tabs/Dashboard.svelte:74-79` et CSS `:137-138`.
   Cause : `.grid` = `repeat(auto-fill, minmax(180px,1fr))` ; les tuiles s'étirent dans une ligne mais pas d'une ligne à l'autre : quand la grille passe à 2 lignes, la tuile « Aujourd'hui » (texte « {change} % par rapport à hier à la même heure », 2-3 lignes) est plus haute que « Temps gagné ».
   Correctif : `grid-auto-rows: 1fr` sur `.grid` (lignes égales) ; option : `.tile { display:flex; flex-direction:column }`.

5. **Menu en fenêtre étroite (≤ 640 px)** — `ui/src/App.svelte:154-158` (`@media (max-width: 640px)` : nav en ligne, `flex-wrap`, masque `.spacer .version .state .brand .group`).
   Cause : 13 boutons « icône + libellé » sur plusieurs lignes, plus l'état de la dictée masqué ; encombrant.
   Correctif proposé : une seule bande défilable (`flex-wrap: nowrap; overflow-x: auto; white-space: nowrap`), boutons compacts (padding réduit, libellé gardé), `position: sticky; top:0` ? (non : éviter de cumuler avec l'en-tête sticky ; garder `static`), « Quitter Scriibe » dans la bande, pas de retour à la ligne. Vérifier à 375 px par capture.

6. **`aria-label` et `aria-current` du menu** — `ui/src/App.svelte:90-97` : `<nav>` sans nom, boutons `class:active` seulement.
   Correctif : `<nav aria-label={t('Menu principal')}>` (+ clé dans `ui/src/lib/locales/en.ts`, `i18n.test.ts` l'exige) ; sur chaque bouton `aria-current={nav.tab === tab.id ? 'page' : undefined}` (idem bouton Aide l.100). Icône `<span class="icon" aria-hidden="true">`.

7. **`inert` sur les commandes vocales décochées** — `ui/src/tabs/Commands.svelte:19` (`<div class:off=…>`, CSS `.off{opacity:.5; pointer-events:none}` l.29).
   Cause : `pointer-events:none` n'arrête pas le clavier (Tab atteint les champs et boutons).
   Correctif : `<div class:off={…} inert={settings.value?.voiceCommandsEnabled === false}>` (Svelte 5 gère `inert`) ; la carte d'activation (hors du div) reste active.

#### Tâche 2 — Textes selon le système

8. **Texte de la carte d'export** — 3 occurrences de « Une boîte te demande où l'enregistrer (sous Linux, il est placé dans le dossier Documents). » : `ui/src/tabs/Modes.svelte:292`, `ui/src/tabs/Dictionary.svelte:350`, `ui/src/components/commands/CustomCommands.svelte:200` ; traductions `ui/src/lib/locales/en.ts:77, 97, 347` ; Aide `Help.svelte:201, 229, 321` (+ `HelpEn`). `UserExport.svelte` à vérifier (même formule ?).
   Cause : texte unique pour 3 systèmes ; sur Mac et Windows la boîte s'ouvre (vrai), sur Linux le fichier est posé dans Documents sans boîte (le code d'export répond `saved`/`path`, voir `ask_path` côté import).
   Correctif : le texte de la carte dépend de `status.value?.os` (champ déjà là : `status.os === 'macos'`, `status.value?.os === 'linux'`, cf. `App.svelte:114`) : Mac/Windows « Une boîte te demande où l'enregistrer. » ; Linux « Il est placé dans le dossier Documents. ». Deux clés `t()` + traductions `en.ts` ; idem Aide (`mac`/`linux` déjà dérivés dans HelpEn:169). Mutualiser dans `ui/src/lib/` (une fonction `exportWhere()`).

9. **« + » en trop sous GNOME** — `ui/src/tabs/Bench.svelte:176` (`keyName(dictation_label) + ' + ' + keyName(translate_label)`), `ui/src/tabs/Journal.svelte:51` (« {translate} en plus : traduction »), aussi `Install.svelte:91`, `KeyPicker.svelte:152`.
   Cause (probable, à vérifier sous GNOME) : `crates/cli/src/web/mod.rs:787` : sans raccourci GNOME de traduction (`gnome_translate_shortcut()` = None, `wayland.rs:444`), `translate_label` retombe sur le défaut « Maj » (`keys.rs:766`) alors que sous GNOME ce n'est pas une touche de traduction : le banc affiche « Ctrl + ← + Maj » et le journal « Maj en plus : traduction » (faux). Selon la forme de `pretty_trigger` (`wayland.rs:454`), un libellé vide donnerait aussi « X + ».
   Correctif : si `gnome_shortcut` est défini et `gnome_translate_shortcut` nul, `translate_label` = chaîne vide côté Rust ; côté UI une fonction `joinKeys(a, b)` qui omet le « + » et la phrase « en plus : traduction » quand `b` est vide ; test vitest. Vérifier la forme exacte à la tâche 6 (Linux GNOME).

10. **Libellés « (Windows) »** — voir (a) plus bas. Sources : `crates/cli/src/web/downloads.rs:25, 50, 56` (`label`), traductions `ui/src/lib/locales/en-server.ts:120, 122` ; affichage `ui/src/tabs/Install.svelte:122-130` (`{t(it.label)}`).

11. **Aide → Moteurs : champ « Carte graphique »** — Aide `ui/src/tabs/Help.svelte:256-300` (section `engines`, + `HelpEn.svelte`) ; champ `ui/src/tabs/Settings.svelte:237-243` ; voir (b).
    Correctif : une question/paragraphe « À quoi sert le champ Carte graphique ? » (voir texte proposé en (b)), FR + EN.

12. **`tools/captures_interface.js` et l'onglet « Réglages »** — `tools/captures_interface.js:11` (commentaire) et `:37-47` (`if (label === 'Réglages')` + sélecteur `.pills button`).
    Cause : depuis la refonte (v0.9.0) il n'y a plus d'onglet « Réglages » ni de `.pills` ; le menu liste Accueil, Historique, Dictée, Style, Dictionnaire, Commandes vocales, Moteurs et modèles, Profil, Apparence, Données et système, Mesures, Aide (`ui/src/lib/tabs.ts`). Le bloc ne s'exécute jamais (inoffensif) mais l'exemple du commentaire est faux.
    Correctif : supprimer le bloc 36-47 et corriger le commentaire (ex. « Moteurs », « Dictée »). Vérifier si certaines pages ont des sous-onglets (`part`, `section` dans `Journal`/`Settings`) à capturer.

#### Tâche 3 — Carte « Sauvegarder et transférer »

13. **« Exporter… » muet quand il n'y a rien** — `ui/src/components/BackupTransfer.svelte:~94` (bouton → `nav.transfer(r.exp)`) ; les pages ignorent la demande : `ui/src/tabs/Modes.svelte:177-183` (`else if (custom.length === 1)` : 0 mode → rien ; plusieurs → rien non plus : la liste est à choisir à la main), `Dictionary.svelte:266-272` (`else if (words.length>0 || corrections.length>0)`), `CustomCommands.svelte:150-156` (`else if (customs.length > 0)`). `Users.svelte:94-` à vérifier.
    Cause : `nav.transferRequest` effacé sans message quand la page n'a rien à exporter.
    Correctif : un message visible (`role="status"`) : « Aucun mode personnel à exporter. » etc. Le plus simple : `nav.transferNotice = t('…')` lu par `BackupTransfer` (la carte est celle qui a reçu le clic) ou `notice()` (dialog) depuis la page ; test vitest par cas (0 mode, dico vide, 0 commande).

14. **Minuteur de 4 s qui efface la demande suivante** — `ui/src/lib/nav.svelte.ts:63-72` (`setTimeout(…, 4000)` avec test `this.transferRequest === kind`).
    Cause : le test compare le type, pas la demande : deux clics du même type en moins de 4 s (ou une demande prise puis redemandée) → le minuteur du premier efface le second.
    Correctif : numéro de demande (`#seq`) : `const id = ++this.#seq; … if (this.#seq === id && this.transferRequest === kind)` ; ou `clearTimeout` du minuteur précédent. Test vitest avec `vi.useFakeTimers()`. Ajouter : une page qui charge lentement (> 4 s) perd sa demande : envisager d'effacer à la navigation suivante plutôt que sur minuteur.

15. **Bouton « Annuler » au chemin à taper (Linux)** — 4 endroits : `ui/src/components/BackupTransfer.svelte:~77-82` (`restoring.askPath`, aucun bouton Annuler : `cancelRestore` n'existe que dans l'aperçu), `Modes.svelte:253-256`, `Dictionary.svelte:366-369`, `UserImport.svelte:99-102` (CustomCommands.svelte:206-209 a déjà « Annuler » l.227).
    Cause : le bloc `askPath` n'a que « Lire le fichier ».
    Correctif : un bouton `{t('Annuler')}` à côté (« Annuler » est déjà traduit) : `restoring = null` / `importing = null` / `reset()` ; pour l'import, appeler l'API `…/import/cancel` si un jeton existe (rien ici avant la lecture). Test par composant (ou logique extraite).

#### Tâche 4 — README

16. **README** — `README.md:18-37` et `README.en.md:~26-52`. L'« Installer » commence par « Pas à pas, pour une personne qui n'a jamais compilé… » avec 3 listes de guides (les guides `docs/INSTALLATION_*.md` commencent bien par « Étape 1 — Télécharger Scriibe » : OK), puis la phrase sur les Releases (l.26) vient APRÈS les guides, et la sous-section « Compiler soi-même » (l.28) est dans « Installer », avant « Principes ». Le gros paragraphe « État (S16…) » l.16 précède tout.
    Correctif : (1) tout début de « Installer » : 3 lignes « Mac / Windows / Linux : télécharge la dernière version sur Releases (lien), puis suis le guide pas à pas » ; (2) déplacer « Compiler soi-même » en « Pour les développeurs » (il existe déjà une section « Pour les développeurs » en anglais, l.~63) ; (3) raccourcir le bloc « État » (renvoi à CHANGELOG/REPRISE) si l'auteur le veut ; même chose dans `README.en.md`.

#### Tâche 6 — À vérifier sur le Mac

17. **Message de l'assistant « Tout est en place »** — `ui/src/components/Assistant.svelte:83` ; condition : `lacks.items` vide et `lacks.key` faux (`missing()` dans `ui/src/lib/assistant.ts:~70`, `isInstalled` l.45-65).
    Pour le voir : chaîne choisie « sans mise au propre » (`local-raw`, `crates/cli/src/assistant.rs:41`) a besoin de `whisper-server` (brew : `isInstalled` = binaire détecté, OK) et `whisper-base` (`has(s.whisper.models, 'ggml-base')`). Il manque donc seulement le fichier `ggml-base.bin` sur le Mac (à télécharger par la page Installation > Modèles recommandés, ou lien vers un modèle déjà présent : règle R41). Sur Mac l'élément « whisper-server (Windows) » apparaît pourtant dans la liste de la chaîne via `needLabels` (nom neutre « whisper-server », OK) mais pas via le téléchargement (voir (a)).

#### (a) Sens des libellés « (Windows) »

Le code (`crates/cli/src/web/downloads.rs`) :
- `whisper-server` « (Windows) » → zip `whisper-bin-x64.zip` de ggml-org/whisper.cpp ; refus ailleurs : `windows_only` l.200-207 et résolution l.220-224 ;
- `llama-server` « (Windows, processeur) » → `llama-*-bin-win-cpu-x64.zip` ; `llama-vulkan` « (Windows, carte graphique Vulkan) » → `…-bin-win-vulkan-x64.zip` ;
- Linux : `whisper-server-linux`, `llama-server-linux` (zips de la CI Scriibe) ; **Mac : aucun téléchargement** : les moteurs viennent de Homebrew (`engines::whisper::install_hint` « brew install whisper-cpp » ; `llama::install_hint` « brew install llama.cpp » ; guide `docs/INSTALLATION_MACOS.md` étape 5).
- `offered()` l.85-91 : `"whisper-server" | "llama-server" | "llama-vulkan" => !cfg!(target_os = "linux")` → **vrai sur Mac** : la page Installation du Mac montre ces trois programmes Windows, et le clic sur « Voir la taille » renvoie « proposé seulement sous Windows pour l'instant ». Le test `downloads.rs:543-544` fige ce défaut (`offered("llama-vulkan") == !linux`).

Ce sont donc de **vrais programmes propres à Windows** (binaires .exe/DLL), correctement nommés : pas des variantes mal nommées. Défaut : ils sont proposés à tort sur Mac. Aussi la chaîne « gpu » (`crates/cli/src/assistant.rs:24-34`, texte « llama.cpp Vulkan », `needs` = `llama-vulkan`) : sur Mac `for_this_system` (l.~115) ne remplace rien → elle exige un élément Windows ; sur Linux elle se rabat sur la version processeur sans le dire (le titre promet « sa carte graphique »).

**Recommandation (a)** : cacher, pas renommer. (1) `offered()` : `cfg!(windows)` pour les trois ids Windows (Mac : aucun) ; (2) sur Mac, remplacer la carte « Les programmes » par un encart « Installer avec Homebrew » : deux commandes (`brew install whisper-cpp`, `brew install llama.cpp`) tirées de `install_hint()` + état détecté (`s.whisper.binary`, `s.llama.binary`) ; (3) garder les libellés « (Windows) » / « (Linux) » tels quels sur ces systèmes — ils sont utiles pour distinguer les variantes (processeur / Vulkan) et pour l'auteur qui lit les captures ; ou, plus propre, retirer « Windows, » du libellé puisque la page ne montre plus que ceux du système courant (« llama-server (processeur) », « llama-server (carte graphique Vulkan) »), avec les traductions `en-server.ts:120-122` ; (4) assistant : sur Mac, `for_this_system` renvoie `whisper-server`/`llama-server` qui deviennent « à installer avec brew » (texte de `lacks.items` : « installer … ») ; la chaîne « gpu » sur Mac devient « Metal » (llama.cpp de brew l'utilise tout seul) : texte à adapter ; `every_need_is_a_real_download` (assistant.rs:183) à réécrire pour Mac. Raison : un programme Windows proposé sur Mac ne peut que produire une erreur ; la règle du carnet (« aucun texte propre à un autre système visible sur Mac ») le confirme.

#### (b) Rôle réel du champ « Carte graphique »

- Affichage : `ui/src/tabs/Settings.svelte:236-243`, seulement quand la mise au propre est « llama.cpp » (`v.cleanupEngine === 'llamaCpp'`) ; liste « Automatique », « Processeur seulement », + cartes trouvées par le bouton « Lister les cartes » (`GET api/llama/devices` → `llama_devices`, `crates/cli/src/web/mod.rs:1371`, qui lance `llama-server --list-devices`).
- Effet : réglage `llamaDevice` (`crates/core/src/settings.rs:390-392`, validé par `is_device_name` l.612) → `LlamaLauncher::set_device` (`crates/engines/src/llama.rs:237`) → option `--device <nom>` au démarrage de `llama-server` (llama.rs:268-274) ; « none » = processeur seul. Vide : llama.cpp choisit et répartit le modèle entre toutes les cartes.
- Rien d'autre : ne touche pas Whisper (transcription), pas LM Studio ni Ollama. Repli automatique : un modèle qui ne démarre pas sur la carte (mémoire) est relancé en `--device none` (llama.rs:272, 297-308) tant que le choix est « Automatique » ; un choix explicite l'emporte et supprime le repli.
- Utilité réelle : portable à deux cartes (intégrée + dédiée) où le choix automatique répartit mal ; forcer le processeur quand la carte (2 Go) est trop petite ; hors Windows-Vulkan le champ a peu d'effet (Linux : binaire processeur seulement ; Mac : Metal automatique, « MTL0 » apparaît dans la liste). Le rapport de support l'imprime (`ui/src/lib/report.ts:13`).
- Texte d'Aide proposé : « Carte graphique (mise au propre llama.cpp seulement) : choisit sur quel matériel tourne le modèle. « Automatique » laisse llama.cpp décider (et Scriibe repasse seul sur le processeur si la carte manque de mémoire) ; « Processeur seulement » force le processeur ; « Lister les cartes » montre celles que llama-server sait utiliser. Utile sur un portable qui a deux cartes, ou quand le modèle ne tient pas dans la mémoire de la carte. Cela ne change pas Whisper. »

#### Incertitudes
- Point 9 (« + » en trop) : cause probable relue dans le code, non reproduite sous GNOME.
- Point 8 : `UserExport.svelte` et `Users.svelte` non relus ligne à ligne pour la formule d'export ; `Help.svelte` : 3 lignes (201, 229, 321) contiennent la formule, à confirmer à la tâche 2.
- Numéros de lignes de `HelpSearchBox.svelte` et `BackupTransfer.svelte` approximatifs (±3).

## Mise en page (tâche 1, 18/11/2026)

Les 7 points du Constat (1 à 7) sont corrigés dans `ui/src`. Tests : `npm run check` (0 erreur), `npm test` (146 passés, dont 7 nouveaux dans `lib/tabs.test.ts`), `npm run build` : verts. `cargo` non relancé (aucun Rust touché).

| Point | Correctif |
|---|---|
| 1 Recherche de l'Aide | `HelpSearchBox.svelte` : `position: sticky` sous l'en-tête, hauteur publiée dans `--help-search` ; `.toc` de `Help` et `HelpEn` en tient compte (bureau et fenêtre étroite) |
| 2 `scroll-padding-top` | `App.svelte` (`:global(html)`) : en-tête + recherche + 8 px. Remplace le `[id] { scroll-margin-top }` de `app.css` (les deux ensemble auraient doublé le décalage des ancres) |
| 3 Bandeau d'erreur | `App.svelte` : `main > .card.bad { margin-bottom: 20px }` (compense la remontée de l'en-tête, plus de 6 px rognés) |
| 4 Tuiles de l'Accueil | `Dashboard.svelte` : `grid-auto-rows: 1fr` |
| 5 Menu étroit (≤ 640 px) | `App.svelte` : une seule bande qui défile de côté (`nowrap`, `overflow-x: auto`), boutons compacts |
| 6 Accessibilité du menu | `<nav aria-label="Menu principal">` (traduit « Main menu »), `aria-current="page"` sur la page courante et sur Aide, icônes `aria-hidden` |
| 7 Commandes décochées | `Commands.svelte` : `inert` en plus de `pointer-events: none` ; la carte d'activation est hors du bloc |

### Vérification à l'écran (Mac, navigateur du volet, serveur Vite sans le programme)

Vu : recherche de l'Aide collée à 120 px sous l'en-tête après défilement, sommaire à 167 px (= en-tête + recherche + 8) ; bandeau « Scriibe ne répond plus » avec 20 px de marge ; menu en bande de côté à 375 px ; `aria-label`, `aria-current` et icônes cachées lus dans le DOM.

**Non vu à l'écran** : tuiles de l'Accueil de même hauteur et `inert` des commandes décochées, car il faut les réglages et les statistiques du programme (sans lui, la page n'a pas de données). Ils sont couverts par les tests de source seulement. À voir à la tâche 6 (essai sur le Mac avec `Scriibe.app`, lancée par `open`).

## Textes (tâche 2, 18/11/2026)

Tests : `npm run check`, `npm test` (148 passés, 2 nouveaux dans `lib/textes.test.ts`), `npm run build`, `cargo fmt`, `cargo clippy --workspace -D warnings`, `cargo test --workspace` : verts (sur le Mac).

| Point | Correctif |
|---|---|
| Texte de la carte d'export | `lib/exportwhere.ts` : « Une boîte te demande où l'enregistrer. » (Mac, Windows) ou « Il est placé dans le dossier Documents. » (Linux), dans `Modes`, `Dictionary`, `CustomCommands` (phrases découpées, traductions en.ts). Aide (`Help`, `HelpEn`, 3 passages chacune) : la phrase « Sous Linux… » n'apparaît plus que sous Linux (`{#if linux}`) |
| « (Windows) » (décision du Constat) | `offered()` : les 3 programmes Windows seulement sous Windows (aucun sur Mac) ; libellés sans « Windows, » (`whisper-server`, `llama-server (processeur)`, `llama-server (carte graphique Vulkan)`, traductions `en-server.ts`, guides d'installation Windows FR et EN) ; Mac : carte « Les programmes » remplacée par l'encart Homebrew (état détecté + 2 commandes) ; ligne d'état « à installer avec Homebrew » ; assistant : sur Mac « llama-vulkan » devient `llama-server`, texte de la chaîne « gpu » sans « Vulkan » ; test `every_need_is_a_real_download` adapté (Mac : Homebrew) |
| « + » en trop | `joinKeys()` (`i18n.svelte.ts`) saute les libellés vides ; Banc, Journal (phrase sans « {translate} en plus »), ligne « Touche globale » de l'état, KeyPicker (« aucune ») ; côté Rust, `translate_label` est vide sous GNOME quand il n'y a pas de raccourci de traduction (avant : « Maj », faux). **Non reproduit sous GNOME** : à voir à la tâche 6 |
| Aide → champ « Carte graphique » | Nouvelle rubrique dans Moteurs (FR et EN), avec une phrase propre à Mac et à Linux |
| `tools/captures_interface.js` | Bloc « Réglages » et sélecteur `.pills` supprimés, commentaire corrigé |

### Vérification et limites
- Vu à l'écran (volet navigateur, Vite sans le programme) : sur Mac, la page Moteurs montre l'encart Homebrew et aucun mot « Windows ».
- **Non fait** : les captures (`tools/captures_interface.js`) n'ont pas été relancées, Playwright n'étant pas installé sur ce Mac (`require('playwright')` échoue) ; le script passe `node --check`. À relancer depuis une session qui l'a, ou après accord de l'auteur pour l'installer.
- Sous Linux, la chaîne « gpu » de l'assistant promet « sa carte graphique » alors que le seul programme proposé est la version processeur : non traité (hors périmètre), à décider.
- Clippy pour Windows (`x86_64-pc-windows-gnu`) non relancé ; `offered()` y est inchangé en logique pour les ids Windows.

## Sauvegarde (tâche 3, 18/11/2026)

| Point | Correctif |
|---|---|
| 13. « Exporter… » muet | `nav.transferNothing(message)` : la page visée vide la demande et laisse un message (`nav.transferNotice`, effacé à la demande suivante ou après 8 s) que `BackupTransfer` affiche en `role="status"` sous la liste. Cas : 0 mode, plusieurs modes (« exporte celui que tu veux avec son bouton », car l'export se fait mode par mode), dictionnaire vide, 0 commande, utilisateurs indisponibles. Textes dans `en.ts` |
| 14. Minuteur de 4 s | Numéro de demande (`#seq`) : le minuteur n'efface que la demande qui l'a lancé (le même type cliqué deux fois ne perd plus la seconde). **Non traité** : une page qui charge plus de 4 s perd encore sa demande (rare ; la demande est vidée et le message n'apparaît pas) |
| 15. « Annuler » au chemin à taper | Bouton ajouté aux 4 endroits : `BackupTransfer` (ferme sans appel serveur : rien n'a été lu), `Modes` et `Dictionary` (`closeImport`), `UserImport` (`cancel`). `CustomCommands` l'avait déjà |

Tests : `lib/transfer.test.ts` (6 cas : minuteur deux clics, effacement à 4 s, message puis effacé, effacement à 8 s, pages et carte câblées, « Annuler » dans chaque bloc de chemin). Il n'y a pas de banc de composants dans le projet : le câblage des pages est vérifié sur le texte des sources (comme `links.test.ts`). `npm test` 154 passés, `npm run check` 0 erreur. Pas vu à l'écran.

## README (tâche 4, 18/11/2026)

`README.md` et `README.en.md` : « Installer » s'ouvre sur 3 lignes (Mac, Windows, Linux : télécharger sur les Releases, puis suivre le guide) ; « Compiler soi-même » est déplacé dans « Pour les développeurs » (créée dans le README français) ; le bloc « État » est réduit à une ligne avec renvois (CHANGELOG, ARCHITECTURE, ROADMAP, ESSAI_*), l'ancien texte reste dans l'historique git.

## Relecture (tâche 5, 18/11/2026)

Sous-agent Sonnet (effort élevé), lecture seule, sur `e87b0d8..HEAD` : 1 bloquant, 3 importants, 9 mineurs. Tests de départ : `npm test` (154), `npm run check`, `cargo test --workspace` verts.

**Corrigés** (tests dans `lib/transfer.test.ts`)
- **Bloquant B1** : le message « rien à exporter » était posé dans la carte de la page Données, démontée dès que `nav.transfer` change de page : toujours muet. `transferNothing` ouvre maintenant une boîte (`dialog.notice`) sur la page d'arrivée ; `transferNotice` et son affichage dans `BackupTransfer` sont supprimés. Test : la boîte est appelée et la demande vidée. (Le test d'origine vérifiait seulement le texte des sources : il ne pouvait pas voir ce défaut.)
- **I1** : guides Mac (débutant et expert, FR et EN) : la carte « Les programmes » montre les commandes Homebrew, elle ne propose plus de programmes Windows et Linux.
- **I2** : « Un fichier qui existe déjà n'est jamais écrasé » remis dans `{#if linux}` (Aide FR et EN, 3 passages chacune) ; vrai seulement avec le chemin Linux, la boîte du système permettant de remplacer. Test sur les sources.
- **I3** : touche de traduction vide (GNOME) : Aide FR et EN (`{#if translate}`, sinon phrase sans touche) et rapport (`report.ts`, plus de « (traduction : ) »). Test pour le rapport.
- M1 (test « Annuler » creux pour `BackupTransfer`) : borné au bloc du chemin. M2 et M3 (minuteur du message, style) : sans objet avec la boîte. M6 : intitulé anglais du README.

**Mineurs restants**
- M4 : la carte « Mes commandes » est dans une zone `inert` quand les commandes vocales sont décochées : un export lancé depuis Données y ouvre une carte inerte.
- M5 : « Réglages → Moteurs » (menu devenu « Moteurs et modèles » depuis 0.9.0) dans les guides Windows, Mac et Linux (débutant et expert) : défaut ancien, à corriger en bloc.
- M7 : « version 0.10.1 publiée » en dur dans les README : à ajouter à la liste de clôture (`docs/CLOTURE.md`).
- M8 : Linux, la chaîne « gpu » de l'assistant promet une carte graphique alors que seule la version processeur est proposée (déjà noté).
- M9 : 20 px au lieu de 14 entre deux bandeaux `config_problems` (cosmétique).

Limites : clippy Windows non lancé ; rien vu à l'écran pour ces corrections ; tests à relancer sur Mac : `npm test` 155 passés, `npm run check` 0 erreur, `npm run build` ok.

## Essai (tâche 6, 18/11/2026)

Branche temporaire `essai-c2` poussée pour amener le code sur les PC (`pilote maj` ne tire que depuis GitHub), compilée en release sur les trois machines (c385f21 : 6919a82 + les corrections de l'essai). Gestes faits par l'auteur, une étape à la fois.

### Mac (Scriibe.app lancée par `open`)
| Point | Résultat |
|---|---|
| Tuiles de l'Accueil | les 4 du haut égales ; **les cartes « Mots dictés par jour » et « Où je dicte » ne l'étaient pas** → `align-items: stretch` (`Dashboard.svelte`), revu : égales |
| En-tête et champ de recherche fixes (Aide) | OK ; le champ glissait de 8 px avant de se coller (marge de 8 px sous l'en-tête) → `margin-top: -8px`, revu : fixe. Le bandeau « Salut… » n'est pas un en-tête et ne doit pas rester fixe |
| Menu étroit | OK : bande qui défile de côté, tout accessible |
| Textes sans autre système (Moteurs et modèles) | OK : encart Homebrew, aucun « Windows », « Vulkan » ni « Linux » |
| Carte de sauvegarde | « Restaurer une sauvegarde… » ouvre la boîte du système, annulée : OK |
| « Exporter… » → Mes commandes | arrivait **à côté de la carte** (cartes du dessus encore en train de grandir) → `nav.go` attend 6 images stables avant de défiler ; revu : OK. Cas « rien à exporter » (boîte) **non vu** : l'auteur a des commandes |
| Assistant « Tout est en place » | OK avec `ggml-base.bin` (147 951 465 octets, téléchargé sur Hugging Face avec l'accord de l'auteur, rangé dans `~/Library/Application Support/Scriibe/models/`) ; réglages remis comme avant |

### Windows
Programmes « whisper-server », « llama-server (processeur) » et « (carte graphique Vulkan) » sans le mot Windows : OK. Carte d'export : « Une boîte te demande où l'enregistrer. » : OK. Restaurer : boîte du système, annulée : OK.

### Linux (GNOME)
Programmes « whisper-server », « llama-server (processeur) » : OK. Carte d'export : « Il est placé dans le dossier Documents. » : OK. Restaurer : champ du chemin avec **Annuler**, qui le ferme : OK. Touches affichées = raccourcis GNOME (Ctrl+Gauche, Ctrl+Haut, Ctrl+Bas) : OK. Aucun « + » en trop dans « Dicter avec la touche dans une application » : OK (Banc et ligne « Touche globale » non relus).
**Piège** : Scriibe lancée par SSH n'a pas l'autorisation d'accès à l'écran (« Touche globale indisponible : écran X11 introuvable », touche par défaut fausse). Sous Linux, la lancer depuis la session graphique (Activités), jamais par SSH.

## Ajouts
1. Cartes « Mots dictés par jour » / « Où je dicte » de hauteurs différentes : traité (voir Mac).
2. Champ de recherche de l'Aide qui bouge de 8 px : traité.
3. Police du champ de recherche trop petite (`type="search"` hors de la règle commune de `app.css`) : traité.
4. Recherche de l'Aide qui montre les mots intermédiaires (« rac »…) : traité, attente de 180 ms après la frappe.
5. « Exporter… » sur Mes commandes arrive à côté de la carte : traité.
