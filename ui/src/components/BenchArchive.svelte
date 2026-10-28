<script lang="ts">
  import { confirmBox } from '../lib/dialog.svelte';
  import { onMount } from 'svelte';
  import { api, type Comparison, type RunSummary } from '../lib/api';
  import { bench } from '../lib/bench.svelte';
  import { fmt, when } from '../lib/format';
  import BenchCompare from './BenchCompare.svelte';

  let runs = $state<RunSummary[]>([]);
  let older = $state('');
  let newer = $state('');
  let comparison = $state<Comparison | null>(null);
  let message = $state('');

  async function load() {
    runs = (await api<{ runs: RunSummary[] }>('bench/runs')).runs;
    if (runs.length < 2) {
      comparison = null;
      older = newer = '';
      return;
    }
    // Par défaut : le plus récent et son partenaire (le plus récent fait avec d'autres réglages).
    if (!runs.some((r) => r.id === older) || !runs.some((r) => r.id === newer)) await compare('', '');
    else await compare(older, newer);
  }

  async function compare(o: string, n: string) {
    const out = await api<Comparison & { ok?: boolean; error?: string }>('bench/compare', { older: o, newer: n });
    if (out.error) {
      message = out.error;
      comparison = null;
      return;
    }
    message = '';
    comparison = out;
    older = out.older.id;
    newer = out.newer.id;
  }

  async function remove(run: RunSummary) {
    if (!(await confirmBox(`Supprimer ce passage ?\n${run.summary}`, { confirm: 'Supprimer', danger: true }))) return;
    await api('bench/delete', { id: run.id });
    await load();
  }

  onMount(() => void load());
  // Un passage de cette session qui s'enregistre ou disparaît : la liste se recharge.
  $effect(() => {
    bench.saved;
    void load();
  });
</script>

<section class="card">
  <h2>Passages gardés</h2>
  <p class="muted">Chaque passage est gardé avec ses réglages (version, moteurs, mode, micro) : on compare deux passages phrase par phrase, avant et après un changement.</p>
  {#each runs as r (r.id)}
    <div class="run">
      <div>
        <b>{when(r.date)}</b> · {r.phrases} phrase(s)
        {#if r.checks_total}· contrôles {r.checks_passed}/{r.checks_total}{/if}
        {#if r.average_total !== null}· {fmt(r.average_total)} s en moyenne{/if}
        {#if r.settings_changed}<span class="tag warn">réglages changés pendant le passage</span>{/if}
        <br /><span class="muted">{r.summary}</span>
      </div>
      <button onclick={() => remove(r)}>Supprimer</button>
    </div>
  {:else}
    <p class="muted">Aucun passage gardé : lisez quelques phrases ci-dessus.</p>
  {/each}
</section>

{#if runs.length >= 2}
  <section class="card">
    <h2>Comparer deux passages</h2>
    <div class="row">
      <label>Ancien :
        <select bind:value={older} onchange={() => compare(older, newer)}>{#each runs as r (r.id)}<option value={r.id}>{when(r.date)} · {r.summary}</option>{/each}</select>
      </label>
      <label>Récent :
        <select bind:value={newer} onchange={() => compare(older, newer)}>{#each runs as r (r.id)}<option value={r.id}>{when(r.date)} · {r.summary}</option>{/each}</select>
      </label>
    </div>
    {#if message}<div class="warn">{message}</div>{/if}
    {#if comparison}<BenchCompare {comparison} />{/if}
  </section>
{/if}

<style>
  .run { display: flex; justify-content: space-between; gap: 10px; align-items: flex-start; padding: 8px 0; border-bottom: 1px solid var(--line); }
  label { display: flex; flex-direction: column; flex: 1 1 280px; min-width: 0; }
  select { width: 100%; }
</style>
