<script lang="ts">
  import HelpButton from '../components/HelpButton.svelte';
  import { onMount, tick } from 'svelte';
  import { nav } from '../lib/nav.svelte';
  import { api, type HistoryItem, type Stats } from '../lib/api';
  import { status } from '../lib/status.svelte';
  import { spoken } from '../lib/format';
  import { settings } from '../lib/settings.svelte';
  import { i18n, keyName, t } from '../lib/i18n.svelte';
  import Assistant from '../components/Assistant.svelte';
  import TypingTest from '../components/TypingTest.svelte';
  import type { TabId } from '../lib/tabs';

  let { goto }: { goto: (tab: TabId, anchor?: string) => void } = $props();
  let showAssistant = $state(false);

  let s = $state<Stats | null>(null);
  let recent = $state<HistoryItem[]>([]);

  let tallest = $derived(Math.max(1, ...(s?.last_days.map((d) => d.words) ?? [1])));
  const weekday = (iso: string) => new Date(iso).toLocaleDateString(i18n.locale, { weekday: 'narrow' });

  /** Montre l'assistant et le met en vue : la demande vient aussi des Réglages et de l'Aide. */
  async function openAssistant() {
    showAssistant = true;
    await tick();
    document.getElementById('assistant')?.scrollIntoView({ behavior: 'smooth', block: 'start' });
  }

  $effect(() => {
    if (nav.assistantRequested) {
      nav.assistantRequested = false;
      void openAssistant();
    }
  });

  onMount(() => {
    void settings.load();
    const load = () => {
      void api<Stats>('stats').then((r) => (s = r));
      void api<{ items: HistoryItem[] }>('history').then((r) => (recent = r.items.slice(0, 5)));
    };
    load();
    const timer = setInterval(load, 5000);
    return () => clearInterval(timer);
  });
</script>

<div class="row head"><h1>{t('Tableau de bord')}</h1><button onclick={openAssistant} title={t('Mesure l’ordinateur et propose les moteurs qui lui conviennent')}>{t('Relancer l’assistant de premier lancement')}</button></div>

{#if showAssistant || (settings.value && !settings.value.setupDone)}
  <div id="assistant">
    <Assistant onclose={() => { showAssistant = false; void settings.load(); }} ongoto={(tab) => goto(tab)} />
  </div>
{/if}

{#if s}
  <div class="grid">
    <div class="card big"><div class="n">{s.today_words}</div><div class="muted">{t('mots aujourd’hui')}{#if s.today_change !== null}<br />{t('{change} % par rapport à hier à la même heure', { change: (s.today_change >= 0 ? '+' : '') + s.today_change })}{/if}</div></div>
    <div class="card big"><div class="n">{s.total_words}</div><div class="muted">{t('mots dictés au total, en {n} dictée(s)', { n: s.dictations })}</div></div>
    <div class="card big"><div class="n">{s.average_wpm}</div><div class="muted">{t('mots par minute en moyenne (au clavier : {n})', { n: s.typing_wpm })}</div></div>
    <div class="card big"><div class="n">{spoken(s.time_saved)}</div><div class="muted">{t('gagnées par rapport à la frappe')}</div></div>
    <div class="card big"><div class="n">{s.streak}</div><div class="muted">{t('jour(s) d’affilée avec une dictée')}</div></div>
    <div class="card big"><div class="n">{s.corrections_this_week}</div><div class="muted">{t('corrections du dictionnaire cette semaine')}</div></div>
  </div>

  <section class="card">
    <h2>{t('14 derniers jours')}</h2>
    <div class="chart">
      {#each s.last_days as d (d.day)}
        <div class="col" title={t('{n} mots', { n: d.words })}>
          <div class="bar-v" style="height: {Math.round((d.words / tallest) * 100)}%"></div>
          <div class="muted day">{weekday(d.day)}</div>
        </div>
      {/each}
    </div>
  </section>

  <section class="card">
    <div class="row head"><h2>{t('Dernières dictées')}</h2><button onclick={() => goto('history')} disabled={!recent.length}>{t('Tout voir')}</button></div>
    {#each recent as d (d.id)}
      <div class="recent">
        <span class="muted time">{new Date(d.date).toLocaleTimeString(i18n.locale, { hour: '2-digit', minute: '2-digit' })}</span>
        <div><span class="muted">{t('{app} · {n} mots', { app: d.app, n: d.words })}</span><div class="text">{d.has_text ? d.text : t('Texte non gardé (réglage de confidentialité)')}</div></div>
      </div>
    {:else}
      <p class="muted">{t('Maintiens {key} dans n’importe quelle appli et parle : ta première dictée apparaîtra ici.', { key: keyName(status.value?.dictation_label ?? 'Ctrl droit') })}</p>
    {/each}
  </section>

  <section class="card">
    <h2>{t('Où tu dictes')}</h2>
    {#each s.top_apps as a (a.name)}
      <div class="app"><span>{a.name}</span><div class="share"><div style="width: {Math.round(a.share * 100)}%"></div></div><span class="muted">{t('{n} mots · {share} %', { n: a.words, share: Math.round(a.share * 100) })}</span></div>
    {:else}
      <p class="muted">{t('Pas encore de dictée.')}</p>
    {/each}
  </section>

  <section class="card">
    <h2>{t('Ma vitesse au clavier')} <HelpButton section="measures" /></h2>
    <p class="muted">{t('Sert au calcul du temps gagné. Recopiez le texte : seuls les essais complets, à plus de 90 % de lettres justes, comptent.')}</p>
    <TypingTest />
  </section>
{:else}
  <p class="muted">{t('Chargement…')}</p>
{/if}

<style>
  .head { justify-content: space-between; margin: 0 0 6px; }
  .head h2 { margin: 0; }
  .recent { display: flex; gap: 14px; padding: 8px 0; border-top: 1px solid var(--line); }
  .recent:first-of-type { border-top: 0; }
  .time { width: 46px; flex: none; font-variant-numeric: tabular-nums; }
  .text { overflow-wrap: anywhere; display: -webkit-box; -webkit-line-clamp: 3; line-clamp: 3; -webkit-box-orient: vertical; overflow: hidden; }
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
