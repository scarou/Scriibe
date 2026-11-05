# Installing Scriibe on a Mac — step by step

🇫🇷 [Version française](INSTALLATION_MACOS.md)

Other systems: [Windows](INSTALLATION_WINDOWS.en.md) · [Linux](INSTALLATION_LINUX.en.md)

For someone who receives Scriibe from somebody else (no programming knowledge needed). Allow 10 minutes with online engines, 30 minutes with engines running on the Mac (downloads included).

**What you need**: a Mac with Apple silicon (M1 or later), macOS 14 or later, an Internet connection, a microphone.

> Releases are published on the [Releases page](https://github.com/scarou/Scriibe/releases) (new ones are created with `tools/publier_macos.sh`, see the end of this file).
>
> **Status of this guide**: written from what has been tried on the author's own Mac (permissions, first dictation, engines). Steps 1 to 3 (downloading a published release, macOS's warning at first launch) **have not yet been tried on another Mac**: if a step does not go as written, note which one and report it.
>
> The Scriibe window can be switched between French and English (**Settings → Appearance → Interface language**); the labels below are the English ones.

## 1. Download Scriibe

1. Open the releases page: `https://github.com/scarou/Scriibe/releases` (a public page: no GitHub account is needed).
2. Under the latest version (the highest number, at the top of the page), click the file **`Scriibe-…-macos.zip`** in the “Assets” section. It lands in your **Downloads** folder.

## 2. Install

1. In the Finder, open **Downloads** and double-click the zip: **Scriibe** (a feather icon) appears next to it.
2. Drag **Scriibe** into the **Applications** folder in the Finder sidebar.

## 3. First launch (macOS warning)

Scriibe is not signed by Apple (that costs $99 a year), so macOS says so at first launch. This is normal and harmless for an application whose source you know.

1. In **Applications**, double-click **Scriibe**. macOS shows a message saying it cannot verify the developer: click **OK** (or **Done**).
2. Open **System Settings → Privacy & Security** and scroll down to the **Security** section: a line says “Scriibe” was blocked. Click **Open Anyway**, enter your Mac password if asked, then confirm.
3. The Scriibe window opens and a small icon appears in the **menu bar** (top right, near the clock). There is no icon in the Dock: this is intentional.

This warning only appears once.

## 4. Grant the three permissions

macOS asks for them when Scriibe needs them. Each time, go to **System Settings → Privacy & Security**, open the section named below, **turn on the Scriibe switch**, then **quit Scriibe** (menu bar icon → “Quit Scriibe”) and **launch it again**.

| Section | What it is for |
|---|---|
| **Microphone** | Hearing your voice. |
| **Input Monitoring** | Knowing when you hold the dictation key (right Option by default). |
| **Accessibility** (labelled differently on some macOS versions: it is the one that lets an app control the computer) | Pasting the text into the open application (Scriibe sends ⌘V) and noticing a selected text. Without it the text stays on the clipboard: press ⌘V to paste it yourself. |

The exact names of these sections vary a little between macOS versions and languages (the French guide uses “Microphone”, “Surveillance de l’entrée” and “Contrôle de l’appareil et accès aux données”).

## 5. Choose how Scriibe transcribes

In Scriibe: **Settings → Engines**. Two ways, your choice (you can change later).

**A. On the Mac (your voice and text stay on the Mac).** Requires **Homebrew** (`https://brew.sh`, follow the page). Then, in the **Terminal** application, one command after the other:

```bash
brew install whisper-cpp
```

```bash
brew install llama.cpp
```

Then, in Scriibe, the **Installation status**, **Programs** and **Recommended models** cards (same tab, **Settings → Engines**) tell you what is missing (a Whisper model, a cleanup model) and offer the downloads, showing their size and source before asking for your consent. Finally choose **Whisper** (transcription) and **llama.cpp** (cleanup) under **Settings → Engines**. Only downloads, which you accept one by one, and the update alert, if you turn it on, go through the Internet.

**B. Online (as a fallback).** Quicker to set up and on a modest machine, but paid; your voice goes to Deepgram and the text to OpenRouter to be cleaned up. You need an account and a key for each (both services are pay-as-you-go, a few cents per dictation): paste each key under **Settings → Engines → Cloud keys (fallback)**. Keys are stored in the Mac's keychain and are never displayed.

## 6. Try it

1. Open **TextEdit** (or Notes) and click in an empty page.
2. **Hold the right Option key** (bottom right of the keyboard), say a sentence, **release**: the text appears.
3. If nothing happens: in Scriibe, **Settings → Measurements and trials** → card “Report to paste to me” → “Copy the report”, then send it to the person who gave you Scriibe.

## Dictating over a selection

If a word or a group of words is selected when you release the key, the dictated text replaces it **without a capital letter at the start and without a final period, comma or semicolon**, since it goes in the middle of a sentence (question marks, exclamation marks, colons and ellipses are kept). Names from your dictionary, acronyms and words like “iPhone” keep their capital. This works on a Mac; it is not available on Windows or Linux yet.

## Installing an update

**From the version that contains the built-in update (S15)**: in Scriibe, **Settings → Update** → **Check for a new version** → **Install and restart**, then confirm. Scriibe downloads the Release zip, checks the signature of the new application, replaces `Scriibe.app` (the old one stays alongside as `Scriibe.app.old` until the next launch) and reopens by itself. Nothing to do by hand.

**From an older version, or if the built-in installation fails** (for example `Scriibe.app` is not in a folder you can write to):

1. Quit Scriibe: menu bar icon → **Quit Scriibe**.
2. Go back to `https://github.com/scarou/Scriibe/releases` and download the zip of the **latest version** (compare its number with the one in Scriibe's **Settings → Update** tab).
3. Double-click the zip in **Downloads**, then drag **Scriibe** into **Applications** and click **Replace**.
4. Open Scriibe. Settings, history and dictionary are kept (they are stored separately). Permissions are normally kept; if macOS asks again, grant them again as in step 4.

The **Update** tab warns you when a newer version exists (see “Automatic alert” in that tab).

---

## For the maintainer: publishing a release (author's Mac)

A release is built and signed on the author's Mac (“Scribe Dev” certificate), then published with one command:

```bash
tools/publier_macos.sh 0.2.0
```

The script requires an up-to-date build (`cd ui && npm ci && npm run build`, then `cargo build --release`), assembles `Scriibe.app`, zips it, **shows a preview** (name, version, size) and only publishes after a “oui” (yes). It creates a tag and a release on `scarou/Scriibe` with the zip attached; a workflow then adds the Windows file.
