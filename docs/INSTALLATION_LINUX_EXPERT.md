# Scriibe sur Linux — guide pour connaisseurs

🇬🇧 [English version](INSTALLATION_LINUX_EXPERT.en.md)

Autres guides : [installation pas à pas pour débutants](INSTALLATION_LINUX.md) · [Windows](INSTALLATION_WINDOWS_EXPERT.md) · [macOS](INSTALLATION_MACOS_EXPERT.md)

Ce guide suppose que vous savez utiliser un terminal. Il regroupe ce qui ne figure pas dans le guide pour débutants : les sessions X11 et les autres bureaux, les dépendances système, la compilation de `whisper-server`, les moteurs en ligne, la mise à jour à la main et la compilation depuis les sources. Le détail de ce qui a été essayé est dans `docs/ESSAI_LINUX.md` et `docs/ESSAI_LINUX_PC.md`.

## Type de session

```bash
echo $XDG_SESSION_TYPE
```

- **`x11`** : pris en charge (touche globale par XInput2, collage par XTest, application active par `_NET_ACTIVE_WINDOW`). La touche de dictée par défaut est **Ctrl droit**.
- **`wayland` avec GNOME 48 ou plus récent** (Ubuntu 25.04 et suivantes) : pris en charge par les « portails » de GNOME. GNOME n'accepte pas une touche seule comme Ctrl droit : choisir une combinaison avec une touche ordinaire. Pour la changer ensuite : **Paramètres → Applications → Scriibe → Raccourcis clavier**, puis relancer Scriibe. Le collage demande l'autorisation de contrôler le clavier.
- **`wayland` avec un autre bureau** (KDE, ou GNOME plus ancien) : pas encore pris en charge. Ouvrir une session X11 : à l'écran de connexion, la roue dentée propose une session **Xorg** / **X11** sur beaucoup de distributions. **Ubuntu 25.10 et suivantes** n'ont plus de session X11 avec GNOME : installer Xfce (`sudo apt update && sudo apt install -y xfce4`) et choisir **Xfce Session** à la connexion ; GNOME reste installé.

## Dépendances système

Le micro demande ALSA (présente sur les bureaux courants) ; la fenêtre et l'icône demandent WebKitGTK et AppIndicator (présents d'office sur Ubuntu avec GNOME). Si le lancement se plaint :

```bash
sudo apt install -y unzip
sudo apt install -y libasound2t64          # libasound2 sur les versions plus anciennes
sudo apt install -y libwebkit2gtk-4.1-0 libayatana-appindicator3-1
```

Sur un GNOME sans l'extension AppIndicator (Fedora), l'icône n'apparaît pas : rouvrir la page par l'adresse affichée par le terminal. Les sons sont joués par `paplay`, `pw-play` ou `aplay` (le premier trouvé) ; la pause des médias passe par `pactl` et `playerctl`.

## Les deux moteurs locaux

Scriibe appelle deux programmes externes en HTTP : `whisper-server` (transcription) et `llama-server` (mise au propre). Les cartes **Les programmes** et **Modèles recommandés** de **Réglages → Moteurs** les proposent (`whisper-server (Linux)` : whisper.cpp statique, AVX2 ; `llama-server (Linux, processeur)` : version officielle remballée par la CI), avec taille et source avant l'accord. Ils se rangent dans `~/.local/share/Scriibe/engines/`, avec leurs droits d'exécution et `LD_LIBRARY_PATH` posés par Scriibe. Le processeur doit connaître AVX2 (tout PC depuis 2015 environ). Les modèles GGUF servent à la mise au propre, les `ggml-….bin` à Whisper.

### Compiler `whisper-server` soi-même

Si vous préférez ne pas utiliser la version de la CI :

```bash
sudo apt install -y build-essential cmake git
git clone --depth 1 https://github.com/ggml-org/whisper.cpp ~/whisper.cpp
cd ~/whisper.cpp && cmake -B build -DBUILD_SHARED_LIBS=OFF && cmake --build build -j --config Release --target whisper-server
mkdir -p ~/.local/share/Scriibe/engines/whisper && cp ~/whisper.cpp/build/bin/whisper-server ~/.local/share/Scriibe/engines/whisper/
```

(`BUILD_SHARED_LIBS=OFF` : un seul fichier, sans bibliothèques à côté.) `llama-server` peut aussi venir de la distribution ou des versions de llama.cpp.

## Moteurs en ligne (dépannage)

Plus rapides à mettre en place, ou utiles sur une machine modeste, mais payants à l'usage : la voix part chez Deepgram, le texte chez OpenRouter pour être mis au propre. Sous Linux il n'y a pas de trousseau : les clés se donnent par des variables d'environnement, **avant** de lancer Scriibe, dans le même terminal :

```bash
export DEEPGRAM_API_KEY='…votre clé…'
export OPENROUTER_API_KEY='…votre clé…'
~/scriibe/scriibe
```

## Terminaux

Dans un terminal, le collage se fait par Ctrl+Maj+V : Scriibe envoie Ctrl+V, qui n'y marche pas ; le texte reste dans le presse-papiers (et « Copier la dernière dictée » est dans le menu de l'icône).

## Mise à jour à la main

L'installation intégrée (**Réglages → Mise à jour → Chercher une nouvelle version → Installer et redémarrer**) télécharge `scriibe-linux-x64.zip` de la dernière Release, remplace le programme (l'ancien reste à côté sous le nom `scriibe.old` jusqu'au lancement suivant) et relance Scriibe ; le dossier du programme doit être à vous (c'est le cas de `~/scriibe`). À la main, si elle échoue ou depuis une version plus ancienne :

1. Quitter Scriibe : menu de l'icône → Quitter (ou Ctrl+C dans le terminal).
2. Télécharger le dernier `scriibe-linux-x64.zip` et refaire :

```bash
mkdir -p ~/scriibe && unzip -o ~/Téléchargements/scriibe-linux-x64.zip -d ~/scriibe && chmod +x ~/scriibe/scriibe
```

3. Relancer. Réglages, historique et dictionnaire sont conservés (rangés dans `~/.local/share/scriibe`).

## Compiler depuis les sources

Prérequis : Rust, Node.js, Git, `libasound2-dev`, les en-têtes de WebKitGTK et d'AppIndicator.

```bash
cd ui && npm ci && npm run build && cd ..
cargo build --release
```

L'exécutable est `target/release/scriibe`. Construire `ui/dist` **avant** `cargo build` : sinon l'interface est remplacée par un message « Interface absente ». Pour un essai sans toucher à vos données : `SCRIIBE_DATA=/tmp/scriibe target/release/scriibe lancer --sans-navigateur`.

## Pour le créateur : publier une version

Le workflow `release-linux.yml` compile `scriibe-linux-x64.zip` (sur Ubuntu 22.04) ainsi que les moteurs Linux et les ajoute à la Release publiée par `tools/publier_macos.sh`. Attendre qu'il ait fini avant de demander la mise à jour sur un PC.
