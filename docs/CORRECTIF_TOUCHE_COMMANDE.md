# Correctif : touche de commande sous Windows (chantier 1)

Découpage : `decoupage-2026-11-17-c1-touche-commande-windows.md` (vault). Session A, tâches 0 et 1.

## Cause

**Pas une régression de code.** La refonte (0.9.0, tâche 2) a seulement déplacé la ligne de la touche de commande dans la carte « Activer les commandes vocales » de la page Commandes vocales (`ui/src/components/commands/ActivationCommands.svelte:30`) ; elle y est, mais l'auteur ne l'y a pas vue. Rust inchangé depuis la 0.8.0 (`git diff v0.8.0 HEAD`), rendu de la page vérifié par un rendu serveur. Les vrais défauts, présents depuis la 0.6.0, sont trois :

1. **Touche par défaut inexistante sur le PC** : sous Windows la touche de commande par défaut est « Windows droite » (`crates/platform/src/keys.rs`, `Layout::default_command`, lignes ~377-390 ; libellé `default_command_label`). Beaucoup de claviers n'ont pas de touche Windows droite : la page affiche une touche impossible à tenir. (Le 16/11 la touche essayée était Maj gauche, acceptée seulement parce que la traduction du PC était réglée autrement : `validate_command` la refuse sinon.)
2. **AltGr refusée** : `CHOICES` (`keys.rs:95-`) exclut AltGr (Alt droit, 0xA5) pour toutes les touches, par crainte qu'elle serve à taper « @ » et « € ». Cette raison ne vaut que pour la touche de **dictée** (tenue seule, elle déclencherait l'écoute). La touche de **commande** n'est tenue que pendant une dictée : aucun risque. La capture répond « elle sert à taper (ou à AltGr) » (`crates/cli/src/web/mod.rs:~936`, message « de dictée » aussi pour la commande).
3. **Une seule touche** : `validate_command` (`keys.rs`) refuse toute combinaison (« une seule touche pour les commandes »), et `Binding::command_key_held` (`crates/core/src/hotkey.rs:121`) lit la liste comme « l'une ou l'autre » et non comme une combinaison.

### Tests qui échouent (commit de la tâche 0)

- `keys::tests::a_command_key_may_be_altgr_since_it_is_only_held_during_a_dictation` : AltGr non reconnue (`None` au lieu de `Some("right_alt")`).
- `keys::tests::a_command_key_may_be_a_combination` : `Err("une seule touche pour les commandes")`.
- `hotkey::tests::a_combined_command_key_needs_all_its_keys` : `command_key_held` vrai avec une seule des deux touches.
- `keys::tests::the_default_command_key_exists_on_every_windows_keyboard` (`#[cfg(windows)]`) : écrit, **non exécuté ici** (ne tourne que sous Windows, CI et PC).

### Pistes pour la tâche 1 (à valider)

- Défaut Windows : une touche présente partout, libre de la dictée (Ctrl droit) et de la traduction (Maj) ; Ctrl gauche pose le problème d'AltGr (Windows envoie Ctrl gauche + Alt droit), à vérifier avant de choisir.
- AltGr reconnue (`right_alt`) pour la commande seulement ; combinaison de plusieurs touches admise, toutes à tenir.
- Message de capture adapté à la touche de commande.
