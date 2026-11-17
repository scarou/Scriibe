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

## Correctif

Corrigé le 17/11 (tâche 1), les trois défauts de § Cause :

1. **Défaut Windows : Alt gauche** (`Layout::default_command`, `default_command_label`, `keys.rs`). Choix et raison : présente sur tous les claviers (Windows droite manque à beaucoup de portables) ; libre de la dictée (Ctrl droit) et de la traduction (Maj) ; le menu de l'appli qu'ouvre Alt au relâchement est déjà masqué par `CommandMenuMask`, seulement derrière une dictée tenue. Ctrl gauche a été écarté : Windows l'envoie avec AltGr, donc AltGr déclencherait la commande. **À confirmer à l'essai sur le PC (tâche 2)**, en particulier qu'Alt gauche tenue avec Ctrl droit n'ouvre aucun menu.
2. **AltGr admise pour la commande seulement** : nouvelle touche `right_alt` (« AltGr (Alt droit) », `VK_RMENU`), listée dans `COMMAND_ONLY` ; `validate_dictation_and_translate` et `validate_translate` la refusent (« … ne peut servir qu'à dicter une commande : elle sert à taper »). Raison : tenue seule elle taperait « @ » et « € » ; tenue pendant une dictée elle ne gêne rien. Sous Windows AltGr arrive comme Ctrl gauche + Alt droit : « Ctrl gauche + AltGr » se capte telle quelle et fonctionne (Ctrl gauche fictif n'annule pas la dictée, voir `handle_change`).
3. **Combinaison de commande** : jusqu'à trois touches (`MAX_COMMAND_KEYS`), sans doublon, jamais celle de la dictée ou de la traduction, une seule touche ordinaire ; `Binding::command_key_held` exige maintenant **toutes** les touches (`crates/core/src/hotkey.rs`). Les valeurs par défaut sont à une seule touche : aucun changement pour qui n'a rien réglé. Un `commandKeys` déjà enregistré avec une seule touche reste valide.

Capture (`crates/cli/src/web/mod.rs`, `capture_view`) : la combinaison est validée en entier, message propre à la commande pour une touche qui sert à taper. Interface : texte de capture (« touche ou combinaison »), aide du sélecteur et de l'Aide (AltGr possible pour la commande), « Alt gauche » cité comme défaut (`Help.svelte`, `HelpEn.svelte`), traductions anglaises ajoutées.

### Tests

- Rouges avant, verts après : `a_command_key_may_be_altgr_since_it_is_only_held_during_a_dictation`, `a_command_key_may_be_a_combination` (platform), `a_combined_command_key_needs_all_its_keys` (core). `the_default_command_key_exists_on_every_windows_keyboard` (`#[cfg(windows)]`) : à exécuter par la CI Windows.
- Nouveaux : `no_key_choice_is_empty_on_any_system` (noms par défaut non vides, touche de commande donnée avec la dictée par défaut, Windows et Mac ; Linux par la disposition de Windows), `ctrl_left_plus_altgr_is_a_valid_command_combination`, test d'interface « aucun sélecteur vide » (`tabs.test.ts`).
- Autres sélecteurs déplacés par la refonte : dictée et traduction restent sur Dicter, commande sur Commandes vocales ; aucun autre choix de touche (reformulation : pas de touche propre, elle passe par la touche de commande) n'existe.

## Essai

Tâche 2 (17/11), par l'auteur, sur les trois machines. Le correctif n'étant pas poussé, les PC ont reçu les commits par bundle git (`pilote maj` ne tire que de GitHub) puis compilé en release (`pilote compiler --mode release`) ; Scriibe lancé par une tâche planifiée « EssaiScriibe » sur Windows (à supprimer en fin d'essai), par `setsid` dans la session GNOME sur Linux, par `open` sur le Mac.

| Machine | Essai | Résultat |
|---|---|---|
| Windows (962a1d7) | Commandes vocales → touche de commande → AltGr | Affichée « AltGr (Alt droit) » seule après correction du Ctrl gauche fictif, enregistrée sans erreur, commande vocale lancée avec elle (reconnaissance Whisper difficile, sans rapport) |
| Windows | Touche par défaut Alt gauche, tenue avec Ctrl droit dans le Bloc-notes | La commande s'exécute, aucun menu ne s'ouvre : **défaut confirmé** |
| Linux (GNOME, Wayland) | Carte « Activer les commandes vocales » | Le raccourci se change dans GNOME, pas dans Scriibe (voulu) ; texte corrigé : Paramètres → Applications → Scriibe → Raccourcis clavier (fenêtre « Raccourcis globaux ») |
| Mac (8fa4c81) | Commandes vocales → « Changer… » | « Maj gauche » par défaut affichée ; Ctrl gauche choisie et enregistrée, ça marche |

Corrigés pendant l'essai (voir Ajouts) : AltGr affichée avec un Ctrl gauche fictif ; flèches et AltGr permises pour traduire et commander (flèches sous Windows seulement) ; touche de dictée admise dans la combinaison de commande ; chemin des raccourcis GNOME ; liste complète des touches proposées ajoutée à l'Aide (rubrique des touches).

Pas essayé : une flèche choisie sous Windows (crochet qui la retient, couvert par un test seulement) ; dictée de commande avec combinaison de plusieurs touches sur le PC. Le PC Windows et le PC Linux ont le code de 962a1d7 et 002a0aa, pas des derniers commits (flèches réservées à Windows, Aide).

## Ajouts

1. **AltGr s'affiche « Ctrl gauche + AltGr » à la capture (Windows).** Windows envoie un Ctrl gauche fictif avec AltGr ; la capture le garde. À corriger : ignorer ce Ctrl gauche fictif, afficher « AltGr » seule. Remarque de l'auteur : la touche de dictée est Ctrl droit, pas Ctrl gauche.
2. **Restrictions sur le choix de la touche de commande.** L'auteur demande pourquoi on ne peut pas choisir « Ctrl droit + AltGr » ni la flèche gauche (à côté de Ctrl droit), alors qu'elles ne servent à rien pendant une dictée. Réponse : aucune impossibilité technique, restrictions de prudence du code (`validate_command` refuse la touche de dictée ; `CHOICES` exclut les flèches « parce qu'elles servent à taper », raison qui ne vaut que pour la dictée, comme pour AltGr). À lever pour la commande seulement.
3. **Libellé de la touche de traduction** : l'auteur emploie Maj droit ; par défaut la touche est « Maj, l'une ou l'autre ». À vérifier que l'interface le dit clairement.

### Traité pendant l'essai (commit 962a1d7)

- Ajouts 1 et 2 : le Ctrl gauche fictif d'AltGr est ignoré à la capture (`keys::without_altgr_ctrl`) ; flèches (haut, bas, gauche, droite) et AltGr permises pour **traduire et commander**, interdites seulement pour **lancer la dictée** (`NOT_FOR_DICTATION`) ; la touche de dictée dans une combinaison de commande est admise (« Ctrl droit + AltGr » revient à AltGr) ; le crochet Windows retient flèches et touches ordinaires de traduction et de commande tant que la dictée est tenue (`HookRules::swallow_in_dictation`). Sur Linux et Mac aucun crochet ne retient de touche : sur Linux une flèche choisie déplacerait le curseur de l'appli (pas de flèches proposées sur Mac). Lettres, chiffres, Espace et Entrée restent hors liste : à décider.
- Essai Windows : AltGr choisie, enregistrée sans erreur, une commande vocale lancée avec elle (reconnaissance par Whisper difficile, sans rapport avec la touche).

### À reporter au carnet

4. **Liste des applications incomplète sous Windows** (page Commandes vocales, nouvelle commande « ouvrir une application ») : Bloc-notes et WhatsApp absents. Cause : `installed_apps.rs` ne lit que les raccourcis `.lnk` du menu Démarrer ; les applications du Microsoft Store (Bloc-notes de Windows 11, WhatsApp) n'en ont pas. Piste : lister les applications du dossier `shell:AppsFolder` et les lancer par leur identifiant d'application. Plus que quelques minutes (sécurité du lancement, tests) : ligne de carnet proposée.

5. **Mac : mêmes restrictions de touches jugées inutiles par l'auteur** (essai du 17/11). Réponse : sur Mac (et Linux hors GNOME) le crochet est un « tap » d'événements **en écoute seule** (`kCGEventTapOptionListenOnly`) : il ne peut pas retenir une touche, qui arrive aussi à l'appli active. Or les commandes agissent sur le **texte sélectionné** : une flèche le désélectionne, une lettre le remplace. Sous Windows le crochet retient ces touches, d'où la levée. Décision : flèches réservées à Windows (`Layout::swallows`, `LINUX_LAYOUT`, test `arrows_are_only_offered_where_the_hook_can_hold_them_back`) ; sur Mac et Linux restent les touches sans effet sur la sélection (modificateurs, touches de fonction, Début, Fin…). Lever la limite sur Mac demande un tap actif (autorisation d'Accessibilité supplémentaire, `ListenOnly` → `Default`) : ligne de carnet proposée.
6. **GNOME (Wayland) : chemin du raccourci** : la carte des commandes disait seulement « même endroit pour le changer ». Corrigé : Paramètres de GNOME → Applications → Scriibe → Raccourcis clavier (une fenêtre « Raccourcis globaux » s'ouvre), puis relancer Scriibe (carte des commandes et Réglages → Dictée).
7. **Liste des touches autorisées** (demande de l'auteur, Mac) : le bouton « Changer… » n'attend qu'une touche, sans liste. Fait : l'Aide (rubrique des touches) affiche la liste complète des touches proposées sur la machine. Reste à proposer au carnet : une liste déroulante à côté de « Changer… », avec les touches prises et leur motif.
