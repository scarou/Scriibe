# Scriibe on Mac — guide for experienced users

🇫🇷 [Version française](INSTALLATION_MACOS_EXPERT.md)

Other guides: [step-by-step installation for beginners](INSTALLATION_MACOS.en.md) · [Windows](INSTALLATION_WINDOWS.en.md) · [Linux](INSTALLATION_LINUX.en.md)

This guide assumes you can use the Terminal. It gathers what is not in the beginners' guide: online engines, Homebrew in detail, manual update, building from source, dictating over a selection and publishing a release.

## Homebrew and the two local engines

Scriibe calls two external programs over HTTP, like Scribe does: `whisper-server` (transcription) and `llama-server` (cleanup).

```bash
brew install whisper-cpp
brew install llama.cpp
```

Scriibe looks for them in Homebrew's `PATH` (`/opt/homebrew/bin` on Apple silicon). The **Programs** card in **Settings → Engines** offers no download for Mac (it lists Windows and Linux programs): on a Mac the programs come from Homebrew. Models are downloaded from the **Recommended models** and **Add a model from Hugging Face** cards, or dropped by hand into `~/Library/Application Support/Scriibe/models/`. GGUF files are for cleanup, `ggml-….bin` files for Whisper.

## Online engines (fallback)

Quicker to set up, or useful on a modest machine, but pay-as-you-go: your voice goes to Deepgram and the text to OpenRouter to be cleaned up. You need an account and a key for each: paste each key under **Settings → Engines → Cloud keys (fallback)**. Keys are stored in the Mac's keychain and are never displayed.

## Manual update

The built-in installation (**Settings → Update → Check for a new version → Install and restart**) downloads the Release zip, checks the signature of the new application, replaces `Scriibe.app` (the old one stays alongside as `Scriibe.app.old` until the next launch) and reopens Scriibe. By hand, if it fails (for example `Scriibe.app` is not in a folder you can write to) or from an older version:

1. Quit Scriibe: menu bar icon → **Quit Scriibe**.
2. Download the zip of the latest version from `https://github.com/scarou/Scriibe/releases` (compare its number with the one in **Settings → Update**).
3. Unzip, drag **Scriibe** into **Applications**, **Replace**.
4. Reopen. Settings, history and dictionary are kept (stored separately). Permissions normally are too; otherwise grant them again (step 4 of the beginners' guide).

## Permissions and signature

Scriibe is signed with a local certificate (“Scribe Dev”), not with an Apple developer account: macOS shows the first-launch warning and, after a rebuild signed differently, may ask again for the permissions or for keychain access. The three permissions (Microphone, Input Monitoring, Accessibility) go to **`Scriibe.app`** launched by `open` or from the Finder. A program launched from another process (for example the `target/release/scriibe` binary launched by a tool) has none: see `docs/PIEGES.md`.

## Building from source

Requirements: Rust, Node.js, Xcode Command Line Tools.

```bash
cd ui && npm ci && npm run build && cd ..
cargo build --release
tools/faire_app_macos.sh --lancer
```

The script assembles `Scriibe.app` from `target/release/scriibe`, signs it with “Scribe Dev” and installs it in `~/Applications` (or in `SCRIIBE_INSTALL=path/Scriibe.app`). Build `ui/dist` **before** `cargo build`: otherwise the interface is replaced by an “Interface absente” message. The log is `~/Library/Application Support/Scriibe/scriibe.log`. See `docs/ESSAI_MACOS.md`.

## Dictating over a selection

If a word or a group of words is selected when you release the key, the dictated text replaces it **without a capital letter at the start and without a final period, comma or semicolon**, since it goes in the middle of a sentence (question marks, exclamation marks, colons and ellipses are kept). Names from your dictionary, acronyms and words like “iPhone” keep their capital. This works on a Mac; it is not available on Windows or Linux yet.

## For the maintainer: publishing a release

A release is built and signed on the author's Mac (“Scribe Dev” certificate), then published with one command:

```bash
tools/publier_macos.sh 0.2.0
```

The script requires an up-to-date build (`cd ui && npm ci && npm run build`, then `cargo build --release`), assembles `Scriibe.app`, zips it, **shows a preview** (name, version, size) and only publishes after a “oui” (yes). It creates a tag and a release on `scarou/Scriibe` with the zip attached; a workflow then adds the Windows file.
