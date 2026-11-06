<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="logo/scriibe-logotype-windows-sombre.svg">
    <img src="logo/scriibe-logotype-windows.svg" alt="Scriibe" width="420">
  </picture>
</p>

# Scriibe

🇫🇷 [Version française](README.md)

**Voice dictation for Windows, macOS and Linux.** Hold a key, speak, release: your speech is transcribed, cleaned up and pasted into whatever application is open — an email, a chat, a document, a search box.

Scriibe is the cross-platform successor of [Scribe](https://github.com/scarou/scribe) (macOS, Swift), which remains the **reference for behaviour**. The goal is the same experience on all three systems, from a single code base (Rust + Tauri, with a Svelte interface).

> Most of the project's documentation (architecture, changelog, roadmap, notes on pitfalls) is written in French. The user interface, the in-app Help and the installation guides are available in both French and English.

## What it does

- **Hold, speak, release.** The default key is right Ctrl on Windows and right Option on a Mac; it can be changed (a single key or a combination such as Option + F9, on every system). A short sound confirms the start and the end, and an optional on-screen indicator shows the state and the microphone level.
- **Transcription** by [Whisper](https://github.com/ggml-org/whisper.cpp) running on your own computer, or by [Deepgram](https://deepgram.com) online.
- **Cleanup**: a language model removes the “uh”s, adds punctuation and, depending on the mode, rephrases. It runs locally with [llama.cpp](https://github.com/ggml-org/llama.cpp), [LM Studio](https://lmstudio.ai) or [Ollama](https://ollama.com), or online with [OpenRouter](https://openrouter.ai).
- **Modes**: *Literal* (no model, only your dictated signs and your dictionary), *Simple*, style modes, and **modes of your own** (an instruction and a few examples given to the model).
- **Translation**: add a second key while you hold the dictation key (Shift or ⌘ by default) and the text is pasted translated into English, Spanish, German, Italian or Brazilian Portuguese.
- **Dictionary**: words to write exactly (names, brands, jargon) and “heard → written” corrections.
- **Dictated signs and lists**: say “open parenthesis”, “new line”, “new paragraph”, “dash”, “point one”… and get the sign (these signs are French-specific, as in Scribe).
- **Dictating over a selection** (macOS): if a word or group of words is selected, the dictated text replaces it without a capital letter at the start nor a final period, so it fits in the middle of a sentence.
- **History** kept only on your computer, with filters (text, app, period), and **privacy controls**: keep only the statistics, or only the last few days.
- **Dashboard**: words dictated, speed, time saved compared with typing, daily chart, where you dictate.
- **Bench** to measure the speed and quality of your engines and modes, with comparison between runs.
- Pauses music or video while you speak, restores your clipboard after pasting, light/dark/system appearance, French and English interface, an in-app **Help** written for non-technical users.
- **Updates**: Scriibe tells you when a newer release exists, and on Windows and on a Mac it can install it for you — no account, no token.

## Install

Step by step, for someone who has never built a program:

- **Mac** (Apple silicon, macOS 14 or later): [step by step for beginners](docs/INSTALLATION_MACOS.en.md) · [français](docs/INSTALLATION_MACOS.md) · [guide for experienced users](docs/INSTALLATION_MACOS_EXPERT.en.md) · [français](docs/INSTALLATION_MACOS_EXPERT.md)
- **Windows** (10 or 11, 64-bit): [docs/INSTALLATION_WINDOWS.en.md](docs/INSTALLATION_WINDOWS.en.md) · [français](docs/INSTALLATION_WINDOWS.md)
- **Linux** (X11; Wayland is not supported; experimental): [docs/INSTALLATION_LINUX.en.md](docs/INSTALLATION_LINUX.en.md) · [français](docs/INSTALLATION_LINUX.md). The `scriibe-linux-x64.zip` file is added to the Releases from the next published version; the state of the trial is in [docs/ESSAI_LINUX.md](docs/ESSAI_LINUX.md) (French), and you can also build it yourself (see below).

Downloads are on the [Releases](https://github.com/scarou/Scriibe/releases) page. The local engines (Whisper, llama.cpp) are installed or downloaded from within the application, with your consent for every download; the online engines (Deepgram, OpenRouter) need a personal key.

### Build it yourself

Prerequisites: [Rust](https://rustup.rs) and [Node.js](https://nodejs.org). The order matters (the interface is embedded in the executable):

```bash
cd ui && npm ci && npm run build && cd ..
cargo build --release
```

The executable is `target/release/scriibe` (`scriibe.exe` on Windows). On a Mac, `tools/faire_app_macos.sh` turns it into a signed `Scriibe.app` (it expects a local code-signing certificate; see the script's header). On Linux the build needs at least `libasound2-dev`.

## Principles

- **Local first**: transcription by `whisper-server` (whisper.cpp) and cleanup by `llama-server`, LM Studio or Ollama. The cloud (Deepgram, OpenRouter) is a choice, never an obligation. Nothing is downloaded without your consent, and keys are stored in the system's credential store (Keychain, Credential Manager).
- **Behaviour is measured**: the text logic is checked against reference cases (`fixtures/`) taken from Scribe; deliberate differences are listed in [docs/ECARTS.md](docs/ECARTS.md) (French).
- **Open**: MIT licence; interface in French and English.

## For developers

The workspace is split into `crates/core` (pure text logic), `crates/engines` (Whisper and model processes, HTTP clients), `crates/platform` (per-system layer: key, paste, microphone, sounds, secrets), `crates/pipeline` (the dictation controller) and `crates/cli` (the `scriibe` executable, local web server, Tauri window). The interface is in `ui/` (Svelte 5 + TypeScript). Before a commit: `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace`, and `cd ui && npm run check && npm test && npm run build`. See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) (French) for the plan, [ROADMAP.md](ROADMAP.md) for the steps and [CHANGELOG.md](CHANGELOG.md) for the history.

## Licence

[MIT](LICENSE)
