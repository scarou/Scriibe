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
