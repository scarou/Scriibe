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

**03/11/2026, session Xfce (X11), Scriibe compilé depuis les sources (`55dc789`), Whisper `ggml-base`, mise au propre éteinte :**

- Compilation sur le PC : `apt install git build-essential libasound2-dev pkg-config cmake curl nodejs npm gh` (Node 22.22 d'Ubuntu 26.04, environ 130 Mo), Rust par rustup ; `npm ci && npm run build`, `cargo build --release -p scriibe-cli` en 2 min. Rien à corriger.
- `whisper-server` compilé à la main (section ci-dessus), modèle `ggml-base.bin` téléchargé par la page : sans souci.
- **Touche globale (Ctrl droit, XInput2)** : marche ; appui et relâchement vus, appli au premier plan reconnue (`gnome-text-edit`).
- **Micro (`cpal`, ALSA → PipeWire)** : s'ouvre (`default`, 44 100 Hz, 2 canaux). Le bruit `ALSA lib …`/`jack server is not running` reste dans la sortie mais ne gêne pas.
- **Collage (presse-papiers + Ctrl+V par XTest)** : marche dans l'Éditeur de texte de GNOME, en 0,04 s.
- Transcription : environ 0,5 à 0,8 s après le relâchement pour 2 à 5 s de parole (processeur, 4 fils).
- Premiers textes sans rapport avec la dictée : **micro intégré saturé** (niveau 1,000) ; voir `PIEGES.md`. Après avoir baissé le gain, phrases justes (« bonjour ceci est une phrase test », « Retour à la ligne, ceci est la troisième phrase test. »).
- Le niveau relevé monte encore parfois à 1,000 sur une dictée juste (pic isolé ?) : à surveiller.

**03/11/2026, session GNOME Wayland (Ubuntu 26.04, GNOME 50, portail `xdg-desktop-portal` 1.21, `xdg-desktop-portal-gnome` 50), module `crates/platform/src/linux/wayland.rs` :**

- **Touche globale (portail `GlobalShortcuts`)** : marche. Au premier lancement, GNOME ouvre sa fenêtre de raccourci ; il **refuse une touche de modification seule** (Ctrl droit) ; Christophe a choisi **Ctrl + flèche gauche**. `Activated`/`Deactivated` arrivent bien à l'appui et au relâchement (son de fin aussitôt). Au relancement, rien n'est redemandé (GNOME garde le raccourci sous l'identifiant `fr.scriibe.Scriibe`, déclaré par `register_host_app`).
- **Collage (presse-papiers par XWayland + Ctrl+V par le portail `RemoteDesktop`)** : marche dans l'Éditeur de texte (0,04 à 0,06 s). Autorisation de contrôle demandée une fois ; le jeton (`portail-wayland.json` du dossier de données) évite de la redemander au relancement (vérifié).
- **Appli au premier plan** : « Inconnue » (Wayland ne la donne pas).
- Le portail `GlobalShortcuts` de ce GNOME est en **version 1** : pas de `ConfigureShortcuts` (fenêtre de changement du raccourci) ; on change le raccourci dans **Paramètres → Applications → Scriibe → Raccourcis clavier** (chemin vérifié par Christophe), puis on relance Scriibe. La page l'explique (Réglages → Dictée) et affiche le raccourci (« Prêt · Ctrl + ← »).
- Non essayé : la traduction (aucun raccourci de traduction n'est déclaré pour l'instant), la pause des médias, les terminaux, une appli Wayland native autre que l'Éditeur de texte (Firefox, LibreOffice…).

**03/11/2026, fenêtre et icône (Tauri) sous GNOME Wayland :**

- Scriibe ouvre maintenant **sa propre fenêtre** (WebKitGTK, comme sur Mac et Windows) au lieu du navigateur, et met une **icône dans la barre du haut** (AppIndicator, extension `ubuntu-appindicators` d'Ubuntu) avec le même menu que sur Mac. Pas de pastille (sous Wayland, une appli ne peut pas placer une fenêtre toujours devant). **Vues par Christophe** : la fenêtre, l’icône et le menu « Ouvrir Scriibe » (une fenêtre réduite revient : sous Wayland, `show()` cache puis remontre la fenêtre).
- Dictée avec la fenêtre ouverte : texte collé dans l'Éditeur de texte (31 signes en 0,08 s), après avoir monté le micro USB devenu source par défaut (`PIEGES.md`).
