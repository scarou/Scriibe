<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from './lib/api';
  import { status } from './lib/status.svelte';
  import Settings from './tabs/Settings.svelte';
  import Install from './tabs/Install.svelte';
  import Bench from './tabs/Bench.svelte';
  import Update from './tabs/Update.svelte';
  import Journal from './tabs/Journal.svelte';

  const tabs = [
    { id: 'settings', label: 'Réglages', icon: '⚙' },
    { id: 'install', label: 'Installation', icon: '⬇' },
    { id: 'bench', label: 'Banc d’essai', icon: '🎤' },
    { id: 'update', label: 'Mise à jour', icon: '⟳' },
    { id: 'journal', label: 'Journal et rapport', icon: '☰' },
  ] as const;
  type TabId = (typeof tabs)[number]['id'];

  const saved = (() => {
    try {
      return sessionStorage.getItem('tab');
    } catch {
      return null;
    }
  })();
  let tab = $state<TabId>(tabs.some((t) => t.id === saved) ? (saved as TabId) : 'settings');
  let stopped = $state(false);

  function choose(id: TabId) {
    tab = id;
    try {
      sessionStorage.setItem('tab', id);
    } catch {}
  }

  async function quit() {
    if (!confirm('Quitter Scriibe ?')) return;
    await api('quit', {});
    stopped = true;
  }

  onMount(() => status.start());
</script>

{#if stopped}
  <main class="stopped"><p>Scriibe est arrêté. Vous pouvez fermer cette page.</p></main>
{:else}
  <div class="shell">
    <nav>
      <div class="brand">Scriibe</div>
      {#each tabs as t (t.id)}
        <button class="nav" class:active={tab === t.id} onclick={() => choose(t.id)}><span class="icon">{t.icon}</span>{t.label}</button>
      {/each}
      <div class="spacer"></div>
      <div class="muted version">{status.value ? 'version ' + status.value.version : ''}</div>
      <button class="nav" onclick={quit}>Quitter Scriibe</button>
    </nav>
    <main>
      {#if status.failed}
        <div class="card bad">Scriibe ne répond plus. Relancez-le depuis l’icône de la zone de notification.</div>
      {/if}
      {#if tab === 'settings'}<Settings />
      {:else if tab === 'install'}<Install />
      {:else if tab === 'bench'}<Bench />
      {:else if tab === 'update'}<Update />
      {:else}<Journal />{/if}
    </main>
  </div>
{/if}

<style>
  .shell { display: flex; min-height: 100vh; }
  nav { width: 210px; flex: none; background: var(--side); border-right: 1px solid var(--line); padding: 14px 10px; display: flex; flex-direction: column; gap: 2px; position: sticky; top: 0; height: 100vh; }
  .brand { font-weight: 700; font-size: 1.2rem; padding: 4px 10px 12px; }
  .nav { text-align: left; border: 0; background: transparent; padding: 8px 10px; border-radius: 8px; }
  .nav:hover { background: var(--line); }
  .nav.active { background: var(--accent); color: var(--accent-ink); }
  .icon { display: inline-block; width: 1.6em; }
  .spacer { flex: 1; }
  .version { padding: 4px 10px; font-size: .78rem; word-break: break-all; }
  main { flex: 1; min-width: 0; padding: 20px; max-width: 900px; }
  main.stopped { padding: 40px; }
  @media (max-width: 640px) {
    .shell { flex-direction: column; }
    nav { width: auto; height: auto; position: static; flex-direction: row; flex-wrap: wrap; }
    .spacer, .version, .brand { display: none; }
  }
</style>
