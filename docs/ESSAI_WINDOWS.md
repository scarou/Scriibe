# Essai de Scriibe sur Windows (étapes 5 et 6)

Tout se fait dans la **fenêtre de Scriibe** (WebView2 ; si elle ne peut pas s'ouvrir, la même interface s'ouvre dans le navigateur) : pas de commande à copier. Fermer la fenêtre ne quitte pas Scriibe : l'icône près de l'horloge la rouvre (menu « Ouvrir Scriibe »).

## 0. Récupérer et lancer `scriibe.exe`

GitHub → dépôt `scarou/scriibe` → onglet **Actions** → dernière exécution verte de « CI » → section **Artifacts** → `scriibe-windows-x64` (un zip contenant `scriibe.exe`). Extraire le zip dans un dossier simple (par exemple `C:\scriibe`), puis **double-cliquer sur `scriibe.exe`**. Windows SmartScreen peut avertir (binaire non signé) : « Informations complémentaires » → « Exécuter quand même ».

Une fenêtre noire (journal) s'ouvre, une icône ronde apparaît près de l'horloge (grise au repos) et **la fenêtre de Scriibe s'ouvre**. Si elle ne s'ouvre pas : clic droit sur l'icône → « Ouvrir Scriibe ». L'interface est servie par un petit serveur local (adresse à jeton secret, ne répond qu'à cette machine) ; `scriibe lancer --navigateur` l'ouvre dans le navigateur à la place.

## 0 bis. Mettre à jour sans télécharger de zip

L'onglet **« Mise à jour »** de la fenêtre. Une seule fois, il faut un **jeton GitHub en lecture seule** : GitHub → photo de profil → Settings → Developer settings → Personal access tokens → **Fine-grained tokens** → Generate new token ; durée au choix (90 jours), **Only select repositories** → `Scriibe`, droit **Actions : Read-only** (rien d'autre), puis copier le jeton (il ne s'affiche qu'une fois). Le coller dans la carte et « Enregistrer le jeton » : il est gardé dans le Gestionnaire d'identifiants, jamais affiché ni écrit dans le journal.

Ensuite : **« Chercher une nouvelle version »** affiche la version (commit et titre), la taille et la source ; **« Installer et redémarrer »** demande confirmation, télécharge la dernière exécution verte de la CI, remplace `scriibe.exe` (l'ancien reste un moment sous le nom `scriibe.exe.old`, supprimé au lancement suivant) et relance Scriibe : la fenêtre se rouvre. Le dossier de `scriibe.exe` doit être un dossier où l'on peut écrire (pas `Program Files`).

## 1. Les onglets

- **Tableau de bord** : mots dictés, temps gagné, série de jours, 14 derniers jours, applis ; le **test de frappe** (« Ma vitesse au clavier ») ; au premier lancement, l'**assistant** : il mesure l'ordinateur (cœurs, mémoire, cartes graphiques) et propose la chaîne adaptée avec les temps **mesurés** ; « Appliquer ce choix » règle les moteurs puis dit ce qu'il reste à installer (rien n'est téléchargé sans votre accord).
- **Historique** : les dictées (copier, supprimer), recherche, durée de conservation et « Garder le texte des dictées » (un aperçu de ce qui sera supprimé avant de confirmer).
- **Modes** : le mode en cours, la consigne de chaque mode, vos propres modes avec leurs exemples.
- **Dictionnaire** : mots à écrire exactement et corrections « entendu → écrit » (enregistrées toutes seules).
- **Réglages** : moteurs (Whisper ou Deepgram ; aucune mise au propre, llama.cpp ou OpenRouter), modèles, carte graphique et durée de conservation du modèle (llama.cpp), langue, micro, pause des médias, clés du cloud.
- **Installation** (refondue en S07) : l'état de ce qui est trouvé ; **Les programmes** (whisper-server, llama-server processeur ou Vulkan) ; **Modèles installés** (ceux du dossier de Scriibe, avec leur taille, « utilisé » pour celui qui sert, « Supprimer » après confirmation ; un `.part` est un téléchargement interrompu) ; **Modèles recommandés** (courte liste de départ) ; **Ajouter un modèle depuis Hugging Face** : des mots à chercher (classés par téléchargements, tendance ou nouveauté) ou un dépôt (`propriétaire/nom` ou son adresse) → ses fichiers avec la **vraie taille**, un clic puis une confirmation (nom, taille, source, destination) lance le téléchargement, qui reprend tout seul s'il est coupé. Le modèle installé se choisit ensuite dans Réglages → Moteurs. « Voir la taille et la source » affiche le nom, **la taille réelle** et l'adresse ; un téléchargement coupé **reprend tout seul**. Les programmes vont dans `%LOCALAPPDATA%\Scriibe\engines\whisper` et `...\llama`, les modèles dans `%APPDATA%\Scriibe\models`.
- **Banc d'essai** : les 17 phrases du banc de dictée de Scribe. Lire la phrase à voix haute, « Terminer » : on voit ce que Whisper a entendu, ce qui serait collé, les temps, les contrôles automatiques (✔/✘ par phrase) et on peut réécouter la prise. Rien n'est collé ni gardé dans l'historique.
- **Mise à jour** : voir plus haut.
- **Journal et rapport** : le journal en direct (une ligne par étape, jamais le texte dicté) pendant une dictée avec la touche (**Ctrl droit** maintenu, **Maj** en plus : traduction, autre touche : annulation) ; « Copier le rapport » met dans le presse-papiers l'état, les résultats du banc et le journal : à coller dans la conversation.
- **Aide** : huit sections.

## Choisir ses touches (essayé en S06)

Onglet **Réglages**, carte **Dictée** : « Changer… » à côté de « Touche de dictée » ; appuyer sur la touche ou la combinaison voulue (jusqu'à trois touches, dont une seule ordinaire), tout relâcher, puis « Utiliser cette touche ». Même chose pour la touche tenue en plus pour traduire. « Tester ma touche » (15 s) vérifie que Scriibe passe en écoute ; « Rétablir » remet Ctrl droit et Maj.

Pendant la capture, la fenêtre de Scriibe transmet elle-même les touches qu'elle voit ; **Windows et Alt gauche** se captent depuis une autre fenêtre (appuyer dans le Bloc-notes, puis revenir). Dicter *dans* la fenêtre de Scriibe (le Dictionnaire, un mode…) marche depuis S08 : le crochet du système n'y reçoit rien, alors la page relaie ses touches (`api/keys/page`) ; **à essayer sur le PC**. Limites : la touche Windows ouvre le menu Démarrer au relâchement, et une touche qui écrit un signe (« < > ») l'écrit aussi dans la zone de texte. Essayé sur le PC : F9, « < > », Ctrl gauche + F10, **Windows**, **Alt gauche**, **Maj gauche**, traduction avec F9. Reste à essayer : Fn, « Tester ma touche », « Rétablir ». À essayer : la touche **Windows**, **Maj gauche**, la touche **« < > »** (à gauche de W), Ctrl gauche + une touche de fonction, une combinaison de touches de modification seules (il faut la tenir 0,3 s avant que l'écoute commence) ; vérifier qu'**Alt gauche** et **Windows** n'ouvrent ni menu ni Démarrer au relâchement, que le collage garde le focus, et que la touche ordinaire de la combinaison (F9, « < > ») n'écrit rien dans l'appli. **Fn** : probablement invisible pour Windows (la capture le dira).

## Sons, menu de l'icône, moteurs (S06)

- **Réglages → Dictée** : « Son au début », « Son à la fin » (douze sons + « Aucun », bouton ▶), volume des sons. Les sons sont synthétisés.
- **Clic droit sur l'icône** : Ouvrir Scriibe, les touches, **Copier la dernière dictée**, sous-menu **Mode** (coché), version, dossier des données, journal, Quitter.
- **Réglages → Moteurs** : mise au propre **LM Studio** ou **Ollama** (liste des modèles du serveur, « Actualiser », adresse), modèle Deepgram, « Choisir un fichier… » pour Whisper, « Libérer Whisper au repos ».
- **Modes** : « Essayer sur les phrases types » (12 phrases, garde-fou et contrôles). **Banc d'essai** : « Banc d'essai de la mise au propre » (une phrase, plusieurs modèles).
- **Historique** : chaque dictée indique son mode et le sort de la mise au propre.

## Cloud en dépannage (à comparer au local)

Onglet Réglages, « Clés du cloud » : coller la clé Deepgram et/ou OpenRouter (champ masqué, « Enregistrer ») ; elles vont dans le Gestionnaire d'identifiants, jamais affichées ni journalisées. Puis, dans « Moteurs », choisir **Transcription : Deepgram (cloud)** et/ou **Mise au propre : OpenRouter (cloud)** (modèle par défaut de Scribe : `anthropic/claude-haiku-4.5`). Refaire les mêmes dictées qu'en local et comparer les temps du journal (« transcription N s », « mise au propre N s », « total après relâche N s »). Le local reste la priorité ; le cloud sert de dépannage quand la machine ne suit pas.

## Carte graphique et durée de conservation du modèle (llama.cpp)

- **Version Vulkan** : onglet Installation, « llama-server (Windows, carte graphique Vulkan) » remplace la version processeur (retélécharger la version processeur pour revenir). Onglet Réglages : « Lister les cartes » puis « Carte graphique ». Le rapport cite l'appareil choisi, `llama-server --list-devices` et le début du journal du moteur. Sur le portable de test, aucune carte n'a donné d'avantage net par dictée (voir `CHANGELOG.md`, S03).
- **Garder le modèle chargé** : 30 min (comme Scribe), 2 h, 8 h ou tant que Scriibe tourne. Après une libération, la dictée suivante coûte 15 à 45 s sur ce portable.
- Les deux choix sont gardés dans les réglages (l'ancien `moteurs.json` est repris au premier lancement).

## Pause des médias (essayée : marche avec Chrome et, depuis S05, avec VLC ; appel non essayé)

Cocher **« Mettre en pause la musique ou la vidéo pendant la dictée »** dans l'onglet Réglages (coupé par défaut, comme dans Scribe). Lancer une vidéo (YouTube) ou de la musique, puis dicter avec Ctrl droit : le son doit s'arrêter une fraction de seconde après l'appui, et reprendre au relâchement. Le journal (onglet Journal et rapport) note chaque étape (« médias: pause demandée … », « relance demandée »). Cas à vérifier : pas de pause si rien ne joue (la vidéo ne doit **pas** démarrer), pas de pause pendant un appel, un appui très bref ne fait pas clignoter la vidéo.

## Ce qu'on relève

- Les temps de chaque étape, à comparer au tableau « Temps mesurés » du README de Scribe (M1 Max : 0,3 à 0,6 s).
- Tout ce qui ne marche pas : message affiché, ligne du journal, fenêtre dans laquelle on dictait (le collage est bloqué dans une fenêtre lancée en administrateur).

## En ligne de commande (diagnostic, sans la fenêtre)

`scriibe verifier`, `scriibe micro [secondes]`, `scriibe essai [fichier.wav] [--traduire]`, `scriibe cle deepgram|openrouter`, `scriibe aide`. `scriibe lancer --sans-navigateur` n'ouvre rien au démarrage (le menu de l'icône ouvre alors l'interface dans le navigateur) ; `--navigateur` ouvre l'interface dans le navigateur au lieu de la fenêtre.

## Notes

- PC de test de Christophe : AMD Radeon RX 540 (2 Go) et Intel UHD 620. Avec 2 Go de mémoire vidéo, on commence par la version **processeur** de `llama-server`, puis on essaie Vulkan en comparant les temps.
- Un micro de casque filaire peut être très faible (niveau mesuré 0,006 contre 0,02 à 0,08 pour une voix normale) : Scriibe amplifie jusqu'à ×10 un micro faible avant Whisper et l'indique dans le journal (`gain ×N`).
