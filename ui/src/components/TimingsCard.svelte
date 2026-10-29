<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type HistoryItem } from '../lib/api';
  import { fmt, when } from '../lib/display.svelte';
  import { t } from '../lib/i18n.svelte';
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
  <h2>{t('Temps des dictées')} <HelpButton section="measures" /></h2>
  <p class="muted">{t('Du relâchement de la touche au collage, sur tes vraies dictées : utile pour voir si un réglage ou un modèle te fait gagner du temps.')}</p>
  {#if !summary.count}
    <p class="muted">{t('Les temps s’afficheront après ta prochaine dictée.')}</p>
  {:else}
    <div class="stats">
      <div><span class="muted">{t('Transcription')}</span><b>{sec(summary.transcription)}</b></div>
      <div><span class="muted">{t('Mise au propre')}</span><b>{sec(summary.cleanup)}</b></div>
      <div><span class="muted">{t('Total avec modèle')}</span><b>{sec(summary.totalWithModel)}</b></div>
      <div><span class="muted">{t('Total sauté')}</span><b>{sec(summary.totalSkipped)}</b></div>
    </div>
    <p class="muted">{t('Moyennes des {n} dernières dictées mesurées', { n: summary.count })}{summary.skipped ? ' · ' + t('{n} sautée(s) car déjà propres', { n: summary.skipped }) : ''}</p>
    <table>
      <thead><tr><th>{t('Date')}</th><th>{t('Mots')}</th><th>{t('Parole')}</th><th>{t('Transcription')}</th><th>{t('Mise au propre')}</th><th>{t('Total')}</th><th>{t('Décision')}</th></tr></thead>
      <tbody>
        {#each summary.rows as d (d.id)}
          <tr>
            <td>{when(d.date)}</td><td>{d.words}</td><td>{fmt(d.duration)} s</td><td>{fmt(d.timing!.transcription)} s</td>
            <td>{d.timing!.cleanup > 0 ? fmt(d.timing!.cleanup) + ' s' : '—'}</td><td>{fmt(d.timing!.total)} s</td>
            <td>{t(outcomeLabels[d.timing!.outcome] ?? d.timing!.outcome)}</td>
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
