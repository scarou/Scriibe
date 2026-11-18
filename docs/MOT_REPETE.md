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
