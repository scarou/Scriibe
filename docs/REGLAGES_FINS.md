# Réglages fins (chantier 6)

## Veille et couleurs (tâche 0, 18/11/2026)

**Durées de veille** : Whisper et llama.cpp proposent les mêmes huit choix : 1, 2, 5, 10, 15, 30, 60 min et « Jamais » (`0`, le moteur reste en mémoire tant que Scriibe tourne). Liste unique : `IDLE_CHOICES` (`crates/core/src/settings.rs`) et `ui/src/lib/idle.ts`.

- Anciens réglages lus tels quels : Whisper 0, 5, 10, 30, 60 ; llama.cpp 30, 120 (2 h), 480 (8 h), 0. Un ancien réglage hors liste (120, 480) reste dans le menu de llama.cpp, avant « Jamais », et s'affiche « 2 h » / « 8 h ». Défauts inchangés (Whisper 10, llama.cpp 30).
- Une valeur de Whisper écrite à la main hors liste (14) se ramène à la plus proche des durées de Scribe (cas `settings-020`, non modifié) ; llama.cpp la garde (plafond 24 h).
- « Jamais » : `idle_limit(0)` = `None`, le moteur n'est jamais libéré (tests `llama_with_never_is_not_released`, `no_idle_limit_means_never`, `idle_limit`).

**Couleurs de l'indicateur** : 12 couleurs, sans couleur libre : les cinq de Scribe (Scriibe, gris, rouge, bleu, noir) et sept ajouts (orange, vert, turquoise, violet, rose, indigo, marron). Choix : teintes soutenues, dégradé clair → foncé comme les autres, pour que le texte blanc de la pastille reste lisible sur fond clair comme sombre ; pas de jaune (illisible avec du blanc). Une couleur déjà choisie est relue telle quelle ; une couleur inconnue redonne celle de Scriibe.

**Tests** : `settings_scriibe.rs` (`idle_choices_…`, `old_idle_settings_…`, `old_and_new_indicator_colors_…`), `engines/tests/server.rs`, `app.rs`, `idle.test.ts`, `appearance.test.ts`. Écart noté dans `docs/ECARTS.md`. Cartes mesurées : `cargo test --workspace`, clippy `-D warnings`, `vitest` (172) verts.

**Pas fait ici** : aperçu à l'écran des sept couleurs (essai visuel sur machine) ; Aide (`Help.svelte`, `HelpEn.svelte`) mise à jour pour les durées et les douze couleurs.
