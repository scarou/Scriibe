<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type HistoryItem } from '../lib/api';
  import { fmt, when } from '../lib/format';
  import { outcomeLabels, summarize } from '../lib/timings';
  import HelpButton from './HelpButton.svelte';

  let items = $state<HistoryItem[]>([]);
  let summary = $derived(summarize(items));

  onMount(async () => {
    items = (await api<{ items: HistoryItem[] }>('history')).items;
  });

  const sec = (v: number | null) => (v === null ? '—' : fmt(v) + ' s');
</script>

<section class="card">
  <h2>Temps des dictées <HelpButton section="measures" /></h2>
  <p class="muted">Du relâchement de la touche au collage, sur tes vraies dictées : utile pour voir si un réglage ou un modèle te fait gagner du temps.</p>
  {#if !summary.count}
    <p class="muted">Les temps s’afficheront après ta prochaine dictée.</p>
  {:else}
    <div class="stats">
      <div><span class="muted">Transcription</span><b>{sec(summary.transcription)}</b></div>
      <div><span class="muted">Mise au propre</span><b>{sec(summary.cleanup)}</b></div>
      <div><span class="muted">Total avec modèle</span><b>{sec(summary.totalWithModel)}</b></div>
      <div><span class="muted">Total sauté</span><b>{sec(summary.totalSkipped)}</b></div>
    </div>
    <p class="muted">Moyennes des {summary.count} dernières dictées mesurées{summary.skipped ? ` · ${summary.skipped} sautée(s) car déjà propres` : ''}</p>
    <table>
      <thead><tr><th>Date</th><th>Mots</th><th>Parole</th><th>Transcription</th><th>Mise au propre</th><th>Total</th><th>Décision</th></tr></thead>
      <tbody>
        {#each summary.rows as d (d.id)}
          <tr>
            <td>{when(d.date)}</td><td>{d.words}</td><td>{fmt(d.duration)} s</td><td>{fmt(d.timing!.transcription)} s</td>
            <td>{d.timing!.cleanup > 0 ? fmt(d.timing!.cleanup) + ' s' : '—'}</td><td>{fmt(d.timing!.total)} s</td>
            <td>{outcomeLabels[d.timing!.outcome] ?? d.timing!.outcome}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</section>

<style>
  .stats { display: flex; gap: 10px; flex-wrap: wrap; margin: 10px 0 4px; }
  .stats > div { flex: 1 1 130px; padding: 10px; border-radius: 10px; background: var(--bg); display: flex; flex-direction: column; gap: 2px; }
  .stats b { font-size: 1.15rem; font-variant-numeric: tabular-nums; }
  table { width: 100%; border-collapse: collapse; font-size: 0.85rem; font-variant-numeric: tabular-nums; }
  th { text-align: left; color: var(--muted); font-weight: 600; }
  th, td { padding: 5px 6px; border-bottom: 1px solid var(--line); }
</style>
