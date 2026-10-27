# Essai de Scriibe sur Windows (étape 5)

Tout se fait dans une **page web locale** : pas de commande à copier.

## 0. Récupérer et lancer `scriibe.exe`

GitHub → dépôt `scarou/scriibe` → onglet **Actions** → dernière exécution verte de « CI » → section **Artifacts** → `scriibe-windows-x64` (un zip contenant `scriibe.exe`). Extraire le zip dans un dossier simple (par exemple `C:\scriibe`), puis **double-cliquer sur `scriibe.exe`**. Windows SmartScreen peut avertir (binaire non signé) : « Informations complémentaires » → « Exécuter quand même ».

Une fenêtre noire (journal) s'ouvre, une icône ronde apparaît près de l'horloge (grise au repos) et **la page d'essais s'ouvre dans le navigateur**. Si elle ne s'ouvre pas : clic droit sur l'icône → « Ouvrir la page d'essais ». L'adresse de la page contient un jeton secret et ne répond qu'à cette machine.

## 0 bis. Mettre à jour sans télécharger de zip

La page d'essais a une carte **« Mise à jour de Scriibe »** (en haut). Une seule fois, il faut un **jeton GitHub en lecture seule** : GitHub → photo de profil → Settings → Developer settings → Personal access tokens → **Fine-grained tokens** → Generate new token ; durée au choix (90 jours), **Only select repositories** → `Scriibe`, droit **Actions : Read-only** (rien d'autre), puis copier le jeton (il ne s'affiche qu'une fois). Le coller dans la carte et « Enregistrer le jeton » : il est gardé dans le Gestionnaire d'identifiants, jamais affiché ni écrit dans le journal.

Ensuite : **« Chercher une nouvelle version »** affiche la version (commit et titre), la taille et la source ; **« Installer et redémarrer »** demande confirmation, télécharge la dernière exécution verte de la CI, remplace `scriibe.exe` (l'ancien reste un moment sous le nom `scriibe.exe.old`, supprimé au lancement suivant) et relance Scriibe : une nouvelle page d'essais s'ouvre. Le dossier de `scriibe.exe` doit être un dossier où l'on peut écrire (pas `Program Files`).

## 1. La page

1. **État de l'installation** : ce qui est trouvé ou manquant (whisper-server, modèles, llama-server, micro, touche globale), et les réglages de base (mise au propre aucune / llama.cpp, choix des modèles).
2. **Installer ce qui manque** : pour chaque élément, « Voir la taille et la source » affiche le nom du fichier, **la taille réelle** et l'adresse ; un second clic, puis une confirmation, lance le téléchargement avec une barre de progression. Rien ne part sans cette confirmation. Les programmes vont dans `%LOCALAPPDATA%\Scriibe\engines\whisper` et `...\llama`, les modèles dans `%APPDATA%\Scriibe\models`.
3. **Dictée d'essai** : les 17 phrases du banc de dictée de Scribe. Lire la phrase à voix haute, « Terminer » : la page montre ce que Whisper a entendu, ce qui serait collé, les temps, les contrôles automatiques (✔/✘ par phrase) et permet de réécouter la prise. Rien n'est collé ni gardé dans l'historique.
4. **Dictée avec la touche**, dans une vraie appli (Bloc-notes) : maintenir **Ctrl droit**, parler, relâcher ; **Maj** en plus : traduction ; toute autre touche pendant l'appui : annulation. Le journal s'affiche en direct dans la page.
5. **Rapport** : « Copier le rapport » met dans le presse-papiers un texte avec l'état, les résultats et le journal : à coller dans la conversation.

## Cloud en dépannage (à comparer au local)

Section « Clés du cloud » de la page : coller la clé Deepgram et/ou OpenRouter (champ masqué, « Enregistrer ») ; elles vont dans le Gestionnaire d'identifiants, jamais affichées ni journalisées. Puis, dans la section 1, choisir **Transcription : Deepgram (cloud)** et/ou **Mise au propre : OpenRouter (cloud)** (modèle par défaut de Scribe : `anthropic/claude-haiku-4.5`). Refaire les mêmes dictées qu'en local et comparer les temps du journal (« transcription N s », « mise au propre N s », « total après relâche N s »). Le local reste la priorité ; le cloud sert de dépannage quand la machine ne suit pas.

## Carte graphique et durée de conservation du modèle (llama.cpp)

- **Version Vulkan** : section 2, « llama-server (Windows, carte graphique Vulkan) » remplace la version processeur (retélécharger la version processeur pour revenir). Section 1 : « Lister les cartes » puis « Carte graphique de llama.cpp » (automatique, processeur seulement, ou une carte). Le rapport cite l'appareil choisi, `llama-server --list-devices` et le début du journal du moteur. Sur le portable de test, aucune carte n'a donné d'avantage net par dictée (voir `CHANGELOG.md`, S03).
- **Garder le modèle chargé** : 30 min (comme Scribe), 2 h, 8 h ou tant que Scriibe tourne. Après une libération, la dictée suivante coûte 15 à 45 s sur ce portable.
- Les deux choix sont gardés dans les réglages (onglet Réglages ; l’ancien `moteurs.json` est repris au premier lancement).

## Pause des médias (essayée : marche avec Chrome)

Cocher **« Mettre en pause la musique ou la vidéo pendant la dictée »** dans la section 1 de la page (coupé par défaut, comme dans Scribe). Lancer une vidéo (YouTube) ou de la musique, puis dicter avec Ctrl droit : le son doit s'arrêter une fraction de seconde après l'appui, et reprendre au relâchement. Le journal de la page note chaque étape (« médias: pause demandée … », « relance demandée »). Cas à vérifier : pas de pause si rien ne joue (la vidéo ne doit **pas** démarrer), pas de pause pendant un appel, un appui très bref ne fait pas clignoter la vidéo.

## Ce qu'on relève

- Les temps de chaque étape, à comparer au tableau « Temps mesurés » du README de Scribe (M1 Max : 0,3 à 0,6 s).
- Tout ce qui ne marche pas : message affiché, ligne du journal, fenêtre dans laquelle on dictait (le collage est bloqué dans une fenêtre lancée en administrateur).

## En ligne de commande (diagnostic, sans la page)

`scriibe verifier`, `scriibe micro [secondes]`, `scriibe essai [fichier.wav] [--traduire]`, `scriibe cle deepgram|openrouter`, `scriibe aide`. `scriibe lancer --sans-navigateur` ne rouvre pas la page.

## Notes

- PC de test de Christophe : AMD Radeon RX 540 (2 Go) et Intel UHD 620. Avec 2 Go de mémoire vidéo, on commence par la version **processeur** de `llama-server`, puis on essaie Vulkan en comparant les temps.
- Un micro de casque filaire peut être très faible (niveau mesuré 0,006 contre 0,02 à 0,08 pour une voix normale) : Scriibe amplifie jusqu'à ×10 un micro faible avant Whisper et l'indique dans le journal (`gain ×N`).
