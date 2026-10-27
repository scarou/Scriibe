<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type Stats } from '../lib/api';
  import { spoken } from '../lib/format';
  import { settings } from '../lib/settings.svelte';
  import Assistant from '../components/Assistant.svelte';
  import TypingTest from '../components/TypingTest.svelte';
  import type { TabId } from '../lib/tabs';

  let { goto }: { goto: (tab: TabId) => void } = $props();
  let showAssistant = $state(false);

  let s = $state<Stats | null>(null);

  let tallest = $derived(Math.max(1, ...(s?.last_days.map((d) => d.words) ?? [1])));
  const weekday = (iso: string) => new Date(iso).toLocaleDateString('fr-FR', { weekday: 'narrow' });

  onMount(() => {
    void settings.load();
    const load = () => void api<Stats>('stats').then((r) => (s = r));
    load();
    const timer = setInterval(load, 5000);
    return () => clearInterval(timer);
  });
</script>

<h1>Tableau de bord</h1>

{#if showAssistant || (settings.value && !settings.value.setupDone)}
  <Assistant onclose={() => { showAssistant = false; void settings.load(); }} ongoto={(tab) => goto(tab)} />
{/if}

{#if s}
  <div class="grid">
    <div class="card big"><div class="n">{s.today_words}</div><div class="muted">mots aujourd’hui{#if s.today_change !== null}<br />{s.today_change >= 0 ? '+' : ''}{s.today_change} % par rapport à hier à la même heure{/if}</div></div>
    <div class="card big"><div class="n">{s.total_words}</div><div class="muted">mots dictés au total, en {s.dictations} dictée(s)</div></div>
    <div class="card big"><div class="n">{s.average_wpm}</div><div class="muted">mots par minute en moyenne (au clavier : {s.typing_wpm})</div></div>
    <div class="card big"><div class="n">{spoken(s.time_saved)}</div><div class="muted">gagnées par rapport à la frappe</div></div>
    <div class="card big"><div class="n">{s.streak}</div><div class="muted">jour(s) d’affilée avec une dictée</div></div>
    <div class="card big"><div class="n">{s.corrections_this_week}</div><div class="muted">corrections du dictionnaire cette semaine</div></div>
  </div>

  <section class="card">
    <h2>14 derniers jours</h2>
    <div class="chart">
      {#each s.last_days as d (d.day)}
        <div class="col" title="{d.words} mots">
          <div class="bar-v" style="height: {Math.round((d.words / tallest) * 100)}%"></div>
          <div class="muted day">{weekday(d.day)}</div>
        </div>
      {/each}
    </div>
  </section>

  <section class="card">
    <h2>Où tu dictes</h2>
    {#each s.top_apps as a (a.name)}
      <div class="app"><span>{a.name}</span><div class="share"><div style="width: {Math.round(a.share * 100)}%"></div></div><span class="muted">{a.words} mots · {Math.round(a.share * 100)} %</span></div>
    {:else}
      <p class="muted">Pas encore de dictée.</p>
    {/each}
  </section>
  <div class="row"><button onclick={() => (showAssistant = true)}>Relancer l’assistant de premier lancement</button></div>

  <section class="card">
    <h2>Ma vitesse au clavier</h2>
    <p class="muted">Sert au calcul du temps gagné. Recopiez le texte : seuls les essais complets, à plus de 90 % de lettres justes, comptent.</p>
    <TypingTest />
  </section>
{:else}
  <p class="muted">Chargement…</p>
{/if}

<style>
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(230px, 1fr)); gap: 12px; margin-bottom: 14px; }
  .big { margin: 0; }
  .n { font-size: 2rem; font-weight: 700; line-height: 1.1; }
  .chart { display: flex; gap: 6px; align-items: flex-end; height: 140px; }
  .col { flex: 1; height: 100%; display: flex; flex-direction: column; justify-content: flex-end; align-items: stretch; }
  .bar-v { background: var(--accent); border-radius: 4px 4px 0 0; min-height: 2px; }
  .day { text-align: center; font-size: .75rem; }
  .app { display: grid; grid-template-columns: 9em 1fr auto; gap: 10px; align-items: center; margin: 6px 0; }
  .share { height: 8px; background: var(--line); border-radius: 4px; overflow: hidden; }
  .share > div { height: 100%; background: var(--accent); }
</style>
