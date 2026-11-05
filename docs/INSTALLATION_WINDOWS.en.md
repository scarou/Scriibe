# Installing Scriibe on Windows — step by step

🇫🇷 [Version française](INSTALLATION_WINDOWS.md)

Other systems: [macOS](INSTALLATION_MACOS.en.md) · [Linux](INSTALLATION_LINUX.en.md)

For someone who receives Scriibe from somebody else (no programming knowledge needed). Allow 10 minutes with online engines, 30 minutes with engines running on the PC (downloads included).

**What you need**: Windows 10 or 11 (64-bit), an Internet connection, a microphone. A free GitHub account is only needed for route B in step 1.

> **Status of this guide**: written from what has been tried on the author's own PC (`docs/ESSAI_WINDOWS.md`, in French). The Windows file is added to every release by the CI a few minutes after the release is published (version 0.1.0, published before, has none: use route B). The steps have not yet been tried by anyone else: if one does not go as written, note which one and report it.
>
> The Scriibe window can be switched between French and English (**Settings → Appearance → Interface language**); the labels below are the English ones.

## 1. Download Scriibe

**Route A — published release (the simplest, no account).**

1. Open `https://github.com/scarou/Scriibe/releases`.
2. Under the latest version (highest number), click the file **`scriibe-windows-x64.zip`** in the “Assets” section (if it is not there yet, wait a few minutes or use route B).

**Route B — latest build made by GitHub (meanwhile).**

1. Sign in to GitHub (a free account: `https://github.com/signup` if needed; GitHub only lets signed-in people download these files).
2. Open `https://github.com/scarou/Scriibe/actions/workflows/ci.yml`.
3. Click the first run marked with a **green check** (the most recent one), scroll down to the **Artifacts** section and click **`scriibe-windows-x64`**. The zip arrives in **Downloads**. (GitHub deletes these files after 90 days: if the list is empty, ask for a new version.)

## 2. Install

1. In File Explorer, open **Downloads**, right-click the zip → **Extract All…**.
2. Choose a simple folder, for example **`C:\scriibe`**, then **Extract**. It contains `scriibe.exe`.

## 3. First launch (Windows warning)

Scriibe is not code-signed (that is expensive), so Windows SmartScreen says so at first launch. This is normal for an application whose source you know.

1. Double-click **`scriibe.exe`**.
2. If a blue “Windows protected your PC” window appears: click **More info**, then **Run anyway**.
3. A black window (the log) opens, a **round icon appears near the clock** (grey when idle) and **the Scriibe window opens**. If it does not open: right-click the icon → **Open Scriibe**. Closing the window does not quit Scriibe: to quit it, right-click the icon → **Quit Scriibe**.

## 4. Allow the microphone

If Windows asks for microphone access, accept. Otherwise: **Settings → Privacy & security → Microphone**, and turn on “Let desktop apps access your microphone”.

## 5. Choose how Scriibe transcribes

In Scriibe: **Settings → Engines** (your choice, can be changed later).

**A. On the PC (your voice and text stay on the PC).** In the same tab (**Settings → Engines**), the **Installation status**, **Programs** and **Recommended models** cards say what is missing (the `whisper-server` and `llama-server` programs, models) and offer each download with its **size and source before asking for your consent**. Accept them one by one. Then choose **Whisper** (transcription) and **llama.cpp** (cleanup) under **Settings → Engines**. Only downloads, which you accept one by one, and the update alert, if you turn it on, go through the Internet.

**B. Online (as a fallback).** Quicker on a modest machine, but paid; your voice goes to Deepgram and the text to OpenRouter to be cleaned up. You need an account and a key for each (pay-as-you-go services, a few cents per dictation): paste each key under **Settings → Engines → Cloud keys (fallback)**. Keys are stored in the Windows Credential Manager and are never displayed.

## 6. Try it

1. Open Notepad and click in an empty page.
2. **Hold the right Ctrl key** (the default dictation key, which can be changed under **Settings → Dictation**), say a sentence, **release**: the text appears.
3. If nothing happens: **Settings → Measurements and trials** → card “Report to paste to me” → “Copy the report”, then send it to the person who gave you Scriibe.

## Installing an update

1. Quit Scriibe: right-click the icon near the clock → **Quit Scriibe**.
2. Download the latest version as in step 1 and extract the zip **into the same folder** (`C:\scriibe`), accepting to **replace** `scriibe.exe`.
3. Launch `scriibe.exe` again. Settings, history and dictionary are kept (they are stored separately).

Scriibe's **Settings → Update** tab can also do it for you, with no account and no token: **Check for a new version** shows the latest published version, its date and the download size; **Install and restart** asks for confirmation, downloads, replaces `scriibe.exe` and restarts Scriibe. The old program stays next to it as `scriibe.exe.old` until the next launch.
