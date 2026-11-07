<script lang="ts">
  import Card from './Card.svelte';
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import { settings } from '../lib/settings.svelte';
  import { t, tx } from '../lib/i18n.svelte';

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
      errors['shortcut:' + name] = t('Raccourci illisible : écris par exemple « Mod+B » (⌘B sur Mac, Ctrl+B ailleurs), avec au moins Mod, Ctrl ou Alt.');
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

  onMount(() => {
    void settings.load().then(refreshLabels);
    void reload();
  });
</script>

<Card
  id="dictionary-commandes-vocales"
  title={t('Commandes vocales')}
  help="dictionary"
  intro={t('Quand du texte est sélectionné, une phrase dite seule (rien d’autre dans la dictée) agit sur lui au lieu d’être collée. Sans sélection, ou avec d’autres mots, elle reste du texte. Sur Mac pour l’instant.')}
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
      {#each group.commands as def (def.id)}
        {@const c = byId(def.id)}
        {#if c}
          <div class="command">
            <div class="head">
              <b>{t(def.label)}</b>
              {#if customized(c)}<button class="link" onclick={() => restore(c)}>{t('Rétablir')}</button>{/if}
            </div>
            {#each ['fr', 'en'] as const as lang (lang)}
              <div class="langrow">
                <span class="lang muted">{lang === 'fr' ? t('Français') : t('Anglais')}</span>
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
                  <input
                    type="text"
                    class="shortcut"
                    autocapitalize="off"
                    autocorrect="off"
                    autocomplete="off"
                    spellcheck="false"
                    bind:value={shortcutDrafts[def.shortcut]}
                    onchange={() => setShortcut(def.shortcut, shortcutDrafts[def.shortcut] ?? '')}
                  />
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
  </div>
  <p class="muted">{t('Une phrase compte au moins deux mots et ne sert qu’à une commande. Majuscules, accents et ponctuation sont ignorés. « Rétablir » remet les phrases d’origine.')}</p>
</Card>

<style>
  h3 { margin: 14px 0 4px; font-size: 1rem; }
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
  .shortcut { width: 10rem; }
  .link { border: 0; background: transparent; color: var(--accent); padding: 0; text-decoration: underline; cursor: pointer; }
  .error { color: var(--bad); margin: 4px 0 0 5.5rem; }
  .off { opacity: 0.5; pointer-events: none; }
</style>
