# Refonte de l'interface par thématiques

Suivi de la refonte (découpage du 17/11/2026, tâches 0 à 13). Chaque tâche ajoute sa section ; ce fichier ne contient pour l'instant que le constat (tâche 0).

## Décisions de la tâche 0

Le constat (ci-dessous) relève 7 points « À TRANCHER ». Pour la suite, la recommandation du constat est retenue pour chacun, faute d'objection de l'auteur et parce qu'aucun n'engage l'usage, la sécurité ni l'irréversible :

- **T1** Confidentialité de l'historique : reste en tête de l'Historique (elle agit sur la liste affichée).
- **T2** Exports et imports : chacun reste avec son objet (dictionnaire, utilisateurs, modes, commandes) ; la carte unique « Sauvegarder et transférer » de la tâche 8 les regroupe dans Données et système sans retirer les boutons des autres pages (comme le prévoit le découpage).
- **T3** `KeyPicker` : découpé par touche ; Dicter (dictée, traduction, essai), Commandes vocales (commande) ; un « Rétablir » par page, qui ne remet que ses touches.
- **T4** Reformulation de la sélection : nouvelle section de Commandes vocales (explication + case de consentement), tâche 2.
- **T5** Journal : le journal en direct va dans Mesures et essais, le chemin du journal dans Données et système.
- **T6** Langue de l'interface : Apparence.
- **T7** Tableau « État de l'installation » : en tête de Moteurs et modèles ; les deux lignes qui ne sont pas des moteurs (micro, touche globale) sont déplacées vers Dicter (tâche 3) ou supprimées.

## Constat

Constat fait le 2026-11-17 par un sous-agent de recherche (lecture seule) sur l'état du dépôt à `a6c6af1` (v0.8.0). Chemins relatifs au dépôt. Les cases du tableau seront cochées à la tâche 1.


Cibles : Accueil, Historique, Dicter, Style d'écriture, Dictionnaire, Commandes vocales, Moteurs et modèles, Profil, Apparence, Données et système, Mesures et essais, Aide. « Coque » = éléments hors page (barre latérale, bandeaux, dialogue). « (hors plan) » = ne devient pas une page.

Remarques d'ensemble, utiles pour lire le reste :
- Aujourd'hui : 5 pages (`ui/src/lib/tabs.ts:2-8`), dont « Réglages » à 7 onglets (`tabs.ts:13-21`). La page Réglages n'est qu'un aiguillage : `ui/src/tabs/SettingsPage.svelte:30-36` pose des composants dans l'ordre. Les composants `Users`, `Install`, `Modes`, `Bench`, `Journal`, `System`, `Update` existent déjà séparément. Seuls `Settings.svelte` (413 lignes, 9 cartes en 3 sections `dictation`/`engines`/`appearance`, ligne 16) et `Dictionary.svelte` (qui porte `VoiceCommands`) devront être découpés.
- Il y a 50 occurrences de `<Card>` dans le code (voir « Décomptes »). Toutes ont un `id` utilisé comme ancre (`document.getElementById`) et comme clé de mémoire du pli (`localStorage` `fold:<id>`, `ui/src/lib/folds.ts:2,7,16`). Renommer un id fait perdre le pli retenu.
- La « reformulation de la sélection » n'a aucune carte ni section propre aujourd'hui (voir À TRANCHER T4).
- Il n'y a aucun test Playwright dans le dépôt (seulement l'outil `tools/captures_interface.js`). Les tests sont Vitest (`ui/src/lib/*.test.ts`) et Rust.

---

### Tableau carte → page

Légende de la colonne « Carte » : `C01…C50` = composant `<Card>` (liste complète dans « Décomptes ») ; `S01…S12` = zone d'interface qui n'est pas une `<Card>` ; « (page) » = élément de la page elle-même. Un contrôle = case, champ, menu, curseur, bouton d'action ou liste d'éléments éditables. Les contrôles identiques répétés dans une liste (par exemple « phrases + Retirer + Ajouter » de chaque commande) sont comptés sur une seule ligne par commande.

#### 1. Coque (App.svelte, Dialog.svelte)

| Page actuelle (onglet) | Carte | Réglage / contrôle | Fichier:ligne | Page cible | Remarque |
|---|---|---|---|---|---|
| Coque | S01 Barre latérale | Logo de marque (image claire/sombre, animée ou fixe selon C24) | ui/src/App.svelte:83 (calcul `logotype` : 19) | (hors plan) reste en tête de barre | Dépend du réglage « Logo animé » (C24). `ui/src/components/Logo.svelte` n'est utilisé nulle part (code mort). |
| Coque | S01 Barre latérale | Boutons de page (5 aujourd'hui, boucle sur `tabs`) | ui/src/App.svelte:84-86 ; liste `ui/src/lib/tabs.ts:2-8` | (hors plan) remplacés par 11 entrées groupées | Les titres de groupe (Utiliser, Ma dictée, Configurer, Avancé) n'existent pas : `App.svelte:84` ne sait afficher qu'une liste plate. |
| Coque | S01 Barre latérale | Indicateur d'état « Prêt · {touche} / J'écoute… / Mise au propre… / Touche indisponible » | ui/src/App.svelte:88-94 | (hors plan) reste dans la barre | Texte construit avec `keyName(dictation_label)` : déjà adapté au système (fourni par Rust). |
| Coque | S01 Barre latérale | Bouton « ↑ Nouvelle version disponible » (si `status.value.update`) | ui/src/App.svelte:95-97 | Données et système | Le plan veut un point sur l'entrée « Données et système » à la place de ce bouton ; ce bouton ouvre aujourd'hui `choose('update')`. |
| Coque | S01 Barre latérale | Numéro de version | ui/src/App.svelte:98 | (hors plan) reste dans la barre | |
| Coque | S01 Barre latérale | Bouton « Quitter Scriibe » (confirmation) | ui/src/App.svelte:99 ; `quit()` : 29-33 | (hors plan) reste dans la barre | Absent de la liste des 11 entrées du plan : place à confirmer (bas de barre). |
| Coque | S02 Bandeaux | « Scriibe ne répond plus… » (3 variantes selon le système) | ui/src/App.svelte:102-104 | (hors plan) haut de toute page | Cite « l'icône de la barre des menus / la liste des applications / la zone de notification ». |
| Coque | S02 Bandeaux | Problèmes de fichiers de configuration (`config_problems`) | ui/src/App.svelte:105-112 | (hors plan) haut de toute page | |
| Coque | S03 Page arrêtée | « Scriibe est arrêté. Vous pouvez fermer cette page. » | ui/src/App.svelte:78-80 | (hors plan) | Texte au « vous ». |
| Coque | S03 Page arrêtée | Notice « L'utilisateur actif a changé… » au rechargement | ui/src/App.svelte:55-62 | (hors plan) | |
| Coque | S04 Dialogue | Boîte de confirmation / d'avis (Annuler, OK, bouton de confirmation) | ui/src/components/Dialog.svelte:31-34 ; `ui/src/lib/dialog.svelte.ts` | (hors plan) | Utilisée par presque toutes les pages. |

#### 2. Tableau de bord → Accueil

| Page actuelle (onglet) | Carte | Réglage / contrôle | Fichier:ligne | Page cible | Remarque |
|---|---|---|---|---|---|
| Tableau de bord | S05 Bandeau | Salutation « Salut {prénom} » | ui/src/tabs/Dashboard.svelte:53 | Accueil | Vient du réglage Prénom (aujourd'hui dans C18 → Profil). |
| Tableau de bord | S05 Bandeau | Phrase d'accroche « Maintiens la touche, parle… » | ui/src/tabs/Dashboard.svelte:54 | Accueil | |
| Tableau de bord | S05 Bandeau | Ligne « Utilisateur : X · changer » (bouton, visible dès 2 utilisateurs) | ui/src/tabs/Dashboard.svelte:55-57 | Accueil | Le bouton appelle `goto('settings','utilisateurs')` : lien à repointer sur Profil. |
| Tableau de bord | S05 Bandeau | Rappel de la touche « {touche} maintenir pour dicter » | ui/src/tabs/Dashboard.svelte:58-61 | Accueil | Repli `'Ctrl droit'` écrit en dur (`:59`), affiché tant que le statut n'est pas chargé. |
| Tableau de bord | S05 Bandeau | Onde décorative | ui/src/tabs/Dashboard.svelte:63-65 | Accueil | |
| Tableau de bord | C01 Assistant de premier lancement | Bloc `#assistant` (s'ouvre au premier lancement ou sur demande) | ui/src/tabs/Dashboard.svelte:15,26-37,68-72 ; ancre `assistant` | Accueil | Déclenché par `settings.setupDone` (`:68`) et par `nav.openAssistant()` (boutons de C20 et de l'Aide). Reste sur l'Accueil ; le bouton « Relancer l'assistant » de C20 passe dans Moteurs et modèles. |
| Tableau de bord | C01 Assistant | Machine mesurée (cœurs, mémoire, carte graphique) | ui/src/components/Assistant.svelte:46-51 | Accueil | Texte « Votre ordinateur : » au « vous ». |
| Tableau de bord | C01 Assistant | Choix d'une chaîne (cases radio, une par chaîne) | ui/src/components/Assistant.svelte:55-64 | Accueil | Textes des chaînes fournis par Rust (`crates/cli/src/assistant.rs:25-80`). |
| Tableau de bord | C01 Assistant | Bouton « Appliquer ce choix » | ui/src/components/Assistant.svelte:66 | Accueil | Écrit les moteurs (réglages de Moteurs et modèles). |
| Tableau de bord | C01 Assistant | Bouton « Garder mes réglages actuels » | ui/src/components/Assistant.svelte:67 | Accueil | |
| Tableau de bord | C01 Assistant | « Il reste à faire » + bouton « Aller à l'installation » | ui/src/components/Assistant.svelte:71-78 | Accueil | Appelle `ongoto('install')` → lien à repointer sur Moteurs et modèles. |
| Tableau de bord | C01 Assistant | Bouton « Aller aux Réglages » (clé OpenRouter manquante) | ui/src/components/Assistant.svelte:79 (texte 75) | Accueil | Appelle `ongoto('settings')` → à repointer sur Profil (clés API). Texte « (onglet Réglages) ». |
| Tableau de bord | C01 Assistant | Boutons « Fermer » | ui/src/components/Assistant.svelte:80,84 | Accueil | Message `:83` « maintenez Ctrl droit » sans condition de système. |
| Tableau de bord | S06 Tuiles | 4 tuiles chiffrées (Mots dictés, Aujourd'hui, Vitesse, Temps gagné) | ui/src/tabs/Dashboard.svelte:75-81 | Accueil | `div.card`, pas de `<Card>`. « Temps gagné » dépend de « Ma vitesse au clavier » (C05, déplacée en Mesures et essais). |
| Tableau de bord | C02 Mots dictés par jour | Graphique 14 jours + série de jours | ui/src/tabs/Dashboard.svelte:84-95 | Accueil | `?` vise la rubrique d'aide `measures`. |
| Tableau de bord | C03 Où je dicte | Part des mots par appli | ui/src/tabs/Dashboard.svelte:97-104 | Accueil | `?` → `measures`. |
| Tableau de bord | C04 Dernières dictées | Liste des 5 dernières dictées | ui/src/tabs/Dashboard.svelte:107-117 | Accueil | `?` → `measures`. |
| Tableau de bord | C04 Dernières dictées | Bouton « Tout voir » (→ Historique) | ui/src/tabs/Dashboard.svelte:108 | Accueil | `goto('history')` : le lien reste valable. |
| Tableau de bord | C05 Ma vitesse au clavier (repliée par défaut) | Texte à recopier + zone de saisie (collage interdit) | ui/src/components/TypingTest.svelte:46-61 ; carte `Dashboard.svelte:119-121` | Mesures et essais | Le plan place « Ma vitesse au clavier » en Mesures et essais. Les tuiles de l'Accueil lisent son résultat. |
| Tableau de bord | C05 Ma vitesse au clavier | Boutons « Terminer » / « Nouvel essai » | ui/src/components/TypingTest.svelte:63-68 | Mesures et essais | |
| Tableau de bord | C05 Ma vitesse au clavier | Bouton « Utiliser comme ma vitesse au clavier » (écrit `typingWPM`) | ui/src/components/TypingTest.svelte:76-81 | Mesures et essais | Seul réglage de la carte ; recalcule le temps gagné de tout l'historique. |
| Tableau de bord | C05 Ma vitesse au clavier | Ligne « Réglage actuel : N mots par minute (entre a et b) » | ui/src/components/TypingTest.svelte:82 | Mesures et essais | |

#### 3. Historique

| Page actuelle (onglet) | Carte | Réglage / contrôle | Fichier:ligne | Page cible | Remarque |
|---|---|---|---|---|---|
| Historique | (page) | Titre « Historique » + « Gardé sur cet ordinateur seulement » | ui/src/tabs/History.svelte:136-140 | Historique | |
| Historique | (page) | Bouton « Tout effacer » (confirmation, définitif) | ui/src/tabs/History.svelte:141 ; confirmation `:107` | Historique | Le texte de confirmation cite « les chiffres du tableau de bord » : à renommer en « Accueil ». |
| Historique | C06 Confidentialité de l'historique | Case « Garder le texte des dictées » | ui/src/tabs/History.svelte:146 | Historique | À TRANCHER (T1) : la carte n'est pas citée par le plan. Je propose de la laisser en tête de l'Historique (elle agit sur la liste, aperçu avant effacement, lien d'aide `history`/`conservation`). Alternative : Données et système. |
| Historique | C06 Confidentialité | Menu « Garder l'historique : sans limite / 2, 7, 14, 30, 90 jours / 1 an » | ui/src/tabs/History.svelte:148-153 | Historique | À TRANCHER (T1). |
| Historique | C06 Confidentialité | Ligne d'aperçu « N dictées plus anciennes seraient supprimées… » | ui/src/tabs/History.svelte:155 | Historique | À TRANCHER (T1). |
| Historique | C06 Confidentialité | Bouton « Appliquer » (confirmation, définitif) | ui/src/tabs/History.svelte:157 ; confirmation `:122` | Historique | À TRANCHER (T1). `help="privacy"` ; ancre `conservation` visée par l'Aide (`Help.svelte:343`). |
| Historique | S07 Barre de filtres | Champ « Chercher dans les textes » | ui/src/tabs/History.svelte:163 | Historique | `div.card.filters`, pas une `<Card>`. |
| Historique | S07 Barre de filtres | Boutons de période (Aujourd'hui, 7 jours, 30 jours, Tout, Dates choisies) | ui/src/tabs/History.svelte:164-168 (liste `:31-35`) | Historique | |
| Historique | S07 Barre de filtres | Menu « Toutes les applis » | ui/src/tabs/History.svelte:169-172 | Historique | |
| Historique | S07 Barre de filtres | Case « Dictées reformulées » | ui/src/tabs/History.svelte:173 | Historique | |
| Historique | S07 Barre de filtres | Boutons « Tout déplier » / « Tout replier » (jours) | ui/src/tabs/History.svelte:174-179 | Historique | |
| Historique | S07 Barre de filtres | Case « Tout sélectionner » | ui/src/tabs/History.svelte:183 | Historique | |
| Historique | S07 Barre de filtres | Bouton « Effacer la sélection » | ui/src/tabs/History.svelte:186 | Historique | |
| Historique | S07 Barre de filtres | Dates « du / au » (période choisie) | ui/src/tabs/History.svelte:193-194 | Historique | |
| Historique | S07 Barre de filtres | Bouton « Tout afficher » (réinitialise les filtres) | ui/src/tabs/History.svelte:198 | Historique | |
| Historique | C07 Jour d'historique (une carte par jour, id `historique-<jour>`, sans `?`) | Case « Cocher ce jour » | ui/src/tabs/History.svelte:206-208 | Historique | Cartes dynamiques : les ids changent chaque jour. |
| Historique | C07 Jour d'historique | Bouton « Effacer ce jour » | ui/src/tabs/History.svelte:209 | Historique | |
| Historique | C07 Jour d'historique | Case de chaque dictée | ui/src/tabs/History.svelte:213 | Historique | |
| Historique | C07 Jour d'historique | Boutons « Copier » | ui/src/tabs/History.svelte:231 | Historique | |
| Historique | C07 Jour d'historique | Boutons « Supprimer » | ui/src/tabs/History.svelte:232 | Historique | |
| Historique | C07 Jour d'historique | Détail « Ce que Whisper a entendu » | ui/src/tabs/History.svelte:219 | Historique | |
| Historique | C07 Jour d'historique | Ligne mode, décision et temps de chaque dictée | ui/src/tabs/History.svelte:223-228 | Historique | |

#### 4. Dictionnaire (ce qui reste dans Dictionnaire)

| Page actuelle (onglet) | Carte | Réglage / contrôle | Fichier:ligne | Page cible | Remarque |
|---|---|---|---|---|---|
| Dictionnaire | (page) | Titre « Dictionnaire » + sous-titre + mention « Enregistré ✔ » | ui/src/tabs/Dictionary.svelte:265-266 | Dictionnaire | Enregistrement automatique 500 ms après la dernière modification (`:110-116`). |
| Dictionnaire | S08 Chiffres | « N mots dans ton dictionnaire » / « N corrections cette semaine » | ui/src/tabs/Dictionary.svelte:268-271 | Dictionnaire | `div.card`, pas de `<Card>`. |
| Dictionnaire | S09 Poignée | Poignée de hauteur des deux cartes Mots/Corrections (souris + flèches) | ui/src/tabs/Dictionary.svelte:314-330 (état `:36-78`) | Dictionnaire | Mémorisée dans `localStorage` `dictionary:height`. Libellé « deux cartes » à revoir si le nombre de cartes change. |
| Dictionnaire | C08 Mots à écrire exactement | Champ « Ajouter un mot ou une expression » | ui/src/tabs/Dictionary.svelte:277 | Dictionnaire | `?` → `dictionary`. Plafond 1 000 mots. |
| Dictionnaire | C08 Mots | Bouton « Ajouter » | ui/src/tabs/Dictionary.svelte:278 | Dictionnaire | |
| Dictionnaire | C08 Mots | Pastilles de mots avec « × Retirer » | ui/src/tabs/Dictionary.svelte:281-285 | Dictionnaire | |
| Dictionnaire | C09 Corrections « entendu → écrit » | Champ « Entendu » | ui/src/tabs/Dictionary.svelte:291 | Dictionnaire | Rempli par la commande vocale « ajouter au dictionnaire » (`nav.dictionaryPrefill`, `:26-34`). Ancre visée : `dictionary-corrections-entendu-ecrit`. |
| Dictionnaire | C09 Corrections | Champ « Écrit » | ui/src/tabs/Dictionary.svelte:293 | Dictionnaire | Reçoit le focus après un préremplissage. |
| Dictionnaire | C09 Corrections | Bouton « Ajouter » | ui/src/tabs/Dictionary.svelte:294 | Dictionnaire | |
| Dictionnaire | C09 Corrections | Tableau des corrections avec « × Retirer » | ui/src/tabs/Dictionary.svelte:296-309 | Dictionnaire | |
| Dictionnaire | C10 Exporter et importer le dictionnaire | Bouton « Exporter… » | ui/src/tabs/Dictionary.svelte:334 | Dictionnaire | À TRANCHER (T2) : le plan met « export du dictionnaire » dans Dictionnaire ET « sauvegarde/exports-imports » dans Données et système. Je propose de garder l'export/import de chaque objet avec son objet. |
| Dictionnaire | C10 Exporter et importer | Bouton « Importer… » | ui/src/tabs/Dictionary.svelte:335 | Dictionnaire | À TRANCHER (T2). Le plan ne cite que l'« export » : l'import suit. |
| Dictionnaire | C11 Exporter le dictionnaire (carte transitoire) | Cases « N mot(s) » / « N correction(s) » | ui/src/tabs/Dictionary.svelte:342,344 | Dictionnaire | À TRANCHER (T2). |
| Dictionnaire | C11 Exporter le dictionnaire | Détails « Voir les mots » / « Voir les corrections » | ui/src/tabs/Dictionary.svelte:343,345 | Dictionnaire | À TRANCHER (T2). |
| Dictionnaire | C11 Exporter le dictionnaire | Boutons « Enregistrer le fichier » / « Fermer » | ui/src/tabs/Dictionary.svelte:350-351 | Dictionnaire | À TRANCHER (T2). Texte `:341` : « (sous Linux, il est placé dans le dossier Documents) ». |
| Dictionnaire | C12 Importer un dictionnaire (carte transitoire) | Champ chemin + « Lire le fichier » (Linux seulement) | ui/src/tabs/Dictionary.svelte:358-361 | Dictionnaire | À TRANCHER (T2). |
| Dictionnaire | C12 Importer un dictionnaire | Boutons « Tout cocher / Tout décocher » (mots et corrections) | ui/src/tabs/Dictionary.svelte:366,378 | Dictionnaire | À TRANCHER (T2). |
| Dictionnaire | C12 Importer un dictionnaire | Case de chaque mot et de chaque correction | ui/src/tabs/Dictionary.svelte:370,382 | Dictionnaire | À TRANCHER (T2). |
| Dictionnaire | C12 Importer un dictionnaire | Boutons « Importer » / « Annuler » / « Fermer » | ui/src/tabs/Dictionary.svelte:397-400 | Dictionnaire | À TRANCHER (T2). |
| Dictionnaire | C13 Signes dictés | Liste des signes reconnus (dans la langue de l'interface) | ui/src/tabs/Dictionary.svelte:406-410 | Dictionnaire | `?` → `dictionary`. Liste lue via l'API `signs`. |
| Dictionnaire | C13 Signes dictés | Sous-partie « Listes » : exemples (tiret, point un, numéro suivant) | ui/src/tabs/Dictionary.svelte:411-417 | Dictionnaire | |
| Dictionnaire | C13 Signes dictés | Menu « Caractère devant chaque élément » (Tiret / Pastille / Étoile) | ui/src/tabs/Dictionary.svelte:418-426 | Dictionnaire | Réglage `listBullet` (mise en forme) : reste avec les signes dictés. |

#### 5. Commandes vocales (carte C14, aujourd'hui dans Dictionnaire)

Tout `ui/src/components/VoiceCommands.svelte` (746 lignes) devient la page Commandes vocales. Chaque commande intégrée a la même ligne de contrôles : « Phrases » (pastilles « × Retirer », champ « Ajouter une phrase », bouton « Ajouter », bouton « Rétablir » si modifiée ; une commande garde au moins une phrase).

| Page actuelle (onglet) | Carte | Réglage / contrôle | Fichier:ligne | Page cible | Remarque |
|---|---|---|---|---|---|
| Dictionnaire | C14 Commandes vocales | Titre, introduction et `?` de la carte | ui/src/components/VoiceCommands.svelte:465-470 | Commandes vocales | `help="dictionary"` (rubrique « Dictionnaire et signes »). L'intro dit « Sur Mac pour l'instant » et « Réglages, onglet Apparence ». Titre de la carte = futur titre de page : doublon. |
| Dictionnaire | C14 | Case « Activer les commandes vocales » | ui/src/components/VoiceCommands.svelte:472-478 | Commandes vocales | Réglage `voiceCommandsEnabled`. |
| Dictionnaire | C14 | Commande « Ajouter au dictionnaire » (phrases) | ui/src/components/VoiceCommands.svelte:22 | Commandes vocales | Agit ensuite sur la page Dictionnaire (`status.svelte.ts:34-35`). |
| Dictionnaire | C14 | Commande « Entre parenthèses » (phrases) | ui/src/components/VoiceCommands.svelte:26 | Commandes vocales | |
| Dictionnaire | C14 | Commande « Entre guillemets » (phrases) | ui/src/components/VoiceCommands.svelte:27 | Commandes vocales | |
| Dictionnaire | C14 | Commande « Entre crochets » (phrases) | ui/src/components/VoiceCommands.svelte:28 | Commandes vocales | |
| Dictionnaire | C14 | Commande « Entre accolades » (phrases) | ui/src/components/VoiceCommands.svelte:29 | Commandes vocales | |
| Dictionnaire | C14 | Commande « Gras » (phrases + raccourci `Mod+B`) | ui/src/components/VoiceCommands.svelte:35 ; champ raccourci 514-526 | Commandes vocales | Raccourci saisi par `ShortcutField.svelte`. Messages d'erreur `:114`, `:326` (« ⌘B sur Mac, Ctrl+B ailleurs »). |
| Dictionnaire | C14 | Commande « Italique » (phrases + raccourci `Mod+I`) | ui/src/components/VoiceCommands.svelte:36 | Commandes vocales | |
| Dictionnaire | C14 | Commande « Souligné » (phrases + raccourci `Mod+U`) | ui/src/components/VoiceCommands.svelte:37 | Commandes vocales | |
| Dictionnaire | C14 | Recherche « Google » (phrases) | ui/src/components/VoiceCommands.svelte:44 | Commandes vocales | Sous la touche de commande seulement. |
| Dictionnaire | C14 | Recherche « YouTube » (phrases) | ui/src/components/VoiceCommands.svelte:45 | Commandes vocales | |
| Dictionnaire | C14 | Recherche « Wikipédia » (phrases) | ui/src/components/VoiceCommands.svelte:46 | Commandes vocales | |
| Dictionnaire | C14 | Recherche « DuckDuckGo » (phrases) | ui/src/components/VoiceCommands.svelte:47 | Commandes vocales | |
| Dictionnaire | C14 | Commande « Nouveau mail » (phrases) | ui/src/components/VoiceCommands.svelte:53 | Commandes vocales | |
| Dictionnaire | C14 | Textes d'introduction des groupes « Recherches sur internet » et « Nouveau mail » | ui/src/components/VoiceCommands.svelte:42,52 | Commandes vocales | Parlent de « la touche de commande », que le plan rapatrie sur cette page. |
| Dictionnaire | C14 | « Mes sites de recherche » : liste (nom, hôte, phrases, « Supprimer ») | ui/src/components/VoiceCommands.svelte:532-554 | Commandes vocales | |
| Dictionnaire | C14 | « Nouveau site » : champs Nom, Adresse (`{q}`), Phrase + bouton « Ajouter le site » | ui/src/components/VoiceCommands.svelte:555-564 | Commandes vocales | |
| Dictionnaire | C14 | « Mes extraits » : liste (nom, taille, phrases, « Supprimer ») | ui/src/components/VoiceCommands.svelte:566-590 | Commandes vocales | Plafond 5 000 signes (`:200`). |
| Dictionnaire | C14 | Zone « Texte à coller » de chaque extrait | ui/src/components/VoiceCommands.svelte:586 | Commandes vocales | Enregistrée à la perte de focus. |
| Dictionnaire | C14 | « Nouvel extrait » : Nom, Phrase, Texte + bouton « Ajouter l'extrait » | ui/src/components/VoiceCommands.svelte:591-600 | Commandes vocales | |
| Dictionnaire | C14 | « Mes applications » : liste (nom, phrases, « Supprimer ») | ui/src/components/VoiceCommands.svelte:602-624 | Commandes vocales | Liste « non exportée avec l'utilisateur » (texte `:603`). |
| Dictionnaire | C14 | « Nouvelle application » : menu des applications installées, Phrase, « Ajouter l'application » | ui/src/components/VoiceCommands.svelte:625-636 | Commandes vocales | Liste lue via l'API `installed-apps`. |
| Dictionnaire | C14 | « Mes commandes » : liste (titre, case « Seulement si du texte est sélectionné », phrases, raccourci, « Supprimer ») | ui/src/components/VoiceCommands.svelte:638-671 | Commandes vocales | |
| Dictionnaire | C14 | Boutons « Exporter mes commandes… » / « Importer des commandes… » | ui/src/components/VoiceCommands.svelte:672-675 | Commandes vocales | À TRANCHER (T2) : même question export/import par objet. |
| Dictionnaire | C14 | Panneaux d'export et d'import des commandes (aperçu, cases, « Enregistrer le fichier », « Importer ») | ui/src/components/VoiceCommands.svelte:676-709 | Commandes vocales | À TRANCHER (T2). |
| Dictionnaire | C14 | « Nouvelle commande » : Titre, Phrase, Raccourci, case, « Ajouter la commande » | ui/src/components/VoiceCommands.svelte:710-720 | Commandes vocales | |
| Dictionnaire | C14 | Note de pied « Une phrase compte au moins deux mots… « Rétablir »… » | ui/src/components/VoiceCommands.svelte:722 | Commandes vocales | |
| (aucune) | (aucune carte) | « Reformulation de la sélection » : section annoncée par le plan | absent de `VoiceCommands.svelte` ; décrite seulement dans ui/src/tabs/Help.svelte:290 (et HelpEn.svelte:302) | Commandes vocales | À TRANCHER (T4) : le plan place « reformulation de la sélection » dans la carte VoiceCommands, mais aucun contrôle ni texte de ce nom n'existe dans cette carte. Aujourd'hui il n'y a que la case de consentement (C20, `Settings.svelte:232-238`, visible seulement pour un moteur hors de l'ordinateur et hors Linux, `crates/cli/src/web/mod.rs:815-821`). Proposition : créer une section « Reformuler la sélection » avec le texte d'explication + la case de consentement. |

À ajouter à la page Commandes vocales, venant d'autres cartes (lignes comptées dans leur carte d'origine) : « touche de commande » (C18, `KeyPicker.svelte:143-146`) ; case « Autoriser l'envoi de la sélection à ce moteur » (C20, `Settings.svelte:232-238`).

#### 6. Réglages › Dictée (composants `Users` puis `Settings section="dictation"`)

| Page actuelle (onglet) | Carte | Réglage / contrôle | Fichier:ligne | Page cible | Remarque |
|---|---|---|---|---|---|
| Réglages › Dictée | C15 Utilisateurs | Introduction (un utilisateur = dictionnaire, modes, commandes, historique, statistiques) | ui/src/components/Users.svelte:92 | Profil | `?` → `start`. Cite « le menu de l'icône ». Ancre `utilisateurs` (visée par l'Aide et l'Accueil). |
| Réglages › Dictée | C15 Utilisateurs | Liste des utilisateurs avec marque « actif » | ui/src/components/Users.svelte:100-110 | Profil | |
| Réglages › Dictée | C15 Utilisateurs | Bouton « Utiliser » (recharge la page) | ui/src/components/Users.svelte:111 ; `use()` 62-65 | Profil | |
| Réglages › Dictée | C15 Utilisateurs | Bouton « Exporter… » (ouvre C16) | ui/src/components/Users.svelte:112 | Profil | À TRANCHER (T2). |
| Réglages › Dictée | C15 Utilisateurs | « Renommer » + champ de nom + « Enregistrer » + « Annuler » | ui/src/components/Users.svelte:104-108,113 | Profil | |
| Réglages › Dictée | C15 Utilisateurs | Bouton « Supprimer » (mise de côté, confirmation) | ui/src/components/Users.svelte:114 ; confirmation 67-75 | Profil | |
| Réglages › Dictée | C15 Utilisateurs | Champ « Nom du nouvel utilisateur » + « Créer un utilisateur » | ui/src/components/Users.svelte:119-123 | Profil | |
| Réglages › Dictée | C15 Utilisateurs | Bouton « Importer un utilisateur… » (ouvre C17) | ui/src/components/Users.svelte:124 | Profil | À TRANCHER (T2). |
| Réglages › Dictée | C15 Utilisateurs | Case « Copier l'utilisateur actif… » | ui/src/components/Users.svelte:125 | Profil | |
| Réglages › Dictée | C16 Exporter un utilisateur (transitoire) | Case « Prénom et nom affiché » | ui/src/components/UserExport.svelte:89 | Profil | À TRANCHER (T2) pour toute la carte (lignes suivantes). |
| Réglages › Dictée | C16 Exporter un utilisateur | Case « Réglages de personne » | ui/src/components/UserExport.svelte:93 | Profil | À TRANCHER (T2). Libellé « Réglages » : à revoir si le mot disparaît de l'interface. |
| Réglages › Dictée | C16 Exporter un utilisateur | Case « Mots » (+ « Voir la liste ») | ui/src/components/UserExport.svelte:97-99 | Profil | À TRANCHER (T2). |
| Réglages › Dictée | C16 Exporter un utilisateur | Case « Corrections » (+ liste) | ui/src/components/UserExport.svelte:102-104 | Profil | À TRANCHER (T2). |
| Réglages › Dictée | C16 Exporter un utilisateur | Cases « Modes personnels » et « Inclure leurs exemples » | ui/src/components/UserExport.svelte:107-110 | Profil | À TRANCHER (T2). |
| Réglages › Dictée | C16 Exporter un utilisateur | Case « Consignes modifiées des modes livrés » | ui/src/components/UserExport.svelte:113 | Profil | À TRANCHER (T2). |
| Réglages › Dictée | C16 Exporter un utilisateur | Case « Commandes et leurs phrases » | ui/src/components/UserExport.svelte:117 | Profil | À TRANCHER (T2). |
| Réglages › Dictée | C16 Exporter un utilisateur | Case « Sites de recherche vocale » | ui/src/components/UserExport.svelte:122 | Profil | À TRANCHER (T2). |
| Réglages › Dictée | C16 Exporter un utilisateur | Case « Extraits vocaux (texte compris) » | ui/src/components/UserExport.svelte:127 | Profil | À TRANCHER (T2). |
| Réglages › Dictée | C16 Exporter un utilisateur | Boutons « Enregistrer… » / « Fermer » | ui/src/components/UserExport.svelte:133-134 | Profil | À TRANCHER (T2). |
| Réglages › Dictée | C17 Importer un utilisateur (transitoire) | Champ chemin + « Lire le fichier » (Linux) | ui/src/components/UserImport.svelte:99-102 | Profil | À TRANCHER (T2) pour toute la carte. |
| Réglages › Dictée | C17 Importer un utilisateur | Champ « Nom du nouvel utilisateur » | ui/src/components/UserImport.svelte:105-107 | Profil | À TRANCHER (T2). |
| Réglages › Dictée | C17 Importer un utilisateur | Cases par catégorie (prénom, réglages de personne, mots, corrections, modes + exemples, consignes, commandes, sites, extraits) | ui/src/components/UserImport.svelte:109-157 | Profil | À TRANCHER (T2). |
| Réglages › Dictée | C17 Importer un utilisateur | Boutons « Importer » / « Annuler » | ui/src/components/UserImport.svelte:158-161 | Profil | À TRANCHER (T2). |
| Réglages › Dictée | C17 Importer un utilisateur | Boutons « Passer à cet utilisateur » / « Fermer » | ui/src/components/UserImport.svelte:166-173 | Profil | À TRANCHER (T2). |
| Réglages › Dictée | C18 Dictée (13 réglages) | Introduction de la carte + `?` | ui/src/tabs/Settings.svelte:263 | Dicter | `help="dictate"`. Intro au « vous » (5 occurrences). Titre « Dictée » = titre de l'onglet actuel (doublon) et proche du futur « Dicter ». Ancre `dictee` (visée par l'Aide et le banc). |
| Réglages › Dictée | C18 Dictée | Texte GNOME (Wayland) : raccourci de dictée | ui/src/tabs/Settings.svelte:264-265 | Dicter | Affiché seulement sous GNOME. |
| Réglages › Dictée | C18 Dictée | Texte GNOME : raccourci de traduction | ui/src/tabs/Settings.svelte:266 | Dicter | Affiché seulement sous GNOME. |
| Réglages › Dictée | C18 Dictée | Texte GNOME : raccourci de commande | ui/src/tabs/Settings.svelte:267 | Commandes vocales | Affiché seulement sous GNOME. Touche de commande : à rapatrier. |
| Réglages › Dictée | C18 Dictée (réglage 1) | Touche de dictée + « Changer… » | ui/src/components/KeyPicker.svelte:135-138 (rendu `Settings.svelte:269`) | Dicter | Composant partagé : voir T3. |
| Réglages › Dictée | C18 Dictée (réglage 2) | Touche tenue en plus pour traduire + « Changer… » | ui/src/components/KeyPicker.svelte:139-142 | Dicter | |
| Réglages › Dictée | C18 Dictée (réglage 3) | Touche tenue en plus pour dicter une commande + « Changer… » | ui/src/components/KeyPicker.svelte:143-146 | Commandes vocales | Le plan la rapatrie sur cette page. `KeyPicker` gère les 3 touches ensemble (même capture, mêmes états) : à découper. |
| Réglages › Dictée | C18 Dictée | Panneau de capture (Annuler, Réessayer, Utiliser cette touche, Recommencer) | ui/src/components/KeyPicker.svelte:148-173 | Dicter | À TRANCHER (T3) : panneau commun aux 3 touches (`kind`), à instancier sur les deux pages. |
| Réglages › Dictée | C18 Dictée | Bouton « Tester ma touche » | ui/src/components/KeyPicker.svelte:176-180 (`test()` 113-124) | Dicter | À TRANCHER (T3) : ne teste que la touche de dictée. |
| Réglages › Dictée | C18 Dictée | Bouton « Rétablir {touches par défaut} » | ui/src/components/KeyPicker.svelte:178 (`reset()` 107-110) | Dicter | À TRANCHER (T3) : remet les 3 touches d'un coup ; le libellé cite les 3 touches. |
| Réglages › Dictée | C18 Dictée | Détail « Quelles touches choisir ? » | ui/src/components/KeyPicker.svelte:181-188 | Dicter | Deux textes selon Mac / autre ; tout le texte parle des 3 touches. |
| Réglages › Dictée | C18 Dictée (réglage 4) | Menu « Langue dictée » (Français, Anglais, Plusieurs langues) | ui/src/tabs/Settings.svelte:271-277 | Dicter | |
| Réglages › Dictée | C18 Dictée (réglage 5) | Champ « Prénom dans la salutation » | ui/src/tabs/Settings.svelte:278-283 | Profil | Plan : Profil. Texte « Le Tableau de bord vous salue par ce prénom » : à renommer. |
| Réglages › Dictée | C18 Dictée (réglage 6) | Menu « Genre de la personne qui dicte » | ui/src/tabs/Settings.svelte:284-293 | Profil | Plan : Profil. Le texte explique l'effet sur les modes (Style d'écriture). |
| Réglages › Dictée | C18 Dictée (réglage 7) | Menu « Traduire vers » (5 langues) | ui/src/tabs/Settings.svelte:294-300 | Dicter | |
| Réglages › Dictée | C18 Dictée (réglage 8) | Menu « Son au début » + bouton ▶ | ui/src/tabs/Settings.svelte:301-313 | Dicter | |
| Réglages › Dictée | C18 Dictée (réglage 9) | Menu « Son à la fin » + bouton ▶ | ui/src/tabs/Settings.svelte:301-313 | Dicter | Même boucle que la ligne précédente. |
| Réglages › Dictée | C18 Dictée (réglage 10) | Curseur « Volume des sons » | ui/src/tabs/Settings.svelte:315-320 | Dicter | |
| Réglages › Dictée | C18 Dictée (réglage 11) | Menu « Micro » | ui/src/tabs/Settings.svelte:321-328 | Dicter | |
| Réglages › Dictée | C18 Dictée (réglage 12) | Case « Mettre en pause la musique ou la vidéo pendant la dictée » | ui/src/tabs/Settings.svelte:329-332 | Dicter | |
| Réglages › Dictée | C18 Dictée (réglage 13) | Case « Remettre le presse-papiers d'avant après le collage » | ui/src/tabs/Settings.svelte:333-336 | Dicter | |
| Réglages › Dictée | C19 Ce que devient ton texte | Introduction + `?` (`modes`) | ui/src/tabs/Settings.svelte:341 | Style d'écriture | Intro cite « (onglet Modes) » et « l'onglet Moteurs ». Ancre `texte`. |
| Réglages › Dictée | C19 Ce que devient ton texte | Case « Sauter la mise au propre quand Whisper rend déjà un texte propre (mode Simple) » | ui/src/tabs/Settings.svelte:342-345 | Style d'écriture | Plan : « sauter la mise au propre ». |

Les lignes « C18 réglage 1 à 13 » correspondent aux 13 réglages annoncés pour la carte Dictée (3 touches, langue, prénom, genre, traduction, son début, son fin, volume, micro, médias, presse-papiers). Répartition : 10 vers Dicter (touche de dictée, touche de traduction, langue, traduction, son début, son fin, volume, micro, médias, presse-papiers), 1 vers Commandes vocales (touche de commande), 2 vers Profil (prénom, genre).

#### 7. Réglages › Moteurs (`Settings section="engines"` puis `Install`)

| Page actuelle (onglet) | Carte | Réglage / contrôle | Fichier:ligne | Page cible | Remarque |
|---|---|---|---|---|---|
| Réglages › Moteurs | C20 Moteurs | Introduction + `?` (`privacy`) | ui/src/tabs/Settings.svelte:137 | Moteurs et modèles | Titre « Moteurs » = titre de l'onglet actuel (doublon). Ancre `moteurs` (visée 3 fois par l'Aide). Intro au « vous » (votre voix, votre texte). |
| Réglages › Moteurs | C20 Moteurs | Résumé « Ce qui tourne quand vous dictez : transcription / mise au propre » | ui/src/tabs/Settings.svelte:138-140 | Moteurs et modèles | « vous » dans le texte. |
| Réglages › Moteurs | C20 Moteurs | Bouton « Relancer l'assistant » | ui/src/tabs/Settings.svelte:142 | Moteurs et modèles | Appelle `nav.openAssistant()` : navigue vers l'Accueil (C01). |
| Réglages › Moteurs | C20 Moteurs | Bouton « À quoi servent ces choix ? » | ui/src/tabs/Settings.svelte:143 | Moteurs et modèles | Appelle `nav.openHelp('start')`. |
| Réglages › Moteurs | C20 Moteurs | Texte « La transcription est le moteur qui… » | ui/src/tabs/Settings.svelte:145 | Moteurs et modèles | « vous » (votre voix). |
| Réglages › Moteurs | C20 Moteurs | Menu « Transcription » (Whisper / Deepgram) | ui/src/tabs/Settings.svelte:147-152 | Moteurs et modèles | |
| Réglages › Moteurs | C20 Moteurs | Menu « Modèle Whisper » | ui/src/tabs/Settings.svelte:154-160 | Moteurs et modèles | |
| Réglages › Moteurs | C20 Moteurs | Bouton « Choisir un fichier… » (modèle Whisper ailleurs) | ui/src/tabs/Settings.svelte:161 ; `chooseWhisperFile` 78-82 | Moteurs et modèles | Sous Linux : « pas encore pris en charge » (`crates/cli/src/web/mod.rs:747-750`). |
| Réglages › Moteurs | C20 Moteurs | Menu « Modèle Deepgram » (nova-3 / nova-2) | ui/src/tabs/Settings.svelte:163-168 | Moteurs et modèles | |
| Réglages › Moteurs | C20 Moteurs | Menu « Libérer Whisper au repos » + note mémoire | ui/src/tabs/Settings.svelte:171-184 | Moteurs et modèles | Note au « tu » (`:182`) juste après des textes au « vous ». |
| Réglages › Moteurs | C20 Moteurs | Texte « La mise au propre est le modèle d'IA… (voir l'onglet Modes) » | ui/src/tabs/Settings.svelte:185 | Moteurs et modèles | Cite un onglet qui disparaît. |
| Réglages › Moteurs | C20 Moteurs | Menu « Mise au propre » (Aucune, llama.cpp, LM Studio, Ollama, OpenRouter) | ui/src/tabs/Settings.svelte:186-195 | Moteurs et modèles | |
| Réglages › Moteurs | C20 Moteurs | Menu « Modèle GGUF » (si llama.cpp) | ui/src/tabs/Settings.svelte:196-203 | Moteurs et modèles | |
| Réglages › Moteurs | C20 Moteurs | Menu « Modèle LM Studio / Ollama » (si l'un des deux) | ui/src/tabs/Settings.svelte:205-213 | Moteurs et modèles | |
| Réglages › Moteurs | C20 Moteurs | Bouton « Actualiser » + ligne d'état du serveur | ui/src/tabs/Settings.svelte:214,216 | Moteurs et modèles | |
| Réglages › Moteurs | C20 Moteurs | Champ « Adresse du serveur » | ui/src/tabs/Settings.svelte:217-222 | Moteurs et modèles | |
| Réglages › Moteurs | C20 Moteurs | Champ « Modèle OpenRouter » | ui/src/tabs/Settings.svelte:224-231 | Moteurs et modèles | |
| Réglages › Moteurs | C20 Moteurs | Case « Autoriser l'envoi de la sélection à ce moteur » + note | ui/src/tabs/Settings.svelte:232-238 | Commandes vocales | Plan : à rapatrier. Visible seulement hors Linux et pour un moteur hors de l'ordinateur ; l'accord tombe si moteur, modèle ou adresse changent. Message Rust lié : « Reformulation impossible… (Réglages → Moteurs) ». Voir T4. |
| Réglages › Moteurs | C20 Moteurs | Menu « Carte graphique » (si llama.cpp) | ui/src/tabs/Settings.svelte:239-245 | Moteurs et modèles | Dépend de la liste lue par « Lister les cartes ». |
| Réglages › Moteurs | C20 Moteurs | Bouton « Lister les cartes » | ui/src/tabs/Settings.svelte:246 | Moteurs et modèles | |
| Réglages › Moteurs | C20 Moteurs | Menu « Garder le modèle chargé » (30 min / 2 h / 8 h / toujours) | ui/src/tabs/Settings.svelte:248-257 | Moteurs et modèles | Libellé « (comme Scribe) » à la ligne 251. |
| Réglages › Moteurs | C27 État de l'installation | Tableau d'état : whisper-server, modèle Whisper, llama-server, modèle GGUF, Micro, Touche globale | ui/src/tabs/Install.svelte:97-105 (lignes `:83-94`) | Moteurs et modèles | À TRANCHER (T7) : les lignes « Micro » (`:90`) et « Touche globale » (`:91`) ne concernent pas les moteurs (elles iraient avec Dicter). La ligne `:91` écrit « Ctrl droit (Maj : traduire) » sans condition de système. |
| Réglages › Moteurs | C28 Les programmes | Liste des programmes : « Voir la taille et la source » puis « Télécharger (taille) » + barre de progression | ui/src/tabs/Install.svelte:107-131 (bouton `:122`, téléchargement `:120`) | Moteurs et modèles | Libellés venant de Rust : « whisper-server (Windows) » etc. (`crates/cli/src/web/downloads.rs:25,50,56`), voir Textes (b). |
| Réglages › Moteurs | C29 Modèles installés | Tableau des modèles du dossier + bouton « Supprimer » | ui/src/components/InstalledModels.svelte:37-55 (bouton `:46`) | Moteurs et modèles | Le plan dit « moteurs puis modèles ». |
| Réglages › Moteurs | C30 Modèles recommandés | Liste des modèles proposés (même mécanique que C28) | ui/src/tabs/Install.svelte:135-137 | Moteurs et modèles | Intro : « dans la carte suivante » (position relative). |
| Réglages › Moteurs | C31 Ajouter un modèle depuis Hugging Face | Menu « Genre de modèle » (GGUF / Whisper) | ui/src/components/HubSearch.svelte:43-46 | Moteurs et modèles | Intro cite « Réglages → Moteurs » (`:41`). |
| Réglages › Moteurs | C31 Hugging Face | Champ de recherche + bouton « Chercher » | ui/src/components/HubSearch.svelte:47-48 | Moteurs et modèles | |
| Réglages › Moteurs | C31 Hugging Face | Choix « Classer les résultats » (3 boutons radio) | ui/src/components/HubSearch.svelte:50-55 | Moteurs et modèles | |
| Réglages › Moteurs | C31 Hugging Face | Liste des dépôts (bouton par dépôt) puis fichiers avec « Télécharger » | ui/src/components/HubSearch.svelte:57-98 (dépôt `:63`, téléchargement `:89`) | Moteurs et modèles | |
| Réglages › Moteurs | C21 Clés du cloud (dépannage) | Introduction + note sur le coffre (3 textes par système) + `?` (`privacy`) | ui/src/tabs/Settings.svelte:383-384 | Profil | Plan : clés API → Profil. Texte conditionné par `osName`. Ancre `cles`. « votre identifiant » (vous). |
| Réglages › Moteurs | C21 Clés du cloud | Champ « Deepgram » + « Enregistrer » + marque présente/absente | ui/src/tabs/Settings.svelte:385-393 | Profil | Écrit dans le trousseau / Gestionnaire d'identifiants (`crates/platform/src/lib.rs:54-64`). |
| Réglages › Moteurs | C21 Clés du cloud | Champ « OpenRouter » + « Enregistrer » | ui/src/tabs/Settings.svelte:385-393 | Profil | Même boucle. |
| Réglages › Moteurs | C22 Emplacements | Trois lignes : Réglages : chemin, Modèles : chemin, Journal : chemin | ui/src/tabs/Settings.svelte:397-401 | Données et système | Le plan supprime cette carte des Moteurs et la met dans Données et système. `help="privacy"` ; l'Aide dit « Réglages → Emplacements » (`Help.svelte:339`). |

#### 8. Réglages › Apparence

| Page actuelle (onglet) | Carte | Réglage / contrôle | Fichier:ligne | Page cible | Remarque |
|---|---|---|---|---|---|
| Réglages › Apparence | C23 Apparence de la fenêtre | Menu « Apparence » (Automatique / Claire / Sombre) | ui/src/tabs/Settings.svelte:110-118 | Apparence | `?` → `start`. Ancre `theme`. Le titre de la carte reprend le nom de l'onglet. |
| Réglages › Apparence | C24 Logo animé | Case « Logo animé » | ui/src/tabs/Settings.svelte:120-123 | Apparence | `?` → `start`. Le texte dit « en haut de la barre latérale ». |
| Réglages › Apparence | C25 Langue de l'interface | Menu « Langue » (Automatique / Français / English) | ui/src/tabs/Settings.svelte:125-133 | Apparence | À TRANCHER (T6) : le plan ne la nomme pas (« thème, logo, affichage, etc. »). Je propose Apparence. `?` → `dictate` (rubrique « Dicter et traduire », qui parle d'autre chose). Ancre `langue`. Le texte cite « la langue que vous dictez » : à ne pas confondre avec C18. |
| Réglages › Apparence | C26 Apparence de l'indicateur | Introduction (3 textes selon Mac / Linux / Windows) + `?` (`indicator`) | ui/src/tabs/Settings.svelte:350 | Apparence | « affichage » du plan = probablement cette carte. Ancre `indicateur`. Texte conditionné par système. |
| Réglages › Apparence | C26 Indicateur | Case « Afficher la pastille à l'écran (bêta) » | ui/src/tabs/Settings.svelte:351-354 | Apparence | |
| Réglages › Apparence | C26 Indicateur | Menu « Style » (pastille, point, vumètre) | ui/src/tabs/Settings.svelte:356-360 | Apparence | |
| Réglages › Apparence | C26 Indicateur | Menu « Position » | ui/src/tabs/Settings.svelte:361-365 | Apparence | |
| Réglages › Apparence | C26 Indicateur | Menu « Couleur » | ui/src/tabs/Settings.svelte:366-370 | Apparence | |
| Réglages › Apparence | C26 Indicateur | Curseur « Transparence » | ui/src/tabs/Settings.svelte:373-376 | Apparence | |
| Réglages › Apparence | C26 Indicateur | Bouton « Aperçu » | ui/src/tabs/Settings.svelte:377 | Apparence | Désactivé si la pastille est coupée (`overlay/preview`). |

#### 9. Réglages › Modes

| Page actuelle (onglet) | Carte | Réglage / contrôle | Fichier:ligne | Page cible | Remarque |
|---|---|---|---|---|---|
| Réglages › Modes | (page) | Trois paragraphes d'introduction (mode en cours, case de menu, exemple Simple) | ui/src/tabs/Modes.svelte:176-179 | Style d'écriture | Parlent du « menu de l'icône ». |
| Réglages › Modes | C32 Modes de base / Modes professionnels (une `<Card>` en boucle, 2 occurrences à l'exécution) | Bouton radio du mode choisi | ui/src/tabs/Modes.svelte:186-192 | Style d'écriture | `?` → `modes`. Ids `modes-groupe-0` et `-1`. |
| Réglages › Modes | C32 Modes | Case « Afficher dans le menu de l'icône » | ui/src/tabs/Modes.svelte:194 | Style d'écriture | Touche le menu de la barre des menus / zone de notification (Rust `tray_source.rs`). |
| Réglages › Modes | C32 Modes | Bouton « Essayer sur les phrases types » (ouvre C38) | ui/src/tabs/Modes.svelte:195-197 | Style d'écriture | |
| Réglages › Modes | C32 Modes | Menu « Fidélité » du mode Simple | ui/src/tabs/Modes.svelte:198-203 | Style d'écriture | Plan : « fidélité ». Réglage `simpleFidelity`. |
| Réglages › Modes | C32 Modes | Zone « consigne de style » modifiable | ui/src/tabs/Modes.svelte:204-206 | Style d'écriture | |
| Réglages › Modes | C32 Modes | Bouton « Rétablir la consigne d'origine » | ui/src/tabs/Modes.svelte:207 | Style d'écriture | |
| Réglages › Modes | C34 Mes modes | Liste des modes personnels (radio, case de menu, Essayer, Modifier, Exporter, Supprimer) | ui/src/tabs/Modes.svelte:214-229 | Style d'écriture | `?` → `modes`. Intro « créez votre mode » (vous). |
| Réglages › Modes | C34 Mes modes | Bouton « Créer un mode » | ui/src/tabs/Modes.svelte:230 | Style d'écriture | |
| Réglages › Modes | C34 Mes modes | Bouton « Importer un mode… » | ui/src/tabs/Modes.svelte:231 | Style d'écriture | À TRANCHER (T2) (import/export de modes avec les modes ?). |
| Réglages › Modes | C35 Importer des modes (transitoire) | Champ chemin (Linux), cases par mode, choix « l'ajouter / remplacer », « Importer », « Annuler » | ui/src/tabs/Modes.svelte:234-271 | Style d'écriture | À TRANCHER (T2). |
| Réglages › Modes | C36 Exporter le mode (transitoire) | Case « Inclure les exemples » + « Enregistrer le fichier » + « Fermer » | ui/src/tabs/Modes.svelte:273-288 | Style d'écriture | À TRANCHER (T2). |
| Réglages › Modes | C37 Nouveau mode / Modifier le mode (transitoire) | Champ « Nom » | ui/src/tabs/Modes.svelte:290-292 | Style d'écriture | |
| Réglages › Modes | C37 Nouveau mode | Zone « Consigne de style » | ui/src/tabs/Modes.svelte:293-294 | Style d'écriture | |
| Réglages › Modes | C37 Nouveau mode | Menu « Fidélité à ta dictée » + texte d'aide | ui/src/tabs/Modes.svelte:295-299 | Style d'écriture | Plan : « fidélité ». |
| Réglages › Modes | C37 Nouveau mode | Case « Vouvoie le destinataire » (si Reformulé ou Libre) | ui/src/tabs/Modes.svelte:300-302 | Style d'écriture | Libellé au sens « vouvoiement du destinataire du texte », pas une adresse à l'utilisateur. |
| Réglages › Modes | C37 Nouveau mode | Exemples (deux zones + « × ») et « Ajouter un exemple » | ui/src/tabs/Modes.svelte:303-313 | Style d'écriture | |
| Réglages › Modes | C37 Nouveau mode | Boutons « Annuler » / « Enregistrer » | ui/src/tabs/Modes.svelte:315-316 | Style d'écriture | |
| Réglages › Modes | C38 Essai : {mode} (transitoire) | Bouton « Arrêter / Fermer » + résultats des 12 phrases | ui/src/components/ModeTrial.svelte:52-72 | Style d'écriture | Sans `?`. Dépend du moteur de mise au propre (Moteurs et modèles). |

#### 10. Réglages › Mesures et essais (`Bench` puis `Journal`)

| Page actuelle (onglet) | Carte | Réglage / contrôle | Fichier:ligne | Page cible | Remarque |
|---|---|---|---|---|---|
| Réglages › Mesures et essais | C39 Banc de dictée (17 phrases) | Introduction (lire à voix haute, « Terminer ») | ui/src/tabs/Bench.svelte:141-145 | Mesures et essais | `?` → `measures`. Le texte « Lisez » est un impératif au vous. |
| Réglages › Mesures et essais | C39 Banc de dictée | Ligne « Les phrases suivent la langue dictée » + bouton « Changer la langue dictée » | ui/src/tabs/Bench.svelte:146-149 | Mesures et essais | Bouton : `nav.go('settings','dictee')` → à repointer sur Dicter. |
| Réglages › Mesures et essais | C39 Banc de dictée | Case « Comparer avec le modèle les phrases sautées » | ui/src/tabs/Bench.svelte:151-154 | Mesures et essais | Mémorisée dans `localStorage` `scriibe.bench.compare`. |
| Réglages › Mesures et essais | C39 Banc de dictée | Bouton « Commencer un passage » | ui/src/tabs/Bench.svelte:157 | Mesures et essais | |
| Réglages › Mesures et essais | C39 Banc de dictée | Bouton « Ne pas attendre » | ui/src/tabs/Bench.svelte:160,184 | Mesures et essais | |
| Réglages › Mesures et essais | C39 Banc de dictée | Bouton « ● Enregistrer / ■ Terminer » + jauge | ui/src/tabs/Bench.svelte:169-174 | Mesures et essais | |
| Réglages › Mesures et essais | C39 Banc de dictée | Case « Traduire (comme Ctrl + Maj) » | ui/src/tabs/Bench.svelte:176 | Mesures et essais | Libellé de touche écrit en dur, Windows (voir Textes b). |
| Réglages › Mesures et essais | C39 Banc de dictée | Boutons « Passer », « Relire la dernière », « Arrêter » | ui/src/tabs/Bench.svelte:177-179 | Mesures et essais | |
| Réglages › Mesures et essais | C39 Banc de dictée | Bouton « Recommencer un passage » | ui/src/tabs/Bench.svelte:187 | Mesures et essais | |
| Réglages › Mesures et essais | C39 Banc de dictée | Lecteur audio, résultats, profil du son, contrôles | ui/src/tabs/Bench.svelte:190-217 ; `ProfileBars.svelte` | Mesures et essais | |
| Réglages › Mesures et essais | C40 Ce passage | Récapitulatif phrase par phrase | ui/src/tabs/Bench.svelte:220-251 | Mesures et essais | `?` → `measures`. Note `:249` nomme l'onglet « Journal et rapport », qui n'existe pas (nom périmé). |
| Réglages › Mesures et essais | C41 Passages gardés | Liste des passages + bouton « Supprimer » | ui/src/components/BenchArchive.svelte:56-70 | Mesures et essais | `:70` dit « lisez quelques phrases ci-dessus ». |
| Réglages › Mesures et essais | C42 Comparer deux passages | Menus « Ancien » / « Récent » + comparaison | ui/src/components/BenchArchive.svelte:75-85 ; `BenchCompare.svelte` | Mesures et essais | Affichée seulement avec 2 passages ou plus. |
| Réglages › Mesures et essais | C43 Temps des dictées | Moyennes et tableau des dernières dictées mesurées | ui/src/components/TimingsCard.svelte:19-39 | Mesures et essais | Plan : « temps ». |
| Réglages › Mesures et essais | C44 Banc d'essai de la mise au propre | Bouton « Lancer les essais » | ui/src/components/TrialBench.svelte:97-100 | Mesures et essais | `?` → `measures`. |
| Réglages › Mesures et essais | C44 Banc d'essai | Zone de texte de l'essai | ui/src/components/TrialBench.svelte:103-104 | Mesures et essais | Écrit `trialText`. |
| Réglages › Mesures et essais | C44 Banc d'essai | Bouton « Rétablir l'exemple » | ui/src/components/TrialBench.svelte:105 | Mesures et essais | |
| Réglages › Mesures et essais | C44 Banc d'essai | Menu moteur de chaque candidat | ui/src/components/TrialBench.svelte:110 | Mesures et essais | |
| Réglages › Mesures et essais | C44 Banc d'essai | Champ ou menu « modèle » de chaque candidat | ui/src/components/TrialBench.svelte:114,117 | Mesures et essais | |
| Réglages › Mesures et essais | C44 Banc d'essai | Bouton « 🗑 Retirer de la comparaison » | ui/src/components/TrialBench.svelte:124 | Mesures et essais | |
| Réglages › Mesures et essais | C44 Banc d'essai | Bouton « Utiliser ce modèle » | ui/src/components/TrialBench.svelte:134 | Mesures et essais | Modifie le moteur de mise au propre (réglage de Moteurs et modèles) depuis une page de mesures. |
| Réglages › Mesures et essais | C44 Banc d'essai | Bouton « + Ajouter un modèle » | ui/src/components/TrialBench.svelte:143 | Mesures et essais | |
| Réglages › Mesures et essais | C45 Dictée avec la touche, dans une vraie appli | Texte d'essai + « Journal en direct » (zone `pre`) | ui/src/tabs/Journal.svelte:45-49 | Mesures et essais | À TRANCHER (T5) : le plan met « journal » dans Données et système ET « journal en direct » dans Mesures et essais. Je propose : journal en direct → Mesures et essais ; seule la ligne « Journal : chemin » (C22) et un éventuel bouton d'ouverture du journal vont dans Données et système. `help="problems"`. Texte `:46` « Ouvrez le Bloc-notes, maintenez Ctrl droit » sans condition. |
| Réglages › Mesures et essais | C46 Rapport à me coller | Bouton « Copier le rapport » + zone du rapport | ui/src/tabs/Journal.svelte:51-53 | Mesures et essais | `?` → `measures`. Plan : « rapport ». |

#### 11. Réglages › Système

| Page actuelle (onglet) | Carte | Réglage / contrôle | Fichier:ligne | Page cible | Remarque |
|---|---|---|---|---|---|
| Réglages › Système | C47 Lancer au démarrage | Case « Lancer Scriibe à l'ouverture de session » | ui/src/tabs/System.svelte:167-172 | Données et système | `?` → `start`. Ancre `demarrage`. Intro au « vous ». |
| Réglages › Système | C47 Lancer au démarrage | Notes (raison si non pris en charge, désactivé par le système, où le voir : 3 textes par système) | ui/src/tabs/System.svelte:173-178 | Données et système | Le texte Windows cite le « Gestionnaire des tâches » ; conditionné par `osName`. |
| Réglages › Système | C48 Sauvegarde de vos données | Texte d'introduction + avertissement « en clair » | ui/src/tabs/System.svelte:182-183 | Données et système | Titre « de vos données » et textes au « vous » (5 + 2). Ancre `sauvegarde`. |
| Réglages › Système | C48 Sauvegarde | Bouton « Enregistrer une sauvegarde… » | ui/src/tabs/System.svelte:185 | Données et système | |
| Réglages › Système | C48 Sauvegarde | Bouton « Restaurer une sauvegarde… » | ui/src/tabs/System.svelte:186 | Données et système | |
| Réglages › Système | C48 Sauvegarde | Messages « Sauvegarde enregistrée et relue » / erreurs | ui/src/tabs/System.svelte:188-189 | Données et système | |
| Réglages › Système | C48 Sauvegarde | Panneau de restauration (chemin sous Linux, aperçu, « Restaurer », « Annuler ») | ui/src/tabs/System.svelte:191-218 | Données et système | Confirmation `:59` au « vous ». |
| Réglages › Système | C49 Désinstaller Scriibe | Bouton « Désinstaller Scriibe… » | ui/src/tabs/System.svelte:221-223 | Données et système | Ancre `desinstaller`. |
| Réglages › Système | C49 Désinstaller Scriibe | Panneau : liste exacte, cases « Garder mes modèles » et « Faire d'abord une sauvegarde », « Désinstaller », « Annuler » | ui/src/tabs/System.svelte:224-266 | Données et système | Texte de clés d'API conditionné par `plan.keys` (trousseau / Gestionnaire d'identifiants / environnement). |

#### 12. Réglages › Mise à jour et en-tête de la page Réglages

| Page actuelle (onglet) | Carte | Réglage / contrôle | Fichier:ligne | Page cible | Remarque |
|---|---|---|---|---|---|
| Réglages › Mise à jour | C50 Version de Scriibe | Ligne « Version installée : x » | ui/src/tabs/Update.svelte:70-72 | Données et système | `?` → `problems`. |
| Réglages › Mise à jour | C50 Version de Scriibe | Bouton « Chercher une nouvelle version » | ui/src/tabs/Update.svelte:75 | Données et système | |
| Réglages › Mise à jour | C50 Version de Scriibe | Résultat + bouton « Installer et redémarrer » | ui/src/tabs/Update.svelte:78-101 (confirmation `:48-53`) | Données et système | |
| Réglages › Mise à jour | C50 Version de Scriibe | Case « Chercher automatiquement… (toutes les 6 heures) » | ui/src/tabs/Update.svelte:103-108 | Données et système | |
| Réglages › Mise à jour | C50 Version de Scriibe | Barre de progression et résultats | ui/src/tabs/Update.svelte:110-117 | Données et système | |
| Réglages › Mise à jour | C51 Comment ça marche ? (repliée par défaut) | Mode d'emploi + pas à pas | ui/src/tabs/Update.svelte:119-146 | Données et système | `?` → `problems`. Dit « le bouton « Nouvelle version disponible » apparaît dans la barre latérale, et une ligne en haut du menu de l'icône » (`:124`) et « en haut de cette page » (`:130`). |
| Réglages (page) | (page) | Titre « Réglages » et sous-titre « Comment Scriibe écoute, écrit et colle » | ui/src/tabs/SettingsPage.svelte:18-19 | (disparaît) | Aucun équivalent dans le plan. |
| Réglages (page) | (page) | 7 pilules d'onglets | ui/src/tabs/SettingsPage.svelte:21-26 | (disparaît) | |
| Réglages (page) | (page) | Point « ● » de nouvelle version sur la pilule « Mise à jour » | ui/src/tabs/SettingsPage.svelte:25 | Données et système | Le plan reporte ce point sur l'entrée « Données et système » de la barre latérale. |
| Réglages (page) | (page) | Phrase de description de l'onglet courant | ui/src/tabs/SettingsPage.svelte:27 ; textes `tabs.ts:14-20` | (disparaît) | Les 7 descriptions sont lues par `i18n.test.ts` (voir Liens). |

#### 13. Aide, pastille, fenêtres

| Page actuelle (onglet) | Carte | Réglage / contrôle | Fichier:ligne | Page cible | Remarque |
|---|---|---|---|---|---|
| Aide | S10 Corps de l'Aide | Sommaire (8 rubriques) | ui/src/tabs/Help.svelte:101-105 ; titres `ui/src/lib/help.ts:2-9` | Aide | Rubriques : start, dictate, modes, dictionary, indicator, measures, privacy, problems. Mêmes ids que les `?` (`HelpId`). |
| Aide | S10 | Rubrique « Premiers pas » | ui/src/tabs/Help.svelte:108-188 | Aide | Décrit « la fenêtre a cinq pages » et « Réglages avec 7 onglets » (`:122`). |
| Aide | S10 | Rubrique « Dicter et traduire » | ui/src/tabs/Help.svelte:189-218 | Aide | |
| Aide | S10 | Rubrique « Les modes » | ui/src/tabs/Help.svelte:219-258 | Aide | |
| Aide | S10 | Rubrique « Dictionnaire et signes » (contient aussi les commandes vocales, `:280-293`) | ui/src/tabs/Help.svelte:259-294 | Aide | Le `?` de la future page Commandes vocales visera cette rubrique, qui mélange deux sujets. |
| Aide | S10 | Rubrique « L'indicateur » (3 illustrations de la pastille) | ui/src/tabs/Help.svelte:295-314 | Aide | Utilise `OverlayView` (`:301`). |
| Aide | S10 | Rubrique « Mesurer et comparer » | ui/src/tabs/Help.svelte:315-328 | Aide | |
| Aide | S10 | Rubrique « Sur cet ordinateur ou en ligne » | ui/src/tabs/Help.svelte:329-344 | Aide | |
| Aide | S10 | Rubrique « Problèmes fréquents » (dernière branche `{:else}`) | ui/src/tabs/Help.svelte:345-362 | Aide | |
| Aide | S10 | 19 boutons « → … » vers une page ou une carte | ui/src/tabs/Help.svelte:77-79 (définition), 117-120, 144, 184-186, 217, 257, 293, 313, 327, 343, 361 | Aide | Liste détaillée dans « Liens ». |
| Aide | S10 | Bouton « → Relancer l'assistant de premier lancement » | ui/src/tabs/Help.svelte:124 | Aide | `nav.openAssistant()`. |
| Aide | S10 | Version anglaise (même structure, comptée par un test) | ui/src/tabs/HelpEn.svelte (rubriques 120, 201, 231, 271, 307, 327, 341, 357) | Aide | `App.svelte:116` choisit `HelpEn` si `i18n.lang === 'en'`. Toute modification de `Help.svelte` doit être reportée dans `HelpEn.svelte` (test). |
| Pastille | S11 Pastille à l'écran | Fenêtre transparente rendue par `Overlay.svelte` (interroge l'API `overlay` toutes les 45 ms) | ui/src/overlay/Overlay.svelte:13-34 ; choisie par `ui/src/main.ts:10` (`#overlay`) | (hors plan) | Pas de réglage dans la pastille : les réglages sont dans C26. |
| Pastille | S11 Pastille | Rendu des états (écoute, traitement, collé, message) en 3 styles | ui/src/overlay/OverlayView.svelte:33-73 (textes 38, 44-49) | (hors plan) | Réutilisé en miniature par l'Aide. Aucun texte de lieu. |
| Pastille | S12 Fenêtres Tauri | Titres de fenêtre « Scriibe » et « Scriibe — indicateur » | crates/cli/src/window.rs:53 (principale), :130 (pastille) | (hors plan) | Titre de fenêtre, pas un lieu d'interface. |

#### À TRANCHER — récapitulatif

| N° | Objet | Proposition et raison |
|---|---|---|
| T1 | Carte « Confidentialité de l'historique » (C06), non citée par le plan | Laisser dans Historique, en tête : elle agit directement sur la liste et son aperçu ; l'Aide y renvoie par `history` + ancre `conservation`. Alternative : Données et système. |
| T2 | Export et import par objet (dictionnaire C10-C12, utilisateurs C15-C17, modes C34-C36, commandes C14) contre « sauvegarde/exports-imports » de Données et système | Garder chaque export/import avec son objet (le plan nomme déjà « export du dictionnaire » dans Dictionnaire, et « utilisateurs » dans Profil) ; Données et système ne garde que la sauvegarde complète (C48). À confirmer. |
| T3 | `KeyPicker` partagé entre 3 touches (dictée, traduction, commande), avec capture, test et « Rétablir » communs | Le découper par `kind` : Dicter montre dictée + traduction + test ; Commandes vocales montre commande. « Rétablir » : un bouton par page qui ne remet que ses touches (aujourd'hui il remet les trois). |
| T4 | « Reformulation de la sélection » : aucune section n'existe | Créer une section dans Commandes vocales : explication + case de consentement (C20 « Autoriser l'envoi… »). |
| T5 | « Journal » cité dans Données et système et « journal en direct » dans Mesures et essais | Journal en direct → Mesures et essais ; chemin du journal (C22) → Données et système. |
| T6 | Carte « Langue de l'interface » (C25), absente du plan | Apparence : c'est un réglage de la fenêtre, comme le thème. |
| T7 | Tableau « État de l'installation » (C27) mélange moteurs et lignes « Micro » / « Touche globale » | Moteurs et modèles en tête de page ; déplacer ou supprimer les deux lignes qui ne sont pas des moteurs. |

---

### Décomptes (Partie 1)

| Mesure | Nombre |
|---|---|
| Cartes `<Card>` dans le code (C01 à C51 sauf C33, qui est la 2e occurrence à l'exécution de la `<Card>` de C32 posée dans une boucle) | 50 |
| dont avec bouton « ? » | 47 (sans « ? » : C01 assistant, C07 jours d'historique, C38 essai de mode) |
| dont transitoires (apparaissent sur action) | 8 : C11, C12, C16, C17, C35, C36, C37, C38 |
| dont à ids dynamiques | 2 : C07 (`historique-<jour>`), C32 (`modes-groupe-0/1`) |
| Zones d'interface hors `<Card>` (S01 à S12) | 12 |
| Lignes du tableau (cartes, zones et contrôles) | 273 |
| Lignes « À TRANCHER » | 42 lignes, pour 7 points distincts (T1 à T7) |
| Lignes dont la page cible est « hors plan » (coque, pastille, fenêtres) | 13 |
| Lignes dont la page cible disparaît (en-tête de la page Réglages) | 3 |

Répartition des 273 lignes par page cible : Commandes vocales 32, Profil 29, Mesures et essais 28, Moteurs et modèles 28, Historique 22, Dictionnaire 22, Style d'écriture 21, Données et système 18, Accueil 18, Dicter 17, Aide 12, Apparence 10, hors plan 13, disparaît 3.

Répartition des 50 cartes par page cible (une carte comptée une fois, à sa page principale) : Mesures et essais 9 (C05, C39 à C46), Style d'écriture 7 (C19, C32, C34 à C38), Moteurs et modèles 6 (C20, C27 à C31), Dictionnaire 6 (C08 à C13), Données et système 6 (C22, C47 à C51), Accueil 4 (C01 à C04), Profil 4 (C15, C16, C17, C21), Apparence 4 (C23 à C26), Historique 2 (C06, C07), Dicter 1 (C18), Commandes vocales 1 (C14).

Cartes à découper (leurs lignes vont vers plusieurs pages) :
- C18 « Dictée » : 13 réglages = 10 vers Dicter, 1 vers Commandes vocales (touche de commande), 2 vers Profil (prénom, genre). Texte GNOME de commande aussi vers Commandes vocales.
- C20 « Moteurs » : tout reste dans Moteurs et modèles sauf la case « Autoriser l'envoi de la sélection » (vers Commandes vocales). Le bouton « Relancer l'assistant » ouvre l'Accueil.
- C27 « État de l'installation » : lignes Micro et Touche globale à part (T7).
- `KeyPicker` (dans C18) : composant à découper (T3).
- C14 « Commandes vocales » : change seulement de page, sans découpage ; une section « Reformuler la sélection » est à créer (T4).
- C05 « Ma vitesse au clavier » : quitte l'Accueil pour Mesures et essais.

Cartes qui changent de page sans découpage : C14 (Dictionnaire → Commandes vocales), C05 (Accueil → Mesures et essais), C19 (Dictée → Style d'écriture), C21 (Moteurs → Profil), C22 (Moteurs → Données et système), C15 à C17 (Dictée → Profil), C23 à C26 (de l'onglet Apparence à la page Apparence : pas de changement de contenu), C47 à C51 (Système et Mise à jour → Données et système).

---

### Liens

Mots utilisés dans la colonne « Risque » : **cassé** (le lien ne marchera plus sans modification), **test** (un test échoue), **texte** (texte devenu faux), **perte** (donnée de l'utilisateur perdue, par exemple un pli retenu).

#### A. Pages, onglets et navigation (code)

| Fichier:ligne | Ce qui vise | Cible actuelle (page/onglet/carte/ancre) | Cible après refonte | Risque |
|---|---|---|---|---|
| ui/src/lib/tabs.ts:2-8 | Liste `tabs` : ids `dashboard`, `history`, `dictionary`, `settings`, `help`, libellés, icônes | 5 pages de la barre latérale | 11 entrées en 4 groupes + Aide ; `dashboard` devient Accueil | cassé : l'id `dashboard` est utilisé par `nav.svelte.ts:17,62`, `App.svelte:113`, `Help.svelte:120`, `HelpEn.svelte:132`. test : `tabs.test.ts:11,16-17` (Aide dernière, ids et libellés uniques). |
| ui/src/lib/tabs.ts:13-21 | `settingsTabs` : 7 onglets (`dictation`, `engines`, `appearance`, `modes`, `measures`, `system`, `update`), libellés et descriptions | Onglets de Réglages | Disparaissent | test : `i18n.test.ts:13` lit `label:`/`description:` dans `tabs.ts` ; chaque texte supprimé devient une « traduction orpheline » de `locales/en.ts` (`i18n.test.ts:80-81`). |
| ui/src/lib/tabs.ts:23,26 | Types `SettingsTabId`, `TabId = MainTab \| 'modes' \| 'install' \| 'bench' \| 'update' \| 'journal'` | Ids d'anciennes pages | À remplacer par les nouvelles clés de page | cassé : `Dashboard.svelte:12,14`, `Help.svelte:9`, `HelpEn.svelte:11`, `Assistant.svelte:11` (`'install' \| 'settings'`) typent leurs appels avec ces ids. |
| ui/src/lib/tabs.ts:28-34 | `movedToSettings` : modes→modes, install→engines, bench→measures, journal→measures, update→update | Aiguillage ancienne page → onglet | À supprimer ou à remplacer par ancienne clé → nouvelle page | cassé : tous les `go('install')`, `go('bench')`, `go('journal')`, `go('modes')`, `go('update')` de l'Aide. |
| ui/src/lib/tabs.ts:36-40 | `resolveTab()` | Renvoie `{tab:'settings', settingsTab}` | À simplifier | cassé si `settings` n'existe plus. |
| ui/src/lib/tabs.ts:42-54 | `settingsAnchors` : `dictee`, `utilisateurs`, `texte`, `moteurs`, `cles`, `langue`, `indicateur`, `demarrage`, `sauvegarde`, `desinstaller` → onglet | Ancre de carte → onglet de Réglages | Ancre de carte → nouvelle page | Visées par un lien aujourd'hui : `moteurs`, `dictee`, `utilisateurs`, `indicateur`. Jamais visées : `texte`, `cles`, `langue`, `demarrage`, `sauvegarde`, `desinstaller`. |
| ui/src/lib/tabs.ts:56-58 | `settingsTabOfAnchor()` | Lue par `nav.go` | À remplacer par « page de l'ancre » | cassé. |
| ui/src/lib/nav.svelte.ts:5-13,17 | `saved('tab', tabs, 'dashboard')` : page ouverte retenue dans `sessionStorage` (clé `tab`) | Page affichée | Valeur par défaut = Accueil | Une valeur `settings` déjà retenue n'est plus valable ; la fonction retombe sur le défaut (pas d'erreur). |
| ui/src/lib/nav.svelte.ts:19,31-32,45-54 | `settingsTab` retenu (clé `settingsTab`), `chooseSettingsTab()` | Onglet de Réglages | À supprimer | cassé : appelée par `SettingsPage.svelte:23`. |
| ui/src/lib/nav.svelte.ts:25-43 | `go(id, anchor)` : change de page, choisit l'onglet par ancre, attend `tick()`, déplie la carte repliée (`button.fold`), `scrollIntoView` sinon `scrollTo(0,0)` | Toute la navigation | À réécrire avec les nouvelles pages | cassé : la carte visée doit exister dans le DOM après `tick()` ; une carte dont l'id change n'est plus trouvée (retombe sur le haut de page, sans erreur). |
| ui/src/lib/nav.svelte.ts:22 | `dictionaryPrefill` (texte sélectionné → « Entendu ») | Dictionnaire | Dictionnaire | À garder : lu par `Dictionary.svelte:27-34`. |
| ui/src/lib/nav.svelte.ts:56-63 | `assistantRequested`, `openAssistant()` → `go('dashboard')` | Tableau de bord | Accueil | cassé si `dashboard` est renommé ; lu par `Dashboard.svelte:32-37`. |
| ui/src/lib/nav.svelte.ts:65-69 | `openHelp(section)` → `go('help')` + `helpSection` | Aide (rubrique non retenue au rechargement) | Aide | Sans changement si l'id `help` est gardé. |
| ui/src/lib/help.ts:2-11,13 | 8 `helpSections` (ids `start`…`problems`), rubrique par défaut `start` | Rubriques de l'Aide | Inchangé en principe | test : `tabs.test.ts:8-13` (8 titres distincts, « Premiers pas » par défaut). |
| ui/src/App.svelte:27,84-86 | `choose(id, anchor)` → `nav.go` ; boutons de la barre | Barre latérale | Nouvelle barre à groupes | cassé. |
| ui/src/App.svelte:95-97 | Bouton « Nouvelle version disponible » → `choose('update')` (visible si `status.value.update`) | Onglet Mise à jour | Entrée « Données et système » portant un point | cassé (`update` n'existe plus). |
| ui/src/App.svelte:113-117 | Aiguillage `nav.tab` → `Dashboard`, `History`, `Dictionary`, `Help`/`HelpEn`, sinon `SettingsPage` | Pages | 11 composants de page | cassé : le `{:else}` envoie tout le reste vers `SettingsPage`. |
| ui/src/lib/status.svelte.ts:31-36 | Commande vocale « ajouter au dictionnaire » : `nav.go('dictionary', 'dictionary-corrections-entendu-ecrit')` | Dictionnaire, carte C09 | Dictionnaire (si l'id est gardé) | cassé si l'id de C09 change ; aucun test ne couvre ce lien. |
| ui/src/tabs/SettingsPage.svelte:25 | Point « ● » sur la pilule `update` quand `status.value?.update` | Pilule Mise à jour | Point sur l'entrée « Données et système » | cassé : le plan déplace le point. |
| ui/src/tabs/SettingsPage.svelte:30-36 | Assemblage des 7 onglets (`Users`+`Settings`, `Settings`+`Install`, `Settings`, `Modes`, `Bench`+`Journal`, `System`, `Update`) | Composition des pages | À refaire en 6 pages ; `Settings.svelte` à découper | cassé. |
| ui/src/main.ts:10 | `location.hash === '#overlay'` → `Overlay` sinon `App` | Fenêtre de la pastille | Inchangé | Ne pas utiliser `#...` pour autre chose. |
| ui/src/components/Card.svelte:10,33,40 | `id` de carte = ancre + clé de pli ; `help` → `HelpButton` ; `data-folded` | Toutes les cartes | Idem | perte : un id renommé fait perdre le pli retenu (`localStorage` `fold:<id>`). |
| ui/src/lib/folds.ts:2,7,16 | Clés `fold:<id>` dans `localStorage` | Pli des 50 cartes | Idem | perte (voir ci-dessus). Autres clés `localStorage` : `dictionary:height` (`Dictionary.svelte:37`), `scriibe.bench.compare` (`Bench.svelte:30`), `uiLanguage` (`i18n.svelte.ts:3`). |
| ui/src/tabs/Dashboard.svelte:29,68-70 | `getElementById('assistant')` + `<div id="assistant">` ; `ongoto={(tab) => goto(tab)}` | Assistant sur le tableau de bord | Accueil | cassé si l'id change. |
| ui/src/tabs/Dashboard.svelte:56 | Bouton « changer » : `goto('settings','utilisateurs')` | Réglages › Dictée, carte `utilisateurs` | Profil | cassé. |
| ui/src/tabs/Dashboard.svelte:108 | Bouton « Tout voir » : `goto('history')` | Historique | Historique | OK si l'id `history` est gardé. |
| ui/src/components/Assistant.svelte:11,78 | `ongoto('install')` (« Aller à l'installation ») | Onglet Moteurs, cartes d'installation | Moteurs et modèles | cassé. |
| ui/src/components/Assistant.svelte:11,79 | `ongoto('settings')` (« Aller aux Réglages », pour la clé OpenRouter) | Page Réglages sans ancre : arrive sur l'onglet retenu en dernier, pas sur les clés | Profil (clés API) | cassé ; défaut existant : aucune ancre `cles` n'est passée. |
| ui/src/tabs/Bench.svelte:148 | Bouton « Changer la langue dictée » : `nav.go('settings','dictee')` | Réglages › Dictée, carte `dictee` | Dicter | cassé. |
| ui/src/tabs/Settings.svelte:142-143 | « Relancer l'assistant » → `nav.openAssistant()` ; « À quoi servent ces choix ? » → `nav.openHelp('start')` | Accueil ; Aide › Premiers pas | Idem depuis Moteurs et modèles | Pas cassé tant que `openAssistant` et `openHelp` existent. |
| ui/src/components/HelpButton.svelte:11 | `nav.openHelp(section)` pour tout bouton « ? » | Aide › rubrique | Idem | Voir la liste des `?` plus bas. |

#### B. Boutons « ? » (`<Card help="…">` → `HelpButton`) : 47 occurrences

Toutes passent par `Card.svelte:40` (`<HelpButton section={help} />`) et `HelpButton.svelte:11`. La rubrique visée est un des 8 ids de `help.ts`. Après la refonte, l'id de rubrique ne change pas, mais la rubrique d'Aide devra parler de la nouvelle page. Sans `?` : C01 Assistant, C07 Jour d'historique, C38 Essai de mode.

| Fichier:ligne | Ce qui vise | Cible actuelle (page/onglet/carte/ancre) | Cible après refonte | Risque |
|---|---|---|---|---|
| ui/src/tabs/Dashboard.svelte:84,97,107,119 | `?` des cartes C02, C03, C04, C05 | Aide › `measures` (« Mesurer et comparer ») | C02 à C04 restent sur Accueil, C05 va en Mesures et essais | texte : la rubrique `measures` parle du banc, pas des statistiques de l'Accueil. |
| ui/src/tabs/History.svelte:145 | `?` de C06 | Aide › `privacy` | Idem | |
| ui/src/tabs/Dictionary.svelte:275,289,332,340,357,405 | `?` de C08, C09, C10, C11, C12, C13 | Aide › `dictionary` | Idem | |
| ui/src/components/VoiceCommands.svelte:468 | `?` de C14 | Aide › `dictionary` (qui contient aussi les commandes vocales, `Help.svelte:280-293`) | Commandes vocales → toujours `dictionary` | texte : la rubrique mélange dictionnaire et commandes ; à scinder ou à retitrer. |
| ui/src/components/Users.svelte:92 ; UserExport.svelte:81 ; UserImport.svelte:98 | `?` de C15, C16, C17 | Aide › `start` | Profil | texte : « Premiers pas » contient la gestion des utilisateurs (`Help.svelte:135-147`). |
| ui/src/tabs/Settings.svelte:263 ; :125 | `?` de C18 et de C25 | Aide › `dictate` | Dicter ; Apparence | texte : C25 renvoie à « Dicter et traduire » alors que la carte parle de la langue de l'interface. |
| ui/src/tabs/Settings.svelte:341 | `?` de C19 | Aide › `modes` | Style d'écriture | |
| ui/src/tabs/Settings.svelte:137 ; :383 ; :398 | `?` de C20, C21, C22 | Aide › `privacy` | Moteurs et modèles ; Profil ; Données et système | texte : la rubrique `privacy` parle surtout des moteurs. |
| ui/src/tabs/Settings.svelte:110 ; :120 | `?` de C23, C24 | Aide › `start` | Apparence | |
| ui/src/tabs/Settings.svelte:350 | `?` de C26 | Aide › `indicator` | Apparence | |
| ui/src/tabs/Install.svelte:97,129,135 ; components/InstalledModels.svelte:37 ; components/HubSearch.svelte:41 | `?` de C27, C28, C30, C29, C31 | Aide › `start` | Moteurs et modèles | |
| ui/src/tabs/Modes.svelte:186,214,235,274,291 | `?` de C32, C34, C35, C36, C37 | Aide › `modes` | Style d'écriture | |
| ui/src/tabs/Bench.svelte:141,221 ; components/BenchArchive.svelte:56,75 ; components/TimingsCard.svelte:19 ; components/TrialBench.svelte:95 ; tabs/Journal.svelte:51 | `?` de C39, C40, C41, C42, C43, C44, C46 | Aide › `measures` | Mesures et essais | |
| ui/src/tabs/Journal.svelte:45 | `?` de C45 | Aide › `problems` | Mesures et essais | |
| ui/src/tabs/System.svelte:167,182,221 | `?` de C47, C48, C49 | Aide › `start` | Données et système | texte : « Premiers pas » contient démarrage, sauvegarde et désinstallation (`Help.svelte:126-134`). |
| ui/src/tabs/Update.svelte:69,119 | `?` de C50, C51 | Aide › `problems` | Données et système | |

Comptage par rubrique (relevé par recherche) : `start` 13, `measures` 11, `dictionary` 7, `modes` 6, `privacy` 4, `problems` 3, `dictate` 2, `indicator` 1, soit 47.

#### C. Boutons « → » de l'Aide (`Help.svelte`) et miroir anglais (`HelpEn.svelte`, mêmes liens à la ligne française + 12)

La définition du bouton est `Help.svelte:77-79` (`nav.go(tab, anchor)`) et `HelpEn.svelte:89-91`.

| Fichier:ligne | Ce qui vise | Cible actuelle (page/onglet/carte/ancre) | Cible après refonte | Risque |
|---|---|---|---|---|
| Help.svelte:117 / HelpEn.svelte:129 | « Installer les programmes et modèles » : `go('install')` | Moteurs, cartes d'installation (id `install` → onglet `engines`, sans ancre) | Moteurs et modèles | cassé. |
| Help.svelte:118 / HelpEn.svelte:130 | « Les moteurs » : `go('settings','moteurs')` | Moteurs, carte `moteurs` | Moteurs et modèles | cassé. |
| Help.svelte:119 / HelpEn.svelte:131 | « La touche et le micro » : `go('settings','dictee')` | Dictée, carte `dictee` | Dicter | cassé. |
| Help.svelte:120 / HelpEn.svelte:132 | « Tableau de bord » : `go('dashboard')` | Tableau de bord | Accueil | cassé ; libellé à renommer. |
| Help.svelte:144 / HelpEn.svelte:156 | « Gérer les utilisateurs » : `go('settings','utilisateurs')` | Dictée, carte `utilisateurs` | Profil | cassé. |
| Help.svelte:184-186 / HelpEn.svelte:196-198 | « Choisir les moteurs » (`settings`,`moteurs`) ; « Installer un modèle » (`install`) ; « Comparer avec le banc de dictée » (`bench`) | Moteurs ; Moteurs ; Mesures et essais | Moteurs et modèles ; Moteurs et modèles ; Mesures et essais | cassé (3). |
| Help.svelte:217 / HelpEn.svelte:229 | « Réglages de la dictée » : `go('settings','dictee')` | Dictée, carte `dictee` | Dicter | cassé. |
| Help.svelte:257 / HelpEn.svelte:269 | « Les modes » : `go('modes')` | Modes (id `modes` → onglet `modes`) | Style d'écriture | cassé. |
| Help.svelte:293 / HelpEn.svelte:305 | « Ouvrir le dictionnaire » : `go('dictionary')` | Dictionnaire | Dictionnaire (ou Commandes vocales pour la 2e moitié de la rubrique) | À décider avec la scission de la rubrique. |
| Help.svelte:313 / HelpEn.svelte:325 | « Régler l'indicateur » : `go('settings','indicateur')` | Apparence, carte `indicateur` | Apparence | cassé. |
| Help.svelte:327 / HelpEn.svelte:339 | « Mesures et essais » (`bench`) ; « Rapport et journal » (`journal`) | Mesures et essais (2 liens vers le même onglet) | Mesures et essais | cassé ; « Rapport et journal » est un ancien nom d'onglet. |
| Help.svelte:343 / HelpEn.svelte:355 | « Les moteurs » (`settings`,`moteurs`) ; « Confidentialité de l'historique » (`history`,`conservation`) | Moteurs ; Historique, carte `conservation` | Moteurs et modèles ; Historique (T1) | cassé ; dépend de T1. |
| Help.svelte:361 / HelpEn.svelte:373 | « Installation » (`install`) ; « Rapport et journal » (`journal`) ; « Mise à jour » (`update`) | Moteurs ; Mesures et essais ; Mise à jour | Moteurs et modèles ; Mesures et essais ; Données et système | cassé (3). |
| Help.svelte:124 / HelpEn.svelte:136 | « Relancer l'assistant de premier lancement » : `nav.openAssistant()` | Tableau de bord, assistant | Accueil | Pas cassé tant que `openAssistant` existe. |

#### D. Textes d'aide qui nomment un emplacement (Aide française, `ui/src/tabs/Help.svelte`)

L'anglais `HelpEn.svelte` dit la même chose, aux lignes équivalentes : 35, 42, 110, 124-125, 134-136, 139, 142, 145, 150-151, 155, 159, 171, 194, 207, 210, 212, 225, 243, 250, 253, 260, 263, 268, 286, 292, 294, 297-300, 302, 304, 323, 330-331, 333, 336-338, 351-352, 359-366, 370, 372.

| Fichier:ligne | Ce qui vise | Cible actuelle (page/onglet/carte/ancre) | Cible après refonte | Risque |
|---|---|---|---|---|
| Help.svelte:18,40 | « Réglages → Dictée » (commentaire ; texte affiché « à choisir dans Réglages → Dictée » pour la touche de commande) | Dictée | Commandes vocales (touche de commande) | texte (`:40` s'affiche : « aucune n'est réglée »). |
| Help.svelte:112 | « Les cartes d'installation (Réglages → Moteurs) » | Moteurs | Moteurs et modèles | texte. |
| Help.svelte:113,124,321 | « l'assistant… s'ouvre sur le Tableau de bord » ; « L'assistant du Tableau de bord » | Tableau de bord | Accueil | texte. |
| Help.svelte:122 | « La fenêtre a cinq pages : Tableau de bord, Historique, Dictionnaire, Réglages et Aide. Les Réglages ont des onglets : Dictée, Moteurs, Apparence, Modes, Mesures et essais, Système, et Mise à jour… Réglages → Apparence… Réglages → Dictée, « Prénom dans la salutation » » | Description de toute l'organisation | À réécrire entièrement (11 entrées en 4 groupes) | texte : paragraphe central à réécrire. |
| Help.svelte:123,318,347,348,350 | « banc de dictée / Banc d'essai / journal (Réglages → Mesures et essais) » | Mesures et essais | Mesures et essais | texte (le journal « en bas de l'onglet », `:348,350`). |
| Help.svelte:127 | « Réglages → Système, carte « Lancer au démarrage » » | Système | Données et système | texte. |
| Help.svelte:130,143 | « Réglages → Système, carte « Sauvegarde de vos données » » ; « La sauvegarde complète (Réglages → Système) » | Système | Données et système | texte. |
| Help.svelte:133 | « Réglages → Système, carte « Désinstaller Scriibe » » | Système | Données et système | texte. |
| Help.svelte:138,139,147,159 | « Réglages → Dictée, carte « Utilisateurs » » ; « sous-menu « Utilisateur » de l'icône » | Dictée ; menu de l'icône | Profil ; menu de l'icône (inchangé) | texte. |
| Help.svelte:182,354 | « Réglages → Moteurs » (relancer l'assistant ; Carte graphique) | Moteurs | Moteurs et modèles | texte. |
| Help.svelte:195,198,200,213,238,319,349,351,358 | « Réglages → Dictée » (Traduire vers, Langue dictée, Changer…, pause musique, genre, Tester ma touche, Micro) | Dictée | Dicter (genre → Profil) | texte. |
| Help.svelte:231,241,248,251,256 | « Réglages → Modes » (fidélité, Mes modes, Exporter, Importer, case menu de l'icône) | Modes | Style d'écriture | texte. |
| Help.svelte:274 | « onglet Dictionnaire → carte « Exporter et importer le dictionnaire » » | Dictionnaire, C10 | Dictionnaire (T2) | texte si T2 change. |
| Help.svelte:280 | « Pour changer la touche : Réglages → Dictée, « Touche tenue en plus pour dicter une commande » » | Dictée | Commandes vocales | texte. |
| Help.svelte:282 | « « ajouter au dictionnaire » ouvre cet onglet… » | Dictionnaire | Dictionnaire (la phrase se règle maintenant dans Commandes vocales) | texte : « cet onglet » devient ambigu. |
| Help.svelte:285-288,292 | « carte « Commandes vocales » (onglet Dictionnaire) », « partie « Mes extraits » », « partie « Mes applications » » | Dictionnaire, C14 | Commandes vocales (page) | texte (6 mentions). |
| Help.svelte:290 | « Reformuler la sélection… case « Autoriser l'envoi… » dans Réglages → Moteurs » | Moteurs | Commandes vocales | texte. |
| Help.svelte:311 | « Réglages → Apparence » | Apparence | Apparence | OK si le nom est gardé. |
| Help.svelte:324-325 | « temps gagné du Tableau de bord » ; « Dans le Tableau de bord, « Ma vitesse au clavier » » | Tableau de bord, C05 | Accueil ; Mesures et essais | texte (le second devient faux après le déplacement de C05). |
| Help.svelte:326 | « Réglages → Mesures et essais → « Rapport à me coller » » | Mesures et essais | Mesures et essais | OK. |
| Help.svelte:339,340 | « (Réglages → Emplacements) » ; « Historique, carte « Confidentialité de l'historique » » | Moteurs (carte Emplacements) ; Historique | Données et système ; Historique (T1) | texte. « Réglages → Emplacements » est déjà inexact (c'est une carte de l'onglet Moteurs). |
| Help.svelte:353,360 | « Réglages → Moteurs (cartes d'installation) » ; « Réglages → Mise à jour » | Moteurs ; Mise à jour | Moteurs et modèles ; Données et système | texte. |
| Help.svelte:269,336,357 | « Mes mots » (h3, et deux renvois) | Carte « Mots à écrire exactement » (C08) | Dictionnaire | texte : nom déjà périmé. |

#### E. Textes de l'interface (hors Aide) qui nomment un emplacement

| Fichier:ligne | Ce qui vise | Cible actuelle (page/onglet/carte/ancre) | Cible après refonte | Risque |
|---|---|---|---|---|
| ui/src/components/Assistant.svelte:75 | « enregistrer votre clé OpenRouter (onglet Réglages) » | Réglages | Profil | texte. |
| ui/src/components/Assistant.svelte:78-79 | Libellés « Aller à l'installation », « Aller aux Réglages » | Voir A | Moteurs et modèles ; Profil | texte + cassé. |
| ui/src/tabs/Settings.svelte:185 | « (voir l'onglet Modes) » | Modes | Style d'écriture | texte. |
| ui/src/tabs/Settings.svelte:341 | « (onglet Modes) … se choisit dans l'onglet Moteurs » | Modes ; Moteurs | Style d'écriture ; Moteurs et modèles | texte. |
| ui/src/tabs/Settings.svelte:282 | « Le Tableau de bord vous salue par ce prénom » | Tableau de bord | Accueil | texte. |
| ui/src/tabs/Settings.svelte:120 | Logo « en haut de la barre latérale » | Barre latérale | Barre latérale | OK. |
| ui/src/tabs/Settings.svelte:399 | « Réglages : {chemin} » (étiquette d'un chemin de fichier) | Carte Emplacements | Données et système | texte : « Réglages » veut dire « fichier de réglages ». |
| ui/src/components/VoiceCommands.svelte:469 | « (Réglages, onglet Apparence) » | Apparence | Apparence | texte. |
| ui/src/components/HubSearch.svelte:41 | « le modèle se choisit dans Réglages → Moteurs » | Moteurs | Moteurs et modèles | texte. |
| ui/src/components/InstalledModels.svelte:22 | « il faudra en choisir un autre (Réglages) » | Réglages | Moteurs et modèles | texte. |
| ui/src/tabs/Install.svelte:86-89,135 | « à installer ci-dessous » ; « dans la carte suivante » | Position relative dans l'onglet | À revoir si l'ordre des cartes change | texte. |
| ui/src/tabs/Bench.svelte:249 | « (onglet « Journal et rapport ») » | Nom d'onglet qui n'existe plus | Mesures et essais | texte (déjà périmé). |
| ui/src/components/BenchArchive.svelte:70 | « lisez quelques phrases ci-dessus » | Carte C39 | Idem | OK si l'ordre est gardé. |
| ui/src/tabs/History.svelte:107,145 | « les chiffres du tableau de bord » ; « Le tableau de bord reste juste » | Tableau de bord | Accueil | texte. |
| ui/src/tabs/Update.svelte:124 | « le bouton « Nouvelle version disponible » apparaît dans la barre latérale, et une ligne en haut du menu de l'icône » | Barre latérale ; menu de l'icône | Point sur « Données et système » ; menu de l'icône | texte : à réécrire (le bouton devient un point). |
| ui/src/tabs/Update.svelte:130-131 | « (en haut de cette page) » ; « Si la page écrit… » | Page Mise à jour | Données et système | texte. |
| ui/src/tabs/Modes.svelte:179,194,221 ; ui/src/components/Users.svelte:92 | « menu de l'icône » | Menu de l'icône (Rust) | Inchangé | OK. |
| ui/src/tabs/Dictionary.svelte:321-322 | « Hauteur des deux cartes » | Cartes C08 et C09 | Idem | OK si C08 et C09 restent côte à côte. |
| ui/src/App.svelte:102-104 | « l'icône de la barre des menus / la liste des applications / la zone de notification » | Icône du système | Inchangé | OK. |
| ui/src/lib/assistant.ts:30,44 | Commentaires « page Installation » | — | — | commentaire seulement. |

#### F. Traductions anglaises qui nomment un emplacement (`ui/src/lib/locales/en.ts`, `en-server.ts`)

En français, la clé est le texte français lui-même (en-tête de `en.ts` et de `i18n.ts`) : changer un texte français impose de changer la clé dans `en.ts`.

| Fichier:ligne | Ce qui vise | Cible actuelle (page/onglet/carte/ancre) | Cible après refonte | Risque |
|---|---|---|---|---|
| ui/src/lib/locales/en.ts:24-25,54,57-59 | Clés `Apparence`, `Mesures et essais`, `Tableau de bord`, `Dictionnaire`, `Réglages`, `Mise à jour` (noms de pages et d'onglets) | Libellés des pages | À renommer (Accueil, Dicter, Style d'écriture…) | test : clés orphelines (`i18n.test.ts:80-81`) et textes sans traduction (`:79`). |
| ui/src/lib/locales/en.ts:52 | « …(onglet Modes). Le moteur… se choisit dans l'onglet Moteurs. » / « (Modes tab) » | Modes ; Moteurs | Style d'écriture ; Moteurs et modèles | texte + test (clé identique à `Settings.svelte:341`). |
| ui/src/lib/locales/en.ts:202-203 | « (Réglages, onglet Apparence) » / « (Settings, Appearance tab) » | Apparence | Apparence | texte. |
| ui/src/lib/locales/en.ts:397-398 | « dans la carte suivante » / « in the next card » | Position relative | Idem | texte. |
| ui/src/lib/locales/en.ts:453-454 | « (Réglages) » / « (Settings) » | Réglages | Moteurs et modèles | texte. |
| ui/src/lib/locales/en.ts:464-465 | « Réglages → Moteurs » (Hugging Face) | Moteurs | Moteurs et modèles | texte. |
| ui/src/lib/locales/en.ts:539 | « Le Tableau de bord vous salue… » / « The Dashboard greets you… » | Tableau de bord | Accueil | texte. |
| ui/src/lib/locales/en.ts:661 | « Réglages : {path} » / « Settings: {path} » | Carte Emplacements | Données et système | texte. |
| ui/src/lib/locales/en.ts:683,685 | « (onglet Réglages) », « Aller aux Réglages » / « Go to Settings » | Réglages | Profil | texte. |
| ui/src/lib/locales/en.ts:768 | « (onglet « Journal et rapport »)… » | Onglet inexistant | Mesures et essais | texte. |
| ui/src/lib/locales/en.ts:907 | « (voir l'onglet Modes) » | Modes | Style d'écriture | texte + test. |
| ui/src/lib/locales/en.ts:101-102 | « Relancez-le depuis l'icône de la zone de notification / de la barre des menus » | Icône du système | Inchangé | OK. |
| ui/src/lib/locales/en.ts:658,660,704 (et 411) | « Réglages Système → … » (macOS), « onglet « Applications de démarrage » » (Windows) | Réglages du système d'exploitation | Inchangé | « Réglages Système » (macOS) ressemble au futur « Données et système ». |
| ui/src/lib/locales/en-server.ts:263 | « Reformulation impossible : moteur hors de cet ordinateur, sans accord (Réglages → Moteurs) » / « (Settings → Engines) » | Case de consentement dans Moteurs | Commandes vocales | texte + test : message identique dans `crates/pipeline/src/controller.rs:875` et son test `crates/pipeline/tests/dictation.rs:2113`. |
| ui/src/lib/locales/en-server.ts:120-124 | « llama-server (Windows, …) », « whisper-server (Windows) » | Libellés de programmes à télécharger | Voir Textes (b) | |

#### G. Côté Rust : icône de la barre des menus / de la zone de notification, ouverture de l'interface, messages

L'interface est servie par un petit serveur local sous une adresse secrète `http://127.0.0.1:<port>/<jeton>/` (`crates/cli/src/web/mod.rs:220`). Aucun élément Rust n'ouvre une page, un onglet ou une carte précise : ils ouvrent tous l'adresse de base. Aucun paramètre d'URL (`?…`) ni chemin de page n'est utilisé. Seul le fragment `#overlay` est une adresse spéciale (fenêtre de la pastille).

| Fichier:ligne | Ce qui vise | Cible actuelle (page/onglet/carte/ancre) | Cible après refonte | Risque |
|---|---|---|---|---|
| crates/platform/src/tray_menu.rs:85-95 | `MenuLabels` : `open_page`, `copy_last`, `mode`, `user`, `open_data`, `open_log`, `quit` | Menu de l'icône | Inchangé | Les mots « Mode » et « Utilisateur » du menu doivent rester d'accord avec les noms de pages (« Style d'écriture » pour le mode, « Profil » pour l'utilisateur) : vocabulaire à décider. |
| crates/platform/src/tray_menu.rs:119-120 | Entrée `update` « Nouvelle version disponible (x) » en tête du menu, « qui ouvre la fenêtre » | Ouvre la fenêtre sur la page retenue | À décider : ouvrir directement Données et système ? | Aujourd'hui aucun aiguillage : la fenêtre s'ouvre sur la dernière page vue (`sessionStorage` `tab`). |
| crates/cli/src/tray_source.rs:19,24-30 | `update_text` : « Nouvelle version disponible ({version}) » / « New version available » | Menu de l'icône | Inchangé | test : `tray_source.rs:310-313`. |
| crates/cli/src/tray_source.rs:106,131 | Libellés « Open Scriibe » / « Ouvrir Scriibe », « Ouvrir le dossier des données », « Ouvrir le journal » | Menu de l'icône | Inchangé | test : `tray_source.rs:353` (« Open Scriibe », « Quit Scriibe »). « Ouvrir le journal » ouvre le fichier du journal, pas une page (`mac_tray.rs:162`). |
| crates/platform/src/windows/tray.rs:112-121,248-258,313-318 | Windows : entrées « Ouvrir Scriibe » et « mise à jour » ; clic → `open(page_url)` ; l'entrée mise à jour est insérée en position 0 | Adresse de base | Inchangé | Les deux entrées ouvrent la même adresse. |
| crates/cli/src/mac_tray.rs:22-26,151-155,274-285,325-326 | macOS : ids `tray:open`, `tray:update`, `tray:data`, `tray:log` ; `OPEN_PAGE \| UPDATE` ouvrent `page_url` | Adresse de base | Inchangé | Idem. |
| crates/cli/src/commands.rs:142,234-236,271-272,353 | `page_url` (rempli au démarrage), option `--navigateur`, repli navigateur | Adresse de base | Inchangé | |
| crates/cli/src/web/mod.rs:219-221 | `api/instance/show` : une 2e Scriibe lancée ouvre l'adresse de base | Adresse de base | Inchangé | |
| crates/cli/src/prefill.rs:48-51 ; crates/pipeline/src/controller.rs:37-40 | Commande vocale « ajouter au dictionnaire » : met le texte en mémoire puis ouvre l'adresse de base ; c'est la page qui, en lisant `prefill`, appelle `go('dictionary', 'dictionary-corrections-entendu-ecrit')` | Dictionnaire, carte C09 | Dictionnaire | cassé si l'id de C09 change (lien côté page, `status.svelte.ts:35`). Commentaire `controller.rs:39` : « ouvre Scriibe sur l'onglet Dictionnaire ». |
| crates/cli/src/window.rs:128 ; ui/src/main.ts:10 | `#overlay` | Fenêtre de la pastille | Inchangé | Ne pas réutiliser. |
| crates/pipeline/src/controller.rs:875 | Message affiché sur la pastille « …sans accord (Réglages → Moteurs) » | Case de consentement dans Moteurs | Commandes vocales | texte + test : `crates/pipeline/tests/dictation.rs:2113`, traduction `en-server.ts:263`. |
| crates/engines/src/services.rs:121 ; whisper.rs:220 ; llama.rs:287 | « Clé Deepgram manquante — voir Réglages » ; « Modèle Whisper / llama.cpp introuvable — voir Réglages » | Réglages (sans précision) | Profil (clé) ; Moteurs et modèles (modèle) | texte + test : `engines/tests/services.rs:167`, `engines/tests/server.rs:100,101,291`, `pipeline/tests/dictation.rs:536`. Aucune de ces phrases n'est dans `en-server.ts` (elles restent en français dans l'interface anglaise : à vérifier à l'écran). |
| crates/engines/src/services.rs:245,329 ; crates/cli/src/web/trial.rs:42 | « Aucun moteur de mise au propre : choisis-en un dans Réglages → Moteurs » ; « Choisis un modèle {x} dans Réglages » | Moteurs | Moteurs et modèles | texte + test : `engines/tests/services.rs:475`. Non traduits en anglais (à vérifier à l'écran). |
| crates/pipeline/src/controller.rs:1016 | « Raccourci invalide dans les réglages (exemple : Mod+B) » | Commandes personnalisées | Commandes vocales | texte ; traduit (`en-server.ts:178`). |
| Commentaires Rust : core/src/modes.rs:300, core/src/settings.rs:300,306, core/src/rewrite.rs:48, core/src/microphone.rs:39, core/tests/hidden_modes.rs:1, platform/src/tray_menu.rs:119 | « Réglages → Les modes », « case des Réglages → Moteurs », « Nom affiché dans Réglages » | Anciens emplacements | — | commentaires seulement (non affichés). |

#### H. Tests et outils qui vérifient ou parcourent ces emplacements

| Fichier:ligne | Ce qui vise | Cible actuelle (page/onglet/carte/ancre) | Cible après refonte | Risque |
|---|---|---|---|---|
| ui/src/lib/tabs.test.ts:8-13 | 8 rubriques d'Aide, titres distincts, Aide dernière page, « Premiers pas » par défaut | `help.ts`, `tabs.ts` | `tabs.at(-1).id` doit rester `help` | test : échoue si l'Aide n'est plus la dernière entrée de `tabs`. |
| ui/src/lib/tabs.test.ts:15-18 | Ids et libellés de `tabs` uniques | `tabs` | 11 entrées | test : si un titre de groupe est ajouté dans `tabs`, il faut des libellés distincts. |
| ui/src/lib/tabs.test.ts:21-40 | `Help.svelte` et `HelpEn.svelte` lus tels quels : sommaire fixe (`.toc` sticky…), `helpSection = …; window.scrollTo(0, 0)` | Fichiers d'aide | Inchangé | test : ne pas modifier ces blocs CSS/JS. |
| ui/src/lib/i18n.test.ts:9-17 | Lit tous les `.svelte`/`.ts` et, pour `tabs.ts`, les chaînes `label:`/`description:` | `tabs.ts` | Nouveau fichier de pages | test : la forme `label: '…'` doit être gardée pour que les libellés soient vus. |
| ui/src/lib/i18n.test.ts:60-76 | `Help.svelte` et `HelpEn.svelte` doivent avoir la même structure : 7 rubriques (`{#if open === …}`) + dernière `{:else}`, mêmes nombres de `<h2>`, `<h3>`, `{@render go(`, exemples, astuces, questions, `<li>` | Aide | Idem | test : toute réécriture de l'Aide doit être faite à l'identique dans les deux langues. |
| ui/src/lib/i18n.test.ts:78-82 | Aucun texte de page sans traduction, aucune traduction orpheline | `en.ts`, `en-server.ts` | Idem | test : chaque libellé renommé (page, onglet, carte) demande le changement de la clé dans `en.ts`. |
| ui/src/lib/logic.test.ts:23-26 | Le cas `ui_logic-013` est « non applicable » avec la raison « Réglages en cinq rubriques… » | `fixtures/ui_logic.json:161-166`, `docs/ECARTS.md:45`, `fixtures/A_FAIRE.md` | À supprimer ou à réécrire | test : la raison doit rester cohérente avec la fixture (le test relit les ids des invariants). |
| tools/captures_interface.js:30-47 | Clique `nav button:has-text(libellé)` ; cas spécial `label === 'Réglages'` puis `.pills button` | Barre latérale et pilules | À adapter (groupes, plus de pilules) | cassé (outil de capture d'écran, non lancé par les tests). Ignore les boutons qui commencent par « Quitter ». |
| crates/cli/src/tray_source.rs:230-374 (dont 310-313, 353) | Tests du menu de l'icône (libellés, texte « mise à jour ») | Menu de l'icône | Inchangé | test : seulement si un libellé change. |
| crates/pipeline/tests/dictation.rs:536,2113 ; crates/engines/tests/services.rs:167,475 ; crates/engines/tests/server.rs:100,101,291 | Messages exacts « voir Réglages », « (Réglages → Moteurs) » | Voir G | Idem | test : tout changement de message demande de changer le test. |
| Aucun test Playwright | — | — | — | Le dépôt n'a qu'un outil, `tools/captures_interface.js` ; pas de test de bout en bout sur les liens. |
| README.md (1 ligne), README.en.md (1), docs/ (INSTALLATION_* de 3 à 7 lignes chacun, MISE_A_JOUR 6, COMMANDES 8, PROFILS 8, GESTION_MODES 2, LOGO_ANIME 3), docs/ECARTS.md:45 | Documentation qui dit « Réglages → … », « onglet … » | Anciens emplacements | À mettre à jour | texte (hors interface ; à faire à part). |

---

### Textes

#### (a) « vous / votre / vos » dans l'interface

Aujourd'hui l'interface mélange le « tu » (majorité : Aide, Banc, Dictionnaire, Utilisateurs, messages Rust) et le « vous ». Relevé par recherche des mots `vous`, `votre`, `vos` (et `vôtre`) hors commentaires, puis des impératifs au vous (`-ez`). Entre crochets : nombre d'occurrences sur la ligne.

**Fichiers .svelte (hors Aide)**

| Fichier:ligne | Extrait court |
|---|---|
| ui/src/App.svelte:79 [1] | « Scriibe est arrêté. Vous pouvez fermer cette page. » |
| ui/src/tabs/History.svelte:145 [2] | « Pour que ce que vous dictez ne reste pas sur le disque… Vous pouvez ne garder que les statistiques… » |
| ui/src/tabs/Install.svelte:129 [1] | « Rien n'est téléchargé sans votre accord… » |
| ui/src/tabs/Modes.svelte:214 [1] | « …créez votre mode avec deux ou trois exemples… » |
| ui/src/tabs/Settings.svelte:125 [1] | « …ne change ni la langue que vous dictez ni… » |
| ui/src/tabs/Settings.svelte:137 [2] | « Où passent votre voix et votre texte… » |
| ui/src/tabs/Settings.svelte:138 [1] | « Ce qui tourne quand vous dictez : » |
| ui/src/tabs/Settings.svelte:145 [2] | « …transforme votre voix en texte… (votre voix reste sur cet ordinateur) » |
| ui/src/tabs/Settings.svelte:263 [5] | « Comment vous dictez : vous maintenez la touche, vous parlez, vous relâchez… » |
| ui/src/tabs/Settings.svelte:265 [1] | « Sous GNOME (Wayland), vous dictez avec un raccourci de GNOME… relancez Scriibe » |
| ui/src/tabs/Settings.svelte:266 [1] | « …relancez Scriibe, GNOME vous en fera choisir un. » |
| ui/src/tabs/Settings.svelte:267 [1] | Idem pour le raccourci de commande |
| ui/src/tabs/Settings.svelte:282 [1] | « Le Tableau de bord vous salue par ce prénom. » |
| ui/src/tabs/Settings.svelte:292 [1] | « …accorder ce qui vous concerne… » |
| ui/src/tabs/Settings.svelte:383 [1] | « …une clé est votre identifiant chez Deepgram ou OpenRouter. » |
| ui/src/tabs/Settings.svelte:120 | Impératif « Décochez pour le garder fixe » |
| ui/src/tabs/System.svelte:59 [3] | « Remplacer vos réglages, votre dictionnaire et votre historique… » |
| ui/src/tabs/System.svelte:138 [1] | « …tant que vous ne la videz pas. » |
| ui/src/tabs/System.svelte:167 [2] | « …à l'ouverture de votre session… Désactivé tant que vous ne le cochez pas. » |
| ui/src/tabs/System.svelte:178 [1] | « …de votre environnement de bureau. » |
| ui/src/tabs/System.svelte:182 [5] | Titre « Sauvegarde de vos données » + « avec vos réglages et modes, votre dictionnaire, votre historique et vos statistiques » |
| ui/src/tabs/System.svelte:183 [2] | « …les textes que vous avez dictés… où vous le rangez… » |
| ui/src/tabs/System.svelte:203-205 [1+1+1] | « vos réglages… », « votre dictionnaire… », « votre historique… » (liste de restauration) |
| ui/src/tabs/System.svelte:221 [1] | « …tant que vous ne la videz pas… » |
| ui/src/tabs/System.svelte:238 [2] | « …de votre profil de shell si vous les y aviez mises. » |
| ui/src/tabs/System.svelte:245 [1] | « Vos modèles téléchargés » |
| ui/src/tabs/System.svelte:176,238 | Impératifs « cochez la case… », « Retirez DEEPGRAM_API_KEY… » |
| ui/src/tabs/Update.svelte:92 [1] | « ✔ Vous avez déjà cette version. » |
| ui/src/tabs/Update.svelte:131 [1] | « Si la page écrit « Vous avez déjà cette version »… » |
| ui/src/tabs/Update.svelte:96,98 | Impératifs « réessayez plus tard », « Mettez à jour à la main… » |
| ui/src/tabs/Bench.svelte:143 | Impératifs « Lisez chaque phrase… », « cliquez sur « Terminer » » |
| ui/src/tabs/Journal.svelte:32,34,46,53 | Impératifs « collez-le », « sélectionnez », « Ouvrez le Bloc-notes, maintenez Ctrl droit, parlez, relâchez », « Faites quelques essais, puis cliquez » |
| ui/src/tabs/Dashboard.svelte:119 | Impératif « Recopiez le texte » |
| ui/src/components/Assistant.svelte:47 [1] | « Votre ordinateur : » |
| ui/src/components/Assistant.svelte:75 [1] | « enregistrer votre clé OpenRouter (onglet Réglages) » |
| ui/src/components/Assistant.svelte:83 | Impératif « maintenez Ctrl droit, parlez, relâchez » |
| ui/src/components/HubSearch.svelte:41 [1] | « …avant votre confirmation… » ; impératifs « tapez », « choisissez » (aussi `:69` « Cliquez », `:71` « essayez ») |
| ui/src/components/BenchArchive.svelte:70 | Impératif « lisez quelques phrases ci-dessus » |
| ui/src/components/TypingTest.svelte:60 | Impératif « Recopiez le texte ci-dessus » |
| ui/src/App.svelte:103 | Impératif « Relancez-le depuis l'icône… » (3 variantes) |

Hors liste (ce ne sont pas des adresses à l'utilisateur) : `ui/src/tabs/Modes.svelte:67,301` et `ui/src/lib/api.ts:311` (champ `vouvoie`, libellé « Vouvoie le destinataire ») ; `Dictionary.svelte:405` et `Help.svelte:192,263,265` (phrases dictées données en exemple : « ouvrez la parenthèse », « fermez »).

**Aide française (`ui/src/tabs/Help.svelte`)** : presque tout est au « tu ». Au « vous » : `:130` (titre de carte « Sauvegarde de vos données » cité), `:360` (« que la vôtre » dans « Comment mettre à jour ? »). `:221` (« Courrier vouvoie le destinataire ») et `:192` (exemple dicté) sont hors sujet. `ui/src/lib/help.ts` : aucun. `HelpEn.svelte` est en anglais : sans objet.

**Fichiers de traduction (la clé française est le texte lui-même)**
- `ui/src/lib/locales/en.ts` : 33 clés avec « vous / votre / vos », aux lignes 65, 123, 394, 423, 432, 464, 539, 544, 598, 613, 616, 628, 635, 636, 637, 645, 646, 647, 651, 655, 659, 668, 683, 900, 901, 902, 906, 908, 909, 911, 913, 916, 918 ; 28 clés avec un impératif au vous, aux lignes 15, 101, 102, 103, 117, 218, 261, 410, 413, 438, 439, 441, 447, 464, 476, 477, 613, 655, 657, 687, 733, 792, 845, 908, 911, 913, 916, 918.
- `ui/src/lib/locales/en-server.ts` : 3 clés, lignes 13, 145, 149 (« votre compte », « Votre voix et votre texte »).
- Rappel : changer le français impose de changer la clé (test d'orphelins, `i18n.test.ts:80-81`).

**Messages Rust affichés** : `crates/platform/src/uninstall.rs:417` (« n'est pas modifiable par votre compte : seules vos données partent »), `crates/cli/src/assistant.rs:40` (« votre voix et votre texte restent sur l'ordinateur ») et `:51` (« Votre voix et votre texte restent… »). Les autres messages Rust affichés sont au « tu ». Les textes de `crates/core/src/rewrite.rs`, `prompt_text.rs`, `text.rs` contiennent « vous/votre » mais ce sont des exemples ou des consignes données au modèle, pas de l'interface.

#### (b) Textes propres à Windows affichés sur Mac ou Linux

En une phrase : la plupart des textes de touches, de coffre à clés et de démarrage sont déjà conditionnés (`status.os`, `osName`, `mac`, `linux`, `cfg!`) ; restent 5 endroits de l'interface sans condition (Banc, Journal, Assistant, Install, KeyPicker) et 3 libellés Rust à vérifier sur Mac.

| Fichier:ligne | Extrait | Déjà adapté au système ? |
|---|---|---|
| ui/src/tabs/Bench.svelte:176 | « Traduire (comme Ctrl + Maj) » | Non : sans condition. Faux sur Mac (touche de dictée Option droite, traduction ⌘). |
| ui/src/tabs/Journal.svelte:46 | « Ouvrez le Bloc-notes, maintenez Ctrl droit, parlez, relâchez. Maj en plus : traduction. » | Non : sans condition (Windows). L'Aide, elle, adapte « Bloc-notes / TextEdit / un éditeur » (`Help.svelte:30`). |
| ui/src/components/Assistant.svelte:83 | « Tout est en place : maintenez Ctrl droit, parlez, relâchez. » | Non : sans condition. |
| ui/src/tabs/Install.svelte:91 | « Ctrl droit (Maj : traduire) » (ligne « Touche globale » du tableau d'état) | Non : sans condition. |
| ui/src/tabs/Dashboard.svelte:59,115 | Repli `'Ctrl droit'` quand le statut n'est pas encore chargé | Partiellement : la touche réelle vient de Rust (`dictation_label`) ; le repli est écrit en dur alors que `status.os` donne déjà le système. |
| ui/src/components/KeyPicker.svelte:158,179,186 | « Windows ne la voit pas… (Windows, Ctrl gauche…) », « Passe dans le Bloc-notes », « Les touches Windows, Alt gauche, Maj gauche, Ctrl… » | Oui pour Mac (branche `mac`) ; la branche « autre » sert aussi Linux, qui lit donc « Windows » et « Bloc-notes ». |
| ui/src/components/KeyPicker.svelte:169 | « raccourci clavier (Ctrl+C, Maj+lettre…) » | Non : sans condition (sur Mac c'est ⌘C). |
| ui/src/components/ShortcutField.svelte:24 | « Tiens ⌘ ou Ctrl avec une lettre ou un chiffre (⌘Q est refusé). » | Cite les deux systèmes sans condition (acceptable, non adapté). |
| ui/src/components/VoiceCommands.svelte:114,326 | « « Mod+B » (⌘B sur Mac, Ctrl+B ailleurs) » | Cite les deux sans condition. |
| ui/src/components/VoiceCommands.svelte:469,639 | « Sur Mac pour l'instant. » ; « par exemple ⌘⇧X sur Mac » | Sans condition : le premier dit que les commandes ne marchent que sur Mac (utile hors Mac, superflu sur Mac). |
| ui/src/tabs/Dictionary.svelte:341 ; Modes.svelte:275 ; VoiceCommands.svelte:678 | « (sous Linux, il est placé dans le dossier Documents) » | Sans condition, même affiché sur Mac. |
| ui/src/tabs/Settings.svelte:384 | Coffre des clés : trousseau (Mac) / variables d'environnement (Linux) / « Gestionnaire d'identifiants de Windows » | Oui (`osName`). |
| ui/src/tabs/Settings.svelte:350 | Intro de la pastille : barre des menus (Mac) / pas d'icône (Linux) / « près de l'horloge » (Windows) | Oui (`osName`). |
| ui/src/tabs/Settings.svelte:264-267 | Textes GNOME (Wayland) | Oui : affichés seulement si Rust renvoie un raccourci GNOME. |
| ui/src/tabs/System.svelte:178 | « Gestionnaire des tâches, onglet « Applications de démarrage » » (Windows) ; autres textes Mac et Linux | Oui (`osName`). |
| ui/src/tabs/System.svelte:238 | « Gestionnaire d'identifiants » / trousseau / Linux | Oui (`plan.keys` fourni par Rust). |
| ui/src/App.svelte:103 | « barre des menus » / « liste des applications » / « zone de notification » | Oui. |
| ui/src/tabs/Help.svelte:23-36,122,127,133,200-206,290,341,347,360 | Touches par défaut, Bloc-notes / TextEdit, Gestionnaire des tâches, coffre, `scriibe.exe`… | Oui : conditionnés (`mac`, `linux`) ; `HelpEn.svelte` identique. |
| ui/src/tabs/Help.svelte:287-288 ; HelpEn.svelte:299-300 | « historique du presse-papiers de Windows (Win+V) et… gestionnaires du Mac » ; « Sous Windows, ce sont les raccourcis du menu Démarrer… » | Citent les systèmes sans condition (chaque phrase dit de quel système il s'agit). |
| ui/src/lib/report.ts:9 | Repli `'Maj'` pour la touche de traduction dans le rapport | Repli sans condition (lu seulement si l'état manque). |
| crates/cli/src/web/downloads.rs:25,50,56 (affichés dans C28 et C30) | « whisper-server (Windows) », « llama-server (Windows, processeur) », « llama-server (Windows, carte graphique Vulkan) » | À vérifier sur Mac : `offered()` (`:85-91`) retire ces trois entrées seulement sous Linux ; sur Mac elles s'afficheraient donc avec « (Windows) » (code relu, pas vu à l'écran). Traductions `en-server.ts:120-124`. |
| crates/cli/src/web/downloads.rs:204 | « {x} : proposé seulement sous Windows pour l'instant » (erreur au clic) | Message honnête, mais il montre que ces programmes n'ont pas de téléchargement sur Mac (même point). |
| ui/src/lib/assistant.ts:36 | « llama-server (Vulkan) » (nom d'élément à installer dans C01) | Libellé Windows ; sur Mac il peut apparaître dans la chaîne « gpu » (`crates/cli/src/assistant.rs:25-35` ; adaptation faite seulement pour Linux, `:97-110`) : à vérifier sur Mac. |
| crates/pipeline/src/controller.rs:28 (messages `:900,915,1043,1080,1322`) | « à coller avec Ctrl+V » / « ⌘V » | Oui : constante `PASTE_SHORTCUT` selon le système (compilation). Traduit dans `en-server.ts:191,269,273,279`. |
| crates/cli/src/tray_source.rs:32-34 ; crates/platform/src/keys.rs:98-104,150-153 | Libellés de touches par système, infobulle « maintiens {touche} pour dicter » | Oui : tables séparées Windows / Mac / Linux. |
| crates/cli/src/commands.rs:150-152 | Infobulle Windows « maintiens Ctrl droit pour dicter (+ Maj : traduire) » | Oui : dans un bloc `#[cfg(windows)]`. |
| crates/platform/src/uninstall.rs:604,670 | « Gestionnaire d'identifiants › Informations d'identification Windows », « Windows le supprimerait définitivement » | Oui : par système (`Os::Windows`) ; le second sous `cfg(windows)`. |
| crates/platform/src/lib.rs:54-64 | Nom du coffre renvoyé : « Gestionnaire d'identifiants » (Windows) / « trousseau » (Mac) | Oui (par système). |
| Chemins `C:\…` dans l'interface | — | Aucun dans les textes affichés : les chemins affichés (Emplacements, désinstallation, sauvegarde) sont ceux du système ; les exemples de champ sont de style Unix (`/home/…`, `~/Documents/…` : System.svelte:195, Dictionary.svelte:360, Modes.svelte:238, VoiceCommands.svelte:686, UserImport.svelte:101), affichés seulement quand Rust demande de saisir un chemin (Linux). |

#### (c) Libellés en doublon (même nom pour un onglet et une carte, ou pour deux endroits)

Aujourd'hui :
- Onglet « Dictée » (`tabs.ts:14`) = carte « Dictée » (`Settings.svelte:263`) ; la page future s'appelle « Dicter », et la rubrique d'Aide « Dicter et traduire » (`help.ts:4`).
- Onglet « Moteurs » (`tabs.ts:15`) = carte « Moteurs » (`Settings.svelte:137`) ; la page future « Moteurs et modèles » contient les cartes « Modèles installés » et « Modèles recommandés ».
- Onglet « Apparence » (`tabs.ts:16`) ≈ cartes « Apparence de la fenêtre » (`Settings.svelte:110`) et « Apparence de l'indicateur » (`:350`) ; menu « Apparence : » (`:111`).
- Onglet « Modes » (`tabs.ts:17`) ≈ cartes « Modes de base », « Modes professionnels », « Mes modes » ; futur « Style d'écriture ».
- Page « Dictionnaire » (`tabs.ts:5`, titre `Dictionary.svelte:265`) ≈ carte « Exporter et importer le dictionnaire » ; la carte « Commandes vocales » est aujourd'hui dans le Dictionnaire.
- Carte « Commandes vocales » (`VoiceCommands.svelte:467`) = futur titre de la page « Commandes vocales » (doublon exact après refonte).
- Onglet « Mesures et essais » (`tabs.ts:18`) ≈ rubrique d'Aide « Mesurer et comparer » (`help.ts:7`).
- Onglet « Système » (`tabs.ts:19`) ≈ « Réglages Système » (macOS, cité `Help.svelte:33,347,349`, `KeyPicker.svelte:156`, `Update.svelte:136`) : confusion possible avec le futur « Données et système ».
- Onglet « Mise à jour » (`tabs.ts:20`) et ses cartes « Version de Scriibe » et « Comment ça marche ? ».
- Page « Réglages » (`tabs.ts:6`) ≈ mot « Réglages » dans « Réglages de personne » (`UserExport.svelte:93`, `UserImport.svelte:114`), « Réglages : {chemin} » (`Settings.svelte:399`), « Réglages appliqués » (`Assistant.svelte:70`) : le mot a plusieurs sens.
- « Journal » : ancien onglet « Journal et rapport » (cité `Bench.svelte:249`, `en.ts:768`), carte « Rapport à me coller », ligne « Journal : chemin », entrée de menu « Ouvrir le journal ».
- « Utilisateurs » (carte `Users.svelte:92`) / « Utilisateur » (sous-menu de l'icône, `tray_menu.rs:90`) / futur « Profil ».
- « Mes mots » (Aide `Help.svelte:269,336,357`) contre « Mots à écrire exactement » (carte `Dictionary.svelte:275`).
- « Installation » (nom de page cité par l'Aide `Help.svelte:185,361` et `assistant.ts:30`) contre « État de l'installation » (carte `Install.svelte:97`).
- Dans l'i18n, un même texte français sert d'onglet et de carte (par exemple « Moteurs », « Dictée ») : une seule clé de traduction ; si la carte et la page sont renommées différemment, il faut deux clés.

Aucun doublon d'`id` de carte ni de titre de carte entre deux `<Card>` (contrôlé par recherche).

---

### Difficultés et points à vérifier

- Découpage nécessaire : `Settings.svelte` (9 cartes, 3 sections), `Dictionary.svelte` (porte `VoiceCommands`), `KeyPicker.svelte` (3 touches en un), `Install.svelte` (état mélangeant moteurs et micro/touche).
- La carte VoiceCommands n'a pas de section « reformulation de la sélection » (T4).
- Les ids des cartes servent d'ancres ET de mémoire du pli : décider quels ids sont gardés.
- L'annonce « nouvelle version » existe à 3 endroits : bouton de la barre (`App.svelte:95-97`), point de pilule (`SettingsPage.svelte:25`), entrée de menu Rust ; seule l'entrée Rust n'a aucune cible de page.
- À vérifier sur Mac (code relu, rien lancé) : libellés « (Windows) » des programmes (`downloads.rs:25,50,56`) et « llama-server (Vulkan) » dans l'assistant.
- À vérifier à l'écran : les messages Rust « voir Réglages » ne figurent pas dans `en-server.ts` (ils restent en français dans l'interface anglaise).
- Aucun test Playwright ; trois tests Vitest sont à surveiller : `tabs.test.ts`, `i18n.test.ts` (Aide miroir, orphelins), `logic.test.ts` (ui_logic-013).

---

### Décomptes (Parties 2 et 3)

| Mesure | Nombre |
|---|---|
| Lignes de la partie « Liens » (A à H) | 146 (A navigation 31, B boutons « ? » 16 lignes regroupant 47 occurrences, C boutons « → » de l'Aide 14 lignes regroupant 19 liens + 1 bouton d'assistant, D textes d'aide 23, E textes de l'interface 20, F traductions 15, G Rust 15, H tests et outils 12) |
| Boutons « ? » (`help=`) | 47, répartis sur 8 rubriques d'Aide |
| Liens `nav.go` / `goto` / `ongoto` hors Aide, vers un emplacement précis | 7 (`status.svelte.ts:35`, `Dashboard.svelte:56,108`, `Assistant.svelte:78,79`, `Bench.svelte:148`, plus `nav.openAssistant` à `Settings.svelte:142`) |
| Tests Vitest à surveiller | 3 fichiers : `tabs.test.ts`, `i18n.test.ts`, `logic.test.ts` ; plus le cas `ui_logic-013` de `fixtures/ui_logic.json` |
| Tests Rust qui citent un emplacement dans un message | 4 fichiers : `pipeline/tests/dictation.rs`, `engines/tests/services.rs`, `engines/tests/server.rs`, `cli/src/tray_source.rs` (libellés de menu) |
| Lignes « vous / votre / vos » et impératifs au vous, fichiers .svelte hors Aide (tableau 3a) | 40 lignes de tableau |
| Clés de traduction avec « vous / votre / vos » | 33 dans `en.ts`, 3 dans `en-server.ts` (plus 28 clés avec impératif au vous dans `en.ts`) |
| Messages Rust affichés au « vous » | 3 (`uninstall.rs:417`, `assistant.rs:40`, `assistant.rs:51`) |
| Textes propres à Windows, tableau 3b | 29 lignes ; 5 sans condition de système dans l'interface ; 3 libellés Rust à vérifier sur Mac ; 13 déjà adaptés |
| Libellés en doublon (3c) | 15 groupes |


## Menu et pages (tâche 1)

Fait le 2026-11-17. Résultat : le menu de gauche a quatre groupes (**Utiliser** : Accueil, Historique · **Ma dictée** : Dicter, Style d'écriture, Dictionnaire, Commandes vocales · **Configurer** : Moteurs et modèles, Profil, Apparence, Données et système · **Avancé** : Mesures et essais), plus l'Aide en bas. La page Réglages, ses 7 onglets et `SettingsPage.svelte` n'existent plus.

### Ce qui a été fait

- `ui/src/lib/tabs.ts` : `tabGroups` (groupes, pages, icônes, phrase sous le titre), `helpTab`, `tabs` (liste à plat, Aide en dernier), `resolveTab`.
- Une page par fichier dans `ui/src/tabs/` : `Dictate`, `StylePage`, `Commands`, `EnginesPage`, `Profile`, `AppearancePage`, `Data`, `Measures` (les autres pages gardent leur fichier). Chacune pose ses cartes existantes, inchangées. `Settings.svelte` prend un paramètre `section` étendu (`dictation`, `style`, `engines`, `keys`, `places`, `appearance`) et ne montre que les cartes de la page qui l'appelle.
- Le point « nouvelle version » (●) est sur l'entrée Données et système ; le bouton « Nouvelle version disponible » de la barre mène à la carte « Version de Scriibe » de cette page.
- En-têtes fixes : nouveau composant `ui/src/components/PageHeader.svelte` (titre, phrase, boutons ; `position: sticky; top: 0`), utilisé par toutes les pages sauf l'Accueil. Il publie sa hauteur dans `--page-head`, que lisent le sommaire de l'Aide, le bloc de filtres de l'Historique et les ancres (`scroll-margin-top` dans `app.css`).
- Les anciens identifiants (`settings` + ancre, `modes`, `install`, `bench`, `journal`, `update`) mènent encore à la bonne page par `resolveTab` : les liens existants (boutons « ? », « → » de l'Aide, assistant) marchent sans être touchés. La tâche 5 les remplacera.
- Cartes déplacées : « Ma vitesse au clavier » de l'Accueil vers Mesures et essais ; « Commandes vocales » du Dictionnaire vers sa page ; utilisateurs et clés du cloud vers Profil ; « Ce que devient ton texte » et les modes vers Style d'écriture ; « Emplacements » vers Données et système ; « Mise à jour » dans Données et système.
- Textes : phrases de l'Aide (FR et EN) et des cartes qui décrivaient l'ancien emplacement (« Réglages → Dictée », « onglet Modes », « Tableau de bord »…) remises sur les nouvelles pages ; bouton de l'assistant « Aller aux Réglages » devenu « Aller au Profil » (il ouvre la carte des clés) ; traductions anglaises à jour, orphelines retirées.
- Tests : `tabs.test.ts` vérifie les 4 groupes et leur ordre, que chaque page est branchée dans `App.svelte`, la résolution des anciens identifiants, et l'en-tête fixe. `svelte-check` sans erreur, Vitest 113 tests verts, `npm run build` passe.

### Choix et écarts

- **Identifiant `dashboard` conservé** pour l'Accueil (seul le libellé change) : il est cité par l'assistant, l'Aide et la mémoire de session ; le renommer n'apporte rien.
- **L'Accueil n'a pas d'en-tête fixe** : son bandeau d'accueil (salutation, touche, onde) est grand ; le laisser collé en haut mangerait l'écran. À reprendre si l'auteur veut quand même un titre fixe.
- **Id de la carte « Ma vitesse au clavier » inchangé** (`dashboard-vitesse-clavier`) pour garder son état replié ; à renommer avec la tâche 5 si besoin.
- Cartes à découper laissées entières, comme la consigne le permet : Dictée (Dicter), Moteurs (Moteurs et modèles), Commandes vocales (une seule carte, découpée à la tâche 2).
- **Vérification à l'écran limitée** : la page construite a été ouverte dans le navigateur intégré, sans serveur Scriibe (pas d'API). Vu : les 11 pages et l'Aide s'ouvrent avec leur titre, les groupes du menu s'affichent, l'en-tête et le sommaire de l'Aide restent en haut au défilement. Pas vu : le contenu des cartes qui dépend de l'API, ni l'anglais, ni le mode sombre, ni la fenêtre étroite ; la tâche 5 les voit sur le Mac.
- Restent à reprendre ailleurs : les libellés des boutons « → » de l'Aide (« Réglages de la dictée »…, tâche 5), les messages du programme qui disent « Réglages » (tâche 9), les cartes qui déplacent des réglages (tâches 3 et 4).

### Cartes (cochées : chacune existe sur la page indiquée)

| Fait | N° | Carte | Page |
|---|---|---|---|
| ✔ | C01 | Assistant de premier lancement | Accueil |
| ✔ | C02 | Mots dictés par jour | Accueil |
| ✔ | C03 | Où je dicte | Accueil |
| ✔ | C04 | Dernières dictées | Accueil |
| ✔ | C05 | Ma vitesse au clavier (repliée par défaut) | Mesures et essais |
| ✔ | C06 | Confidentialité de l'historique | Historique |
| ✔ | C07 | Jour d'historique (une carte par jour, id `historique-<jour>`, sans `?`) | Historique |
| ✔ | C08 | Mots à écrire exactement | Dictionnaire |
| ✔ | C09 | Corrections « entendu → écrit » | Dictionnaire |
| ✔ | C10 | Exporter et importer le dictionnaire | Dictionnaire |
| ✔ | C11 | Exporter le dictionnaire (carte transitoire) | Dictionnaire |
| ✔ | C12 | Importer un dictionnaire (carte transitoire) | Dictionnaire |
| ✔ | C13 | Signes dictés | Dictionnaire |
| ✔ | C14 | Commandes vocales | Commandes vocales |
| ✔ | C15 | Utilisateurs | Profil |
| ✔ | C16 | Exporter un utilisateur (transitoire) | Profil |
| ✔ | C17 | Importer un utilisateur (transitoire) | Profil |
| ✔ | C18 | Dictée (13 réglages) | Dicter (carte entière : tâches 2 et 3 en retirent la touche de commande, le prénom et le genre) |
| ✔ | C19 | Ce que devient ton texte | Style d'écriture |
| ✔ | C20 | Moteurs | Moteurs et modèles (carte entière : la case « Autoriser l’envoi de la sélection » part à la tâche 2) |
| ✔ | C21 | Clés du cloud (dépannage) | Profil |
| ✔ | C22 | Emplacements | Données et système |
| ✔ | C23 | Apparence de la fenêtre | Apparence |
| ✔ | C24 | Logo animé | Apparence |
| ✔ | C25 | Langue de l'interface | Apparence |
| ✔ | C26 | Apparence de l'indicateur | Apparence |
| ✔ | C27 | État de l'installation | Moteurs et modèles |
| ✔ | C28 | Les programmes | Moteurs et modèles |
| ✔ | C29 | Modèles installés | Moteurs et modèles |
| ✔ | C30 | Modèles recommandés | Moteurs et modèles |
| ✔ | C31 | Ajouter un modèle depuis Hugging Face | Moteurs et modèles |
| ✔ | C32 | Modes de base / Modes professionnels (une `<Card>` en boucle, 2 occurrences à l'exécution) | Style d'écriture |
| ✔ | C34 | Mes modes | Style d'écriture |
| ✔ | C35 | Importer des modes (transitoire) | Style d'écriture |
| ✔ | C36 | Exporter le mode (transitoire) | Style d'écriture |
| ✔ | C37 | Nouveau mode / Modifier le mode (transitoire) | Style d'écriture |
| ✔ | C38 | Essai : {mode} (transitoire) | Style d'écriture |
| ✔ | C39 | Banc de dictée (17 phrases) | Mesures et essais |
| ✔ | C40 | Ce passage | Mesures et essais |
| ✔ | C41 | Passages gardés | Mesures et essais |
| ✔ | C42 | Comparer deux passages | Mesures et essais |
| ✔ | C43 | Temps des dictées | Mesures et essais |
| ✔ | C44 | Banc d'essai de la mise au propre | Mesures et essais |
| ✔ | C45 | Dictée avec la touche, dans une vraie appli | Mesures et essais |
| ✔ | C46 | Rapport à me coller | Mesures et essais |
| ✔ | C47 | Lancer au démarrage | Données et système |
| ✔ | C48 | Sauvegarde de vos données | Données et système |
| ✔ | C49 | Désinstaller Scriibe | Données et système |
| ✔ | C50 | Version de Scriibe | Données et système |
| ✔ | C51 | Comment ça marche ? (repliée par défaut) | Données et système |

## Commandes vocales (tâche 2)

Fait le 2026-11-17. La page Commandes vocales a ses 7 sections, chacune dans son composant (`ui/src/components/commands/`), assemblées par `ui/src/tabs/Commands.svelte`. `VoiceCommands.svelte` (746 lignes) est supprimé ; le Dictionnaire ne garde que mots exacts, corrections, signes dictés et l'export du dictionnaire.

| Section (carte) | Composant | Id de la carte |
|---|---|---|
| Activation (interrupteur, touche de commande, avertissement) | `ActivationCommands` | `commandes-activation` |
| Commandes intégrées (dictionnaire, entourer, gras/italique/souligné, recherches, nouveau mail) | `BuiltinCommands` | `commandes-integrees` |
| Mes sites de recherche | `SiteCommands` | `commandes-sites` |
| Mes extraits | `SnippetCommands` | `commandes-extraits` |
| Mes applications | `AppCommands` | `commandes-applications` |
| Mes commandes (avec export et import) | `CustomCommands` | `commandes-perso` |
| Reformulation de la sélection | `RewriteCommands` | `commandes-reformulation` |

- Ce qui est partagé (types, libellés des commandes, messages d'erreur communs, recherche du nom d'une commande qui prend déjà une phrase, lecture d'un raccourci) est dans `commands/shared.ts` ; les styles dans `commands/commands.css` (préfixés `.vc`, chaque section est enveloppée dans `<div class="vc">`). Le code des sections est repris tel quel de l'ancien composant.
- L'interrupteur d'activation grise et bloque les six autres sections, comme avant.
- **Touche de commande** : `KeyPicker` prend une propriété `kinds`. Dicter montre dictée et traduction, Commandes vocales montre la commande. Le bouton « Rétablir » ne remet plus que les touches de sa page, et « Tester ma touche » reste sur Dicter (décision T3 du Constat). Sous GNOME, la ligne du raccourci de commande est aussi passée sur Commandes vocales.
- **Accord d'envoi de la sélection** : la case « Autoriser l'envoi de la sélection à ce moteur » est maintenant dans la carte « Reformulation de la sélection » (écriture de `rewriteConsent` inchangée). Choix : l'accord reste lié au moteur de mise au propre (il vaut pour ce moteur, ce modèle et cette adresse), mais se donne ici ; la carte Moteurs et modèles garde une ligne qui le dit, avec un bouton « Voir » vers la carte. Si le moteur est sur l'ordinateur, la carte dit qu'il n'y a rien à accorder.
- **Avertissement** : la carte d'activation dit, sans le cacher, que les commandes qui agissent sur du texte sélectionné ne marchent que sur Mac pour l'instant (la phrase « Sur Mac pour l'instant » a quitté le texte d'introduction pour ne pas être dite deux fois).
- Textes : l'introduction ne renvoie plus à « Réglages, onglet Apparence » mais à la page Apparence ; l'Aide (FR et EN) nomme maintenant la page et la carte de chaque commande ; traductions anglaises ajoutées, anciennes retirées.
- Tests : 4 nouveaux dans `tabs.test.ts` (ordre des 7 sections, le Dictionnaire sans commandes, touche et accord rapatriés, avertissement). `svelte-check` sans erreur, Vitest 117 tests verts, `npm run build` passe.
- Vérifié à l'écran sur la page construite, sans serveur Scriibe : les 7 cartes s'affichent avec leurs titres et l'avertissement. Pas vu : le contenu qui dépend de l'API (listes de phrases, ajout, export, consentement, touche), à voir à l'essai sur le Mac (tâche 5).
- À savoir : le réglage « Rétablir » de la touche de commande n'a pas été essayé sur machine ; il n'écrit que les touches de sa page.

## Dicter, Profil, Style (tâche 3)

Fait le 2026-11-17. Les cartes de l'ancienne carte Dictée sont réparties ; contenu et comportement des réglages inchangés.

- **Dicter** : une carte « Dictée » (touches de dictée et de traduction, langue dictée, traduire vers, sons début et fin, volume, micro, pause des médias, presse-papiers). Le prénom et le genre en sont sortis (la touche de commande l'était depuis la tâche 2).
- **Profil** : `Users` (Utilisateurs), puis nouvelle carte « Prénom et genre » (`profil-personne`, section `identity` de `Settings.svelte`), puis « Comptes en ligne » (l'ancienne carte « Clés du cloud (dépannage) », id `cles` conservé, clés Deepgram et OpenRouter).
- **Style d'écriture** : plus de `Settings` sur cette page. Choix : la case « Sauter la mise au propre… » était seule dans sa carte ; elle est passée dans la carte « Modes de base » (avec Simple, sur lequel elle porte), qui reprend l'introduction de l'ancienne carte « Ce que devient ton texte » (carte supprimée). La fidélité était déjà dans Modes de base (mode Simple). L'ancre `texte` mène toujours à la page.
- **Lien « changer » de l'Accueil** : vérifié, il mène à Profil (ancre `utilisateurs`, `goto('profile', 'utilisateurs')`), rien à changer.
- Textes : Aide FR et EN (« Prénom dans la salutation » dans Dicter → Profil, carte « Prénom et genre ») ; en.ts : « Comptes en ligne », « Prénom et genre », intro de la carte, « Accueil vous salue », retrait des orphelines.
- Tests : 5 nouveaux dans `tabs.test.ts`. `svelte-check` sans erreur, Vitest 122 verts, `npm run build` passe, `cargo test --workspace` tous verts.
- Non vu à l'écran (API absente) : contenu des cartes, à voir à l'essai (tâche 5).

## Moteurs, Données, Mesures (tâche 4)

Fait le 2026-11-17. Cartes déplacées telles quelles ; seuls l'ordre, deux composants à deux parties et quelques titres changent.

- **Moteurs et modèles** : deux parties séparées par un titre de partie (`.part-title`, `app.css`) : **Moteurs** (carte unique : transcription, mise au propre, carte graphique, garder chargé) puis **Modèles** (état de l'installation, programmes, modèles installés, recommandés, Hugging Face). La carte, qui s'appelait aussi « Moteurs » comme sa partie, devient « Transcription et mise au propre » (id `moteurs` inchangé).
- **Données et système** : lancer au démarrage, sauvegarde, version (mise à jour), emplacements (avec la ligne du journal), puis, après un titre « Désinstallation », la désinstallation à part. `System.svelte` prend une propriété `part` (`main` ou `uninstall`), comme `Journal.svelte` (`live` ou `report`), pour poser les cartes dans un ordre voulu sans les copier.
- **Mesures et essais** : bancs (dictée, ce passage, passages gardés, comparaison, banc de la mise au propre), temps des dictées (sorti de `Bench.svelte`), vitesse au clavier, journal en direct, rapport.
- Textes : renvoi de la carte « Ce passage » (« onglet Journal et rapport ») → carte « Rapport à me coller », anglais à jour ; l'Aide décrivait déjà les bonnes pages.
- Tests : 3 nouveaux dans `tabs.test.ts` (ordre des parties et des cartes des 3 pages). `svelte-check` sans erreur, Vitest 125 verts, build passe, `cargo test --workspace` tous verts.
- Non vu à l'écran (pas d'API) : à la tâche 5.

## Liens (tâche 5)

Fait le 2026-11-17. Tous les liens de l'interface visent maintenant une page du menu et l'id réel d'une carte ; les anciens identifiants (`settings`, `modes`, `install`, `bench`, `journal`, `update`) n'existent plus.

- `tabs.ts` : `TabId = MainTab`, `resolveTab`, `oldPages` et `settingsAnchors` supprimés ; `nav.go` ouvre la page demandée telle quelle.
- Boutons « → » de l'Aide (FR et EN) : « Installer les programmes et modèles » → Moteurs et modèles › Les programmes ; « Installer un modèle » → Modèles recommandés ; « Installation » → État de l'installation ; « Les moteurs » → Transcription et mise au propre ; « Comparer avec le banc… » / « Mesures et essais » → Banc de dictée ; « Rapport et journal » → Rapport à me coller ; « Mise à jour » → Version de Scriibe ; « Les modes » → Modes de base ; « Réglages de la dictée » devient « Dicter » (EN : « Dictate »).
- Assistant : « Aller à l'installation » → Moteurs et modèles › État de l'installation ; « Aller au Profil » → Profil › Clés (carte « Comptes en ligne ») ; texte « (onglet Réglages) » → « (page Profil) », anglais à jour.
- Déjà bons : bouton « changer » de l'Accueil (Profil › Utilisateurs), « Tout voir » (Historique), accord de reformulation (Commandes vocales), banc → Dicter, ajout au dictionnaire (Corrections).
- **Test** `ui/src/lib/links.test.ts` : lit tous les fichiers de l'interface ; échoue si un lien (`nav.go`, `goto`, `ongoto`, bouton « → » de l'Aide) vise une page qui n'est pas dans le menu, une carte dont l'id n'existe pas, ou une carte posée sur une autre page ; et si un bouton « ? » ou `openHelp` vise une section de l'Aide absente.
- **Hors périmètre, laissé** : messages du programme Rust qui disent « Réglages » (tâche 9), menu de l'icône (ouvre toujours l'adresse de base, aucun lien de page).

### Vérification à l'écran (Mac)

Copie d'essai `Scriibe-essai.app` (compilée de cette version, `SCRIIBE_INSTALL`) lancée par `open -n --env SCRIIBE_DATA=…` sur une copie des réglages (modèles liés), son adresse locale ouverte dans le navigateur intégré ; copie arrêtée ensuite. `Scriibe.app` habituelle non touchée.

- **Français** : les 12 pages s'ouvrent ; titres de cartes et de parties conformes aux tâches 1 à 4 (Dicter 1 carte ; Style : Modes de base, professionnels, Mes modes ; Moteurs et modèles : parties Moteurs puis Modèles ; Profil : Utilisateurs, Prénom et genre, Comptes en ligne ; Données : démarrage, sauvegarde, version, emplacements, Désinstallation ; Mesures : bancs, temps, vitesse, journal, rapport). Les 18 boutons « → » de l'Aide ouvrent la bonne page ; les 18 ids de carte visés existent dans la page. Copies d'écran vues : Moteurs et modèles, Profil (avec le prénom et le genre lus des vraies données), Données et système.
- **Anglais** (langue changée dans la copie) : menu et titres des 12 pages traduits ; aucun texte français résiduel, hors l'exemple volontaire « je me sens délaissé » du genre.
- **Pas vu** : mode sombre, fenêtre étroite sous 400 px, défilement exact jusqu'à la carte visée (ses ids existent, le scroll animé n'a pas été mesuré), dictée à la touche.

## Essai lot 1 (tâche 6)

Essai réel du 17/11/2026, code au commit `2d6f2de` (les 8 commits de la refonte, poussés sur `main` pour l'occasion, accord de l'auteur). Mac : `Scriibe.app` recompilée et installée. Linux (HP ProBook) et Windows : `pilote maj` puis `pilote compiler --mode release`, lancement de `target/release/scriibe` (Linux relancé par SSH avec l'environnement graphique du processus précédent ; Windows lancé à la main par l'auteur). Réglages relus sur disque après chaque geste.

| Contrôle | Mac | Linux | Windows |
|---|---|---|---|
| Les 4 groupes et chaque page s'ouvrent, aucune page vide | ✔ | ✔ | ✔ |
| Apparence (thème), Dicter (pause des médias), Style (sauter la mise au propre) : gardés après redémarrage | ✔ | ✔ | ✔ |
| Moteurs (délai de veille de Whisper), Commandes vocales (activer), Profil (nom, prénom) : écrits sur disque | ✔ | ✔ (gardés après redémarrage) | ✔ (gardés après redémarrage) |
| Données et système : lancer à l'ouverture de session | ✔ (LaunchAgent créé) | refus voulu : programme dans `target/` | non essayé |
| Case « Comparer avec le modèle » du banc gardée après redémarrage | **✘** | **✘** | **✘** |
| En-tête fixe en bas de page (Moteurs et modèles, Données et système) | ✔ | ✔ | ✔ |
| Boutons « ? » (Dicter, Moteurs et modèles) | ✔ | ✔ | ✔ |
| Assistant : relancer depuis l'Aide, boutons « Aller au Profil » / « Aller à l'installation » | ✔ | ✔ (« Aller au Profil ») | ✔ relance ; boutons non affichés (« Tout est en place », clé déjà rangée) |
| Dictée | ✔ (TextEdit) | ✔ | ✔ (Bloc-notes) |

Réglages et profils d'essai remis par l'auteur sur le Mac ; l'assistant a été essayé avec une sauvegarde de `settings.json` restaurée ensuite (Mac, Linux, Windows). Restes d'essai sur Linux et Windows : thème sombre, pause des médias et « sauter la mise au propre » désactivés, délai de Whisper 30, commandes vocales désactivées ; nom d'utilisateur Linux « Chris ».

### Constats

- **Défaut (avant la refonte, non corrigé) : ce que la page garde dans le navigateur de la fenêtre est perdu à chaque lancement.** Le serveur local écoute sur un port libre qui change à chaque démarrage (`Server::http("127.0.0.1:0")`, `crates/cli/src/web/mod.rs:97`), donc l'origine change et `localStorage` repart vide. Touche : case du banc (`scriibe.bench.compare`), cartes repliées (`fold:<id>`), hauteur des cartes du Dictionnaire (`dictionary:height`), langue de l'interface si seul `localStorage` la porte. Vu sur les trois systèmes pour la case du banc. Correctif possible : ranger ces états dans `settings.json` (ou un fichier d'interface), ou fixer le port ; choix de conception, hors tâche.
- Linux : la case « Lancer à l'ouverture de session » est grisée tant que Scriibe tourne depuis `target/` (garde-fou voulu, `launch_location_problem`) ; la raison est écrite en gris sous la case.
- Linux : la clé OpenRouter ne s'enregistre pas par la page (« pas encore pris en charge ici : définis la variable d'environnement OPENROUTER_API_KEY ») ; manque connu du gestionnaire de secrets Linux (`docs/ECARTS.md`, clés d'API).
- Un renommage de profil sur Linux « perdu » était un renommage non validé : refait, gardé après redémarrage.

### Ajouts

Demandes de l'auteur pendant l'essai :

1. **Choix de durées de veille plus fins** (Moteurs : seulement 10 ou 30 minutes dans la liste). Proposé en ligne du carnet.
2. **Désactiver une commande vocale en particulier** (aujourd'hui seulement « Activer les commandes vocales » pour toutes). Proposé en ligne du carnet.
3. **Garder les états de la page malgré le changement de port** (voir le défaut ci-dessus). Proposé en ligne du carnet.

## Sauvegarder et transférer (tâche 8)

Fait le 2026-11-17. La carte « Sauvegarder et transférer » (id `sauvegarde`, composant `ui/src/components/BackupTransfer.svelte`) remplace « Sauvegarde de vos données » dans Données et système, entre « Lancer au démarrage » et « Version de Scriibe ».

- **En haut** : la sauvegarde complète et sa restauration, déplacées telles quelles (mêmes appels `backup/export`, `backup/import/*`) ; la désinstallation garde son propre appel à `backup/export` dans `System.svelte`.
- **Une ligne par partie** (utilisateur, dictionnaire, modes personnels, commandes vocales personnelles), avec « Exporter… » et « Importer… » : le bouton appelle `nav.transfer(kind)` (`nav.svelte.ts`), qui ouvre la page et la carte, puis y dépose une demande (`nav.transferRequest`) que la page prend dès qu'elle est prête (`$effect` dans `Users`, `Dictionary`, `Modes`, `CustomCommands`) pour ouvrir **la même carte secondaire** que ses propres boutons. Aucun format de fichier nouveau ; les boutons des pages restent.
- **Conservation de l'historique** : lien « Régler la conservation → » vers la carte `conservation` de l'Historique.
- **Choix** : l'export « Utilisateur » vise l'utilisateur actif (un autre : liste de la page Profil) ; l'export des modes s'ouvre d'emblée s'il n'y a qu'un mode personnel, sinon la page se place sur « Mes modes » où chaque mode a son « Exporter » ; rien à exporter → la page se place sur la carte sans rien ouvrir ; une demande non prise est effacée après 4 s. `Dictionary.loaded` devient réactif pour que la demande attende la lecture.
- **Textes** : Aide FR/EN et `en.ts` à jour ; textes nouveaux au tutoiement.

### Tests

- `svelte-check` sans erreur ; Vitest 130 verts (nouveau test dans `links.test.ts` : les 8 demandes visent une page et une carte existantes, ont une ligne dans la carte et une page qui les prend ; `tabs.test.ts` : nouvel ordre de la page Données) ; `cargo fmt`, `clippy -D warnings` et `cargo test --workspace` verts.
- **Essai sur le Mac** : copie `~/Applications/Scriibe-essai.app` lancée avec `SCRIIBE_DATA=~/Library/Application Support/Scriibe-essai-t8` (données fictives : 2 mots, 1 correction, 1 mode, 1 commande, 3 dictées). Les données de l'auteur ont d'abord été copiées (sans les modèles) dans `~/Library/Application Support/Scriibe-sauvegardes/avant-tache8-20261117-132536`, hors iCloud ; son dossier n'a pas été touché.
  - **Sauvegarde** : enregistrée et relue (« ✔ … relue »).
  - **Restauration** : après avoir ajouté un mot et vidé modes et commandes, l'aperçu annonce 1 mode, 2 mots, 1 correction, 3 dictées ; après « Remplacer » et relance, les fichiers contiennent de nouveau 2 mots, 1 correction, 3 dictées, 1 mode, 1 commande, et la copie `avant-import-…` existe.
  - **Lignes de la carte** : « Exporter… » ouvre bien la carte d'export du dictionnaire, de l'utilisateur actif (« Exporter l'utilisateur « Principal » »), du mode (un seul) et des commandes ; « Importer… » du dictionnaire ouvre la boîte de fichier de macOS (annulée).
- **Non vu** : les trois autres « Importer… » à l'écran (même chemin de code, couverts par le test de liens), l'écriture d'un export de partie, la carte en anglais, les systèmes Linux et Windows.
- Piège : les boîtes de fichier de macOS (osascript) ne se pilotent pas depuis une session ; l'auteur a dû les valider. Un fichier d'état modifié est apparu dans Téléchargements pendant l'essai (rangé dans `Scriibe-essai-t8-sorties`).

## Textes (tâche 9)

Fait le 2026-11-17, d'après la liste du § Constat (Textes).

- **Tutoiement** : tous les « vous / votre / vos » et les impératifs au vous de l'interface passent au « tu » (une soixantaine de textes : arrêt de Scriibe, historique, système, sauvegarde, Réglages, Mises à jour, banc, journal, Hugging Face, assistant…), y compris les messages Rust affichés (`uninstall.rs`, `assistant.rs`, `web/mod.rs`) et la phrase « que la vôtre » de l'Aide. Les clés de `en.ts` et `en-server.ts` suivent (la clé française est le texte). Laissés volontairement : « Vouvoie le destinataire » (le mode Courrier vouvoie), les phrases dictées en exemple, « rendez-vous », et « Dites sans la touche » (participe, pas impératif).
- **Touches citées, lues dans les touches réellement réglées** (`status.dictation_label` / `translate_label`, jamais « Ctrl droit » écrit en dur) : banc « Traduire (comme {keys} ) », journal en direct (« Ouvre {app}, maintiens {key}… » avec TextEdit / un éditeur de texte / le Bloc-notes selon le système), assistant (« Tout est en place : maintiens {key}… »), ligne « Touche globale » de l'état d'installation.
- **Utilisateur** : le bouton « Supprimer » devient « Mettre de côté » (le dialogue et l'Aide le disaient déjà ; rien n'est effacé) ; Aide FR/EN à jour.
- **« (bêta) »** retiré de « Afficher la pastille à l'écran » et des trois introductions de la carte « Apparence de l'indicateur » (français et anglais).
- **Carte et page de même nom** : la carte « Commandes vocales » (activation) devient « Activer les commandes vocales » ; c'était le seul doublon exact page / carte. L'assistant renvoyait déjà à la page Profil pour les clés (tâche 5).
- **Aide** : une astuce, dans « Ce qui se passe entre ta voix et le texte » (FR et EN), distingue la **mise au propre** (nettoyage automatique après chaque dictée) de la **reformulation** (commande vocale sur une sélection) ; les deux mots sont gardés, chacun pour sa fonction.
- **Test** (`i18n.test.ts`, « tutoiement ») : échoue si un texte français de l'interface (appels `t`/`tx`, clés de `en.ts` et `en-server.ts`, Aide française) contient « vous », « votre », « vos » ou « vôtre » (hors « rendez-vous »).
- **Tests** : svelte-check sans erreur, Vitest 132 verts, `cargo fmt`, `clippy -D warnings`, `cargo test --workspace` verts (un échec isolé de `instance::a_second_instance_is_refused_until_the_first_ends` sous charge, vert seul et à la relance : à surveiller).
- **Non vu à l'écran** : les nouveaux textes du banc, du journal et de l'assistant (non recompilés sur le Mac) ; Linux et Windows. Restent hors tâche : les textes de touches sans condition de `KeyPicker`, `ShortcutField`, `VoiceCommands` (Constat, tableau b) et les libellés « (Windows) » des programmes sur Mac.

## Aide (tâche 10)

Fait le 2026-11-17.

- **Rubriques = pages du menu** (`ui/src/lib/help.ts`, 13 rubriques dans l'ordre du menu) : Premiers pas, Dicter, Style d'écriture, Dictionnaire, **Commandes vocales** (coupée du Dictionnaire), Moteurs et modèles, Sur cet ordinateur ou en ligne, **Profil**, Apparence (l'ancien « L'indicateur »), **Données et système**, Mesures et essais, Problèmes fréquents, **À propos**. « Premiers pas » a été découpé en quatre (le reste va à Données et système, Profil, Moteurs et modèles) ; le texte n'a pas changé, seulement son rangement, dans `Help.svelte` et `HelpEn.svelte` (même structure, vérifiée par `i18n.test.ts`). Les boutons « ? » des cartes visent la rubrique de leur page (`help="data"`, `"profile"`, `"engines"`, `"commands"`, `"indicator"`).
- **Recherche** : champ au-dessus de l'Aide (`HelpSearchBox.svelte`, logique pure dans `lib/helpsearch.ts`) : texte intégral, sans IA, hors ligne ; casse, accents, apostrophes et espaces insécables ignorés ; deux lettres au moins. Pendant une recherche, toutes les rubriques sont posées dans la page, celles qui ne contiennent pas le mot sont cachées (sommaire compris), le mot est surligné (« CSS Custom Highlight », sans toucher au DOM de Svelte), le compte « n résultat(s) dans m rubrique(s) » est affiché, la page défile au premier résultat ; Échap ou « Effacer » remet l'Aide normale. Même chose en anglais. **Limite** : un mot coupé par une balise (gras au milieu) n'est pas trouvé ; le surlignage demande un navigateur récent (WKWebView, WebView2, WebKitGTK ≥ 2.44), sans lui le filtre marche mais rien n'est surligné.
- **À propos** (`HelpAbout.svelte`, noms dans `lib/about.ts`) : nom, version (celle du programme), créateur, licence MIT, adresse du dépôt avec un bouton « Copier l'adresse » (aucun lien ne s'ouvre depuis la page : pas de nouvelle route côté programme pour ouvrir une adresse). Créateur : l'auteur, accord donné le 17/11/2026 (exception à la règle des noms, notée dans `CLAUDE.md`) ; pas de contributeurs affichés.
- **Tests** : `helpsearch.test.ts` (4 : casse, accents, apostrophes, deux lettres), `tabs.test.ts` (13 rubriques, ordre du menu), `i18n.test.ts` (miroir FR/EN sur 13 rubriques) ; svelte-check sans erreur, Vitest 136 verts, build passe ; `cargo fmt`, `clippy`, `cargo test --workspace` verts.
- **Vu à l'écran (Mac, copie d'essai)** : sommaire à 13 rubriques ; recherche « sauvegarde » : 20 résultats dans 5 rubriques, 20 plages surlignées ; « ÉCRAN » trouve « écran » ; « zzzqq » : « Aucun résultat » ; champ vidé : retour à « Premiers pas ». Capture : mot surligné dans « Commandes vocales » et sommaire réduit aux rubriques trouvées. **Non vu** : l'anglais à l'écran, la rubrique « À propos » à l'écran, Linux et Windows.
