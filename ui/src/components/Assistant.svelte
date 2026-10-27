<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import { missing, needLabels, type Assistant } from '../lib/assistant';
  import { settings } from '../lib/settings.svelte';
  import { status } from '../lib/status.svelte';

  let { onclose, ongoto }: { onclose: () => void; ongoto: (tab: 'install' | 'settings') => void } = $props();

  let data = $state<Assistant | null>(null);
  let choice = $state('');
  let applied = $state(false);
  let s = $derived(status.value);
  let chosen = $derived(data?.chains.find((c) => c.id === choice));
  let lacks = $derived(chosen && s ? missing(chosen, s) : null);

  async function apply() {
    const out = await api('assistant/apply', { chain: choice });
    if (!out.ok) {
      alert(out.error);
      return;
    }
    applied = true;
    await settings.load();
    await status.refresh();
  }

  async function skip() {
    await api('assistant/skip', {});
    onclose();
  }

  onMount(async () => {
    data = await api<Assistant>('assistant');
    choice = data.recommended;
  });
</script>

<section class="card assistant">
  <h2>Assistant de premier lancement</h2>
  {#if !data}
    <p class="muted">Mesure de l’ordinateur…</p>
  {:else}
    <p>
      Votre ordinateur : <b>{data.machine.physical_cores} cœurs</b>, <b>{data.machine.ram_gb} Go</b> de mémoire{#if data.machine.gpus.length},
        {#each data.machine.gpus as g, i (g.name)}{i ? ' + ' : ''}{g.name}{g.vram_gb ? ` (${g.vram_gb} Go)` : ''}{/each}{:else if !data.machine.gpus_known}
        ; cartes graphiques non détectées{/if}.
      {#if data.machine.capable_gpu}Une vraie carte graphique est présente.{:else}Pas de carte graphique assez puissante : on calcule sur le processeur.{/if}
    </p>

    {#if !applied}
      <p class="muted">Voici les chaînes possibles. Les temps annoncés sont ceux qu’on a <b>mesurés</b> ; ce qui n’a pas été mesuré est dit.</p>
      {#each data.chains as c (c.id)}
        <label class="chain" class:picked={choice === c.id}>
          <input type="radio" name="chain" value={c.id} bind:group={choice} />
          <span>
            <b>{c.title}</b>{#if c.id === data.recommended}<span class="tag ok">conseillée</span>{/if}<br />
            <span class="muted">{c.detail}</span><br />
            <span class="timing">⏱ {c.timing}</span>
          </span>
        </label>
      {/each}
      <div class="row">
        <button class="primary" onclick={apply} disabled={!choice}>Appliquer ce choix</button>
        <button onclick={skip}>Garder mes réglages actuels</button>
      </div>
    {:else if chosen && lacks}
      <p class="ok">✔ Réglages appliqués : « {chosen.title} ».</p>
      {#if lacks.items.length || lacks.key}
        <p>Il reste à faire :</p>
        <ul>
          {#each lacks.items as id (id)}<li>installer <b>{needLabels[id] ?? id}</b></li>{/each}
          {#if lacks.key}<li>enregistrer votre <b>clé OpenRouter</b> (onglet Réglages)</li>{/if}
        </ul>
        <div class="row">
          {#if lacks.items.length}<button class="primary" onclick={() => ongoto('install')}>Aller à l’installation</button>{/if}
          {#if lacks.key}<button onclick={() => ongoto('settings')}>Aller aux Réglages</button>{/if}
          <button onclick={onclose}>Fermer</button>
        </div>
      {:else}
        <p>Tout est en place : maintenez Ctrl droit, parlez, relâchez.</p>
        <div class="row"><button class="primary" onclick={onclose}>Fermer</button></div>
      {/if}
    {/if}
  {/if}
</section>

<style>
  .assistant { border-color: var(--accent); }
  .chain { display: flex; gap: 10px; align-items: flex-start; padding: 10px; border: 1px solid var(--line); border-radius: 10px; margin: 8px 0; cursor: pointer; }
  .chain.picked { border-color: var(--accent); background: var(--bg); }
  .timing { font-size: 0.85rem; }
  .tag.ok { margin-left: 8px; }
</style>
