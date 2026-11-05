<script lang="ts">
  import { confirmBox } from './lib/dialog.svelte';
  import { onMount } from 'svelte';
  import { api } from './lib/api';
  import { status } from './lib/status.svelte';
  import { createKeyRelay } from './lib/keyrelay';
  import { nav } from './lib/nav.svelte';
  import { i18n, keyName, t } from './lib/i18n.svelte';
  import { settings } from './lib/settings.svelte';
  import { tabs, type TabId } from './lib/tabs';
  import SettingsPage from './tabs/SettingsPage.svelte';
  import Dictionary from './tabs/Dictionary.svelte';
  import History from './tabs/History.svelte';
  import Dashboard from './tabs/Dashboard.svelte';
  import Help from './tabs/Help.svelte';
  import HelpEn from './tabs/HelpEn.svelte';
  import { logotype } from './lib/logotype';
  import { applyTheme, isDark } from './lib/theme';
  import Dialog from './components/Dialog.svelte';

  let stopped = $state(false);
  let logo = $derived(logotype(status.value?.os));
  let systemDark = $state(false);
  let dark = $derived(isDark(settings.value?.theme, systemDark));

  function choose(id: TabId, anchor?: string) {
    void nav.go(id, anchor);
  }

  async function quit() {
    if (!(await confirmBox(t('Quitter Scriibe ?'), { confirm: t('Quitter') }))) return;
    await api('quit', {});
    stopped = true;
  }

  // Le crochet clavier du système ne reçoit rien quand cette fenêtre a le focus : la page relaie ses touches (dicter dans le Dictionnaire…).
  const relay = createKeyRelay((body) => api('keys/page', body));
  const onKeyDown = (e: KeyboardEvent) => relay.down(e);
  const onKeyUp = (e: KeyboardEvent) => relay.up(e);
  const onBlur = () => relay.blur();

  $effect(() => {
    document.documentElement.lang = i18n.lang;
  });

  $effect(() => applyTheme(document.documentElement, settings.value?.theme));

  onMount(() => {
    const scheme = window.matchMedia('(prefers-color-scheme: dark)');
    const followSystem = () => (systemDark = scheme.matches);
    followSystem();
    scheme.addEventListener('change', followSystem);
    status.start();
    void settings.load();
    window.addEventListener('keydown', onKeyDown);
    window.addEventListener('keyup', onKeyUp);
    window.addEventListener('blur', onBlur);
    return () => {
      window.removeEventListener('keydown', onKeyDown);
      window.removeEventListener('keyup', onKeyUp);
      window.removeEventListener('blur', onBlur);
      scheme.removeEventListener('change', followSystem);
    };
  });
</script>

<Dialog />

{#if stopped}
  <main class="stopped"><p>{t('Scriibe est arrêté. Vous pouvez fermer cette page.')}</p></main>
{:else}
  <div class="shell">
    <nav>
      <div class="brand"><img src={dark ? logo.dark : logo.light} alt="Scriibe" /></div>
      {#each tabs as tab (tab.id)}
        <button class="nav" class:active={nav.tab === tab.id} onclick={() => choose(tab.id)}><span class="icon">{tab.icon}</span>{t(tab.label)}</button>
      {/each}
      <div class="spacer"></div>
      {#if status.value}
        {@const a = status.activity}
        <div class="state" title={status.value.keys_error ?? ''}>
          <span class="dot" class:rec={a?.recording} class:busy={!a?.recording && a?.busy} class:off={!!status.value.keys_error}></span>
          <span>{a?.recording ? t('J’écoute…') : a?.busy ? t('Mise au propre…') : status.value.keys_error ? t('Touche indisponible') : t('Prêt · {key}', { key: keyName(status.value.dictation_label) })}</span>
        </div>
      {/if}
      {#if status.value?.update}
        <button class="nav news" onclick={() => choose('update')} title={status.value.update.title}>↑ {t('Nouvelle version disponible')}</button>
      {/if}
      <div class="muted version">{status.value ? t('version {version}', { version: status.value.version }) : ''}</div>
      <button class="nav" onclick={quit}>{t('Quitter Scriibe')}</button>
    </nav>
    <main>
      {#if status.failed}
        <div class="card bad">{status.value?.os === 'macos' ? t('Scriibe ne répond plus. Relancez-le depuis l’icône de la barre des menus.') : status.value?.os === 'linux' ? t('Scriibe ne répond plus. Relancez-le depuis la liste des applications.') : t('Scriibe ne répond plus. Relancez-le depuis l’icône de la zone de notification.')}</div>
      {/if}
      {#if nav.tab === 'dashboard'}<Dashboard goto={choose} />
      {:else if nav.tab === 'history'}<History />
      {:else if nav.tab === 'dictionary'}<Dictionary />
      {:else if nav.tab === 'help'}{#if i18n.lang === 'en'}<HelpEn />{:else}<Help />{/if}
      {:else}<SettingsPage />{/if}
    </main>
  </div>
{/if}

<style>
  .shell { display: flex; min-height: 100vh; }
  nav { width: 210px; flex: none; background: var(--side); border-right: 1px solid var(--line); padding: 14px 10px; display: flex; flex-direction: column; gap: 2px; position: sticky; top: 0; height: 100vh; }
  .brand { display: block; padding: 4px 10px 12px; }
  .brand img { display: block; width: 100%; height: auto; }
  .nav { text-align: left; border: 0; background: transparent; padding: 8px 10px; border-radius: 8px; }
  .nav:hover { background: var(--line); }
  .nav.active { background: var(--accent-soft); color: var(--accent); font-weight: 600; }
  .icon { display: inline-block; width: 1.6em; }
  .spacer { flex: 1; }
  .state { display: flex; align-items: center; gap: 8px; padding: 4px 10px; font-size: .82rem; color: var(--muted); }
  .dot { width: 8px; height: 8px; flex: none; border-radius: 50%; background: var(--ok); opacity: .55; }
  .dot.rec { background: var(--rec); opacity: 1; }
  .dot.busy { background: var(--warn); opacity: 1; }
  .dot.off { background: var(--bad); opacity: 1; }
  .news { color: var(--accent); font-size: .85rem; font-weight: 600; }
  .version { padding: 4px 10px; font-size: .78rem; word-break: break-all; }
  main { flex: 1; min-width: 0; padding: 20px; max-width: 900px; }
  main.stopped { padding: 40px; }
  @media (max-width: 640px) {
    .shell { flex-direction: column; }
    nav { width: auto; height: auto; position: static; flex-direction: row; flex-wrap: wrap; }
    .spacer, .version, .state, .brand { display: none; }
  }
</style>
