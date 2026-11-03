# Installing Scriibe on Linux — step by step

🇫🇷 [Version française](INSTALLATION_LINUX.md)

Other systems: [Windows](INSTALLATION_WINDOWS.en.md) · [macOS](INSTALLATION_MACOS.en.md)

> **State of this guide (S15)**: the Linux program is **experimental**. The global key, pasting and the active window were checked on a virtual screen (`docs/ESSAI_LINUX.md`, in French); trying it on a real desktop is under way on Ubuntu 26.04. The steps below are to be confirmed by that trial: if one does not go as written, note which one and say so.

**What you need**: a 64-bit (x86-64) Linux PC with an **X11 session**, an Internet connection, a microphone. **Wayland is not supported** (see step 1).

## 1. Check that you are on X11

Open a **terminal** (Ctrl+Alt+T) and type:

```bash
echo $XDG_SESSION_TYPE
```

- **`x11`**: good, go to step 2.
- **`wayland`**: Scriibe cannot see the dictation key nor paste into other applications. You need to open an X11 session:
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

**B. On the PC (nothing leaves your computer).** You need `whisper-server` (transcription) and, for the cleanup, `llama-server`. There is no ready-made Linux `whisper-server`, and Scriibe's downloads do not offer them on Linux yet: see [ESSAI_LINUX.md](ESSAI_LINUX.md) (French) for building `whisper.cpp` step by step. The program goes in `~/.local/share/Scriibe/engines/whisper/` (or next to `scriibe`).

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
