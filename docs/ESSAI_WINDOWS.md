# Essai de Scriibe sur Windows (étape 5)

But : vérifier, sur un vrai PC, chaque maillon de la chaîne, l'un après l'autre. À chaque étape, copier la sortie de la fenêtre PowerShell pour la renvoyer.

Rien n'est téléchargé sans confirmation, ni installé en dehors de `%LOCALAPPDATA%\Scriibe` et `%APPDATA%\Scriibe`.

## 0. Récupérer `scriibe.exe`

GitHub → dépôt `scarou/scriibe` → onglet **Actions** → dernière exécution verte de « CI » → section **Artifacts** → `scriibe-windows-x64` (zip contenant `scriibe.exe`). Dézipper dans un dossier, par exemple `C:\scriibe`. Windows SmartScreen peut avertir (binaire non signé) : « Informations complémentaires » → « Exécuter quand même ».

Dans PowerShell : `cd C:\scriibe`.

## 1. `.\scriibe.exe verifier`

Affiche les dossiers utilisés, et ce qui est trouvé ou manquant (programmes, modèles, clés, micros). Au premier lancement, crée `%APPDATA%\Scriibe\settings.json` avec des réglages tout-local : Whisper et llama.cpp sur le PC, touche **Ctrl droit**, **Maj** en plus pour traduire.

## 2. `.\scriibe.exe micro 4`

Enregistre 4 secondes (parler pendant ce temps), dit si le niveau est muet, faible ou correct, et garde la prise dans `%APPDATA%\Scriibe\essai-micro.wav`.

## 3. Les moteurs locaux

`.\scriibe.exe verifier` dit ce qui manque. Deux programmes et deux modèles :

| Quoi | Source | Taille indicative |
|---|---|---|
| `whisper-server.exe` | dernière version de `ggml-org/whisper.cpp` sur GitHub (fichier `whisper-bin-x64.zip`) | quelques dizaines de Mo |
| `llama-server.exe` | dernière version de `ggml-org/llama.cpp` (fichier `llama-…-bin-win-cpu-x64.zip`, ou `…-vulkan-x64.zip` pour la carte graphique) | quelques dizaines à ~150 Mo |
| modèle Whisper | Hugging Face `ggerganov/whisper.cpp` : `ggml-small.bin` (≈ 470 Mo), ou `ggml-base.bin` (≈ 140 Mo) pour aller plus vite | à choisir |
| modèle de mise au propre (GGUF) | un petit modèle d'instructions (≈ 1 Go) : à choisir ensemble selon les mesures | à choisir |

Le script `tools/windows/installer-moteurs.ps1` installe les deux programmes **après avoir affiché nom, taille et source et demandé « o »**. Les modèles se déposent à la main dans `%APPDATA%\Scriibe\models` (les `ggml-*.bin` et les `*.gguf`).

PC de test de Christophe : AMD Radeon RX 540 (2 Go) et Intel UHD 620. Avec 2 Go de mémoire vidéo, on commence par la version **cpu** de `llama-server`, puis on essaie Vulkan en comparant les temps.

## 4. `.\scriibe.exe essai` (ou `essai fichier.wav`, `--traduire`)

Passe la prise du micro par la vraie chaîne (Whisper → corrections → modèle → garde-fou), **deux fois** (modèles à froid, puis chargés), sans toucher au clavier ni coller. Affiche le texte qui serait collé et les temps (`%APPDATA%\Scriibe\scriibe.log` : une ligne par dictée, sans le texte dicté).

## 5. `.\scriibe.exe lancer`

L'icône apparaît dans la zone de notification (couleur : gris repos, rouge écoute, orange traitement, vert collé, bleu message). Ouvrir le Bloc-notes, **maintenir Ctrl droit**, parler, relâcher : le texte est collé. Ajouter **Maj** pendant l'appui pour traduire. Toute autre touche pendant l'appui annule (c'est un raccourci). Menu de l'icône : dossier des données, journal, Quitter.

## Ce qu'on relève

- Les temps de chaque étape (`scriibe.log`), à comparer au tableau « Temps mesurés » du README de Scribe (M1 Max : 0,3 à 0,6 s).
- Tout ce qui ne marche pas : message affiché, ligne du journal, fenêtre dans laquelle on dictait (le collage est bloqué dans une fenêtre lancée en administrateur).
