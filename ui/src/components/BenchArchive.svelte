<script lang="ts">
  import { confirmBox } from '../lib/dialog.svelte';
  import { onMount } from 'svelte';
  import { api, type Comparison, type RunSummary } from '../lib/api';
  import { bench } from '../lib/bench.svelte';
  import { fmt, when } from '../lib/display.svelte';
  import { t } from '../lib/i18n.svelte';
  import Card from './Card.svelte';
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
    if (!(await confirmBox(`${t('Supprimer ce passage ?')}\n${run.summary}`, { confirm: t('Supprimer'), danger: true }))) return;
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

<Card id="passages-gardes" title={t('Passages gardés')} help="measures">
  <p class="muted">{t('Chaque passage est gardé avec ses réglages (version, moteurs, mode, micro) : on compare deux passages phrase par phrase, avant et après un changement.')}</p>
  {#each runs as r (r.id)}
    <div class="run">
      <div>
        <b>{when(r.date)}</b> · {t('{n} phrase(s)', { n: r.phrases })}
        {#if r.checks_total}· {t('contrôles {passed}/{total}', { passed: r.checks_passed, total: r.checks_total })}{/if}
        {#if r.average_total !== null}· {t('{s} s en moyenne', { s: fmt(r.average_total) })}{/if}
        {#if r.settings_changed}<span class="tag warn">{t('réglages changés pendant le passage')}</span>{/if}
        <br /><span class="muted">{r.summary}</span>
      </div>
      <button onclick={() => remove(r)}>{t('Supprimer')}</button>
    </div>
  {:else}
    <p class="muted">{t('Aucun passage gardé : lisez quelques phrases ci-dessus.')}</p>
  {/each}
</Card>

{#if runs.length >= 2}
  <Card id="comparer-deux-passages" title={t('Comparer deux passages')} help="measures">
    <div class="row">
      <label>{t('Ancien :')}
        <select bind:value={older} onchange={() => compare(older, newer)}>{#each runs as r (r.id)}<option value={r.id}>{when(r.date)} · {r.summary}</option>{/each}</select>
      </label>
      <label>{t('Récent :')}
        <select bind:value={newer} onchange={() => compare(older, newer)}>{#each runs as r (r.id)}<option value={r.id}>{when(r.date)} · {r.summary}</option>{/each}</select>
      </label>
    </div>
    {#if message}<div class="warn">{message}</div>{/if}
    {#if comparison}<BenchCompare {comparison} />{/if}
  </Card>
{/if}

<style>
  .run { display: flex; justify-content: space-between; gap: 10px; align-items: flex-start; padding: 8px 0; border-bottom: 1px solid var(--line); }
  label { display: flex; flex-direction: column; flex: 1 1 280px; min-width: 0; }
  select { width: 100%; }
</style>
