<script lang="ts">
  import type { Comparison, DiffWord } from '../lib/api';
  import { fmt, when } from '../lib/format';

  let { comparison }: { comparison: Comparison } = $props();

  const text = (s: string) => s.replace(/\n/g, ' ⏎ ');
</script>

{#snippet words(list: DiffWord[] | null, fallback: string, side: 'old' | 'new')}
  {#if list}
    {#each list as w, i (i)}<span class:gone={w.changed && side === 'old'} class:added={w.changed && side === 'new'}>{w.text}</span>{' '}{/each}
  {:else}{text(fallback)}{/if}
{/snippet}

<div class="head">
  <div><b>Ancien</b> · {when(comparison.older.date)}<br /><span class="muted">{comparison.older.summary}</span></div>
  <div><b>Récent</b> · {when(comparison.newer.date)}<br /><span class="muted">{comparison.newer.summary}</span></div>
</div>
<div class="muted">
  {comparison.common} phrase(s) lue(s) dans les deux passages ·
  <span class="ok">{comparison.improved} améliorée(s)</span> · <span class={comparison.regressed ? 'bad' : ''}>{comparison.regressed} dégradée(s)</span>
  {#if comparison.older.average_total !== null && comparison.newer.average_total !== null}
    · temps moyen après le relâchement : {fmt(comparison.older.average_total)} s → <b>{fmt(comparison.newer.average_total)} s</b>
  {/if}
  · contrôles : {comparison.older.checks_passed}/{comparison.older.checks_total} → <b>{comparison.newer.checks_passed}/{comparison.newer.checks_total}</b>
</div>

{#each comparison.rows as row (row.text)}
  <div class="row-cmp" class:changed={row.in_both && row.changed}>
    <div class="cat"><span class="tag">{row.category}</span> <span class="muted">{row.text}</span></div>
    {#if !row.in_both}
      <div class="muted">Lue seulement dans le passage {row.older ? 'ancien' : 'récent'}.</div>
    {:else}
      <div class="cols">
        <div>{@render words(row.older_words, row.older?.pasted ?? '', 'old')}</div>
        <div>{@render words(row.newer_words, row.newer?.pasted ?? '', 'new')}</div>
      </div>
      {#each row.improvements as label (label)}<div class="ok">✔ gagné : {label}</div>{/each}
      {#each row.regressions as label (label)}<div class="bad">✘ perdu : {label}</div>{/each}
      {#if !row.changed}<div class="muted">texte identique</div>{/if}
      <div class="muted">
        total {fmt(row.older?.timing.total ?? 0)} s → {fmt(row.newer?.timing.total ?? 0)} s
      </div>
    {/if}
  </div>
{/each}

<style>
  .head { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; margin-bottom: 8px; }
  .row-cmp { border-top: 1px solid var(--line); padding: 8px 0; }
  .cols { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; margin: 4px 0; }
  .gone { background: color-mix(in srgb, var(--bad) 22%, transparent); text-decoration: line-through; border-radius: 3px; }
  .added { background: color-mix(in srgb, var(--ok) 22%, transparent); border-radius: 3px; }
  .cat { margin-bottom: 2px; }
  @media (max-width: 640px) { .head, .cols { grid-template-columns: 1fr; } }
</style>
