# Petits plus (chantier 3 du tri du 12/11/2026)

Trois lignes du carnet du 11/11 : prénom de la salutation, phrases de test adaptées au genre, historique filtrable sur les dictées reformulées. Ce fichier garde le constat (tâche 0), puis le résultat de chaque tâche.

## Constat

Recherche faite le 12/11 par un sous-agent Sonnet en lecture seule ; points clés vérifiés par sondage (`Dashboard.svelte:53`, `machine.rs`, `settings.rs`, `history.rs`, nombre de phrases du Banc). Les numéros de ligne sont ceux du commit `4560938` et bougeront. L'interface parle au Rust par un serveur HTTP local (`crates/cli/src/web/mod.rs`, routes `api/…`), sans `invoke()` Tauri.

### 1. Prénom de la salutation

**Le prénom n'est ni en dur ni un réglage : il vient du nom du compte système**, calculé une fois par processus.

Chemin de la donnée :

1. `crates/platform/src/machine.rs` : `first_name()` (l. 123) lit `USER`, à défaut `USERNAME`, puis `first_name_from(&account_full_name(&login), &login)`.
   - `account_full_name` (l. 100) : macOS `id -F` ; Linux `getent passwd` (5ᵉ champ, jusqu'à la virgule) ; **Windows : chaîne vide, donc repli sur le login**.
   - `first_name_from` (l. 91) : premier mot du nom complet, sinon du login ; première lettre en majuscule ; vide si rien.
2. `crates/cli/src/web/mod.rs` : `status()` (l. 579) renvoie `"first_name"` (l. 595), route `GET api/status`.
3. `ui/src/lib/api.ts` : `Status.first_name` ; `ui/src/lib/status.svelte.ts` : `StatusStore.refresh()` (relu toutes les 8 s et après chaque changement de réglage).
4. `ui/src/tabs/Dashboard.svelte` l. 53 : `t('Salut {name}', { name })`, ou `t('Salut')` si le prénom est vide. Traductions : `ui/src/lib/locales/en.ts` l. 31-33 (`Hi {name}`, `Hi`, et la phrase d'accroche).

Conséquences : sous Windows le « prénom » est le login avec une majuscule (ex. « Scarou ») ; sous macOS un nom complet « Nom Prénom » donne le nom ; **rien n'est modifiable dans l'interface** ; salutation unique (ni heure ni genre).

**Réglage `speakerGender` (à côté duquel ranger le prénom)** :

| Élément | Où |
|---|---|
| Type | `crates/core/src/modes.rs` l. 49-79 : `enum Gender { Unspecified (défaut), Masculine, Feminine }`, sérialisé `"unspecified"`, `"masculine"`, `"feminine"` ; `prompt_line()` |
| Champ | `crates/core/src/settings.rs` l. 247-248 (`#[serde(rename = "speakerGender")]`, « propre à Scriibe »), défaut l. 371 |
| Lecture tolérante | `AppSettings::from_value`, l. 474 (valeur inconnue → `Unspecified`) ; test `crates/core/tests/speaker_gender.rs` |
| Persistance | `settings.json` dans le dossier de données (`storage.rs::data_folder`) ; `FileConfig::save_settings` (`storage.rs` l. 116, écriture atomique) ; `Watched<T>` relit à chaud |
| Interface → disque | `ui/src/tabs/Settings.svelte` l. 265-274 → `SettingsStore.change()` (`settings.svelte.ts`) → POST `api/settings` → `merge_settings` (`web/mod.rs` l. 637, **clé inconnue refusée**) → `patch_settings` (l. 657) |
| Type côté interface | `ui/src/lib/api.ts` l. 65 |
| Usage | `settings.rs` l. 602 et 609 (`profile_for`, `active_profile` posent `profile.speaker`) ; `cleaner.rs::system_prompt` l. 327-341 : n'agit que sur les modes qui réécrivent (Reformulé, Libre) ; décision dans `docs/ECARTS.md` l. 87 |
| Aide | `Help.svelte` l. 200, `HelpEn.svelte` l. 213 |

**Profils** : il n'existe aucun profil utilisateur ; le genre est global. `ModeProfile.speaker` (`modes.rs` l. 264) est transitoire, écrasé au vol par le réglage global. Les modes personnels (`CustomMode`, `modes.rs` l. 302, `Serialize` écrit à la main l. 316-330) portent `id`, `name`, `style`, `fidelity`, `vouvoie`, `examples`, **pas** le genre. Un réglage « prénom » placé à côté de `speaker_gender` dans `AppSettings` suivra donc le même chemin (champ, défaut, `from_value`, `merge_settings`, type TS, Settings.svelte) et pourra migrer vers un profil au chantier 4.

### 2. Phrases que Scriibe propose de lire

**Aucune phrase à lire ne s'accorde au genre, et aucune ne contient de mot à accorder** (pas de « content », « prêt », « fatigué », « ravi »…), vérifié dans `crates/core/data`, `ui/src/tabs`, `ui/src/components` et `docs/INSTALLATION_*.md`. **Il n'existe aucun mécanisme d'accord pour un texte affiché** : le seul usage du genre est `Gender::prompt_line()`, pour le modèle. Il n'y a pas de test du micro dédié : le niveau se voit pendant la lecture d'une phrase du Banc.

Phrases réellement lues à voix haute :

| Texte | Emplacement | Accordée au genre |
|---|---|---|
| **17 phrases du Banc de dictée** (fr) ; 17 en anglais, dont certaines différentes (ex. n° 11) | `crates/core/data/bench_phrases.json` et `bench_phrases_en.json` ; `bench.rs::phrases_for(language)` (l. 75) ; route `api/phrases` (`web/mod.rs` l. 201, fn l. 495) ; `ui/src/tabs/Bench.svelte` l. 143 (consigne), l. 190 (phrase) | non |
| « Bonjour, je teste Scriibe. » / « Hello, I am testing Scriibe. » | `docs/INSTALLATION_WINDOWS.md:83`, `_LINUX.md:111`, `_MACOS.md:146` ; `.en.md` : 85, 113, 148 | non |

Les 17 phrases françaises du Banc : « Je serai à la maison vers dix-neuf heures, on pourra dîner ensemble. » ; « J'ai relu le compte rendu de la réunion de mardi, il manque seulement la liste des participants et la date de la prochaine séance. » ; « Est-ce que tu peux m'envoyer le devis avant vendredi midi ? » ; « Euh, je pense qu'on pourrait, euh, décaler la séance à jeudi. » ; « Bon ben, je te rappelle demain matin pour en parler. » ; « Le rendez-vous est à dix heures, non pardon, à onze heures. » ; « Le rendez-vous est à dix heures non à onze heures. » ; « On part lundi, enfin mardi, selon la météo. » ; « On part lundi enfin mardi selon la météo. » ; « Je, je voulais te dire que le colis est bien arrivé. » ; « Le colis ouvrez la parenthèse le gros fermez la parenthèse est arrivé point à la ligne merci encore » ; « Pourquoi le serveur ne répond plus depuis ce matin » ; « Tu m'envoies le devis avant vendredi » ; « J'ai ajouté une skill dans Claude Code pour ranger l'inbox du Vault. » ; « La facture fait mille deux cent cinquante euros, à régler avant le quinze novembre. » ; « Tu vois, le truc, c'est qu'on n'a pas le temps cette semaine. » ; « Pour la séance de demain, je propose qu'on commence par une respiration abdominale de cinq minutes, puis une visualisation du lieu ressource, et qu'on termine par un temps d'échange sur ce que chacun a ressenti pendant la semaine. » (lignes 4, 27, 48, 65, 87, 109, 141, 166, 187, 208, 229, 256, 266, 276, 295, 320, 341 du JSON français.)

Textes proches mais **non lus** (envoyés au modèle, tapés ou recopiés) :

| Texte | Emplacement | Remarque |
|---|---|---|
| 12 phrases types de l'essai d'un mode | `clean_samples.json` l. 2-13, `clean_samples_en.json`, `clean_test.rs` l. 10-27, `ModeTrial.svelte`, `web/trial.rs::sample` | texte envoyé au modèle ; non accordées |
| 5 phrases tendues ajoutées à l'essai d'Apaisé (« t'as encore oublié d'aller chercher les enfants… ») | `clean_test.rs` l. 32-38 (fr), 39-45 (en), `samples_for_mode` l. 49 | texte envoyé au modèle ; adressées à un tiers |
| Texte d'essai du banc de modèles (« euh donc je voulais te dire que… ») | `settings.rs:363` (`trial_text`), `TrialBench.svelte:103-105` | phrase tapée, modifiable |
| Exemples « Tu dis » de l'Aide | `Help.svelte` l. 154-158, 184-186, 215, 219 ; `HelpEn.svelte` l. 167-171, 197-199, 228, 232 | illustrations, pas des invites |
| Commandes vocales (« entre parenthèses »…) | `Help.svelte` ~l. 225-230, `VoiceCommands.svelte` | commandes à dire |
| Test de frappe, assistant de premier lancement | `TypingTest.svelte`, `Assistant.svelte` | sans objet |

**Point d'ancrage si l'on veut des phrases accordées** : `bench::phrases_for` (`bench.rs` l. 75) et la route `phrases(language)` (`web/mod.rs` l. 495), qui liraient `settings.speaker_gender` ; nouveau marquage dans le JSON des phrases (champ par genre ou balise) ; rien à changer dans `Bench.svelte`, qui affiche `phrase.text` tel quel. **Aujourd'hui aucune phrase n'en a besoin** : l'accord au genre ne se justifie que si l'on ajoute des phrases à accorder (ex. pour Apaisé, « je suis content/contente »), à décider à la tâche concernée.

### 3. Historique et dictée reformulée

**Il n'existe aucun marqueur « reformulée » aujourd'hui, et le cran de fidélité n'est pas enregistré.** On peut seulement le déduire.

Champs d'une dictée (`crates/core/src/history.rs`, `struct Dictation` l. 55-81, noms JSON) : `id`, `date`, `appName`, `bundleID`, `rawText` (Whisper), `finalText`, `words`, `duration`, `corrections`, `translation`, `timing` (`DictationTiming { transcription, cleanup, total, outcome }`), `mode` (**nom** du mode actif, texte libre, l. 78-80). `enum Cleanup` (l. 22) : `model`, `skipped`, `short`, `none`, `translated`, `fallback`, `rejected`, `failed`.

Calcul (`crates/pipeline/src/controller.rs`) : `outcome` l. 856-915 (`Model` l. 910 ; `Rejected` si `REWRITE_REJECTED`, sinon `Failed` l. 915) ; `dictation.mode` l. 986 (renseigné seulement si `settings.rewriting_on()`) ; `history.record` l. 987. Garde-fou : `crates/engines/src/services.rs::cleanup` (l. 325-349) : si le modèle est refusé et que le mode réécrit, **second essai en Simple**, sinon texte brut collé ; cette décision n'est pas reportée dans l'historique (sauf `rejected`). Si « Garder le texte » est décoché, `Store::add` (`history.rs` l. 321) vide `rawText` et `finalText`.

Crans (`modes.rs::Fidelity` l. 15-35, `cleaner.rs::accepts` l. 88-130, constantes l. 31-37, mesure `text::overlap` l. 172-195) : Littéral ≥ 90 % de mots gardés dès 5 mots ; Léger (Simple par défaut) ≥ 75 % dès 8 mots ; Reformulé souple (`kept >= 0.15 && own >= 0.3`) ; Libre sans contrôle de mots gardés (mode Apaisé). Mode livré → cran : Apaisé = Libre ; Soutenu et métiers = Reformulé ; Simple = Léger (réglable) ; un mode personnel choisit le sien.

Stockage : `crates/platform/src/storage.rs`, `FileHistory::persist` (l. 200-211), `history.json` (tableau, plus récent en premier) et `history-totals.json` ; lecture tolérante `lossy_list(..., Dictation::from_value)` (l. 177) : **un champ optionnel ajouté ne demande aucune migration** (absent = ancienne dictée). Envoi à l'interface : `crates/cli/src/web/data.rs::dictation` (l. 41-57), `GET api/history` (`web/mod.rs` l. 340) ; type `HistoryItem` (`ui/src/lib/api.ts` l. 273-287).

Affichage actuel (`ui/src/tabs/History.svelte`) : « Ce que Whisper a entendu » (l. 188) si `raw !== text` ; ligne « Mode : … — {libellé de `outcomeLabels`} · transcription · mise au propre · total » (l. 193-198 ; `ui/src/lib/timings.ts` l. 5-14). **Aucune pastille « reformulée »** : « Mode : Simple — Modèle » ressemble à « Mode : Apaisé — Modèle ».

Filtres (côté interface seulement, le serveur renvoie tout) :
- états `$state` l. 12-17 (`items`, `apps`, `app`, `search`, `period`, `from`, `to`) ; dérivés l. 25-27 (`shown`, `groups`, `filtered`) ; barre `.filter-bar` l. 173-208 ; `resetFilters` l. 49-55 ; la sélection et l'effacement groupé agissent sur `shown`.
- logique pure `ui/src/lib/historyfilter.ts` : `HistoryFilter { period, from, to, app, search }` (l. 5-12), `matchesHistory(item, filter, now)` (l. 32-42, l'élément n'a besoin que de `{ date, app, text, raw }`), `groupByDay` ; tests `historyfilter.test.ts`.

**Où brancher un filtre « reformulées »** (deux options, à trancher à la tâche concernée) :
1. **Marqueur enregistré** (le plus fiable) : champ optionnel dans `Dictation` (+ `from_value`, `skip_serializing_if`), calculé dans `controller.rs` près de l. 910 ou 986, exposé par `data.rs::dictation` et `HistoryItem`. Piège : le repli sur Simple de `services.rs::cleanup` fait diverger le mode actif et le texte réellement produit ; `cleanup` devrait signaler l'essai retenu.
2. **Calcul à l'affichage** : à partir de `mode` et de `timing.outcome === 'model'`, ou de la comparaison `raw`/`text`. Limites : `mode` est un texte libre (mode renommé ou supprimé indéchiffrable) ; `raw` et `text` sont vides si le texte n'est pas gardé.

Côté interface dans les deux cas : une clé dans `HistoryFilter` et une condition dans `matchesHistory` (+ test), un état dans `History.svelte` (inclus dans l'appel de l. 25, dans `filtered`, dans `resetFilters`), un contrôle dans `.filter-bar`, un badge « Reformulée » dans `.meta`, la traduction dans `en.ts` (`i18n.test.ts` échoue sinon), puis `ECARTS.md`, `Help.svelte` / `HelpEn.svelte` et `CHANGELOG.md`.

## Prénom (tâche 1)

**Choix** : réglage `firstName` (texte, `AppSettings.first_name`, clé JSON `firstName`) rangé juste après `speakerGender`. **Vide par défaut = le prénom d'aujourd'hui** (celui du compte de l'ordinateur, `machine::first_name()`), plutôt qu'une copie figée à la première lecture : il suit le compte tant que la personne n'a rien saisi, et rien ne change pour les installations existantes. Rempli, il remplace le prénom du compte. Des espaces seuls valent vide ; une valeur qui n'est pas du texte retombe sur vide (lecture tolérante, comme le genre).

**Où** :
- `crates/core/src/settings.rs` : champ, défaut, `from_value`, `AppSettings::greeting_name(system)` (choix entre réglage et compte).
- `crates/cli/src/web/mod.rs::status` : `first_name` renvoie `settings.greeting_name(machine::first_name())` ; la salutation du Tableau de bord n'a pas changé (`Dashboard.svelte`), elle relit `api/status` (relu toutes les 8 s et après chaque réglage).
- `ui/src/tabs/Settings.svelte` (onglet Dictée, avant le genre) : champ de texte (40 signes), le prénom du compte en grisé quand il est vide ; `ui/src/lib/api.ts` ; traductions dans `locales/en.ts`.
- Aide : `Help.svelte` et `HelpEn.svelte`, paragraphe sur les pages de la fenêtre (Premiers pas).
- Persistance : `settings.json` (comme le genre), relu à chaud, gardé après redémarrage. Le champ est accepté par `merge_settings` sans autre changement (la clé existe dans les réglages sérialisés).
- Écart noté dans `docs/ECARTS.md`. Pas de profil à ce stade : `firstName` comme `speakerGender` est global ; pour le chantier 4, les deux sont les champs « de la personne » à déplacer.

**Tests** (12/11, Mac) : nouveau `crates/core/tests/first_name.rs` (défaut vide donc prénom du compte ; réglage prioritaire, espaces seuls ignorés ; aller-retour JSON, absent, espaces rognés, valeur de mauvais type). `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings` sans erreur ; `cargo test --workspace` : aucun échec, dont les 3 tests nouveaux ; interface : `npm run check` 0 erreur, `npm test` 93 passés (dont `i18n.test.ts` et la comparaison Aide FR/EN), `npm run build` ok. Non essayé à la main dans la fenêtre (saisie du prénom, salutation, redémarrage).

## Phrases (tâche 2)

**Résultat : aucune phrase proposée à la lecture ne s'accorde avec la personne qui lit, donc rien n'est réécrit.** Relecture des 17 phrases françaises du Banc, de leurs 17 versions anglaises et de « Bonjour, je teste Scriibe. » / « Hello, I am testing Scriibe. » (guides d'installation) : pas de « je suis content/contente », « prêt/prête », « fatigué/fatiguée ». Les passés composés sont avec « avoir » (« J'ai relu », « J'ai ajouté », « chacun a ressenti ») ; « est bien arrivé » parle du colis ; l'anglais n'accorde rien. La forme neutre (sans accord) existe donc déjà et sert pour Masculin, Féminin et Non précisé.

**Choix** : ne pas bâtir de mécanisme de variantes par genre (`phrases_for` qui lirait `speaker_gender`, champ par genre dans les JSON) sans phrase qui s'en serve : ce serait du code mort, et modifier les phrases du Banc fausserait la comparaison avec Scribe (les 17 phrases françaises viennent de lui, `docs/ECARTS.md`). Les phrases d'essai des modes (dont les tendues d'Apaisé) ne sont pas lues : elles partent au modèle, qui accorde déjà selon `speakerGender` (`Gender::prompt_line()`). Si l'on ajoute un jour une phrase à lire qui s'accorde, le point d'ancrage reste `bench::phrases_for` et la route `phrases` (voir § Constat).

**Garde-fou** : `crates/core/tests/phrases_gender.rs` (4 tests) échoue si une phrase du Banc, français ou anglais, ou l'exemple des guides contient « je suis / je me sens / je me suis / j'étais / je serai / je reste / je deviens » suivi d'un mot qui s'accorde ; un test vérifie que le garde-fou attrape bien « Je suis content » et « Je me sens fatiguée ». Il n'y a donc **pas de test « par phrase accordée »**, puisqu'aucune ne l'est.

**Tests** (12/11, Mac) : `cargo fmt`, `cargo clippy -p scriibe-core --all-targets -- -D warnings` sans erreur ; `cargo test --workspace` sans échec, dont les 4 tests nouveaux. Interface non touchée.

## Historique (tâche 3)

**Définition retenue (validée par l'auteur : « tout texte changé »)** : une dictée est **reformulée** quand la mise au propre par le modèle a été retenue (`timing.outcome == "model"`) **et** que le texte final diffère du texte brut (`raw !== text`), quel que soit le mode, Simple compris. **À part, donc non reformulées** : écartée par le garde-fou (`rejected`, texte brut collé), sautée, courte, sans modèle, traduite, repli, échec, et une dictée que le modèle a rendue identique. **Texte non gardé** (réglage « Garder le texte » décoché : rien à comparer) : reformulée dès que le modèle a répondu (limite acceptée, dite dans l'Aide).

**Choix** : calcul à l'affichage (option 2 du § Constat), sans nouveau champ dans `history.json` ni dans le Rust : les deux données utiles (`outcome`, `rawText`/`finalText`) y sont déjà, et la définition « texte changé » n'a pas besoin du cran de fidélité, qui n'est pas enregistré. Un marqueur enregistré serait à ajouter si l'on veut un jour « modes qui réécrivent seulement » ; il faudrait alors que `services.rs::cleanup` signale le repli sur Simple (piège noté au § Constat).

**Où** :
- `ui/src/lib/historyfilter.ts` : `isRewritten(item)`, clé `rewrittenOnly` de `HistoryFilter`, condition dans `matchesHistory` (cumulable avec période, appli, mots clés).
- `ui/src/tabs/History.svelte` : case « Dictées reformulées » dans la barre de filtres (comptée dans `filtered`, remise à zéro par « Tout afficher »), pastille « Reformulée » à côté de l'appli.
- `ui/src/lib/locales/en.ts` : 3 textes ; Aide : question « Comment retrouver les dictées que le modèle a retouchées ? » dans `Help.svelte` et `HelpEn.svelte`.
- `docs/ECARTS.md` : écart noté.

**Tests** (12/11, Mac) : 2 tests nouveaux dans `historyfilter.test.ts` (définition : modèle retenu et texte changé, garde-fou, issues sans modèle, texte non gardé ; cumul avec période, appli et mots clés). `npm run check` 0 erreur, `npm test` 95 passés (dont `i18n.test.ts` et la comparaison des Aides FR/EN), `npm run build` ok ; `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` et `cargo test --workspace` sans échec. Non essayé à la main dans la fenêtre (case, pastille, cumul sur de vraies dictées).
