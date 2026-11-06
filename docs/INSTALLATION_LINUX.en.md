# Installing Scriibe on Linux — step by step, for beginners

🇫🇷 [Version française](INSTALLATION_LINUX.md)

Other systems: [Windows](INSTALLATION_WINDOWS.en.md) · [macOS](INSTALLATION_MACOS.en.md)

**Comfortable with the terminal, or want Xfce/X11, online engines, building from source or a manual update?** See the [guide for experienced users](INSTALLATION_LINUX_EXPERT.en.md). This guide does not need any of that.

This guide is written for someone who has never installed this kind of program. No computer knowledge is needed, but you will have to **copy a few commands into a terminal**: it is explained each time. Do the steps **in order**, one at a time. Each step ends with a sentence “**It worked when…**”: if that is not the case, do not move on to the next step, go to “If it does not work” at the end.

The Scriibe window can be switched between French and English (**Settings → Appearance → Interface language**); the labels below are the English ones.

**What you need**: a 64-bit (x86-64) PC running **Ubuntu 25.04 or later with the GNOME desktop** (as Ubuntu 26.04 does), an Internet connection, a microphone. **Time**: about 30 minutes, much of it waiting for downloads. No account is needed.

> **Status of this guide**: written from what has been tried on its author's own PC (Ubuntu 26.04, `docs/ESSAI_LINUX.md`, in French). The Linux program is still **experimental**, and this guide has **not yet been followed to the letter by a beginner on a brand-new PC**. If a step does not go as written, note which one and the message shown, and tell the person who gave you Scriibe.

**How to copy a command into the terminal**: select the command in this guide, copy it (Ctrl+C), click in the terminal, paste it with **Ctrl+Shift+V** (not Ctrl+V), then press **Enter**. One command at a time.

## Step 0 — Check your PC

1. Open a **terminal**: press **Ctrl+Alt+T**.
2. Copy this command into it, then Enter:

```bash
echo $XDG_SESSION_TYPE; uname -m
```

**It worked when** the answer is **`wayland`** or **`x11`**, then **`x86_64`** (on two lines). If the second line is anything else (for example `aarch64`), Scriibe does not work on this PC: stop here. If you use a desktop other than GNOME on Wayland (KDE, for example), see the [guide for experienced users](INSTALLATION_LINUX_EXPERT.en.md) (X11 session).

## Step 1 — Download Scriibe

1. Open your browser and go to `https://github.com/scarou/Scriibe/releases`. It is a public page: you do not need an account.
2. Find the latest version: it is the one **at the very top of the page** (the highest number).
3. Under that version, look for the **Assets** section (click it if it is collapsed). Click the file **`scriibe-linux-x64.zip`**. Do not click “Source code”.
4. The file lands in your **Downloads** folder.

**It worked when** the file `scriibe-linux-x64.zip` is in your **Downloads** folder. If it is missing from the “Assets” list, wait a few minutes (it is added shortly after publication) and reload the page.

## Step 2 — Install Scriibe

In the terminal (**Ctrl+Alt+T** if it is closed), copy these commands **one by one**.

1. This first command creates a `scriibe` folder in your home folder and puts the program in it:

```bash
mkdir -p ~/scriibe && unzip -o ~/Téléchargements/scriibe-linux-x64.zip -d ~/scriibe
```

   - If the terminal says the folder does not exist, your folder is probably called `Downloads` (the command uses the French name `Téléchargements`): replace `Téléchargements` with `Downloads` in the command.
   - If the terminal says `unzip` is not found, install it with `sudo apt install -y unzip` (the PC asks for your password: nothing shows as you type it, that is normal), then repeat the command above.

2. This second command allows the program to run:

```bash
chmod +x ~/scriibe/scriibe
```

**It worked when** the following command prints a line containing `scriibe`:

```bash
ls ~/scriibe
```

## Step 3 — Open Scriibe for the first time

1. In the terminal, copy:

```bash
~/scriibe/scriibe
```

2. The terminal stays busy: this is Scriibe's log. **Leave it open** and do not close it as long as you use Scriibe.
3. **On GNOME with Wayland (the usual case on recent Ubuntu)**, GNOME opens some windows, **only once**:
   - a **keyboard shortcut** to dictate, then a second one to dictate and translate. GNOME does not accept a single key such as right Ctrl: choose a combination with an ordinary key (for example **Ctrl + ←** to dictate and **Ctrl + →** to translate, or Ctrl + Alt + Space), then click **Add**;
   - at the first paste, a request for permission to **control the keyboard**: this is what lets Scriibe paste the text. Accept.
4. If the program complains about a missing library, note its name, then go to “If it does not work”.

Closing the Scriibe window does not quit the program: it keeps listening for the key. To really quit: the icon menu in the top bar → quit, or **Ctrl+C** in the terminal.

**It worked when** the Scriibe window opens **and** a small icon appears in the top bar, near the sound and battery. Without the “AppIndicator” extension (the case on Fedora, for example), the icon does not appear: the window is enough.

## Step 4 — The microphone

On Ubuntu there is normally nothing to allow. Just check that your microphone is the right one: **Settings → Sound → Input**, speak, and the bar should move.

**It worked when** the input bar moves while you speak.

## Step 5 — Download the programs inside Scriibe

Scriibe needs two small programs: one transcribes your voice, the other cleans up the text. Scriibe downloads them for you and **downloads nothing without your approval**: before each download it shows the name, the size and the source. They are stored in `~/.local/share/Scriibe/engines/`.

1. In the Scriibe window, click **Settings**, then the **Engines** tab. The **Installation status** card says what is missing.
2. Further down, in the **Programs** card, find **whisper-server (Linux)** (transcription). Click **See the size and source**, read, then click **Download**. Wait for the progress bar to finish: **✔ installed** appears.
3. Do the same for **llama-server (Linux, CPU)** (cleanup).

**It worked when** both programs show **✔ installed**.

## Step 6 — Download the models inside Scriibe

“Models” are the files that hold the intelligence for transcription and cleanup. Same principle: nothing is downloaded without your approval.

1. Still in **Settings → Engines**, scroll down to the **Recommended models** card.
2. Find the **Whisper “base” model**. Click **See the size and source**, read, then click **Download**. Wait for the progress bar to finish.
3. In the same card, do the same for a cleanup model: **“Qwen 2.5 1.5 B”** (about 1 GB) is fine to start with.
4. Higher up in the same **Engines** tab, choose **Whisper** for transcription and **llama.cpp** for cleanup.

**It worked when** the **Installation status** card no longer reports anything missing (only ✔).

## Step 7 — Try it

1. Open a text editor (for example **Text Editor**, through the application search) and click in the empty page.
2. **Hold the dictation shortcut** chosen in step 3 (on X11 it is **right Ctrl** by default). A small indicator appears at the bottom of the screen: wait until it shows “Listening…”.
3. Say a full sentence, for example “Hello, I am testing Scriibe.”
4. **Release** the key. After a second or two, the text appears in the page.

**It worked when** your sentence is written in the text editor.

## If it does not work

| What you see | What to do |
|---|---|
| The terminal mentions `libasound.so.2` | Copy: `sudo apt install -y libasound2t64` (or `libasound2` on an older Ubuntu), then launch Scriibe again. |
| The terminal mentions `libwebkit2gtk-4.1.so.0` or `libayatana-appindicator3.so.1` | Copy: `sudo apt install -y libwebkit2gtk-4.1-0 libayatana-appindicator3-1`, then launch Scriibe again. |
| The window does not open | Icon menu in the top bar → open Scriibe, or open in your browser the address shown by the terminal. |
| Nothing happens when you hold the shortcut | Check that Scriibe is running (terminal open) and that the shortcut is the one from step 3. To change it: **Settings → Apps → Scriibe → Keyboard shortcuts**, then relaunch Scriibe. |
| The indicator appears, but no text shows up | Click in the page where you want to write before holding the shortcut, and check that you accepted the permission to control the keyboard (step 3). |
| In a **terminal**, the text does not appear | That is normal: a terminal pastes with **Ctrl+Shift+V**. The dictated text stays in the clipboard. |
| The indicator stays stuck waiting although you released the key | Press the shortcut again, then release it: the indicator frees itself. |
| The text is empty or wrong | Check step 4 (microphone) and that steps 5 and 6 are all ✔. |
| A download fails | Repeat the same download: a passing failure of GitHub or Hugging Face does happen. |
| Anything else | In Scriibe: **Settings → Measurements and trials**, **Report to paste to me** card, **Copy the report** button. Paste it into a message to the person who gave you Scriibe: the report does not contain the text of your dictations. |

## Later: relaunching and updating

- **Relaunch Scriibe** after restarting the PC: open a terminal, copy `~/scriibe/scriibe`.
- **Install an update**: in Scriibe, **Settings → Update** → **Check for a new version** → **Install and restart**, then confirm. Scriibe relaunches by itself. Settings, history and dictionary are kept. If it fails, see the [guide for experienced users](INSTALLATION_LINUX_EXPERT.en.md) (manual update).
