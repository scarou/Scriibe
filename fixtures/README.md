# Cas de référence

Cas de test tirés des vérifications automatiques de **Scribe** (`Sources/Scribe/Checks.swift`, dépôt `scarou/scribe`, commit `0951ad7`). Le Swift sert d'oracle : le cœur Rust doit les passer **à l'identique**. Un écart voulu se documente dans [`docs/ECARTS.md`](../docs/ECARTS.md), jamais en modifiant un cas en silence.

## Fichiers

| Fichier | Contenu | Cas |
|---|---|---|
| `typography.json` | typographie française (espaces insécables, heures) | 12 |
| `whisper_hallucinations.json` | phrases inventées par Whisper | 5 |
| `cleanup_guard.json` | garde-fou de la mise au propre, nettoyage de la sortie du modèle | 24 |
| `translation_guard.json` | garde-fou de la traduction à la volée | 21 |
| `spoken_signs.json` | signes dictés, listes, « à la ligne », dictionnaire | 100 |
| `already_clean.json` | « sauter si déjà propre » | 37 |
| `settings.json` | lecture tolérante des réglages, valeurs dérivées | 34 |
| `data_files.json` | lecture tolérante de l'historique, du dictionnaire, des totaux | 5 |
| `hotkey.json` | machine à états de la touche de dictée | 11 |
| `media_pause.json` | pause des médias | 11 |
| `microphone.json` | niveau et choix du micro | 19 |
| `local_engines.json` | modèles Whisper et GGUF, options de `llama-server`, libération au repos | 39 |
| `history_retention.json` | conservation de l'historique (scénarios) | 13 |
| `prompts.json` | empreinte des prompts et invariants (énoncés) | 19 |
| `ui_logic.json` | petites logiques d'interface | 18 |
| `playful_schtroumpf.json` | mode Amusant — **reporté** (`status: deferred`) | 9 |

Ce qui n'est pas encore repris est listé dans [`A_FAIRE.md`](A_FAIRE.md).

## Format

Chaque fichier : `{ schema: 1, suite, description, source, notes?, cases: [...] }`. Chaque cas :

```json
{
  "id": "typography-006",
  "name": "deux-points devant espace",
  "line": 33,
  "fn": "french_typography",
  "args": { "text": "Note: rien" },
  "expected": "Note : rien"
}
```

- `id` : `<suite>-<numéro>`, stable ; ne jamais réutiliser un identifiant.
- `line` : ligne du test d'origine dans `Checks.swift` (traçabilité).
- `fn` : fonction à appeler (voir ci-dessous). Une **liste** de fonctions s'applique en chaîne au texte (les `args` vont à la première).
- `comparison` (facultatif) : `equals` (défaut) · `prefix` · `starts_with` · `ends_with` · `contains` · `occurrences` · `fields` (l'attendu est un dictionnaire de champs à comparer un à un) · `contains_all` · `same_set` (mêmes éléments, ordre libre) · `map` · `starts_with_and_contains`.
- `property` (facultatif) : `idempotent` (appliquer deux fois = une fois) ou `max_duration_ms:N`.
- `status: deferred` : cas gardé mais hors du jalon courant.
- Les espaces insécables (U+00A0, U+202F) sont écrits en clair dans les JSON ; ne pas les « normaliser ».
- `text_pattern` : texte construit (`prefix` + `separator` × `repeat`, avec `middle` entre les répétitions + `suffix`).

## Fonctions

| `fn` | Rôle (fonction Swift d'origine) |
|---|---|
| `french_typography` | `TextFixer.frenchTypography` |
| `known_hallucination` | `TextFixer.knownHallucination` (attendu : rejetée ou non) |
| `cleaner_accepts` / `cleaner_strip` | `Cleaner.accepts`, `Cleaner.strip` |
| `accepts_translation` | `Cleaner.acceptsTranslation` |
| `apply_dictionary` | `TextFixer.apply` (dictionnaire, signes, listes ; `bullet`, `lists`) |
| `spoken_signs_apply` | `SpokenSigns.apply` |
| `looks_clean` / `skips_cleanup` | `TextFixer.looksClean`, `TextFixer.skipsCleanup` |
| `decode_settings` / `settings_derived` / `settings_roundtrip` | `AppSettings` décodé, valeurs calculées, aller-retour |
| `decode_history` / `decode_history_entry` / `decode_dictionary` / `decode_day_totals` / `history_roundtrip` | lecture tolérante des fichiers de données |
| `hotkey_sequence` / `command_held` | `HotkeyMonitor.handle`, `Hotkey.commandHeld` (étapes abstraites) |
| `media_decision` / `media_new_players` | `MediaPause.decision`, `MediaPause.newPlayers` |
| `mic_level` / `mic_resolve` / `mic_label` | `MicLevel.assess`, `Microphones.resolve`, libellé |
| `is_whisper_model` / `distinct_models` / `same_model` | sélecteur de modèle Whisper |
| `llama_base_name` / `llama_arguments` / `llama_detect_models` / `llama_assistant` / `llama_status` | `LlamaServer.*` |
| `whisper_should_release` | `WhisperServer.shouldRelease` |
| `cleanup_engine_display` | `CleanupEngine` (nom, local, modèle affiché) |
| `retention_cutoff` / `retention_cutoff_equals` / `history_scenario` | `HistoryRetention`, `Store` (scénarios décrits en étapes) |
| `prompt_fingerprint` | empreinte FNV-1a des prompts des 8 modes (voir le cas) |
| `overlay_origin_x` / `folded_cards_toggle` / `folded_cards_contains` / `dictation_zone_clears` | logiques d'interface |
| `invariant` | énoncé à vérifier par un test écrit à la main (`expected: true`) |
| `schtroumpf_apply` / `schtroumpf_verb_form` | moteur schtroumpf (reporté) |

Les cas `history_scenario` et `invariant` ne sont pas de simples entrées → sorties : ils décrivent un scénario ou un énoncé que le test Rust doit traduire en code.

## Vérifier

```bash
python3 tools/verifier_fixtures.py                      # structure : JSON, identifiants uniques, champs
python3 tools/verifier_litteraux.py <Checks.swift> fixtures   # chaque texte existe tel quel dans Checks.swift, près de la ligne citée
```

La seconde commande est le garde-fou contre une faute de recopie : elle s'exécute avec une copie de `Checks.swift` du dépôt Scribe (jamais modifié).
