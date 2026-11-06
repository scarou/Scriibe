# Installing Scriibe on Windows — step by step, for beginners

🇫🇷 [Version française](INSTALLATION_WINDOWS.md)

Other systems: [macOS](INSTALLATION_MACOS.en.md) · [Linux](INSTALLATION_LINUX.en.md)

**Comfortable with the command line, or want online engines, building from source or a manual update?** See the [guide for experienced users](INSTALLATION_WINDOWS_EXPERT.en.md). This guide does not need any of that.

This guide is written for someone who has never installed this kind of program. No computer knowledge is needed. Do the steps **in order**, one at a time. Each step ends with a sentence “**It worked when…**”: if that is not the case, do not move on to the next step, go to “If it does not work” at the end.

The Scriibe window can be switched between French and English (**Settings → Appearance → Interface language**); the labels below are the English ones.

**What you need**: a PC running Windows 10 or 11 (64-bit), an Internet connection, a microphone. **Time**: about 30 minutes, much of it waiting for downloads. No account is needed.

> **Status of this guide**: written from what has been tried on its author's own PC (`docs/ESSAI_WINDOWS.md`, in French). It has **not yet been followed to the letter by a beginner on a brand-new PC**. If a step does not go as written, note which one and the message shown, and tell the person who gave you Scriibe.

## Step 0 — Check your PC

1. Press the **Windows** key, type **About your PC**, and click the result.
2. Look at the **System type** line: it must say **64-bit operating system**. Also look at the **Edition** line: Windows 10 or Windows 11.

**It worked when** the system is 64-bit and it is Windows 10 or 11. Otherwise Scriibe does not work on this PC: stop here.

## Step 1 — Download Scriibe

1. Open your browser (Edge, Chrome…) and go to `https://github.com/scarou/Scriibe/releases`. It is a public page: you do not need an account.
2. Find the latest version: it is the one **at the very top of the page** (the highest number).
3. Under that version, look for the **Assets** section (click it if it is collapsed). Click the file **`scriibe-windows-x64.zip`**. Do not click “Source code”.
4. If the browser asks you to confirm the download (“Keep”), accept. The file lands in your **Downloads** folder.

**It worked when** the file `scriibe-windows-x64.zip` is in your **Downloads** folder. If it is missing from the “Assets” list, wait a few minutes (it is added shortly after publication) and reload the page.

## Step 2 — Install Scriibe

1. Open **File Explorer** (the yellow folder icon in the taskbar), then click **Downloads** in the left column.
2. **Right-click** the zip file, then **Extract All…**.
3. In the window that opens, clear the suggested path and type **`C:\scriibe`**, then click **Extract**.

**It worked when** the folder `C:\scriibe` opens and contains a file **`scriibe.exe`**.

## Step 3 — Open Scriibe for the first time

Windows shows a warning the first time, because Scriibe is not signed (signing is expensive). This is normal and happens only once.

1. In `C:\scriibe`, double-click **`scriibe.exe`**.
2. If a blue window **“Windows protected your PC”** appears: click **More info**, then **Run anyway**.
3. A black window opens (this is the log: leave it open, do not close it), then the Scriibe window opens.

Closing the Scriibe window does not quit the program: to really quit, right-click its icon near the clock, then **Quit**. To reopen the window: right-click the icon, then **Open Scriibe**.

**It worked when** the Scriibe window opens **and** a small round grey icon appears near the clock, at the bottom right of the screen. If the icon is hidden, click the small arrow **^** to the left of the clock.

## Step 4 — Allow the microphone

1. If Windows asks for access to the microphone, click **Yes**.
2. Otherwise, or if you refused: press the **Windows** key, type **Microphone privacy settings**, open the result, and turn on **Microphone access**, then **Let desktop apps access your microphone**.

**It worked when** both switches are on. No other permission is needed on Windows.

## Step 5 — Download the programs inside Scriibe

Scriibe needs two small programs: one transcribes your voice, the other cleans up the text. Scriibe downloads them for you and **downloads nothing without your approval**: before each download it shows the name, the size and the source. There is nothing to install separately.

1. In the Scriibe window, click **Settings**, then the **Engines** tab. The **Installation status** card says what is missing.
2. Further down, in the **Programs** card, find **whisper-server (Windows)** (transcription). Click **See the size and source**, read, then click **Download**. Wait for the progress bar to finish: **✔ installed** appears.
3. Do the same for **llama-server (Windows, CPU)** (cleanup). Choose the “CPU” version: it is the simplest and suits every PC.

**It worked when** both programs show **✔ installed**.

## Step 6 — Download the models inside Scriibe

“Models” are the files that hold the intelligence for transcription and cleanup. Same principle: nothing is downloaded without your approval.

1. Still in **Settings → Engines**, scroll down to the **Recommended models** card.
2. Find the **Whisper “base” model**. Click **See the size and source**, read, then click **Download**. Wait for the progress bar to finish.
3. In the same card, do the same for a cleanup model: **“Qwen 2.5 1.5 B”** (about 1 GB) is fine to start with.
4. Higher up in the same **Engines** tab, choose **Whisper** for transcription and **llama.cpp** for cleanup.

**It worked when** the **Installation status** card no longer reports anything missing (only ✔).

## Step 7 — Try it

1. Open **Notepad** (press the **Windows** key, type **Notepad**, Enter). Click in the empty page to put the cursor there.
2. **Hold the right Ctrl key** (bottom right of the keyboard, right of the space bar). A small indicator appears: wait until it shows “Listening…”.
3. Say a full sentence, for example “Hello, I am testing Scriibe.”
4. **Release** the key. After a second or two, the text appears in the page.

**It worked when** your sentence is written in Notepad.

## If it does not work

| What you see | What to do |
|---|---|
| The Scriibe window does not open | Right-click the icon near the clock (click the arrow **^** if it is hidden), then **Open Scriibe**. |
| Windows blocks `scriibe.exe` without offering “Run anyway” | Right-click `scriibe.exe` → **Properties** → tick **Unblock** at the bottom → **OK**, then launch again. |
| Nothing happens when you hold the key | Dictate in a window other than Scriibe's own (Notepad). Check that Scriibe is running (icon near the clock) and that step 4 is done. |
| The indicator appears, but no text shows up | Click in the page where you want to write before holding the key. |
| The indicator stays stuck waiting although you released the key | Press the key again, then release it: the indicator frees itself. |
| The text is empty or wrong | Check step 4 (microphone) and that steps 5 and 6 are all ✔. |
| A download fails | Repeat the same download: a passing failure of GitHub or Hugging Face does happen. |
| Anything else | In Scriibe: **Settings → Measurements and trials**, **Report to paste to me** card, **Copy the report** button. Paste it into a message to the person who gave you Scriibe: the report does not contain the text of your dictations. |

## Later: installing an update

In Scriibe: **Settings → Update** → **Check for a new version** → **Install and restart**, then confirm. Scriibe reopens by itself. Settings, history and dictionary are kept. If it fails, see the [guide for experienced users](INSTALLATION_WINDOWS_EXPERT.en.md) (manual update).
