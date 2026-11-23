# Mot dicté collé plusieurs fois (chantier 4)

Ligne du carnet du 13/11/2026 : dicter un seul mot (« fait ») colle plusieurs exemplaires à la suite (« Fé. Fé. Fé. Fé. »). Découpage : `decoupage-2026-11-17-c4-mot-repete.md` (vault, `1 - Projets/scriibe/decoupages/`).

## Cause

**Piste retenue : la 1, Whisper répète sur une prise très courte, et le garde-fou de S13 ne couvre pas ce cas.** Ni la mise au propre ni le collage ne sont en cause.

### Preuves

1. **La répétition est déjà dans la sortie brute de Whisper.** Dictée du 13/11/2026 08:30 (`history.json`) : `rawText` = « Fé. Fé. Fé. Fé. », prise de 1,2 s, mise au propre sautée (`outcome: skipped`, `cleanup: 0`), une seule entrée d'historique. Même profil le 04/11 : « C'est fait. » ×3 (1,2 s, mise au propre sautée).
2. **La mise au propre ne répète pas.** Sur 2 187 dictées de l'historique, 0 cas où le texte final se répète alors que le brut ne se répétait pas (mots isolés ou courts).
3. **Le collage n'est pas lancé plusieurs fois.** Une entrée d'historique et une ligne `collage:` du journal par dictée ; dans le journal encore disponible (17 et 18/11, celui du 13/11 est tourné) : 87 dictées, 87 collages, un par dictée. Le texte collé est celui de `finalText`, à l'identique.
4. **Reproduction en banc (18/11/2026)** : `whisper-server` `large-v3-turbo-q8_0`, mêmes arguments que Scriibe (`-l auto -t 8`), mots « fait », « oui », « non » dits par 4 voix de `say` (Eddy, Amélie, Flo, Reed), converties en WAV 16 kHz, avec 4 marges de silence/bruit (de 0 à 0,8 s) : 48 prises de 1 s environ.
   - **Serveur court (`-ac 512`)** : 2 boucles sur 48 (« 正\n正\n正\n正\n正\n正 » et « の\nの »), sur des prises d'un seul mot.
   - **Serveur normal (sans `-ac`)** : 0 boucle sur les mêmes 48 prises.
   - Au passage : sur un mot isolé, Whisper en détection automatique de langue renvoie souvent un autre alphabet ou un autre mot (« Fe. », « Set. », « の »), comme le « Fé » de l'auteur : transcription fragile, mais sans rapport avec la répétition.
   - Les enregistrements et le script de banc sont dans le dossier temporaire de la session, pas dans le dépôt.

### Pourquoi le garde-fou de S13 ne l'a pas arrêté

`whisper::repeats_itself` (`crates/engines/src/whisper.rs`) ne reconnaît une boucle qu'à partir d'un passage de **4 mots** (`MIN_LOOP_WORDS`) répété deux fois de suite, pour que « oui, oui » ou « très très bien » ne déclenchent pas une seconde transcription. Une boucle de **1 à 3 mots** (« Fé. » ×4, « C'est fait. » ×3) passe donc, et la réponse du serveur court est collée telle quelle. C'est exactement la boucle de S13 (`docs/ECARTS.md`, 01/11), mais sur une prise trop courte pour contenir 4 mots.

### Tests qui échouent à cause du défaut

Dans `crates/engines/tests/services.rs` :

- `a_single_word_looping_on_the_short_server_is_redone_on_the_normal_server` : le serveur court répond « Fé. Fé. Fé. Fé. », le serveur normal « Fait. » ; attendu « Fait. » après deux requêtes ; obtenu « Fé. Fé. Fé. Fé. » après une seule.
- `a_short_phrase_repeated_three_times_or_more_is_a_loop` : `repeats_itself` doit reconnaître « Fé. Fé. Fé. Fé. », « C'est fait. C'est fait. C'est fait. » et « 正\n正\n正\n正\n正\n正 ».

Les deux sont rouges (`cargo test -p scriibe-engines --test services`) ; les autres tests de répétition existants passent.

### Pour la tâche 1 (non traité ici)

- Une répétition de 1 à 3 mots **deux fois seulement** (« Skribet, Skribet, », « Lance yogurt. Lance yogurt. ») est ambiguë : voulue ou boucle ? Il ne faut pas tronquer « très très bien » ni « oui, oui » ; le plus sûr est de ne déclencher la seconde transcription qu'à partir de **3 répétitions** pour les passages courts, ce qui ne coûte qu'une seconde en cas de fausse alerte.
- Le garde-fou ne joue que sur le serveur court ; le serveur normal n'a pas bouclé au banc.

## Correctif

Étage : le garde-fou de la tâche S13, `whisper::repeats_itself` (`crates/engines/src/whisper.rs`). Une boucle est maintenant : un passage de 4 mots ou plus répété **2 fois** de suite (inchangé), ou un passage de **1 à 3 mots** répété **3 fois** de suite (`SHORT_LOOP_REPEATS`). Le serveur court (`-ac 512`) est donc refait sur le serveur normal pour « Fé. Fé. Fé. Fé. », « C'est fait. » ×3 ou « 正 » ×6.

**Pourquoi 3 et pas 2** : « très très bien », « oui, oui » ou « Lance yogurt. Lance yogurt. » sont des répétitions possibles de la personne qui dicte ; les deux fois ne déclenchent rien (le texte reste tel quel, une seule transcription). Une boucle de Whisper va jusqu'à 3 à 6 exemplaires dans les cas relevés. Coût d'une fausse alerte (« non non non » voulu) : une seconde de plus si le serveur normal tourne déjà ; à froid, il faut charger le modèle (0,6 s sur le Mac de l'auteur d'après le journal du 18/11 ; non mesuré sur les PC). C'est le serveur normal, qui n'a pas bouclé au banc, qui répond.

**Limite connue** : une boucle de 2 exemplaires d'un mot isolé (« の\nの » au banc) n'est pas rattrapée. Indistinguable d'un « oui, oui » voulu sans autre indice.

Tests (`crates/engines/tests/services.rs`) : les deux tests rouges de la tâche 0 passent au vert ; `a_wanted_short_repetition_is_kept_untouched` montre que « Très très bien. », « Oui, oui, c'est très très bien. » et « Lance yogurt. Lance yogurt. » ne sont pas refaits ni tronqués. `cargo fmt`, `clippy -D warnings` et `cargo test --workspace` sans erreur.

## Relecture

Sous-agent relecteur Sonnet (effort élevé) sur le commit `0ad43f7` : 0 bloquant, 1 important, 6 mineurs. Aucune panique ni débordement (300 000 textes aléatoires), coût négligeable (2 ms pour 2 000 mots).

**Important, traité (documentation)**
- I1 : une alerte démarre à froid le serveur normal ; le doc annonçait « environ une seconde ». Mesuré au journal du 18/11 : 0,6 s de démarrage sur le Mac ; non mesuré sur les PC. § Correctif corrigé. Pas de changement de code : la règle nouvelle déclenche 13 fois de plus que l'ancienne sur 2 187 dictées réelles (0,6 %), pour un coût de l'ordre d'une seconde.

**Mineurs corrigés (avec test)**
- M2 : `’` valait une coupure de mot alors que `'` non ; le trait d'union coupait aussi (« C’est-à-dire, c’est-à-dire » refait à tort). Les deux restent maintenant dans le mot (`the_edges_of_the_loop_rule`).
- M5 : test « voulu » séparé du test « refait » (`a_wanted_short_repetition_is_kept_untouched`, `a_short_phrase_said_three_times_is_redone`), cas de frontière ajoutés (3 mots ×2, 4 mots ×2, 2 exemplaires, boucle tronquée, boucle encadrée, saut de ligne), et le serveur normal ne démarre pas pour une répétition voulue.
- M6 : `docs/ECARTS.md`, commentaires de `transcribe` et du test remis à jour. L'entrée du `CHANGELOG.md` viendra à la publication de la 0.11.0.

**Mineurs restants (non corrigés, notés)**
- M1 : boucle CJK sans séparateur (« ありがとうありがとうありがとう ») non vue ; les boucles avec sauts de ligne, seules relevées au banc, sont vues.
- M3 : boucle tronquée en son milieu (« C'est fait. C'est fait. C'est ») ou motif alterné deux fois non vue.
- M4 : trois exemplaires voulus (« très très très bien », « ha ha ha ») refaits ; texte jamais altéré.
- Si le serveur normal boucle à son tour, sa réponse est gardée telle quelle (0 boucle en 48 prises au banc).
- Test `instance::tests::a_second_instance_is_refused_until_the_first_ends` (crate `cli`, sans lien avec ce chantier) : échec une fois sur trois passages du workspace, passé aux autres ; à surveiller.

## Essai

Essai réel du 18/11/2026, build `22d746b` (release) sur les trois systèmes. Décompte fait d'après `history.json` (texte brut `rawText` et final) et le journal `scriibe.log` de chaque machine ; l'auteur n'a fait que dicter. Mise à jour des PC par `git bundle` (les commits n'étant pas poussés), compilation par `pilote compiler`, Windows après `taskkill` de l'ancien `scriibe.exe`.

| Système | Moteur de transcription | « fait » | « oui » | « non » | Phrases | Mot collé 3 fois ou plus | Mot doublé (2 exemplaires) |
|---|---|---|---|---|---|---|---|
| Mac | `large-v3-turbo-q8_0`, mise au propre llama.cpp | 9 | 29 | 12 | 5, dites librement (8 à 14 s), intactes | 0 | 3 (« Oui, oui. ») |
| Windows | `small` (serveur court), mise au propre OpenRouter | 11 | 12 | 12 | 3, intactes | 0 | 0 |
| Linux | serveur court, mise au propre llama.cpp `qwen2.5-1.5b` | 10 | 10 | 10 | 3, intactes | 0 | 0 |

**Résultat : aucune boucle de répétition sur 3 systèmes et environ 130 dictées de mots isolés ; phrases intactes.** Critère « aucun mot collé deux fois » : tenu sauf trois « Oui, oui. » au Mac (voir ci-dessous).

- **Lecture des écarts** : les « Oui, oui. » (2 exemplaires, prises de 1,3 à 1,7 s) sont le cas déjà documenté comme limite connue (§ Correctif), indiscernable d'un « oui, oui » dit par la personne ; le journal ne dit pas si le garde-fou a joué (aucune ligne de journal quand il refait la transcription : le seul indice est une durée de transcription plus longue). L'essai ne prouve donc pas que le correctif a rattrapé une vraie boucle : il montre l'absence de régression et l'absence de boucle visible au Mac, sur Windows et sur Linux. Le « Fé. Fé. Fé. Fé. » du 13/11 ne s'est pas reproduit.
- **Mac, « fait »** : 9 dictées et non 10 (une prise au Mac sans collage compte pour le premier appui dans Scriibe elle-même).
- **Reconnaissance, hors sujet mais mesurée** : sur un mot isolé, la reconnaissance reste fragile (« Fé », « Fais », « Faye », « We » pour « oui » sur Windows presque à chaque fois avec `small`, « Wheat », « Yes, »). Sans lien avec la répétition.
- **Hallucination sur 1,1 s** (Mac, dernière dictée de « fait ») : « 3, 2, 3, 4, 5, 6, 7, 8 et 10. », déjà dans le brut de Whisper. Voir Ajout 1.
- **Mise au propre** : sur Linux, « Très très bien » (brut) est sorti « Très bien » (mise au propre par le petit modèle) ; sur Windows Whisper a déjà rendu « Très bien ». Voir Ajout 3.
- **Hypothèse de l'auteur (parler trop tôt, au moment de l'appui)** : testée au Mac par 25 dictées de « non » et « oui » dites dès l'appui : 0 boucle, 1 « Oui, oui. ». Non concluant (les boucles ne se voient pas non plus quand la parole est tardive).

## Ajouts

1. **Hallucination en suite de chiffres** sur une prise courte (Mac, « fait » dit, sortie « 3, 2, 3, 4, 5, 6, 7, 8 et 10. »). Pas une répétition de mot ; non traitée. À ranger au carnet : à mesurer si elle revient (prise de 1 s environ, `large-v3-turbo` en `-ac 512` ?).
2. **Hypothèse « parler trop tôt au moment de l'appui »** : non concluante (voir Essai). À reprendre seulement si une boucle réelle revient, avec le journal de l'appui à la première parole (délai entre l'appui et le début du son, aujourd'hui non journalisé).
3. **« Très très bien » devient « Très bien »** à la mise au propre sur Linux (modèle `qwen2.5-1.5b`) : probablement le retrait de répétitions voulu par la mise au propre ; à confirmer avec le comportement de Scribe avant d'en faire un écart. À ranger au carnet.
4. **Journal du garde-fou** : rien n'est écrit quand `repeats_itself` refait une transcription ; l'essai ne peut donc pas compter les déclenchements. Une ligne de journal (« whisper: boucle vue, seconde transcription ») ferait ce travail, quelques minutes de code ; non faite ici (hors consigne de la tâche 3).

## Suite (0.13.1, tâche 2 du découpage mineurs)

Détail et tests : `docs/MINEURS_0131.md` § Mot répété. Mineurs M1 (boucle CJK), M3 (boucle coupée au milieu) et ajouts 1 (suite de chiffres) et 4 (journal) traités ; M4 : les triples voulus reviennent intacts (le serveur normal les confirme) ; ajout 3 et motif alterné deux fois : notes (limite assumée). Une seconde transcription laisse maintenant une ligne au journal (`whisper: boucle vue` ou `suite de chiffres sur une prise courte`), sans le texte dicté.
