<script lang="ts">
  import Card from './Card.svelte';
  import ShortcutField from './ShortcutField.svelte';
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import { settings } from '../lib/settings.svelte';
  import { i18n, t, tx } from '../lib/i18n.svelte';

  type Lang = 'fr' | 'en';
  type Command = { id: string; fr_default: string[]; en_default: string[]; fr: string[]; en: string[] };
  type Check = { ok: boolean; phrase?: string; error?: 'too_short' | 'taken' | 'unknown'; other?: string };

  let commands = $state<Command[]>([]);
  let drafts = $state<Record<string, string>>({});
  let errors = $state<Record<string, string>>({});
  let shortcutDrafts = $state<Record<string, string>>({});
  let shortcutLabels = $state<Record<string, string>>({});
  let v = $derived(settings.value);

  /** Les commandes, par groupe, avec leur libellé (traduit à l'affichage). */
  const groups = [
    { title: tx('Ajouter au dictionnaire'), commands: [{ id: 'dictionnaire.ajouter', label: tx('Ajouter au dictionnaire') }] },
    {
      title: tx('Entourer la sélection'),
      commands: [
        { id: 'entourer.parentheses', label: tx('Entre parenthèses') },
        { id: 'entourer.guillemets', label: tx('Entre guillemets') },
        { id: 'entourer.crochets', label: tx('Entre crochets') },
        { id: 'entourer.accolades', label: tx('Entre accolades') },
      ],
    },
    {
      title: tx('Gras, italique, souligné'),
      commands: [
        { id: 'format.gras', label: tx('Gras'), shortcut: 'bold' },
        { id: 'format.italique', label: tx('Italique'), shortcut: 'italic' },
        { id: 'format.souligne', label: tx('Souligné'), shortcut: 'underline' },
      ],
    },
    {
      title: tx('Recherches sur internet'),
      intro: tx('Ces phrases ne marchent que sous la touche de commande : « nouvelle recherche YouTube chats rigolos » ouvre la recherche dans ton navigateur. Dites sans la touche, elles restent du texte.'),
      commands: [
        { id: 'recherche.1', label: tx('Google') },
        { id: 'recherche.2', label: tx('YouTube') },
        { id: 'recherche.3', label: tx('Wikipédia') },
        { id: 'recherche.4', label: tx('DuckDuckGo') },
      ],
    },
    {
      title: tx('Nouveau mail'),
      intro: tx('Ces phrases ne marchent que sous la touche de commande : « nouveau mail » ouvre un message vierge dans ta messagerie, « nouveau mail réunion de lundi » préremplit l’objet. Rien n’est envoyé, et il n’y a pas de destinataire : « nouveau mail à Marie » met « à Marie » en objet. Dites sans la touche, elles restent du texte.'),
      commands: [{ id: 'mail.nouveau', label: tx('Nouveau mail') }],
    },
  ];
  const labelOf = (id: string) => groups.flatMap((g) => g.commands).find((c) => c.id === id)?.label ?? id;
  const defaults: Record<string, string> = { bold: 'Mod+B', italic: 'Mod+I', underline: 'Mod+U' };

  const byId = (id: string) => commands.find((c) => c.id === id);
  const customized = (c: Command) => c.fr.join('\n') !== c.fr_default.join('\n') || c.en.join('\n') !== c.en_default.join('\n');

  async function reload() {
    commands = (await api<{ commands: Command[] }>('commands')).commands;
  }

  /** Enregistre les phrases : seules celles qui diffèrent des phrases par défaut sont gardées dans les réglages. */
  async function save(next: Command[]) {
    const fr: Record<string, string[]> = {};
    const en: Record<string, string[]> = {};
    for (const c of next) {
      if (c.fr.join('\n') !== c.fr_default.join('\n')) fr[c.id] = c.fr;
      if (c.en.join('\n') !== c.en_default.join('\n')) en[c.id] = c.en;
    }
    await settings.change({ voiceCommands: { fr, en } });
    await reload();
  }

  async function add(c: Command, lang: Lang) {
    const key = c.id + ':' + lang;
    const typed = (drafts[key] ?? '').trim();
    if (!typed) return;
    const check = await api<Check>('commands/validate-phrase', { id: c.id, lang, phrase: typed });
    if (!check.ok) {
      errors[key] =
        check.error === 'taken'
          ? t('Cette phrase sert déjà à la commande « {name} ».', { name: t(labelOf(check.other ?? '')) })
          : t('Une phrase compte au moins deux mots : un mot seul est trop souvent du texte à coller.');
      return;
    }
    errors[key] = '';
    drafts[key] = '';
    if (c[lang].some((p) => p.toLowerCase() === check.phrase!.toLowerCase())) return;
    await save(commands.map((x) => (x.id === c.id ? { ...x, [lang]: [...x[lang], check.phrase!] } : x)));
  }

  async function remove(c: Command, lang: Lang, index: number) {
    if (c[lang].length < 2) return;
    await save(commands.map((x) => (x.id === c.id ? { ...x, [lang]: x[lang].filter((_, i) => i !== index) } : x)));
  }

  async function restore(c: Command) {
    errors[c.id + ':fr'] = errors[c.id + ':en'] = '';
    await save(commands.map((x) => (x.id === c.id ? { ...x, fr: x.fr_default, en: x.en_default } : x)));
  }

  async function setEnabled(on: boolean) {
    await settings.change({ voiceCommandsEnabled: on });
  }

  /** Raccourci de l'appli appuyé pour gras, italique et souligné : contrôlé avant d'être enregistré. */
  async function setShortcut(name: string, text: string) {
    const check = await api<{ ok: boolean; label?: string }>('commands/validate-shortcut', { text });
    if (!check.ok) {
      errors['shortcut:' + name] = t('Raccourci illisible : écris par exemple « Mod+B » (⌘B sur Mac, Ctrl+B ailleurs), avec Mod ou Ctrl ; Mod+Q (quitter) est refusé.');
      return;
    }
    errors['shortcut:' + name] = '';
    shortcutLabels[name] = check.label ?? '';
    const current = (v?.formatShortcuts ?? defaults) as Record<string, string>;
    await settings.change({ formatShortcuts: { ...defaults, ...current, [name]: text.trim() } as { bold: string; italic: string; underline: string } });
  }

  async function refreshLabels() {
    const current = (v?.formatShortcuts ?? defaults) as Record<string, string>;
    for (const name of Object.keys(defaults)) {
      const text = current[name] ?? defaults[name];
      shortcutDrafts[name] = text;
      const check = await api<{ ok: boolean; label?: string }>('commands/validate-shortcut', { text });
      shortcutLabels[name] = check.label ?? '';
    }
  }

  // ---- Mes sites de recherche : un nom, une adresse https avec {q}, des phrases (sous la touche de commande seulement) ----
  type Site = { id: string; name: string; url: string; phrases: string[] };
  let sites = $derived((v?.webSearches ?? []) as Site[]);
  let siteDraft = $state({ name: '', url: '', phrase: '' });
  let siteError = $state('');
  let siteHosts = $state<Record<string, string>>({});

  const wordsIn = (text: string) => text.trim().split(/\s+/).filter(Boolean).length;
  const siteTooShort = () => t('Une phrase de recherche compte au moins deux mots (par exemple « nouvelle recherche amazon »).');
  const checkUrl = (url: string) => api<{ ok: boolean; host?: string; error?: string }>('web-searches/check', { url });
  const nextSiteId = () => 'recherche.' + (Math.max(9, ...sites.map((s) => Number(s.id.split('.')[1]) || 0)) + 1);

  async function addSite() {
    const name = siteDraft.name.trim();
    const url = siteDraft.url.trim();
    const phrase = siteDraft.phrase.trim();
    if (!name || !url || !phrase) {
      siteError = t('Donne un nom, une adresse avec {q} à l’endroit où va le texte cherché, et une phrase à dire.', { q: '{q}' });
      return;
    }
    const address = await checkUrl(url);
    if (!address.ok) {
      siteError = t(address.error ?? 'Adresse refusée');
      return;
    }
    if (wordsIn(phrase) < 2) {
      siteError = siteTooShort();
      return;
    }
    const check = await api<Check>('commands/validate-phrase', { id: 'recherche.nouveau', lang: i18n.lang, phrase });
    if (!check.ok) {
      siteError = check.error === 'taken' ? takenMessage(check.other) : siteTooShort();
      return;
    }
    siteError = '';
    await settings.change({ webSearches: [...sites, { id: nextSiteId(), name, url, phrases: [check.phrase!] }] });
    siteDraft = { name: '', url: '', phrase: '' };
  }

  const updateSite = (id: string, patch: Partial<Site>) => settings.change({ webSearches: sites.map((s) => (s.id === id ? { ...s, ...patch } : s)) });
  const removeSite = (id: string) => settings.change({ webSearches: sites.filter((s) => s.id !== id) });

  async function addSitePhrase(s: Site) {
    const key = s.id + ':phrase';
    const typed = (drafts[key] ?? '').trim();
    if (!typed) return;
    if (wordsIn(typed) < 2) {
      errors[key] = siteTooShort();
      return;
    }
    const check = await api<Check>('commands/validate-phrase', { id: s.id, lang: i18n.lang, phrase: typed });
    if (!check.ok) {
      errors[key] = check.error === 'taken' ? takenMessage(check.other) : siteTooShort();
      return;
    }
    errors[key] = '';
    drafts[key] = '';
    if (!s.phrases.some((p) => p.toLowerCase() === check.phrase!.toLowerCase())) await updateSite(s.id, { phrases: [...s.phrases, check.phrase!] });
  }

  // L'hôte que fixe l'adresse de chaque site, redemandé quand elle change.
  $effect(() => {
    for (const s of sites) void checkUrl(s.url).then((r) => (siteHosts[s.id] = r.host ?? ''));
  });

  // ---- Mes extraits : un nom, des phrases, un texte collé tel quel (sous la touche de commande seulement) ----
  type Snippet = { id: string; name: string; phrases: string[]; text: string };
  const SNIPPET_MAX = 5000;
  let snippets = $derived((v?.snippets ?? []) as Snippet[]);
  let snippetDraft = $state({ name: '', phrase: '', text: '' });
  let snippetError = $state('');
  let textDrafts = $state<Record<string, string>>({});
  const nextSnippetId = () => 'extrait.' + (Math.max(0, ...snippets.map((s) => Number(s.id.split('.')[1]) || 0)) + 1);
  const checkSnippetText = (text: string) => api<{ ok: boolean; text?: string; error?: string }>('snippets/check', { text });
  const snippetPhraseMessage = () => t('Donne une phrase à dire (un mot seul suffit : elle doit être toute la phrase dite).');

  async function addSnippet() {
    const name = snippetDraft.name.trim();
    const phrase = snippetDraft.phrase.trim();
    if (!name || !phrase || !snippetDraft.text.trim()) {
      snippetError = t('Donne un nom, une phrase à dire et le texte à coller.');
      return;
    }
    const text = await checkSnippetText(snippetDraft.text);
    if (!text.ok) {
      snippetError = t(text.error ?? 'Texte refusé');
      return;
    }
    const check = await api<Check>('commands/validate-phrase', { id: 'extrait.nouveau', lang: i18n.lang, phrase });
    if (!check.ok) {
      snippetError = check.error === 'taken' ? takenMessage(check.other) : snippetPhraseMessage();
      return;
    }
    snippetError = '';
    await settings.change({ snippets: [...snippets, { id: nextSnippetId(), name, phrases: [check.phrase!], text: text.text! }] });
    snippetDraft = { name: '', phrase: '', text: '' };
  }

  const updateSnippet = (id: string, patch: Partial<Snippet>) => settings.change({ snippets: snippets.map((s) => (s.id === id ? { ...s, ...patch } : s)) });
  const removeSnippet = (id: string) => settings.change({ snippets: snippets.filter((s) => s.id !== id) });

  async function addSnippetPhrase(s: Snippet) {
    const key = s.id + ':phrase';
    const typed = (drafts[key] ?? '').trim();
    if (!typed) return;
    const check = await api<Check>('commands/validate-phrase', { id: s.id, lang: i18n.lang, phrase: typed });
    if (!check.ok) {
      errors[key] = check.error === 'taken' ? takenMessage(check.other) : snippetPhraseMessage();
      return;
    }
    errors[key] = '';
    drafts[key] = '';
    if (!s.phrases.some((p) => p.toLowerCase() === check.phrase!.toLowerCase())) await updateSnippet(s.id, { phrases: [...s.phrases, check.phrase!] });
  }

  async function saveSnippetText(s: Snippet) {
    const draft = textDrafts[s.id];
    if (draft === undefined || draft === s.text) return;
    const check = await checkSnippetText(draft);
    const key = s.id + ':text';
    if (!check.ok) {
      errors[key] = t(check.error ?? 'Texte refusé');
      return;
    }
    errors[key] = '';
    delete textDrafts[s.id];
    await updateSnippet(s.id, { text: check.text! });
  }

  // ---- Mes applications : une application installée choisie dans la liste, des phrases (sous la touche de commande seulement) ----
  type VoiceApp = { id: string; name: string; appId: string; phrases: string[] };
  type Installed = { id: string; name: string };
  let voiceApps = $derived((v?.voiceApps ?? []) as VoiceApp[]);
  let installed = $state<Installed[]>([]);
  let installedLoaded = $state(false);
  let appDraft = $state({ appId: '', phrase: '' });
  let appError = $state('');
  const nextAppId = () => 'appli.' + (Math.max(0, ...voiceApps.map((a) => Number(a.id.split('.')[1]) || 0)) + 1);

  async function loadInstalled() {
    try {
      installed = (await api<{ apps: Installed[] }>('installed-apps')).apps;
      installedLoaded = true;
    } catch {
      installedLoaded = false;
    }
  }

  async function addApp() {
    const app = installed.find((a) => a.id === appDraft.appId);
    const phrase = appDraft.phrase.trim();
    if (!app || !phrase) {
      appError = t('Choisis une application et donne une phrase à dire.');
      return;
    }
    const check = await api<Check>('commands/validate-phrase', { id: 'appli.nouveau', lang: i18n.lang, phrase });
    if (!check.ok) {
      appError = check.error === 'taken' ? takenMessage(check.other) : snippetPhraseMessage();
      return;
    }
    appError = '';
    await settings.change({ voiceApps: [...voiceApps, { id: nextAppId(), name: app.name, appId: app.id, phrases: [check.phrase!] }] });
    appDraft = { appId: '', phrase: '' };
  }

  const updateApp = (id: string, patch: Partial<VoiceApp>) => settings.change({ voiceApps: voiceApps.map((a) => (a.id === id ? { ...a, ...patch } : a)) });
  const removeApp = (id: string) => settings.change({ voiceApps: voiceApps.filter((a) => a.id !== id) });

  async function addAppPhrase(a: VoiceApp) {
    const key = a.id + ':phrase';
    const typed = (drafts[key] ?? '').trim();
    if (!typed) return;
    const check = await api<Check>('commands/validate-phrase', { id: a.id, lang: i18n.lang, phrase: typed });
    if (!check.ok) {
      errors[key] = check.error === 'taken' ? takenMessage(check.other) : snippetPhraseMessage();
      return;
    }
    errors[key] = '';
    drafts[key] = '';
    if (!a.phrases.some((p) => p.toLowerCase() === check.phrase!.toLowerCase())) await updateApp(a.id, { phrases: [...a.phrases, check.phrase!] });
  }

  // ---- Mes commandes : un titre, une phrase, un raccourci de l'appli active ----
  type Custom = { id: string; name: string; phrases: string[]; shortcut: string; needsSelection: boolean };
  let customs = $derived((v?.customCommands ?? []) as Custom[]);
  let customDraft = $state({ name: '', phrase: '', shortcut: '', needsSelection: true });
  let customError = $state('');
  let customLabels = $state<Record<string, string>>({});
  let customShortcutDrafts = $state<Record<string, string>>({});

  const nameOf = (id: string) => customs.find((c) => c.id === id)?.name ?? sites.find((c) => c.id === id)?.name ?? snippets.find((c) => c.id === id)?.name ?? voiceApps.find((c) => c.id === id)?.name;
  const takenMessage = (other?: string) => t('Cette phrase sert déjà à la commande « {name} ».', { name: nameOf(other ?? '') ?? t(labelOf(other ?? '')) });
  const tooShortMessage = () => t('Une phrase compte au moins deux mots : un mot seul est trop souvent du texte à coller.');
  const badShortcutMessage = () => t('Raccourci illisible : écris par exemple « Mod+B » (⌘B sur Mac, Ctrl+B ailleurs), avec Mod ou Ctrl ; Mod+Q (quitter) est refusé.');

  async function shortcutLabel(text: string): Promise<{ ok: boolean; label?: string }> {
    return api<{ ok: boolean; label?: string }>('commands/validate-shortcut', { text });
  }

  async function addCustom() {
    const name = customDraft.name.trim();
    const phrase = customDraft.phrase.trim();
    const shortcut = customDraft.shortcut.trim();
    if (!name || !phrase || !shortcut) {
      customError = t('Donne un titre, une phrase à dire et un raccourci.');
      return;
    }
    const check = await api<Check>('commands/validate-phrase', { id: 'perso.nouveau', lang: i18n.lang, phrase });
    if (!check.ok) {
      customError = check.error === 'taken' ? takenMessage(check.other) : tooShortMessage();
      return;
    }
    if (!(await shortcutLabel(shortcut)).ok) {
      customError = badShortcutMessage();
      return;
    }
    customError = '';
    const entry: Custom = { id: 'perso.' + Date.now(), name, phrases: [check.phrase!], shortcut, needsSelection: customDraft.needsSelection };
    await settings.change({ customCommands: [...customs, entry] });
    customDraft = { name: '', phrase: '', shortcut: '', needsSelection: true };
  }

  /** Exporter et importer les commandes perso : aperçu avant tout, l'import ajoute sans jamais remplacer. */
  type Imported = { name: string; phrases: string[]; dropped: string[]; shortcut: string; needs_selection: boolean; problem: 'no_name' | 'bad_shortcut' | 'no_phrase' | null };
  let transfer = $state<{ mode: 'export' | 'import'; items: Imported[]; tick: boolean[]; token: string; askPath: boolean; path: string; busy: boolean; message: string; done: boolean } | null>(null);

  const problemText = (p: Imported['problem']) =>
    p === 'bad_shortcut' ? t('raccourci illisible : cette commande ne sera pas importée') : p === 'no_phrase' ? t('plus aucune phrase utilisable : cette commande ne sera pas importée') : t('sans titre : cette commande ne sera pas importée');

  async function openTransfer(mode: 'export' | 'import') {
    if (transfer && !transfer.done && transfer.mode === 'import') await api('commands/import/cancel', {});
    transfer = { mode, items: [], tick: [], token: '', askPath: false, path: '', busy: false, message: '', done: false };
    if (mode === 'import') await readImport();
  }

  async function exportCommands() {
    if (!transfer) return;
    transfer.busy = true;
    const out = await api<{ ok: boolean; saved?: boolean; path?: string; error?: string }>('commands/export', {});
    transfer.busy = false;
    transfer.message = !out.ok
      ? t(out.error ?? 'enregistrement impossible : l’emplacement choisi n’est pas accessible en écriture')
      : out.saved
        ? t('Enregistré : {path}', { path: out.path ?? '' })
        : t('Export annulé : rien n’a été enregistré.');
  }

  async function readImport(path?: string) {
    if (!transfer) return;
    transfer.busy = true;
    transfer.message = '';
    const out = await api<{ ok: boolean; cancelled?: boolean; ask_path?: boolean; error?: string; token?: string; items?: Imported[] }>('commands/import/read', path ? { path } : {});
    if (!transfer) return;
    transfer.busy = false;
    if (out.ok && out.cancelled) transfer = null;
    else if (out.ok && out.items) {
      transfer.askPath = false;
      transfer.items = out.items;
      transfer.token = out.token ?? '';
      transfer.tick = out.items.map((i) => i.problem === null);
    } else if (out.ask_path) transfer.askPath = true;
    else transfer.message = t(out.error ?? "le fichier n'a pas pu être lu");
  }

  async function applyImport() {
    if (!transfer) return;
    transfer.busy = true;
    const out = await api<{ ok: boolean; added?: string[]; skipped?: number; error?: string }>('commands/import/apply', { token: transfer.token, include: transfer.tick });
    transfer.busy = false;
    if (!out.ok) {
      transfer.message = t(out.error ?? "le fichier n'a pas pu être lu");
      return;
    }
    transfer.items = [];
    transfer.done = true;
    transfer.message = t('Importé : {n} commande(s) ajoutée(s).', { n: out.added?.length ?? 0 });
    // Le serveur a écrit les réglages : la page les relit, pour ne pas les écraser plus tard.
    await settings.load();
  }

  async function closeTransfer() {
    if (transfer && !transfer.done && transfer.mode === 'import') await api('commands/import/cancel', {});
    transfer = null;
  }

  async function updateCustom(id: string, patch: Partial<Custom>) {
    await settings.change({ customCommands: customs.map((c) => (c.id === id ? { ...c, ...patch } : c)) });
  }

  async function removeCustom(id: string) {
    await settings.change({ customCommands: customs.filter((c) => c.id !== id) });
  }

  async function addCustomPhrase(c: Custom) {
    const key = c.id + ':phrase';
    const typed = (drafts[key] ?? '').trim();
    if (!typed) return;
    const check = await api<Check>('commands/validate-phrase', { id: c.id, lang: i18n.lang, phrase: typed });
    if (!check.ok) {
      errors[key] = check.error === 'taken' ? takenMessage(check.other) : tooShortMessage();
      return;
    }
    errors[key] = '';
    drafts[key] = '';
    if (!c.phrases.some((p) => p.toLowerCase() === check.phrase!.toLowerCase())) await updateCustom(c.id, { phrases: [...c.phrases, check.phrase!] });
  }

  async function setCustomShortcut(c: Custom, text: string) {
    const check = await shortcutLabel(text);
    if (!check.ok) {
      errors[c.id + ':shortcut'] = badShortcutMessage();
      return;
    }
    errors[c.id + ':shortcut'] = '';
    await updateCustom(c.id, { shortcut: text.trim() });
  }

  // Le raccourci lu (« ⌘⇧X ») de chaque commande, redemandé quand le raccourci change.
  $effect(() => {
    for (const c of customs) {
      customShortcutDrafts[c.id] ??= c.shortcut;
      void shortcutLabel(c.shortcut).then((r) => (customLabels[c.id] = r.label ?? ''));
    }
  });

  onMount(() => {
    void settings.load().then(refreshLabels);
    void reload();
    void loadInstalled();
  });
</script>

<Card
  id="dictionary-commandes-vocales"
  title={t('Commandes vocales')}
  help="dictionary"
  intro={t('Quand du texte est sélectionné, une phrase dite seule (rien d’autre dans la dictée) agit sur lui au lieu d’être collée. Sans sélection, ou avec d’autres mots, elle reste du texte. Sur Mac pour l’instant. Les phrases affichées sont celles de la langue de l’interface (Réglages, onglet Apparence).')}
>
  {#if v}
    <div class="row">
      <label>
        <input type="checkbox" checked={v.voiceCommandsEnabled !== false} onchange={(e) => setEnabled(e.currentTarget.checked)} />
        {t('Activer les commandes vocales')}
      </label>
    </div>
  {/if}
  <div class:off={v?.voiceCommandsEnabled === false}>
    {#each groups as group (group.title)}
      {#if group.commands.length > 1}<h3>{t(group.title)}</h3>{/if}
      {#if 'intro' in group}<p class="muted">{t(group.intro as string)}</p>{/if}
      {#each group.commands as def (def.id)}
        {@const c = byId(def.id)}
        {#if c}
          <div class="command">
            <div class="head">
              <b>{t(def.label)}</b>
              {#if customized(c)}<button class="link" onclick={() => restore(c)}>{t('Rétablir')}</button>{/if}
            </div>
            {#each [i18n.lang] as lang (lang)}
              <div class="langrow">
                <span class="lang muted">{t('Phrases')}</span>
                <div class="chips">
                  {#each c[lang] as phrase, i (phrase)}
                    <span class="chip">{phrase}<button class="x" title={c[lang].length < 2 ? t('Une commande garde au moins une phrase') : t('Retirer')} disabled={c[lang].length < 2} onclick={() => remove(c, lang, i)}>×</button></span>
                  {/each}
                  <input
                    type="text"
                    class="phrase"
                    autocapitalize="off"
                    autocorrect="off"
                    autocomplete="off"
                    spellcheck="false"
                    placeholder={t('Ajouter une phrase')}
                    bind:value={drafts[c.id + ':' + lang]}
                    onkeydown={(e) => e.key === 'Enter' && add(c, lang)}
                  />
                  <button onclick={() => add(c, lang)} disabled={!(drafts[c.id + ':' + lang] ?? '').trim()}>{t('Ajouter')}</button>
                </div>
              </div>
              {#if errors[c.id + ':' + lang]}<p class="error">{errors[c.id + ':' + lang]}</p>{/if}
            {/each}
            {#if 'shortcut' in def}
              <div class="langrow">
                <span class="lang muted">{t('Raccourci')}</span>
                <div class="chips">
                  <ShortcutField value={shortcutDrafts[def.shortcut] ?? ''} onset={(s) => { shortcutDrafts[def.shortcut] = s; void setShortcut(def.shortcut, s); }} />
                  <span class="muted">→ {shortcutLabels[def.shortcut] ?? ''}</span>
                  {#if (shortcutDrafts[def.shortcut] ?? '') !== defaults[def.shortcut]}
                    <button class="link" onclick={() => { shortcutDrafts[def.shortcut] = defaults[def.shortcut]; void setShortcut(def.shortcut, defaults[def.shortcut]); }}>{t('Rétablir')}</button>
                  {/if}
                </div>
              </div>
              {#if errors['shortcut:' + def.shortcut]}<p class="error">{errors['shortcut:' + def.shortcut]}</p>{/if}
            {/if}
          </div>
        {/if}
      {/each}
    {/each}

    <h3>{t('Mes sites de recherche')}</h3>
    <p class="muted">{t('Ajoute un site de recherche : dis ta phrase suivie de ce que tu cherches, et l’adresse s’ouvre dans ton navigateur, avec ce texte à la place de {q}. L’adresse doit commencer par https:// et le nom du site doit être fixe : {q} ne peut pas être avant le premier /. Ça ne marche que sous la touche de commande.', { q: '{q}' })}</p>
    {#each sites as s (s.id)}
      <div class="command">
        <div class="head">
          <b>{s.name}</b>
          <span class="muted">{t('Ouvre {host}', { host: siteHosts[s.id] ?? '' })}</span>
          <button class="link" onclick={() => removeSite(s.id)}>{t('Supprimer')}</button>
        </div>
        <div class="langrow">
          <span class="lang muted">{t('Phrases')}</span>
          <div class="chips">
            {#each s.phrases as phrase, i (phrase)}
              <span class="chip">{phrase}<button class="x" title={s.phrases.length < 2 ? t('Une commande garde au moins une phrase') : t('Retirer')} disabled={s.phrases.length < 2} onclick={() => updateSite(s.id, { phrases: s.phrases.filter((_, j) => j !== i) })}>×</button></span>
            {/each}
            <input type="text" class="phrase" autocapitalize="off" autocorrect="off" autocomplete="off" spellcheck="false" placeholder={t('Ajouter une phrase')} bind:value={drafts[s.id + ':phrase']} onkeydown={(e) => e.key === 'Enter' && addSitePhrase(s)} />
            <button onclick={() => addSitePhrase(s)} disabled={!(drafts[s.id + ':phrase'] ?? '').trim()}>{t('Ajouter')}</button>
          </div>
        </div>
        {#if errors[s.id + ':phrase']}<p class="error">{errors[s.id + ':phrase']}</p>{/if}
      </div>
    {/each}
    {#if sites.length === 0}<p class="muted">{t('Aucun site à toi pour l’instant.')}</p>{/if}
    <div class="command newcommand">
      <div class="head"><b>{t('Nouveau site')}</b></div>
      <div class="chips">
        <input type="text" placeholder={t('Nom (ex. : Amazon)')} bind:value={siteDraft.name} />
        <input type="text" class="phrase" autocapitalize="off" autocorrect="off" autocomplete="off" spellcheck="false" placeholder={t('Adresse (ex. : https://www.amazon.fr/s?k={q})', { q: '{q}' })} bind:value={siteDraft.url} />
        <input type="text" class="phrase" autocapitalize="off" autocorrect="off" autocomplete="off" spellcheck="false" placeholder={t('Phrase à dire (ex. : nouvelle recherche amazon)')} bind:value={siteDraft.phrase} />
        <button onclick={addSite}>{t('Ajouter le site')}</button>
      </div>
      {#if siteError}<p class="error nomargin">{siteError}</p>{/if}
    </div>

    <h3>{t('Mes extraits')}</h3>
    <p class="muted">{t('Un extrait est un texte que tu colles en disant une phrase : « insérer mon adresse email » colle ton adresse. Il est collé tel quel (plusieurs lignes permises, 5 000 signes au plus), jamais gardé dans l’historique, et le presse-papiers d’avant est remis. Ça ne marche que sous la touche de commande ; dite sans elle, la phrase reste du texte. Un extrait peut contenir une adresse ou un IBAN : il n’est exporté que si tu le coches.')}</p>
    {#each snippets as s (s.id)}
      <div class="command">
        <div class="head">
          <b>{s.name}</b>
          <span class="muted">{t('{n} signe(s)', { n: s.text.length })}</span>
          <button class="link" onclick={() => removeSnippet(s.id)}>{t('Supprimer')}</button>
        </div>
        <div class="langrow">
          <span class="lang muted">{t('Phrases')}</span>
          <div class="chips">
            {#each s.phrases as phrase, i (phrase)}
              <span class="chip">{phrase}<button class="x" title={s.phrases.length < 2 ? t('Une commande garde au moins une phrase') : t('Retirer')} disabled={s.phrases.length < 2} onclick={() => updateSnippet(s.id, { phrases: s.phrases.filter((_, j) => j !== i) })}>×</button></span>
            {/each}
            <input type="text" class="phrase" autocapitalize="off" autocorrect="off" autocomplete="off" spellcheck="false" placeholder={t('Ajouter une phrase')} bind:value={drafts[s.id + ':phrase']} onkeydown={(e) => e.key === 'Enter' && addSnippetPhrase(s)} />
            <button onclick={() => addSnippetPhrase(s)} disabled={!(drafts[s.id + ':phrase'] ?? '').trim()}>{t('Ajouter')}</button>
          </div>
        </div>
        {#if errors[s.id + ':phrase']}<p class="error">{errors[s.id + ':phrase']}</p>{/if}
        <textarea class="snippettext" rows="3" maxlength={SNIPPET_MAX * 2} spellcheck="false" aria-label={t('Texte à coller')} value={textDrafts[s.id] ?? s.text} oninput={(e) => (textDrafts[s.id] = e.currentTarget.value)} onblur={() => saveSnippetText(s)}></textarea>
        {#if errors[s.id + ':text']}<p class="error">{errors[s.id + ':text']}</p>{/if}
      </div>
    {/each}
    {#if snippets.length === 0}<p class="muted">{t('Aucun extrait pour l’instant.')}</p>{/if}
    <div class="command newcommand">
      <div class="head"><b>{t('Nouvel extrait')}</b></div>
      <div class="chips">
        <input type="text" placeholder={t('Nom (ex. : Adresse email)')} bind:value={snippetDraft.name} />
        <input type="text" class="phrase" autocapitalize="off" autocorrect="off" autocomplete="off" spellcheck="false" placeholder={t('Phrase à dire (ex. : insérer mon adresse email)')} bind:value={snippetDraft.phrase} />
      </div>
      <textarea class="snippettext" rows="3" maxlength={SNIPPET_MAX * 2} spellcheck="false" placeholder={t('Texte à coller')} bind:value={snippetDraft.text}></textarea>
      <div class="chips"><button onclick={addSnippet}>{t('Ajouter l’extrait')}</button></div>
      {#if snippetError}<p class="error nomargin">{snippetError}</p>{/if}
    </div>

    <h3>{t('Mes applications')}</h3>
    <p class="muted">{t('Choisis une application installée sur cet ordinateur et une phrase : dite sous la touche de commande, elle ouvre l’application. Seules les applications de cette liste peuvent s’ouvrir, sans rien d’autre (ni commande, ni option). Cette liste reste sur cet ordinateur : elle n’est pas exportée avec l’utilisateur.')}</p>
    {#each voiceApps as a (a.id)}
      <div class="command">
        <div class="head">
          <b>{a.name}</b>
          <button class="link" onclick={() => removeApp(a.id)}>{t('Supprimer')}</button>
        </div>
        <div class="langrow">
          <span class="lang muted">{t('Phrases')}</span>
          <div class="chips">
            {#each a.phrases as phrase, i (phrase)}
              <span class="chip">{phrase}<button class="x" title={a.phrases.length < 2 ? t('Une commande garde au moins une phrase') : t('Retirer')} disabled={a.phrases.length < 2} onclick={() => updateApp(a.id, { phrases: a.phrases.filter((_, j) => j !== i) })}>×</button></span>
            {/each}
            <input type="text" class="phrase" autocapitalize="off" autocorrect="off" autocomplete="off" spellcheck="false" placeholder={t('Ajouter une phrase')} bind:value={drafts[a.id + ':phrase']} onkeydown={(e) => e.key === 'Enter' && addAppPhrase(a)} />
            <button onclick={() => addAppPhrase(a)} disabled={!(drafts[a.id + ':phrase'] ?? '').trim()}>{t('Ajouter')}</button>
          </div>
        </div>
        {#if errors[a.id + ':phrase']}<p class="error">{errors[a.id + ':phrase']}</p>{/if}
        {#if installedLoaded && !installed.some((x) => x.id === a.appId)}<p class="error">{t('Cette application n’est plus installée : la phrase ne l’ouvrira pas.')}</p>{/if}
      </div>
    {/each}
    {#if voiceApps.length === 0}<p class="muted">{t('Aucune application pour l’instant.')}</p>{/if}
    <div class="command newcommand">
      <div class="head"><b>{t('Nouvelle application')}</b></div>
      <div class="chips">
        <select aria-label={t('Application installée')} bind:value={appDraft.appId}>
          <option value="">{t('Choisir une application…')}</option>
          {#each installed as a (a.id)}<option value={a.id}>{a.name}</option>{/each}
        </select>
        <input type="text" class="phrase" autocapitalize="off" autocorrect="off" autocomplete="off" spellcheck="false" placeholder={t('Phrase à dire (ex. : ouvre le navigateur)')} bind:value={appDraft.phrase} />
        <button onclick={addApp}>{t('Ajouter l’application')}</button>
      </div>
      {#if appError}<p class="error nomargin">{appError}</p>{/if}
    </div>

    <h3>{t('Mes commandes')}</h3>
    <p class="muted">{t('Donne un titre à une phrase qui appuie un raccourci de l’appli active, par exemple ⌘⇧X sur Mac. Clique dans le champ du raccourci, puis appuie sur la combinaison voulue. Un raccourci qui existe déjà dans une appli s’appelle ainsi par la voix.')}</p>
    {#each customs as c (c.id)}
      <div class="command">
        <div class="head">
          <b>{c.name}</b>
          <label class="inline">
            <input type="checkbox" checked={c.needsSelection} onchange={(e) => updateCustom(c.id, { needsSelection: e.currentTarget.checked })} />
            {t('Seulement si du texte est sélectionné')}
          </label>
          <button class="link" onclick={() => removeCustom(c.id)}>{t('Supprimer')}</button>
        </div>
        <div class="langrow">
          <span class="lang muted">{t('Phrases')}</span>
          <div class="chips">
            {#each c.phrases as phrase, i (phrase)}
              <span class="chip">{phrase}<button class="x" title={c.phrases.length < 2 ? t('Une commande garde au moins une phrase') : t('Retirer')} disabled={c.phrases.length < 2} onclick={() => updateCustom(c.id, { phrases: c.phrases.filter((_, j) => j !== i) })}>×</button></span>
            {/each}
            <input type="text" class="phrase" autocapitalize="off" autocorrect="off" autocomplete="off" spellcheck="false" placeholder={t('Ajouter une phrase')} bind:value={drafts[c.id + ':phrase']} onkeydown={(e) => e.key === 'Enter' && addCustomPhrase(c)} />
            <button onclick={() => addCustomPhrase(c)} disabled={!(drafts[c.id + ':phrase'] ?? '').trim()}>{t('Ajouter')}</button>
          </div>
        </div>
        {#if errors[c.id + ':phrase']}<p class="error">{errors[c.id + ':phrase']}</p>{/if}
        <div class="langrow">
          <span class="lang muted">{t('Raccourci')}</span>
          <div class="chips">
            <ShortcutField value={customShortcutDrafts[c.id] ?? c.shortcut} onset={(s) => { customShortcutDrafts[c.id] = s; void setCustomShortcut(c, s); }} />
            <span class="muted">→ {customLabels[c.id] ?? ''}</span>
          </div>
        </div>
        {#if errors[c.id + ':shortcut']}<p class="error">{errors[c.id + ':shortcut']}</p>{/if}
      </div>
    {/each}
    {#if customs.length === 0}<p class="muted">{t('Aucune commande à toi pour l’instant.')}</p>{/if}
    <div class="row">
      <button onclick={() => openTransfer('export')} disabled={customs.length === 0 || transfer !== null}>{t('Exporter mes commandes…')}</button>
      <button onclick={() => openTransfer('import')} disabled={transfer !== null}>{t('Importer des commandes…')}</button>
    </div>
    {#if transfer?.mode === 'export'}
      <div class="command">
        <p>{t('Le fichier (.scriibe.json) contient tes {n} commande(s) : titre, phrases et raccourci. Une boîte te demande où l’enregistrer (sous Linux, il est placé dans le dossier Documents). Le fichier n’est pas chiffré : relis-le avant de le partager.', { n: customs.length })}</p>
        {#if transfer.message}<p role="status"><b>{transfer.message}</b></p>{/if}
        <div class="chips"><button onclick={closeTransfer}>{t('Fermer')}</button><button class="primary" disabled={transfer.busy} onclick={exportCommands}>{t('Enregistrer le fichier')}</button></div>
      </div>
    {:else if transfer?.mode === 'import'}
      <div class="command">
        {#if transfer.askPath}
          <p>{t('Sur ce système, indique le chemin du fichier .scriibe.json à importer :')}</p>
          <div class="chips"><input type="text" class="phrase" bind:value={transfer.path} placeholder="/home/…/scriibe-commandes.scriibe.json" disabled={transfer.busy} />
            <button class="primary" disabled={transfer.busy || !transfer.path.trim()} onclick={() => readImport(transfer!.path)}>{t('Lire le fichier')}</button></div>
        {:else if transfer.items.length > 0}
          <p>{t('Ce fichier contient {n} commande(s). Rien n’est modifié tant que tu n’as pas cliqué sur « Importer ». L’import ajoute : tes commandes ne sont jamais remplacées. Si le fichier vient de quelqu’un d’autre, relis-le : une commande appuie un raccourci de l’appli active quand on dit sa phrase.', { n: transfer.items.length })}</p>
          {#each transfer.items as item, i (i)}
            <label class="pickrow">
              <input type="checkbox" bind:checked={transfer.tick[i]} disabled={transfer.busy || item.problem !== null} />
              <span><b>{item.name || t('(sans titre)')}</b> · {item.shortcut || '—'}<br />
                <span class="muted">{item.phrases.map((p) => '« ' + p + ' »').join(' ')}</span>
                {#if item.dropped.length > 0}<br /><span class="warn">{t('Phrase(s) écartée(s) (déjà prises ou trop courtes) : {list}', { list: item.dropped.map((p) => '« ' + p + ' »').join(' ') })}</span>{/if}
                {#if !item.needs_selection}<br /><span class="warn">{t('Agit même sans texte sélectionné : le raccourci est appuyé à chaque fois que la phrase est dite.')}</span>{/if}
                {#if item.problem}<br /><span class="warn">{problemText(item.problem)}</span>{/if}</span>
            </label>
          {/each}
        {:else if transfer.busy}
          <p class="muted">{t('Chargement…')}</p>
        {/if}
        {#if transfer.message}<p role="status" class:warn={!transfer.done}><b>{transfer.message}</b></p>{/if}
        <div class="chips">
          <button onclick={closeTransfer}>{transfer.done ? t('Fermer') : t('Annuler')}</button>
          {#if transfer.items.length > 0}<button class="primary" disabled={transfer.busy || !transfer.tick.some(Boolean)} onclick={applyImport}>{t('Importer')}</button>{/if}
        </div>
      </div>
    {/if}
    <div class="command newcommand">
      <div class="head"><b>{t('Nouvelle commande')}</b></div>
      <div class="chips">
        <input type="text" placeholder={t('Titre (ex. : Barré)')} bind:value={customDraft.name} />
        <input type="text" class="phrase" autocapitalize="off" autocorrect="off" autocomplete="off" spellcheck="false" placeholder={t('Phrase à dire (ex. : mettre en barré)')} bind:value={customDraft.phrase} />
        <ShortcutField value={customDraft.shortcut} onset={(s) => (customDraft.shortcut = s)} />
        <label class="inline"><input type="checkbox" bind:checked={customDraft.needsSelection} /> {t('Seulement si du texte est sélectionné')}</label>
        <button onclick={addCustom}>{t('Ajouter la commande')}</button>
      </div>
      {#if customError}<p class="error nomargin">{customError}</p>{/if}
    </div>
  </div>
  <p class="muted">{t('Une phrase compte au moins deux mots et ne sert qu’à une commande. Majuscules, accents et ponctuation sont ignorés. « Rétablir » remet les phrases d’origine.')}</p>
</Card>

<style>
  h3 { margin: 14px 0 4px; font-size: 1rem; }
  .snippettext { width: 100%; box-sizing: border-box; margin-top: 6px; font: inherit; resize: vertical; }
  .command { border-top: 1px solid var(--line); padding: 8px 0; }
  .head { display: flex; align-items: center; gap: 12px; }
  .langrow { display: flex; gap: 10px; align-items: flex-start; margin-top: 6px; }
  .lang { min-width: 5.5rem; padding-top: 4px; }
  .chips { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; flex: 1; }
  .chip { display: inline-flex; align-items: center; gap: 4px; background: var(--bg); border: 1px solid var(--line); border-radius: 999px; padding: 2px 4px 2px 12px; }
  .x { border: 0; background: transparent; padding: 0 8px; font-size: 1.1rem; color: var(--muted); }
  .x:hover:not(:disabled) { color: var(--bad); }
  .x:disabled { opacity: 0.35; cursor: not-allowed; }
  .phrase { min-width: 12rem; }
  .link { border: 0; background: transparent; color: var(--accent); padding: 0; text-decoration: underline; cursor: pointer; }
  .error { color: var(--bad); margin: 4px 0 0 5.5rem; }
  .pickrow { display: flex; gap: 10px; align-items: flex-start; padding: 4px 0; cursor: pointer; }
  .warn { color: var(--warn); }
  .inline { display: inline-flex; align-items: center; gap: 6px; }
  .error.nomargin { margin-left: 0; }
  .newcommand { margin-top: 10px; }
  .off { opacity: 0.5; pointer-events: none; }
</style>
