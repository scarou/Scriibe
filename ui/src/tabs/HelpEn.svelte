<script lang="ts">
  // L'Aide en anglais : même structure que Help.svelte (sections, exemples, liens) — à tenir à jour avec elle (structure vérifiée par i18n.test.ts).
  // Les phrases dictées des exemples restent en français : la dictée, les signes dictés et les listes sont ceux du français.
  import { onMount } from 'svelte';
  import OverlayView, { type View } from '../overlay/OverlayView.svelte';
  import { helpSections } from '../lib/help';
  import { nav } from '../lib/nav.svelte';
  import { settings } from '../lib/settings.svelte';
  import { keyName } from '../lib/i18n.svelte';
  import { status } from '../lib/status.svelte';
  import type { TabId } from '../lib/tabs';

  let s = $derived(status.value);
  let v = $derived(settings.value);
  let open = $derived(nav.helpSection);
  /** Les touches en vigueur (réglables dans Réglages → Dictée). */
  /** The system Scriibe runs on: the Help only talks about that one (keys, icon, permissions, keychain). */
  let os = $derived(status.os);
  let mac = $derived(os === 'macos');
  let linux = $derived(os === 'linux');
  let defaultDictate = $derived(mac ? 'right Option' : 'right Ctrl');
  let defaultTranslate = $derived(mac ? '⌘' : 'Shift');
  let copyKey = $derived(mac ? '⌘C' : 'Ctrl+C');
  let pasteKey = $derived(mac ? '⌘V' : 'Ctrl+V');
  let trayNear = $derived(mac ? 'in the menu bar' : 'near the clock');
  let trayIcon = $derived(mac ? 'The menu bar icon' : 'The icon near the clock');
  let editor = $derived(mac ? 'TextEdit' : linux ? 'a text editor' : 'Notepad');
  let pasteNote = $derived(
    mac
      ? 'Most often, the “Accessibility” permission (labelled “Device control and data access” on some macOS versions) is missing, in System Settings → Privacy & Security: turn on Scriibe’s switch, then quit Scriibe and restart it.'
      : linux
        ? 'In a terminal, paste with Ctrl+Shift+V.'
        : 'Pasting is blocked by Windows in a window started as administrator.',
  );
  let dictate = $derived(keyName(s?.dictation_label ?? (mac ? 'Option droite' : 'Ctrl droit')));
  let translate = $derived(keyName(s?.translate_label ?? (mac ? '⌘' : 'Maj')));
  let language = $derived(
    ({ en: 'English', es: 'Spanish', de: 'German', it: 'Italian', pt: 'Portuguese (Brazil)' } as Record<string, string>)[v?.translationLanguage ?? 'en'] ?? 'English',
  );
  const titles: Record<string, string> = {
    start: 'Getting started',
    dictate: 'Dictating and translating',
    modes: 'Modes',
    dictionary: 'Dictionary and signs',
    indicator: 'The indicator',
    measures: 'Measuring and comparing',
    privacy: 'On this computer or online',
    problems: 'Common problems',
  };

  onMount(() => void settings.load());

  /** A sentence about what leaves the computer or not, from the engines chosen. */
  let privacySummary = $derived.by(() => {
    if (!v) return '';
    const voice = v.transcriptionEngine === 'whisper' ? 'your voice stays on this computer' : 'the recording of each dictation goes to Deepgram';
    const local = v.cleanupEngine === 'llamaCpp' || v.cleanupEngine === 'lmStudio' || v.cleanupEngine === 'ollama';
    const text =
      v.cleanupEngine === 'none'
        ? 'no model cleans the text up (translation is then unavailable)'
        : local
          ? 'the text is cleaned up on this computer'
          : 'the text goes to OpenRouter to be cleaned up';
    const everythingLocal = v.transcriptionEngine === 'whisper' && local;
    return `With your current settings, ${voice}, and ${text}.${everythingLocal ? ' Your voice and text stay on this computer.' : ''}`;
  });

  /** The indicator in its three states, with the style, colour and transparency set. */
  function sample(state: View['state']): View {
    return {
      visible: true,
      state,
      message: null,
      translation: null,
      mic: 'ok',
      level: 0.6,
      appearance: { enabled: true, style: v?.captureStyle ?? 'pill', position: 'center', color: v?.captureColor ?? 'scribe', opacity: v?.captureOpacity ?? 1 },
    };
  }
  const curve = Array.from({ length: 40 }, (_, i) => Math.max(0.1, 0.35 + 0.3 * Math.sin(i / 3) + 0.2 * Math.sin(i / 1.3)));
</script>

{#snippet go(label: string, tab: TabId, anchor?: string)}
  <button class="go" onclick={() => nav.go(tab, anchor)} title="Open: {label}">→ {label}</button>
{/snippet}

{#snippet example(said: string, pasted: string, note?: string)}
  <div class="example">
    <div class="line"><span class="who">You say</span><i>“{said}”</i></div>
    <div class="line"><span class="who strong">Scriibe pastes</span><b>{pasted}</b></div>
    {#if note}<div class="note">{note}</div>{/if}
  </div>
{/snippet}

{#snippet tip(text: string)}
  <div class="tip">💡 {text}</div>
{/snippet}

{#snippet question(q: string, answer: string)}
  <div class="q"><b>{q}</b><div>{answer}</div></div>
{/snippet}

<h1>Help</h1>
<p class="muted lead">How to use Scriibe, and what each thing is for. The <b>→</b> buttons open the right setting; the <b>?</b> on each card opens the explanation that concerns it.</p>

<div class="layout">
  <ul class="toc">
    {#each helpSections as sec (sec.id)}
      <li><button class:active={open === sec.id} onclick={() => (nav.helpSection = sec.id)}>{titles[sec.id]}</button></li>
    {/each}
  </ul>

  <section class="card body">
    {#if open === 'start'}
      <h2>Getting started</h2>
      <p>Scriibe writes for you in any app: you hold a key, you speak, you release, and the text, clean and punctuated, is pasted where your cursor is.</p>
      <ol class="steps">
        <li><b>Install what is needed.</b> To do everything on this computer you need <b>whisper-server</b> and a Whisper model (the transcription), then, if you want cleanup without the Internet, <b>llama-server</b> and a model. The installation cards (Settings → Engines) tell you what is missing and, before any download, shows the name, the real size and the source: nothing is downloaded without your consent.{#if mac} On Mac, the two programs are installed with Homebrew (“brew install whisper-cpp” then “brew install llama.cpp”, in Terminal); the cards then offer the models.{/if}</li>
        <li><b>Choose your engines.</b> Whisper and llama.cpp do everything on this computer, without the Internet; Deepgram and OpenRouter use online services and need a key: they are a fallback when the computer is too slow. At first launch, the Dashboard assistant measures your computer and suggests the setup that suits it.</li>
        <li><b>Dictate.</b> Click in a document, hold <b>{dictate}</b>, say a sentence, release: it is written.</li>
      </ol>
      <div class="links">
        {@render go('Install programs and models', 'install')}
        {@render go('The engines', 'settings', 'moteurs')}
        {@render go('The key and the microphone', 'settings', 'dictee')}
        {@render go('Dashboard', 'dashboard')}
      </div>
      <p>The window has five pages: <b>Dashboard</b>, <b>History</b>, <b>Dictionary</b>, <b>Settings</b> and <b>Help</b>. Settings has tabs: Dictation, Engines, Appearance, Modes, Measurements and trials, and Update. Each card folds with a click on its title (the fold is remembered), and its “?” button opens the Help section that covers it. Settings → Appearance: light, dark or automatic (the system’s) theme, and the interface language.</p>
      {@render tip('To check the microphone and see what Whisper hears, read a phrase of the dictation bench (Settings → Measurements and trials): it shows the microphone level before you go and dictate in a real app.')}
      <div class="links"><button class="go" onclick={() => nav.openAssistant()} title="Open the assistant on the Dashboard">→ Run the first-launch assistant again</button></div>

      <h3>What happens between your voice and the text</h3>
      <p>When you release the key, your sentence goes through several steps, each done by someone (or something) different:</p>
      <ol class="steps">
        <li><b>The microphone</b> records what you say while you hold the key. Nothing else: it is a plain sound file.</li>
        <li><b>The transcription</b> (Whisper, or Deepgram online) listens to this sound and writes the words. It is like a subtitler: it hears, it does not interpret. It writes what it thinks it hears, with approximate punctuation, and it can get a proper name or a rare word wrong.</li>
        <li><b>The text corrections</b> apply fixed rules, with no model and no waiting: dictated signs (“point à la ligne”, “ouvrez la parenthèse”), lists, and your dictionary (“cloud code” → “Claude Code”).</li>
        <li><b>The cleanup</b> is done by a <b>language model</b> (llama.cpp on this computer, or OpenRouter online). It understands the meaning: it removes the “uh”s and false starts, fixes the punctuation, rephrases if the mode asks for it, and translates if you hold the translation key. It follows the instruction of the chosen mode, and a safeguard checks that it has not answered in your place or invented text.</li>
        <li><b>The French typography</b> puts the spaces before “?” or “:”, the « » and the apostrophes right, then Scriibe <b>pastes</b> the result where your cursor is.</li>
      </ol>
      {@render tip('So two different models work one after the other: the one that listens (Whisper) and the one that writes (the cleanup model). They do not do the same job, and you can choose one locally and the other online.')}

      <h3>A program, a model: what is the choice for?</h3>
      <p>Each step has two parts. The <b>program</b> (<i>whisper-server</i>, <i>llama-server</i>) is the engine: small, it knows how to run a model. The <b>model</b> is a big file that holds everything the artificial intelligence has learned: it is what makes the quality. The same program can run very different models, hence the choice.</p>
      <p>Choosing a model means choosing a compromise between four things:</p>
      <ul>
        <li><b>Quality.</b> A big model hears better (Whisper “small” more than “base”) or writes better (a 3-billion-parameter model more than a 0.5-billion one). A small model makes more mistakes: a misheard word, a badly rephrased sentence, a false start (“no, sorry”) left uncorrected.</li>
        <li><b>Speed.</b> A big model does more calculations, so it takes longer. On a laptop without a real graphics card, Whisper “small” takes several times longer than “base”, and a 3-billion-parameter writing model takes about ten seconds per sentence. An online service answers in about two seconds, because it runs on powerful machines.</li>
        <li><b>Space and memory.</b> A model takes from a few hundred MB to several GB on disk, and as much in memory while it works. A 2 GB graphics card cannot hold a 2 GB model: Scriibe then switches to the processor by itself.</li>
        <li><b>Privacy and cost.</b> Locally, your voice and text stay on the computer, and it is free. Online (Deepgram, OpenRouter), it is fast, but your voice or your text goes to the service, and you need a key (often paid).</li>
      </ul>
      <p>If you have no precise idea, let the assistant choose: it measures your computer and suggests the combination that suits it, with the times that were really measured. You can <b>run it again whenever you want</b> (button above, or Settings → Engines) if you change computer, graphics card or mind.</p>
      <div class="links">
        {@render go('Choose the engines', 'settings', 'moteurs')}
        {@render go('Install a model', 'install')}
        {@render go('Compare with the dictation bench', 'bench')}
      </div>

    {:else if open === 'dictate'}
      <h2>Dictating and translating</h2>
      <p>Hold <b>{dictate}</b> all the time you speak; when you release, Scriibe transcribes, cleans up and pastes. Speak naturally: the “uh”s, false starts and repetitions are removed for you (except in Literal mode, or if the cleanup engine is “None”).</p>
      {@render example('euh, le rendez-vous est à dix heures, non pardon, à onze heures', 'Le rendez-vous est à 11 h.', 'A false start (“non pardon”, “enfin”) keeps only the corrected version.')}
      {@render example('est-ce que tu peux m’envoyer le devis avant vendredi midi', 'Est-ce que tu peux m’envoyer le devis avant vendredi midi ?', 'A question or a request stays a question: Scriibe writes it, it does not answer it (see “Common problems” if that happens).')}
      <h3>Translating while dictating</h3>
      <p>Add <b>{translate}</b> during the dictation: you speak in French, the text is pasted in <b>{language}</b>. The language is chosen in Settings → Dictation (“Translate into”), and so is the key (“Key held in addition to translate”).</p>
      {@render example('tu peux me rappeler demain matin', 'Can you call me back tomorrow morning?', 'Here in English.')}
      <p>Translation always goes through the cleanup engine, even in Literal mode: with OpenRouter, the text then goes online. Without an engine (“None”), {translate} does not translate: the text stays in French. <i>Why:</i> it is the model that translates, in a single call together with the cleanup; without a model, nothing can do it.</p>
      {@render tip('The interface language and the dictation language are two different settings. The first changes Scriibe’s texts; the second (Settings → Dictation → “Dictation language”) says which language you speak: it is the one Whisper listens for, and the one of the Test bench and mode trial phrases. If you dictate in French with an English interface, leave it on “French”; to try the bench in English, choose “English”, then put “French” back afterwards, otherwise Whisper will transcribe your voice as English.')}
      <h3>Choosing your keys</h3>
      <p>By default: <b>{defaultDictate}</b> to dictate, <b>{defaultTranslate}</b> in addition to translate. To change: Settings → Dictation → “Change…”, then <b>press directly</b> the key or combination you want (for example {mac ? 'left Option + F9' : 'left Ctrl + F9, or the Windows key'}) and release everything: Scriibe remembers it, and “Test my key” checks that it is recognised.</p>
      <ul>
        <li>{#if mac}Possible: Option, Command (⌘), Fn, Ctrl, Shift, function keys (F1 to F20), Home, End, Page Up and Page Down. Not possible: letters, digits, Space, Enter and the arrow keys. On most Macs, F1 to F12 need Fn in addition, unless “standard function keys” is turned on.{:else}Possible: Ctrl, left Alt, Shift, the Windows key, function keys, “&lt; &gt;” (left of W), Pause, Insert… Not possible: letters, digits, Space, Enter, and AltGr (it is used to type “@” or “€” on an AZERTY keyboard).{/if}</li>
        <li>{#if mac}<b>Modifier keys on their own</b>: Ctrl and Shift must be held for a fraction of a second (0.3 s) before listening starts; Option, ⌘ and Fn listen right away. <i>Why:</i> keyboard shortcuts (⌘C, Shift+letter) also press Ctrl and Shift, and must not open the microphone.{:else}<b>Modifier keys on their own</b> (left Shift, Windows, Ctrl + Alt…): you must hold them for a fraction of a second (0.3 s) before listening starts. <i>Why:</i> keyboard shortcuts (Ctrl+C, Shift+letter, Windows+E) press them too, and must not open the microphone.{/if}</li>
        {#if !mac}<li><b>Left Alt and Windows</b> would normally open the app menu or the Start menu on release, and the paste would lose the focus: Scriibe slips in a harmless key during the press to avoid it.</li>{/if}
        <li>An ordinary key in the combination (F9, “&lt; &gt;”) writes nothing in the app during the dictation.</li>
        <li>{#if mac}<b>The Fn key</b> is offered on Mac, but has not yet been tried as a dictation key: if “Change…” receives nothing when you press it, choose another key.{:else}<b>The Fn key</b> is not seen by {linux ? 'Linux' : 'Windows'} on most laptops: the keyboard handles it itself. If “Change…” receives nothing when you press it, that is the case; choose another key.{/if}</li>
      </ul>
      <h3>Good to know</h3>
      <ul>
        <li>Pressing <b>another key</b> during the press (for example {dictate} + a letter) cancels the dictation: it is a shortcut, not a dictation. <i>Why:</i> {copyKey} must not open the microphone for nothing.</li>
        <li>A dictation of <b>three words or fewer</b> is pasted without going through the model, in Simple mode (translation aside): it is faster, but an “uh” stays.</li>
        <li>What you had copied <b>comes back to the clipboard</b> after the paste (adjustable). <i>Why:</i> Scriibe uses the clipboard to paste, and must not take it from you. Only text is restored: an image or files copied before the dictation are lost.</li>
        <li><b>“Pause music or video”</b> (Settings → Dictation, off at first): a video or music that is playing stops when you hold the key and resumes when you release. Scriibe looks at which apps are producing sound. A player with its own window (VLC, Spotify…) receives the Play/Pause command directly; browsers receive it through the keyboard’s Play/Pause key. {#if mac}<b>With Chrome</b>, Chrome keeps its sound “open” for about ten seconds after a video is paused: so that an already stopped video is not restarted, Scriibe asks Chrome whether a page is really playing something, then pauses and resumes the video itself through the page (the keyboard key does not always stop YouTube). For this, once: in Chrome, View menu → Developer → “Allow JavaScript from Apple Events”, then accept macOS’s “Scriibe wants to control Google Chrome” prompt. Without it, a video paused less than ten seconds earlier may start again during the dictation. {/if}If that key starts a different app from the one that was playing, Scriibe cancels it. During a call (Teams, Meet, Zoom…), Scriibe touches nothing, and if you start something else during the dictation, it does not resume the old media. <i>Why:</i> interrupting a call or resuming the wrong player would be worse than letting it play.</li>
        <li>{#if linux}There is no icon on Linux yet: Scriibe’s page opens in the browser, and “Quit Scriibe” (in the page) stops the program.{:else}The <b>icon menu</b>, {trayNear}, opens the window, the data folder and the log, or quits Scriibe.{/if}</li>
        <li><b>A selection</b> is replaced by the dictation: if a word or a group of words is selected when you release the key, the dictated text takes its place <b>without a capital letter at the start nor a period, comma or semicolon at the end</b>, since it fits in the middle of a sentence (the “?”, the “!” and ellipses stay; names from your dictionary and acronyms keep their capital). <i>Why:</i> Whisper puts a capital and a period on every dictation, which does not fit in the middle of a sentence. {#if mac}Scriibe sends a ⌘C to find out whether a text is selected and puts your clipboard back right away.{:else}This only works on a Mac for now.{/if}</li>
      </ul>
      <div class="links">{@render go('Dictation settings', 'settings', 'dictee')}</div>

    {:else if open === 'modes'}
      <h2>Modes</h2>
      <p>The mode decides what your text becomes. All of them keep your information and your questions, and must never answer what you dictate (a safeguard watches over this); the modes that rephrase change the tone, and Letter addresses the recipient formally (“vous”).</p>
      {@render example('bon ben j’ai regardé le devis que tu m’as envoyé et franchement ça me va sauf le prix de la livraison', 'J’ai regardé le devis que tu m’as envoyé et franchement ça me va, sauf le prix de la livraison.', 'Simple: removes the “uh”s, punctuates, keeps your words.')}
      {@render example('bon ben j’ai regardé le devis que tu m’as envoyé et franchement ça me va sauf le prix de la livraison', 'J’ai examiné le devis que tu m’as adressé et, franchement, cela me convient, à l’exception du coût de la livraison.', 'Formal: rephrases in careful French.')}
      {@render example('est-ce que tu as pensé à réserver le restaurant pour l’anniversaire de maman', 'Tu as pensé à réserver le resto pour l’anniv de maman ?', 'A custom “SMS” mode, with three examples.')}
      <ul>
        <li><b>Literal</b>: no model, only the dictated signs, the dictionary and the typography.</li>
        <li><b>Simple</b> (default), <b>Formal</b>, and four professional modes: sophrology, IT and AI, official letters, sales.</li>
      </ul>
      <h3>Create your own mode</h3>
      <ol class="steps">
        <li>Settings → Modes → “My modes”: a name, an instruction (“Write like a short, friendly text message”).</li>
        <li>Leave “Rephrases sentences” unchecked to keep your words with the instruction added; check it to rewrite in the style, like the professional modes.</li>
        <li>Add two or three <b>examples</b>: a dictation the way you would say it, and the text you expect. They are what really gives the style.</li>
        <li>Choose it in the list, then dictate a text to check that questions stay questions and that false starts are corrected.</li>
      </ol>
      {@render tip('With a small model on an ordinary computer, an instruction alone changes the text little: measured on a Mac, an “SMS” mode without examples wrote like Simple, and with three examples it wrote “resto” and “anniv”. The examples count more than the instruction.')}
      <p class="muted">Scribe’s Playful mode (Smurf language) does not exist in Scriibe.</p>
      <div class="links">{@render go('The modes', 'modes')}</div>

    {:else if open === 'dictionary'}
      <h2>Dictionary and signs</h2>
      <p>The dictionary teaches Scriibe your own words, and the signs are dictated out loud.</p>
      <h3>Dictated signs</h3>
      {@render example('le colis ouvrez la parenthèse le gros fermez la parenthèse est arrivé point à la ligne merci encore', 'Le colis (le gros) est arrivé.\nMerci encore.', 'The variants are recognised: “ouvre” or “ouvrez”, “ferme” or “fermez”, “retour à la ligne”, “nouveau paragraphe”… (when dictating in French).')}
      <p>“À la ligne” said on its own also breaks the line (“merci beaucoup, à la ligne, cordialement”), provided it is set apart by punctuation: at the start of the dictation, or between commas or full stops, which is what Whisper writes when you pause. Inside a sentence, it stays text: “aller à la ligne 5”, “arrivé à la ligne d’arrivée”. In front of the start of a list (“ma liste à la ligne tiret du lait”), “à la ligne” counts even without punctuation. Elsewhere, “retour à la ligne” is safer: it works even without a pause. <i>Why:</i> “à la ligne” is also an ordinary expression; without these rules, Scriibe would cut your sentences in the wrong place.</p>
      {@render tip('The signs depend on what Whisper hears. A smaller model or a weak microphone sometimes hears “ouvrait” instead of “ouvrez”: the conversion then does not trigger. Say the signs distinctly, or add an automatic correction for the variant heard.')}
      <h3>Lists</h3>
      {@render example('les courses retour à la ligne tiret du lait retour à la ligne tiret du pain', 'Les courses\n- du lait\n- du pain', '“tiret” or “puce” at the start of a line gives a dash. For a numbered list: “point 1”, “point 2”… (in words, “point un,” with a pause) or “numéro suivant” to go on to the next number.')}
      <p>These words only count at the start of a line, that is at the start of the dictation or after a line break (“retour à la ligne”, “point à la ligne”, “nouveau paragraphe”): “il a mis un tiret entre les deux mots” stays an ordinary sentence. But a line that starts with “Point 3 du contrat” becomes “3. du contrat”: that is the price of dictating by voice. “Tirer”, “tirez”, “tiré” or “tirée”, which Whisper sometimes hears instead of “tiret”, also count, but only after a line break. Not in English dictation nor after a translation.</p>
      <h3>Words to write exactly</h3>
      <p>Names, brands, jargon (“Claude Code”, a client’s name): added here, they are given as <b>hints</b> to the transcription and to the model, and so are better recognised. For a certain spelling, also add an automatic correction: a hint helps, it guarantees nothing.</p>
      <h3>Automatic corrections</h3>
      <p>A word that is heard is replaced by the one you choose: useful for a word that is always misheard (“cloud code” → “Claude Code”), or for a sign of your own (“trois points d’exclamation” → !!!). The correction always applies, at every dictation.</p>
      <h3>Voice commands</h3>
      <p>When text is selected, a phrase said on its own (nothing else in the dictation) acts on it instead of being pasted. With no selection, or with other words around it, the phrase stays ordinary text. Neither the selected text nor the command is kept in the history.</p>
      <ul>
        <li><b>Add to dictionary</b>: “add to dictionary” opens this tab with the selected text in “Heard”.</li>
        <li><b>Characters around</b>: “in parentheses”, “in quotes” (« » in French, " " in English), “in brackets” or “in braces” wrap the selection; a single ⌘Z undoes it.</li>
        <li><b>Bold, italic, underline</b>: “make it bold”, “make it italic”, “make it underlined” press the app’s own shortcut (⌘B, ⌘I, ⌘U on Mac, adjustable). Scriibe does not write Markdown: it uses the app.</li>
      </ul>
      {@render tip('The phrases are changed in the “Voice commands” card of the Dictionary tab: at least two words, and a phrase cannot serve two commands. A switch turns all the commands off. For now they only work on Mac.')}
      <div class="links">{@render go('Open the dictionary', 'dictionary')}</div>

    {:else if open === 'indicator'}
      <h2>The indicator</h2>
      <p>During the dictation, a small pill at the bottom of the screen shows where Scriibe is, without you having to look elsewhere. It never takes the focus: what you type keeps going into your app. <i>Why:</i> without it, you would not know whether Scriibe hears you until you had spoken into the void.</p>
      <div class="samples">
        {#each [['Listening', 'listening'], ['Cleaning up', 'working'], ['Pasted', 'done']] as [name, state] (state)}
          <div class="sample">
            <div class="stagebox"><OverlayView view={sample(state as View['state'])} levels={curve} inline /></div>
            <span class="muted">{name}</span>
          </div>
        {/each}
      </div>
      <ul>
        <li><b>Listening</b>: “Listening…”, or a dot or a curve that follows your voice depending on the style. “I can’t hear anything…” or “I can barely hear you…” if the microphone is too weak.</li>
        <li><b>Cleaning up</b>: a turning arc (with “Cleaning up…” in the pill), while the text is transcribed and cleaned up. For a translation, the language is announced.</li>
        <li><b>Pasted</b>: a ✓ confirms that the text has arrived.</li>
        <li>{#if linux}There is no icon on Linux yet: the state can be read in the page’s sidebar.{:else}<b>{trayIcon}</b> always changes colour: <b>grey</b> at rest, <b>red</b> while listening, <b>orange</b> while processing, <b>green</b> once the text is pasted, <b>blue</b> for a message. It is useful when the pill is turned off.{/if}</li>
        <li>Three styles (pill, plain dot, level meter), three positions, five colours and an adjustable transparency. The pill is on by default; it is turned off in the settings. The window appearance (light, dark or the system one) is set in the same place, Settings → Appearance.</li>
      </ul>
      <div class="links">{@render go('Adjust the indicator', 'settings', 'indicateur')}</div>

    {:else if open === 'measures'}
      <h2>Measuring and comparing</h2>
      <p>These tools are not for everyday use: they answer a precise question. Choose them according to what you want to know.</p>
      {@render question('Does Scriibe work well for me, for real?', 'The Test bench (Settings → Measurements and trials): you read 17 tricky phrases (hesitations, false starts, questions, dictated signs), and each one is checked automatically. It shows what Whisper heard, what would be pasted, the time of each step, and the microphone level.')}
      {@render question('The bench phrases are not in the right language.', 'They follow the dictation language (Settings → Dictation → “Dictation language”), not the interface language: Whisper transcribes in the language chosen, so you read phrases in that language. To read the 17 phrases in English, choose “English”, then put “French” back when you have finished. The typing test, on the other hand, follows the interface language.')}
      {@render question('What did a change modify?', 'Each bench run is kept with its settings (version, engines, models, microphone). Do a new run after changing a setting or installing an update, then “Compare two runs”: the changed words, and the checks gained or lost. That is how you judge Whisper “small” against “base”, or the cloud against local.')}
      {@render question('Which engine should I choose on my computer?', 'The Dashboard assistant suggests a setup according to the cores, the memory and the graphics card, with the times measured on an ordinary laptop. A bench run then tells you whether it suits you. Careful: a single run of 17 phrases gives a trend, not a firm measurement; reading the phrases again also changes what Whisper hears.')}
      {@render question('How long do I wait?', 'The History gives, for each dictation, the transcription time, the cleanup time and the total since the key was released. The Log keeps one line per step (never the text).')}
      {@render question('How is the Dashboard’s time saved calculated?', `For each dictation, Scriibe compares the time spent speaking with the time it would have taken you to type the same text, at your typing speed (set to ${v?.typingWPM ?? 40} words per minute in “My typing speed”, for the whole history, purged days included: only the day’s totals remain).`)}
      {@render question('What is my real typing speed?', 'In the Dashboard, “My typing speed”: you copy out a text of a few lines, the timer starts at your first letter, mistakes turn red, and the attempt stops when the text is identical (or with “Finish”). The result is net (words of the text × share of correct letters ÷ minutes). Not kept: an attempt under 90% correct letters, shorter than 12 s, longer than 10 min or above 200 words/min. “Use” carries the average of the attempts kept over to your typing speed; the time saved is then recalculated for the whole history.')}
      {@render question('What is the report for?', 'Settings → Measurements and trials → “Report to paste to me” → “Copy the report” puts the installation state, the last bench run and the last lines of the log on the clipboard: that is what to paste so that you can be helped with a problem.')}
      <div class="links">{@render go('Measurements and trials', 'bench')}{@render go('Report and log', 'journal')}</div>

    {:else if open === 'privacy'}
      <h2>On this computer or online</h2>
      <p>{privacySummary}</p>
      <ul>
        <li><b>Whisper</b> (transcription) and <b>llama.cpp</b> (cleanup) run on this computer: your voice and text stay on the computer.</li>
        <li><b>Deepgram</b> receives the recording of each dictation, once the key is released (nothing outside your dictations); <b>OpenRouter</b> receives the transcribed text to clean it up.</li>
        <li><b>Memory</b>: Whisper stops after 10 minutes without dictation and comes back when you press the key, while you speak; llama.cpp’s cleanup model is freed after 30 minutes by default (adjustable: 2 h, 8 h or “as long as Scriibe is running”). <i>Why:</i> they take several GB of memory. After a release, the first dictation is slower: on a computer without a graphics card, count 15 to 45 s for the local model; preloading at launch avoids it for the first one.</li>
        <li>The words in “Words to write exactly” go along with the dictation: to Deepgram and OpenRouter if you use them.</li>
        <li>Translation always goes through the cleanup engine, even in Literal mode.</li>
        <li>Choose “None” as the cleanup to load no model at all: the text is transcribed as is (dictated signs, dictionary, typography). The modes and translation are then unavailable.</li>
        <li>The history, the dictionary, the settings and the bench runs stay on this computer, in the data folder (Settings → Locations).</li>
        <li>History, “History privacy” card: uncheck “Keep the text of dictations” to keep only the statistics (date, app, words, duration, times), or “Keep the history” for a number of days: older dictations are deleted, only per-day totals remain, without text, for the Dashboard. A preview of the number of dictations affected is shown before any deletion. This does not change what Deepgram or OpenRouter receive during the dictation.</li>
        <li>The Deepgram and OpenRouter keys are never displayed or written to the log; {#if mac}they are stored in the Mac’s keychain.{:else if linux}on Linux, Scriibe does not store them: you give them through environment variables, before launching Scriibe.{:else}they are stored in the Windows Credential Manager.{/if}</li>
      </ul>
      <div class="links">{@render go('The engines', 'settings', 'moteurs')}{@render go('History privacy', 'history', 'conservation')}</div>

    {:else}
      <h2>Common problems</h2>
      {@render question('Scriibe hears nothing.', `The microphone level shows in the Test bench (Settings → Measurements and trials). ${mac ? 'On Mac, check “Microphone” in System Settings → Privacy & Security: Scriibe’s switch must be on; then quit Scriibe and restart it.' : linux ? 'Check the input volume in the system’s sound settings.' : 'On Windows, check Settings → Privacy → Microphone: microphone access must be allowed.'} Also check the chosen microphone (Settings → Dictation → Microphone).`)}
      {@render question('Nothing is pasted, or a phrase is missing.', `Look at the log (Settings → Measurements and trials, at the bottom of the tab): each dictation leaves a line there, “collage: … signes envoyés à …”. If the line says “copié seulement”, the text is on the clipboard: ${pasteKey} pastes it. ${pasteNote} If the line says the text was sent but nothing appears, the app did not read the clipboard: try in ${editor} to see whether the problem comes from the app, and copy the report.`)}
      {@render question('My key does nothing.', `Settings → Dictation → “Test my key”, then hold the key: if Scriibe does not switch to listening, another program may be capturing it (macro tool, keyboard software). Choose another key or a combination. ${mac ? 'On Mac, first check that Scriibe has the “Input Monitoring” permission (System Settings → Privacy & Security), then quit Scriibe and restart it: without it, the log says that macOS refuses the global key.' : linux ? '' : 'If the log mentions a refused keyboard hook, restart Scriibe.'}`)}
      {@render question('The indicator stays stuck waiting although I released the key.', 'Most of the time, pressing the dictation key again and releasing it is enough: the indicator unblocks. If it stays stuck anyway, open the journal (Settings → Measurements and trials, at the bottom of the tab) to see the last step recorded, or quit Scriibe and start it again.')}
      {@render question('The beginning of my sentence is lost.', `With a Bluetooth headset, the system switches to “hands-free” mode when the microphone opens (1 to 3 s): wait until the pill shows “Listening…” before speaking, or better, choose a wired or USB microphone in Settings → Dictation → Microphone. A laptop microphone is often weak: the Test bench shows its level.`)}
      {@render question('The first phrase after a long pause is slow.', 'To give memory back, Whisper stops after 10 minutes without dictation and llama.cpp’s cleanup model after the set duration (“Keep the model loaded”). They come back when you press the key: Whisper in about a second, the local model in 15 to 45 s without a graphics card. Choose “as long as Scriibe is running” to keep it loaded.')}
      {@render question('A program or a model is missing.', `Settings → Engines (installation cards): whisper-server, a Whisper model (ggml-base.bin to begin with), and for local cleanup llama-server and a GGUF model. Each download shows its size and source before asking for your consent; an interrupted connection resumes where it stopped.${mac ? ' On Mac, the programs come from Homebrew: “brew install whisper-cpp” and “brew install llama.cpp”.' : ''}`)}
      {@render question('With llama.cpp, cleanup does not start.', 'It needs llama-server and a .gguf model file (Settings → Engines, installation cards, which tell you what is missing). If the engine log mentions memory, try “CPU only” in Settings → Engines → Graphics card: on a 2 GB card, the model does not always fit.')}
      {@render question('My question got an answer instead of being written.', 'The safeguard rejects most of these outputs: a mode that rephrases tries again in Simple, otherwise the text is pasted without cleanup. If one gets through with a custom mode, add a question example to that mode.')}
      {@render question('Whisper writes “Mardi” or “Vendredi” with a capital letter.', 'In French the days are written without a capital, but Whisper sometimes writes them the English way. It is only a matter of presentation: the meaning does not change. When a cleanup model is active, it fixes this; without it (Literal mode, cleanup “None”), the capital stays. To avoid it, add an automatic correction “mardi” → “mardi” in the Dictionary (one per day): it replaces any spelling with yours, capital included. Careful, it also puts the day in lower case at the start of a sentence.')}
      {@render question('A word is always misspelled.', 'Add it to “Words to write exactly”, or an automatic correction (what Whisper hears → what it should write). A bigger Whisper model hears better but takes longer: the Test bench lets you measure it on your phrases.')}
      {@render question('The music or video does not stop.', `Check that “Pause music or video” is ticked (Settings → Dictation). The log says what Scriibe sees: “rien ne joue” lists the audio apps and their level, “pause demandée” says who the command went to. During a call, nothing is paused, on purpose.`)}
      {@render question('The window is empty or does not open.', mac ? 'Click the icon in the menu bar → “Open Scriibe”. Closing the window does not quit Scriibe: to stop it, “Quit Scriibe”.' : linux ? 'Search for “Scriibe” in the application list, or click its icon at the top right → “Open Scriibe”. Closing the window does not quit Scriibe: to stop it, “Quit Scriibe”.' : 'Right-click the icon near the clock → “Open Scriibe”. If WebView2 is missing, the same interface opens in your browser. Closing the window does not quit Scriibe: to stop it, “Quit Scriibe”.')}
      {@render question('How do I update?', `Settings → Update: “Check for a new version” tells you whether a version newer than yours is published (version, date, source); no account or token is needed. ${mac ? '“Install and restart” asks for confirmation, downloads the new version, replaces the Scriibe.app application and restarts it, keeping the old one alongside.' : linux ? '“Install and restart” asks for confirmation, downloads the new version, replaces the scriibe program and restarts Scriibe, keeping the old one alongside (scriibe.old).' : '“Install and restart” asks for confirmation, downloads the new version, replaces scriibe.exe and restarts Scriibe, keeping the old one alongside (scriibe.exe.old).'} The automatic search (every 6 hours) is unticked at first. That page also explains what goes out on the Internet, which is nothing personal.`)}
      <div class="links">{@render go('Installation', 'install')}{@render go('Report and log', 'journal')}{@render go('Update', 'update')}</div>
    {/if}
  </section>
</div>

<style>
  .lead { margin: 0 0 12px; }
  .layout { display: flex; gap: 14px; align-items: flex-start; flex-wrap: wrap; }
  .toc { list-style: none; margin: 0; padding: 0; width: 220px; flex: none; display: flex; flex-direction: column; gap: 2px; }
  .toc button { width: 100%; text-align: left; border: 0; background: transparent; padding: 8px 10px; border-radius: 8px; }
  .toc button:hover { background: var(--line); }
  .toc button.active { background: var(--accent); color: var(--accent-ink); }
  .body { flex: 1 1 360px; margin: 0; max-width: 1100px; }
  .body h3 { margin: 18px 0 6px; font-size: 1rem; }
  .body p, .body li { line-height: 1.5; }
  .body ul, .body ol { padding-left: 1.3rem; display: flex; flex-direction: column; gap: 8px; }
  .example { margin: 10px 0; padding: 10px 12px; border-radius: 10px; background: var(--bg); }
  .example .line { display: flex; gap: 10px; align-items: baseline; white-space: pre-line; }
  .example .who { flex: none; width: 96px; font-size: 0.78rem; font-weight: 600; color: var(--muted); }
  .example .who.strong { color: var(--accent); }
  .example .note { margin: 4px 0 0 106px; font-size: 0.85rem; color: var(--muted); }
  .tip { margin: 12px 0; padding: 10px 12px; border-radius: 10px; background: color-mix(in srgb, var(--accent) 12%, transparent); }
  .q { margin: 12px 0; }
  .q > div { margin-top: 2px; line-height: 1.5; }
  .links { display: flex; gap: 8px; flex-wrap: wrap; margin-top: 14px; }
  .go { font-size: 0.9rem; }
  .samples { display: flex; gap: 10px; flex-wrap: wrap; margin: 10px 0; }
  .sample { display: flex; flex-direction: column; align-items: center; gap: 4px; }
  .stagebox { min-width: 150px; min-height: 64px; display: flex; align-items: center; justify-content: center; border-radius: 10px; background: var(--stage); }
</style>
