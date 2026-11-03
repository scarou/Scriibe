# Essai de Scriibe sur Linux (X11) — étape 7, premier jalon (S07)

Écrit et essayé **dans le cloud, sur un écran virtuel (Xvfb)** : pas encore sur un vrai bureau Linux. Wayland n'est pas pris en charge.

## Ce qui marche (vérifié sur Xvfb, `crates/platform/tests/linux_x11.rs`)

- **Touche globale** : événements bruts de XInput2 sur la fenêtre racine (aucune bibliothèque système, X11 par le protocole seul). Ctrl droit, F1 à F24, « < > », Windows (Super), Pause… comme sous Windows ; les codes de touche de X11 sont ramenés aux codes de Windows (`linux/evdev.rs`). Les touches envoyées par programme (notre Ctrl+V) sont ignorées, sauf avec `SCRIIBE_X11_ACCEPT_XTEST=1` (essais avec `xdotool`).
- **Collage** : presse-papiers (`arboard`) + Ctrl+V par XTest avec la disposition du clavier en vigueur, attente du relâchement de Maj, Ctrl et Alt, puis l'ancien presse-papiers remis (texte seulement) ; « Copier la dernière dictée ».
- **Appli au premier plan** : `_NET_ACTIVE_WINDOW` puis `_NET_WM_PID` (nom de l'exécutable). Sans gestionnaire de fenêtres compatible : « Inconnue ».
- Toute la chaîne démarre sur Xvfb : l'appui sur Ctrl droit ouvre le micro (qui échoue seulement parce que le cloud n'a pas de carte son).

## Écrit, pas encore essayé sur un vrai bureau

- **Micro** : `cpal` (ALSA, donc PulseAudio ou PipeWire par le greffon ALSA). Nécessite `libasound2` ; pour compiler, `libasound2-dev`.
- **Sons** : synthétisés, joués par `paplay`, `pw-play` ou `aplay` (le premier trouvé).
- **Pause des médias** : flux audio lus avec `pactl` (PulseAudio ou PipeWire-pulse), pause envoyée par `playerctl` (MPRIS) au lecteur concerné, sinon par la touche Lecture/Pause (XTest) ; appel détecté comme sous Windows (un processus qui joue et capte le micro).

## Limites connues (à traiter ensuite)

- **Session Wayland refusée** avec un message : il n'y a ni touche globale ni collage simulé pour une appli ordinaire (portails, `libei`, repli « copie seule » : voir `ROADMAP.md`).
- Une touche **ordinaire** de la combinaison (F9, « < > ») n'est pas retenue : elle écrit aussi dans l'appli active (Ctrl droit par défaut ne pose pas ce problème). À faire : `XGrabKey`.
- Pas d'**icône** (la zone de notification de Linux passe par GTK et AppIndicator) ni de **fenêtre Tauri** : l'interface s'ouvre dans le navigateur (`xdg-open`).
- **Terminaux** : ils collent avec Ctrl+Maj+V ; le texte reste dans le presse-papiers.
- **Clés d'API** (Deepgram, OpenRouter) : par les variables `DEEPGRAM_API_KEY` et `OPENROUTER_API_KEY` ; pas de trousseau encore.
- **Moteurs** : `whisper-server` n'a pas de version Linux toute faite (à compiler ou à installer par la distribution) ; `llama-server` par la distribution ou depuis les versions de llama.cpp. Les téléchargements de l'onglet Installation ne les proposent pas encore sous Linux.

## Essayer

```
cargo build -p scriibe-cli      # libasound2-dev nécessaire
SCRIIBE_DATA=/tmp/scriibe target/debug/scriibe lancer --sans-navigateur
```

Sur un écran virtuel : `Xvfb :99 &`, puis `DISPLAY=:99 SCRIIBE_X11_ACCEPT_XTEST=1 … scriibe lancer`, et `DISPLAY=:99 xdotool keydown 105` (Ctrl droit) pour dicter.

## Essai sur un vrai bureau Linux (S15, 03/11/2026)

PC de Christophe : HP ProBook 450 G6, **Ubuntu 26.04.1 LTS**, session **Wayland** à l'origine. Ubuntu 25.10 et suivantes n'ont plus de session X11 avec GNOME : choix de Christophe (03/11) d'installer le bureau **Xfce** (`sudo apt install xfce4`, session « Xfce Session » au choix à la connexion) pour essayer la couche X11 existante. Wayland reste le vrai chantier pour les utilisateurs d'Ubuntu récent (portails, `libei` : `ROADMAP.md`).

Livrable : `scriibe-linux-x64.zip` (exécutable `scriibe`, x86-64), compilé sur Ubuntu 22.04 par `.github/workflows/release-linux.yml` et ajouté à la Release publiée. Guide : [INSTALLATION_LINUX.md](INSTALLATION_LINUX.md).

### Moteurs (S15)

Depuis le commit « Moteurs Linux » : `release-linux.yml` compile `whisper-server` (whisper.cpp v1.9.5, statique, AVX2 ; vérifié en CI : ne dépend que de libstdc++, libm, libgcc, libc) et remballe en zip le `llama-server` officiel de llama.cpp (processeur, variantes CPU en `.so`) ; l'onglet Installation les propose sous Linux (`whisper-server (Linux)`, `llama-server (Linux, processeur)`), droits d'exécution posés après dépaquetage, `LD_LIBRARY_PATH` du dossier du moteur à son lancement. **Pas encore essayé sur le PC** (il faut une version publiée après la 0.2.2).

### Compiler `whisper-server` soi-même (version 0.2.2 sans les cartes Linux)

À confirmer par l'essai. Une commande après l'autre :

```bash
sudo apt install -y build-essential cmake git
git clone --depth 1 https://github.com/ggml-org/whisper.cpp ~/whisper.cpp
cd ~/whisper.cpp && cmake -B build -DBUILD_SHARED_LIBS=OFF && cmake --build build -j --config Release --target whisper-server
mkdir -p ~/.local/share/Scriibe/engines/whisper && cp ~/whisper.cpp/build/bin/whisper-server ~/.local/share/Scriibe/engines/whisper/
```

(`BUILD_SHARED_LIBS=OFF` : un seul fichier, sans bibliothèques à côté.) Le modèle (`whisper-base`) se télécharge depuis la page de Scriibe, qui le propose sous Linux.

Bruit d'ALSA : au premier lancement, des lignes `ALSA lib pcm_dsnoop.c … unable to open slave` défilent. Cause trouvée dans le code : la page redemande la liste des micros à chaque rafraîchissement et `cpal` parcourt à chaque fois les périphériques ALSA, qui écrivent ces lignes ; la liste est maintenant gardée 5 s sous Linux (`recorder.rs`). Que le micro s'ouvre vraiment reste à vérifier sur le PC.

Constat sur Wayland : Christophe n'acceptera pas de changer de bureau pour utiliser Scriibe (« le logiciel doit s'adapter à mon environnement ») ; la cible réelle est GNOME Wayland. Pistes dans `ROADMAP.md` (portails GlobalShortcuts et RemoteDesktop).

### Résultats

(à remplir pendant l'essai)
