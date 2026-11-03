# Installing Scriibe on Linux — step by step

🇫🇷 [Version française](INSTALLATION_LINUX.md)

Other systems: [Windows](INSTALLATION_WINDOWS.en.md) · [macOS](INSTALLATION_MACOS.en.md)

> **State of this guide (S15)**: the Linux program is **experimental**. The global key, pasting and the active window were checked on a virtual screen (`docs/ESSAI_LINUX.md`, in French); trying it on a real desktop is under way on Ubuntu 26.04. The steps below are to be confirmed by that trial: if one does not go as written, note which one and say so.

**What you need**: a 64-bit (x86-64) Linux PC with an **X11** session, or **GNOME 48 or later on Wayland** (Ubuntu 25.04 and later; from the version released after 0.2.2), an Internet connection, a microphone (see step 1).

## 1. Check the session type

Open a **terminal** (Ctrl+Alt+T) and type:

```bash
echo $XDG_SESSION_TYPE
```

- **`x11`**: good, go to step 2.
- **`wayland`** with **GNOME 48 or later** (Ubuntu 26.04): it works, through GNOME's “portals” (tried on 2026-11-03). On first launch, GNOME opens two windows, only once:
  - a **keyboard shortcut** to dictate: GNOME does not accept a single key such as Right Ctrl, pick a combination with an ordinary key (for example Ctrl + Left arrow, or Ctrl + Alt + Space), then confirm. To change it later: **Settings → Apps → Scriibe → Global Shortcuts**, then restart Scriibe;
  - on the first paste, permission to **control the keyboard** (this is what lets Scriibe paste): accept it.
- **`wayland`** with another desktop (KDE, or an older GNOME): not supported yet. You need to open an X11 session:
  - On many distributions, at the login screen, click the **small gear** next to the sign-in button and pick an **Xorg** / **X11** session (for example “Ubuntu on Xorg”).
  - **Ubuntu 25.10 and later (including 26.04)** no longer have an X11 session with the GNOME desktop. Install another desktop, for example Xfce (which runs on X11): `sudo apt update && sudo apt install -y xfce4`, then, at the login screen, choose **Xfce Session** with the gear. GNOME stays installed.

## 2. Download Scriibe

1. Open `https://github.com/scarou/Scriibe/releases` (public page: no account needed).
2. Under the latest version (highest number), click the **`scriibe-linux-x64.zip`** file in the “Assets” section (if it is not there yet, wait a few minutes: GitHub adds it after the release is published). It lands in **Downloads**.

## 3. Install

In the terminal, one command after the other:

```bash
mkdir -p ~/scriibe && unzip -o ~/Downloads/scriibe-linux-x64.zip -d ~/scriibe
```

```bash
chmod +x ~/scriibe/scriibe
```

(If your folder is called `Téléchargements` rather than `Downloads`, change the name in the first command. If `unzip` is not found: `sudo apt install -y unzip`.)

The microphone needs the ALSA library, present on common desktops. If launching complains about `libasound.so.2`: `sudo apt install -y libasound2t64` (or `libasound2` on older versions).

## 4. Launch

```bash
~/scriibe/scriibe
```

The terminal stays busy (it is Scriibe's log) and **Scriibe's page opens in the browser**. There is no icon near the clock and no window of its own for Scriibe on Linux yet. To quit: **Ctrl+C** in the terminal. To reopen the page: open the address shown by the terminal.

## 5. Choose how Scriibe transcribes

In the page: **Settings → Engines**.

**A. Online.** Your voice goes to Deepgram, and the text goes to OpenRouter to be cleaned up. There is no keychain on Linux yet: keys are given through environment variables, **before** launching Scriibe, in the same terminal:

```bash
export DEEPGRAM_API_KEY='…your key…'
export OPENROUTER_API_KEY='…your key…'
~/scriibe/scriibe
```

**B. On the PC (nothing leaves your computer).** Under **Settings → Engines**, the **Programs** and **Recommended models** cards offer, showing size and source before asking for your consent: `whisper-server (Linux)` and `llama-server (Linux, CPU)` (built by GitHub for Linux x86-64 and added to every Release), then a Whisper model and a cleanup model. Accept them one by one, then choose **Whisper** and **llama.cpp**. *(These cards come with the version after 0.2.2; with 0.2.2 you have to build `whisper-server` yourself: see [ESSAI_LINUX.md](ESSAI_LINUX.md), in French.)* The programs are stored in `~/.local/share/Scriibe/engines/`. The processor must support AVX2 (any PC since about 2015).

## 6. Try it

1. Open a text editor and click in an empty page.
2. **Hold the right Ctrl key** (the default dictation key, changeable under **Settings → Dictation**), say a sentence, **release**: the text appears.
3. If nothing happens: **Settings → Measurements and trials** → card “Report to paste to me” → “Copy the report”, then send it to the person who gave you Scriibe.

In a **terminal**, pasting is done with Ctrl+Shift+V: Scriibe sends Ctrl+V, which does not work there; the text stays on the clipboard.

## Installing an update

The built-in update (**Settings → Update**) **warns** about a new version but does not install it on Linux yet. By hand:

1. Quit Scriibe: Ctrl+C in the terminal.
2. Download the latest `scriibe-linux-x64.zip` as in step 2 and redo step 3 (the command replaces the old program).
3. Launch again. Settings, history and dictionary are kept (they are stored in `~/.local/share/scriibe`).
