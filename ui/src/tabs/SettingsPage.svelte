<script lang="ts">
  import { nav } from '../lib/nav.svelte';
  import { t } from '../lib/i18n.svelte';
  import { status } from '../lib/status.svelte';
  import { settingsTabs } from '../lib/tabs';
  import Bench from './Bench.svelte';
  import Install from './Install.svelte';
  import Journal from './Journal.svelte';
  import Modes from './Modes.svelte';
  import Settings from './Settings.svelte';
  import Update from './Update.svelte';

  let current = $derived(settingsTabs.find((x) => x.id === nav.settingsTab) ?? settingsTabs[0]);
</script>

<h1>{t('Réglages')}</h1>
<p class="muted lead">{t('Comment Scriibe écoute, écrit et colle')}</p>

<div class="pills" role="tablist">
  {#each settingsTabs as tab (tab.id)}
    <button class="pill" class:active={nav.settingsTab === tab.id} role="tab" aria-selected={nav.settingsTab === tab.id} onclick={() => nav.chooseSettingsTab(tab.id)}>
      {t(tab.label)}{#if tab.id === 'update' && status.value?.update}<span class="new" title={t('Nouvelle version disponible')}>●</span>{/if}
    </button>
  {/each}
</div>
<p class="muted lead">{t(current.description)}</p>

{#if nav.settingsTab === 'dictation'}<Settings section="dictation" />
{:else if nav.settingsTab === 'engines'}<Settings section="engines" /><Install />
{:else if nav.settingsTab === 'appearance'}<Settings section="appearance" />
{:else if nav.settingsTab === 'modes'}<Modes />
{:else if nav.settingsTab === 'measures'}<Bench /><Journal />
{:else}<Update />{/if}

<style>
  .lead { margin: 0 0 12px; }
  .pills { display: flex; flex-wrap: wrap; gap: 8px; margin: 0 0 8px; }
  .pill { border-radius: 999px; padding: 6px 16px; background: var(--card); }
  .new { color: var(--warn); margin-left: 6px; font-size: .7rem; }
  .pill.active { background: var(--accent); border-color: var(--accent); color: var(--accent-ink); font-weight: 600; }
</style>
