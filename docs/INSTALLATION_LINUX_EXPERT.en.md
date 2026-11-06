# Scriibe on Linux — guide for experienced users

🇫🇷 [Version française](INSTALLATION_LINUX_EXPERT.md)

Other guides: [step-by-step installation for beginners](INSTALLATION_LINUX.en.md) · [Windows](INSTALLATION_WINDOWS_EXPERT.en.md) · [macOS](INSTALLATION_MACOS_EXPERT.en.md)

This guide assumes you can use a terminal. It gathers what is not in the beginners' guide: X11 sessions and other desktops, system dependencies, building `whisper-server`, online engines, the manual update and building from source. What has been tried is in `docs/ESSAI_LINUX.md` and `docs/ESSAI_LINUX_PC.md` (in French).

## Session type

```bash
echo $XDG_SESSION_TYPE
```

- **`x11`**: supported (global key through XInput2, pasting through XTest, active application through `_NET_ACTIVE_WINDOW`). The default dictation key is **right Ctrl**.
- **`wayland` with GNOME 48 or later** (Ubuntu 25.04 and later): supported through GNOME's “portals”. GNOME does not accept a single key such as right Ctrl: choose a combination with an ordinary key. To change it later: **Settings → Apps → Scriibe → Keyboard shortcuts**, then relaunch Scriibe. Pasting asks for permission to control the keyboard.
- **`wayland` with another desktop** (KDE, or an older GNOME): not supported yet. Open an X11 session: on many distributions the login screen's cogwheel offers an **Xorg** / **X11** session. **Ubuntu 25.10 and later** no longer have an X11 session with GNOME: install Xfce (`sudo apt update && sudo apt install -y xfce4`) and choose **Xfce Session** at login; GNOME stays installed.

## System dependencies

The microphone needs ALSA (present on common desktops); the window and the icon need WebKitGTK and AppIndicator (present by default on Ubuntu with GNOME). If the launch complains:

```bash
sudo apt install -y unzip
sudo apt install -y libasound2t64          # libasound2 on older versions
sudo apt install -y libwebkit2gtk-4.1-0 libayatana-appindicator3-1
```

On a GNOME without the AppIndicator extension (Fedora), the icon does not appear: reopen the page through the address shown by the terminal. Sounds are played by `paplay`, `pw-play` or `aplay` (the first one found); media pause goes through `pactl` and `playerctl`.

## The two local engines

Scriibe calls two external programs over HTTP: `whisper-server` (transcription) and `llama-server` (cleanup). The **Programs** and **Recommended models** cards in **Settings → Engines** offer them (`whisper-server (Linux)`: static whisper.cpp, AVX2; `llama-server (Linux, CPU)`: the official build repackaged by the CI), with size and source shown before you agree. They are stored in `~/.local/share/Scriibe/engines/`, with execute permission and `LD_LIBRARY_PATH` set by Scriibe. The processor must support AVX2 (any PC since about 2015). GGUF models are for cleanup, `ggml-….bin` files for Whisper.

### Building `whisper-server` yourself

If you prefer not to use the CI build:

```bash
sudo apt install -y build-essential cmake git
git clone --depth 1 https://github.com/ggml-org/whisper.cpp ~/whisper.cpp
cd ~/whisper.cpp && cmake -B build -DBUILD_SHARED_LIBS=OFF && cmake --build build -j --config Release --target whisper-server
mkdir -p ~/.local/share/Scriibe/engines/whisper && cp ~/whisper.cpp/build/bin/whisper-server ~/.local/share/Scriibe/engines/whisper/
```

(`BUILD_SHARED_LIBS=OFF`: a single file, no libraries alongside.) `llama-server` can also come from the distribution or from llama.cpp's releases.

## Online engines (fallback)

Quicker to set up, or useful on a modest machine, but paid by use: the voice goes to Deepgram, the text to OpenRouter to be cleaned up. Linux has no keychain support: keys are given through environment variables, **before** launching Scriibe, in the same terminal:

```bash
export DEEPGRAM_API_KEY='…your key…'
export OPENROUTER_API_KEY='…your key…'
~/scriibe/scriibe
```

## Terminals

In a terminal, pasting is done with Ctrl+Shift+V: Scriibe sends Ctrl+V, which does not work there; the text stays in the clipboard (and “Copy the last dictation” is in the icon menu).

## Manual update

The built-in installer (**Settings → Update → Check for a new version → Install and restart**) downloads `scriibe-linux-x64.zip` from the latest Release, replaces the program (the old one stays alongside as `scriibe.old` until the next launch) and relaunches Scriibe; the program's folder must be yours (as `~/scriibe` is). By hand, if it fails or from an older version:

1. Quit Scriibe: icon menu → Quit (or Ctrl+C in the terminal).
2. Download the latest `scriibe-linux-x64.zip` and run again:

```bash
mkdir -p ~/scriibe && unzip -o ~/Téléchargements/scriibe-linux-x64.zip -d ~/scriibe && chmod +x ~/scriibe/scriibe
```

3. Relaunch. Settings, history and dictionary are kept (stored in `~/.local/share/scriibe`).

## Building from source

Prerequisites: Rust, Node.js, Git, `libasound2-dev`, the WebKitGTK and AppIndicator headers.

```bash
cd ui && npm ci && npm run build && cd ..
cargo build --release
```

The executable is `target/release/scriibe`. Build `ui/dist` **before** `cargo build`: otherwise the interface is replaced by an “Interface absente” message. For a trial without touching your data: `SCRIIBE_DATA=/tmp/scriibe target/release/scriibe lancer --sans-navigateur`.

## For the maintainer: publishing a release

The `release-linux.yml` workflow builds `scriibe-linux-x64.zip` (on Ubuntu 22.04) and the Linux engines and adds them to the Release published by `tools/publier_macos.sh`. Wait for it to finish before asking a PC to update.
