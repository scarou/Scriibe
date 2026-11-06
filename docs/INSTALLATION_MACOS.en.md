# Installing Scriibe on a Mac — step by step, for beginners

🇫🇷 [Version française](INSTALLATION_MACOS.md)

Other systems: [Windows](INSTALLATION_WINDOWS.en.md) · [Linux](INSTALLATION_LINUX.en.md)

**Comfortable with the Terminal, or want online engines, building from source or a manual update?** See the [guide for experienced users](INSTALLATION_MACOS_EXPERT.en.md). This guide does not need any of that.

This guide is written for someone who has never installed this kind of program. No computer knowledge is needed. Do the steps **in order**, one at a time. Each step ends with a sentence “**It worked when…**”: if that is not the case, do not move on to the next step, go to “If it does not work” at the end.

The Scriibe window can be switched between French and English (**Settings → Appearance → Interface language**); the labels below are the English ones.

**What you need**: a Mac with Apple silicon (M1 or later), macOS 14 or later, an Internet connection, a microphone (the Mac's own is enough). **Time**: about 30 minutes, much of it waiting for downloads.

> **Status of this guide**: written from what has been tried on its author's own Mac. It has **not yet been followed to the letter by a beginner on a brand-new Mac** (steps 1 to 3 and the Homebrew installation in step 5 in particular). If a step does not go as written, note which one and the message shown, and tell the person who gave you Scriibe.

## Step 0 — Check your Mac

1. Click the Apple menu  at the top left of the screen, then **About This Mac**.
2. Look at the **Chip** line: it must start with **Apple M** (for example “Apple M1”). Also look at the **macOS** version: 14 or later.

**It worked when** the chip is an “Apple M…” and macOS is version 14 or later. If the chip says “Intel”, Scriibe does not work on this Mac yet: stop here.

## Step 1 — Download Scriibe

1. Open your browser (Safari, for example) and go to `https://github.com/scarou/Scriibe/releases`. It is a public page: you do not need an account.
2. Find the latest version: it is the one **at the very top of the page** (the highest number).
3. Under that version, look for the **Assets** section. Click the file whose name ends in **`-macos.zip`** (it starts with “Scriibe”). Do not click “Source code”.
4. The file lands in your **Downloads** folder.

**It worked when** the file `Scriibe-…-macos.zip` is in your **Downloads** folder.

## Step 2 — Install Scriibe

1. Open the **Finder** (the blue and white face icon in the Dock at the bottom of the screen), then click **Downloads** in the left column.
2. Double-click the zip file: a **Scriibe** icon (a feather) appears next to it.
3. Drag this **Scriibe** icon onto the **Applications** folder in the left column of the Finder (hold the click, move, release).

**It worked when** you see **Scriibe** when you open the **Applications** folder.

## Step 3 — Open Scriibe for the first time

macOS shows a warning the first time, because Scriibe is not signed by Apple. This is normal, and it only happens once.

1. In **Applications**, double-click **Scriibe**. macOS shows a message saying it cannot verify the developer. Click **OK** (or **Done**). Scriibe does not open: this is expected.
2. Click the Apple menu , then **System Settings**. In the left column, click **Privacy & Security**.
3. Scroll down to the **Security** section. A line says “Scriibe” was blocked. Click **Open Anyway**.
4. If the Mac asks for your password (your session's), enter it. Confirm by clicking **Open**.

**It worked when** the Scriibe window opens **and** a small icon appears at the top right of the screen, near the clock (this is the “menu bar”). There is no Scriibe icon in the Dock: this is intentional.

## Step 4 — Grant the three permissions

Scriibe needs three permissions. For **each** one, do this:

1. Click the Apple menu , **System Settings**, then **Privacy & Security**.
2. Click the section named in the table below.
3. In the list, **turn on the Scriibe switch**. If Scriibe is not in the list, it usually appears after macOS's first request; otherwise click the **+** at the bottom of the list and choose Scriibe in **Applications**.
4. Click the Scriibe icon in the menu bar, then **Quit Scriibe**. Open it again (double-click Scriibe in **Applications**). Without this restart the permission is not taken into account.

| Section | What it is for |
|---|---|
| **Microphone** | Hearing your voice. |
| **Input Monitoring** | Knowing when you hold the dictation key (the **right Option** key). |
| **Accessibility** (on some macOS versions it is labelled differently: it is the one that lets an app control the computer) | Pasting the text where you are writing, and noticing a selected text. Without it the text stays waiting: **⌘V** pastes it by hand. |

macOS may also show a request window of its own: click **Allow**, then quit and reopen Scriibe as above.

**It worked when** the three Scriibe switches are on and you have reopened Scriibe after the last one.

## Step 5 — Install two small programs

Scriibe transcribes your voice with two programs that run **on your Mac** (your voice and text do not leave the Mac). On a Mac they are installed through the **Terminal**, a window where you type commands. You do not have to understand anything: just **copy** each command and **paste** it.

**How to open the Terminal**: press **⌘ (Command) and Space** together, type **Terminal**, then press **Return**. A white or black window opens.

**How to run a command**: select the command in this guide, copy it (**⌘C**), click inside the Terminal window, paste (**⌘V**), then press **Return**. Wait for the Terminal to be “done”: it is when a new line ends with the **%** sign and the cursor waits. Do not close the window before.

### 5a — Check whether Homebrew is already there

Homebrew is a program that installs other programs. Paste this command, then Return:

```bash
brew --version
```

- If the Terminal answers with a number (for example “Homebrew 4…”), **go straight to step 5c**.
- If it answers “**command not found: brew**”, do step 5b.

### 5b — Install Homebrew (only if step 5a answered “command not found”)

1. Open `https://brew.sh` in the browser. The command to copy is shown at the top of the page, under “Install Homebrew”. It is also reproduced here:

```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

2. Paste it into the Terminal, then Return.
3. The Terminal asks for a **password**: it is your Mac session's. **Nothing appears as you type**, not even dots: this is normal. Type it, then Return.
4. It then asks you to press **Return** to continue (“Press RETURN”). Do so. macOS may offer to install “command line developer tools”: accept, and wait until it finishes (several minutes).
5. At the end, Homebrew prints a text containing the words **Next steps**. Then run these two commands, one after the other (paste, Return, then paste the next, Return):

```bash
echo 'eval "$(/opt/homebrew/bin/brew shellenv)"' >> ~/.zprofile
```

```bash
eval "$(/opt/homebrew/bin/brew shellenv)"
```

6. Redo step 5a: this time `brew --version` must answer with a number.

### 5c — Install the transcription program

```bash
brew install whisper-cpp
```

Wait for it to finish (a few minutes; many lines scroll by, this is normal).

### 5d — Install the cleanup program

```bash
brew install llama.cpp
```

Wait for it to finish, the same way.

**It worked when** both commands have finished (the **%** sign is back) without the word **Error** in red in the last lines. You can close the Terminal.

## Step 6 — Download the models in Scriibe

“Models” are the files that hold the intelligence of transcription and cleanup. Scriibe downloads them for you and **downloads nothing without your consent**: before each download it shows the name, the size and the source.

1. Open Scriibe (menu bar icon → **Open Scriibe**, or the window is already open).
2. Click **Settings**, then the **Engines** tab. Find the **Installation status** card: it says what is missing.
3. Further down, in the **Recommended models** card, find the **Whisper “base” model**. Click **See the size and source**, read, then click **Download**. Wait for the progress bar to finish.
4. In the same card, do the same for a cleanup model: **“Qwen 2.5 1.5 B”** (about 1 GB) is fine to start with.
5. **Ignore the “Programs” card**: it mostly offers programs for Windows and Linux, already replaced here by step 5.
6. Higher up in the same **Engines** tab, choose **Whisper** for transcription and **llama.cpp** for cleanup.

**It worked when** the **Installation status** card no longer reports anything missing.

## Step 7 — Try it

1. Open **TextEdit** (look for it with **⌘ + Space**, like the Terminal) and choose **File → New**: an empty page opens. Click in it to put the cursor there.
2. **Hold the right Option key** (bottom right of the keyboard, to the right of the space bar). A small indicator appears: wait until it shows “Listening…”.
3. Say a complete sentence, for example “Hello, I am testing Scriibe.”
4. **Release** the key. After a second or two, the text appears on the page.

**It worked when** your sentence is written in TextEdit.

## If it does not work

| What you see | What to do |
|---|---|
| Nothing happens when you hold the key | Check the **Input Monitoring** permission (step 4), then quit and reopen Scriibe. |
| The indicator shows, but no text appears | Check the **Accessibility** permission (step 4). Meanwhile the text is ready to paste: press **⌘V**. |
| The indicator stays stuck waiting although you released the key | Most of the time, press the key again, then release it: the indicator unblocks. |
| The text is empty or wrong | Check the **Microphone** permission (step 4) and that you downloaded a Whisper model (step 6). |
| Anything else | In Scriibe: **Settings → Measurements and trials**, card **Report to paste to me**, button **Copy the report**. Paste it into a message to the person who gave you Scriibe: the report does not contain the text of your dictations. |

## Later: installing an update

In Scriibe: **Settings → Update** → **Check for a new version** → **Install and restart**, then confirm. Scriibe reopens by itself. Settings, history and dictionary are kept. If this fails, see the [guide for experienced users](INSTALLATION_MACOS_EXPERT.en.md) (manual update).
