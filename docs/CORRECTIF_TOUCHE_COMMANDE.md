# Correctif : touche de commande sous Windows (chantier 1)

Découpage : `decoupage-2026-11-17-c1-touche-commande-windows.md` (vault). Session A, tâches 0 et 1.

## Cause

**État au 17/11 : cause non confirmée, pas de test rouge encore.** Ce qui est établi :

- **Rust inchangé** : `git diff v0.8.0 HEAD -- crates/platform crates/core/src/settings.rs` ne touche pas aux touches (liste des touches, `validate_command`, `command_keys`, `command_label`, valeur `commandKeys`). Pas de régression côté Rust.
- **Interface** : seul `KeyPicker.svelte` a changé (prop `kinds`). Un rendu serveur de `ActivationCommands` avec un état Windows par défaut (`command_label` = « Windows droite ») affiche bien la ligne « Touche tenue en plus pour dicter une commande : Windows droite » et le bouton « Changer… » : la ligne n'est donc pas supprimée par la refonte.
- **Valeurs par défaut** : sous Windows la touche de commande par défaut est Windows droite (`crates/platform/src/keys.rs`, `default_command`), la même qu'en 0.8.0. Maj gauche (touche essayée le 16/11) est refusée comme touche de commande tant que la traduction est à son réglage par défaut (Maj), message « Maj gauche sert déjà à traduire » (`validate_command`), en 0.8.0 comme aujourd'hui : elle n'a donc marché que si `translateKeys` était réglé autrement sur le PC.
- **Le journal du PC (17/11 16:07 à 16:09)** montre `commande: dictionnaire.ajouter` : en 0.10.0 une commande vocale s'exécute sur le PC.
- Seul cas où la page écrit « aucune (à choisir) » : `command_label` vide, c'est-à-dire la touche par défaut prise par la dictée ou la traduction réglées (`binding()` et `command_label()` de `keys.rs`).

Manque : ce que la page affiche réellement sur le PC, et les valeurs `dictationKeys`, `translateKeys`, `commandKeys` de son `settings.json`.
