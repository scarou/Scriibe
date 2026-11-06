# Scriibe on Windows — guide for experienced users

🇫🇷 [Version française](INSTALLATION_WINDOWS_EXPERT.md)

Other guides: [step-by-step installation for beginners](INSTALLATION_WINDOWS.en.md) · [macOS](INSTALLATION_MACOS_EXPERT.en.md) · [Linux](INSTALLATION_LINUX.en.md)

This guide gathers what is not in the beginners' guide: the version built by the CI (route B), online engines, the engines in detail, the manual update, building from source and publishing a release. What has been tried on the author's PC is in `docs/ESSAI_WINDOWS.md` (in French).

## Route B: latest version built by GitHub

Useful between two releases, or if the zip is not yet attached to the latest one (the `release-windows.yml` workflow adds it a few minutes after publication).

1. Sign in to GitHub (free account: `https://github.com/signup` if needed; GitHub only lets signed-in people download these files).
2. Open `https://github.com/scarou/Scriibe/actions/workflows/ci.yml`.
3. Click the first run marked with a **green tick**, scroll down to **Artifacts**, click **`scriibe-windows-x64`**. The zip lands in **Downloads**. GitHub deletes these files after 90 days: if the list is empty, ask for a new version.

## The two local engines

Scriibe calls two external programs over HTTP, like Scribe: `whisper-server` (transcription) and `llama-server` (cleanup). The **Programs** card in **Settings → Engines** downloads them one by one, with size and source shown before you agree:

- `whisper-server (Windows)`: whisper.cpp with its DLLs;
- `llama-server (Windows, CPU)` or `llama-server (Windows, Vulkan graphics card)`. The Vulkan version replaces the CPU version: cleanup runs on the graphics card (compare with the CPU timings; to go back, download the CPU version again). The **Dashboard** wizard measures the machine and suggests the matching chain with measured timings.

Models are downloaded from the **Recommended models** and **Add a model from Hugging Face** cards, or dropped by hand into Scriibe's models folder. GGUF files are for cleanup, `ggml-….bin` files for Whisper (**Settings → Engines → Choose a file…** to point at a file elsewhere). Cleanup can also go through LM Studio or Ollama (list of the server's models, adjustable address).

## Online engines (fallback)

Quicker to set up, or useful on a modest machine, but paid by use (a few cents per dictation): the voice goes to Deepgram, the text to OpenRouter to be cleaned up. You need an account and a key for each: paste each key in **Settings → Engines → Cloud keys (fallback)**. Keys are stored in the Windows Credential Manager and never displayed.

## Manual update

The built-in installer (**Settings → Update → Check for a new version → Install and restart**) downloads `scriibe-windows-x64.zip` from the latest Release (no account or token), replaces `scriibe.exe` and relaunches Scriibe; the old program stays alongside as `scriibe.exe.old` until the next launch. By hand, if it fails or from an older version:

1. Quit Scriibe: right-click the icon near the clock → **Quit**.
2. Download the zip of the latest version (Releases, or route B above).
3. Extract it **into the same folder** (`C:\scriibe`), agreeing to **replace** `scriibe.exe`.
4. Relaunch `scriibe.exe`. Settings, history and dictionary are kept (stored separately).

## SmartScreen warning

Scriibe is not signed (code-signing certificates are paid): SmartScreen announces it on first launch (**More info → Run anyway**). If that button is missing: right-click `scriibe.exe` → **Properties** → **Unblock**.

## Dictation key

By default the **right Ctrl** key held (**Shift** added: translation; another key: cancel), changeable in **Settings → Dictation** (up to three keys). The Windows keyboard hook receives nothing while Scriibe's own window has focus: the page then relays its keys; “Test my key” is done in another window. See `docs/ESSAI_WINDOWS.md`.

## Building from source

Prerequisites: Rust (`x86_64-pc-windows-msvc` target), Node.js, Git. In a working folder (for example `C:\scriibe-dev`):

```powershell
git pull
cd ui; npm ci; npm run build; cd ..
cargo build --release
```

The executable is `target\release\scriibe.exe`. Build `ui\dist` **before** `cargo build`: otherwise the interface is replaced by an “Interface absente” message. Close the running version before rebuilding. `C:\scriibe` keeps the `scriibe.exe` installed by the in-window update.

## For the maintainer: publishing a release

Set the number in `Cargo.toml`, then `tools/publier_macos.sh x.y.z` (on the Mac) publishes the Release; the `release-windows.yml` workflow then adds `scriibe-windows-x64.zip`. Wait for it to finish before asking a PC to update.
