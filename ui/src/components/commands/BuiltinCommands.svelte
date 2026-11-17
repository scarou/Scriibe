<script lang="ts">
  import Card from '../Card.svelte';
  import { api } from '../../lib/api';
  import { settings } from '../../lib/settings.svelte';
  import { i18n, t } from '../../lib/i18n.svelte';
  import '../commands/commands.css';
  import ShortcutField from '../ShortcutField.svelte';
  import { onMount } from 'svelte';
  import { groups, labelOf, type Check } from './shared';

  type Lang = 'fr' | 'en';
  type Command = { id: string; fr_default: string[]; en_default: string[]; fr: string[]; en: string[] };

  let commands = $state<Command[]>([]);
  let drafts = $state<Record<string, string>>({});
  let errors = $state<Record<string, string>>({});
  let shortcutDrafts = $state<Record<string, string>>({});
  let shortcutLabels = $state<Record<string, string>>({});
  let v = $derived(settings.value);

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


  onMount(() => {
    void settings.load().then(refreshLabels);
    void reload();
  });
</script>

<Card id="commandes-integrees" title={t('Commandes intégrées')} help="dictionary" intro={t('Ajouter au dictionnaire, entourer la sélection, gras, italique, souligné, recherches sur internet, nouveau mail : les phrases par défaut, que tu peux changer.')}>
<div class="vc">
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
  <p class="muted">{t('Une phrase compte au moins deux mots et ne sert qu’à une commande. Majuscules, accents et ponctuation sont ignorés. « Rétablir » remet les phrases d’origine.')}</p>
</div>
</Card>
