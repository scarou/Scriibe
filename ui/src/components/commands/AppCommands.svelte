<script lang="ts">
  import Card from '../Card.svelte';
  import { api } from '../../lib/api';
  import { settings } from '../../lib/settings.svelte';
  import { i18n, t } from '../../lib/i18n.svelte';
  import '../commands/commands.css';
  import { onMount } from 'svelte';
  import { isOff, takenMessage, snippetPhraseMessage, type Check, type VoiceApp } from './shared';
  import CommandSwitch from './CommandSwitch.svelte';

  let v = $derived(settings.value);
  let drafts = $state<Record<string, string>>({});
  let errors = $state<Record<string, string>>({});
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

  onMount(() => void loadInstalled());
</script>

<Card id="commandes-applications" title={t('Mes applications')} help="commands">
<div class="vc">
    <p class="muted">{t('Choisis une application installée sur cet ordinateur et une phrase : dite sous la touche de commande, elle ouvre l’application. Seules les applications de cette liste peuvent s’ouvrir, sans rien d’autre (ni commande, ni option). Cette liste reste sur cet ordinateur : elle n’est pas exportée avec l’utilisateur.')}</p>
    {#each voiceApps as a (a.id)}
      <div class="command" class:off={isOff(a.id)}>
        <div class="head">
          <b>{a.name}</b>
          <button class="link" onclick={() => removeApp(a.id)}>{t('Supprimer')}</button>
          <CommandSwitch id={a.id} />
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

</div>
</Card>
