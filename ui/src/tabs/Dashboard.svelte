<script lang="ts">
  import Card from '../components/Card.svelte';
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
  const shortDay = (iso: string) => new Date(iso).toLocaleDateString(i18n.locale, { day: 'numeric', month: 'short' });
  /** L'onde du bandeau : la hauteur relative de chaque barre. */
  const wave = [0.18, 0.3, 0.5, 0.36, 0.7, 0.95, 0.62, 0.82, 0.45, 0.6, 0.32, 0.5, 0.24, 0.36, 0.16];

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

<div class="banner">
  <div class="welcome">
    <h1>{status.value?.first_name ? t('Salut {name}', { name: status.value.first_name }) : t('Salut')}</h1>
    <p>{t('Maintiens la touche, parle, relâche : ton texte s’écrit tout seul là où tu es.')}</p>
    <div class="hint">
      <span class="key">{keyName(status.value?.dictation_label ?? 'Ctrl droit')}</span>
      <span>{t('maintenir pour dicter')}</span>
    </div>
  </div>
  <div class="wave" aria-hidden="true">
    {#each wave as h, i (i)}<span style="height: {Math.round(h * 100)}%; opacity: {0.35 + 0.6 * h}"></span>{/each}
  </div>
</div>

{#if showAssistant || (settings.value && !settings.value.setupDone)}
  <div id="assistant">
    <Assistant onclose={() => { showAssistant = false; void settings.load(); }} ongoto={(tab) => goto(tab)} />
  </div>
{/if}

{#if s}
  <div class="grid">
    <div class="card tile"><div class="muted">{t('Mots dictés')}</div><div class="n">{s.total_words}</div><div class="muted">{t('{n} dictée(s)', { n: s.dictations })}</div></div>
    <div class="card tile"><div class="muted">{t('Aujourd’hui')}</div><div class="n">{s.today_words}</div>
      <div class="muted" class:bad={s.today_change !== null && s.today_change < 0}>{s.today_change !== null ? t('{change} % par rapport à hier à la même heure', { change: (s.today_change >= 0 ? '+' : '') + s.today_change }) : t('rien hier à cette heure')}</div></div>
    <div class="card tile"><div class="muted">{t('Vitesse')}</div><div class="n">{s.average_wpm} <small>{t('mots/min')}</small></div><div class="muted">{t('en moyenne (au clavier : {n})', { n: s.typing_wpm })}</div></div>
    <div class="card tile"><div class="muted">{t('Temps gagné')}</div><div class="n">{spoken(s.time_saved)}</div><div class="muted">{t('par rapport à la frappe')}</div></div>
  </div>

  <div class="two">
    <Card id="dashboard-mots-par-jour" title={t('Mots dictés par jour')}>
      {#if s.streak > 0}<div class="streak">🔥 {t('{n} jour(s) d’affilée', { n: s.streak })}</div>{/if}
      <p class="muted">{t('14 derniers jours. Les chiffres restent justes même si l’historique est limité dans le temps.')}</p>
      <div class="chart">
        {#each s.last_days as d, i (d.day)}
          <div class="col" title={t('{n} mots', { n: d.words })}>
            <div class="bar-v" style="height: {Math.round((d.words / tallest) * 100)}%"></div>
            <div class="muted day">{i % 2 === 0 ? shortDay(d.day) : ''}</div>
          </div>
        {/each}
      </div>
    </Card>

    <Card id="dashboard-ou-je-dicte" title={t('Où je dicte')}>
      <p class="muted">{t('Part des mots par appli')}</p>
      {#each s.top_apps as a (a.name)}
        <div class="app"><div class="line"><span>{a.name}</span><span class="pct">{Math.round(a.share * 100)} %</span></div><div class="share"><div style="width: {Math.round(a.share * 100)}%"></div></div></div>
      {:else}
        <p class="muted">{t('Pas encore de dictée.')}</p>
      {/each}
    </Card>
  </div>

  <Card id="dashboard-dernieres-dictees" title={t('Dernières dictées')}>
    <div class="row head"><span class="muted">{t('Les cinq dernières, sur cet ordinateur.')}</span><button onclick={() => goto('history')} disabled={!recent.length}>{t('Tout voir')}</button></div>
    {#each recent as d (d.id)}
      <div class="recent">
        <span class="muted time">{new Date(d.date).toLocaleTimeString(i18n.locale, { hour: '2-digit', minute: '2-digit' })}</span>
        <div><span class="muted">{t('{app} · {n} mots', { app: d.app, n: d.words })}</span><div class="text">{d.has_text ? d.text : t('Texte non gardé (réglage de confidentialité)')}</div></div>
      </div>
    {:else}
      <p class="muted">{t('Maintiens {key} dans n’importe quelle appli et parle : ta première dictée apparaîtra ici.', { key: keyName(status.value?.dictation_label ?? 'Ctrl droit') })}</p>
    {/each}
  </Card>

  <Card id="dashboard-vitesse-clavier" folded title={t('Ma vitesse au clavier')} help="measures" intro={t('Sert au calcul du temps gagné. Recopiez le texte : seuls les essais complets, à plus de 90 % de lettres justes, comptent.')}>
    <TypingTest />
  </Card>
{:else}
  <p class="muted">{t('Chargement…')}</p>
{/if}

<style>
  .banner { display: flex; align-items: center; justify-content: space-between; gap: 20px; padding: 24px 28px; margin: 0 0 14px; border-radius: 20px; color: #fff; background: linear-gradient(135deg, var(--brand-deep), var(--brand) 55%, var(--brand-bright)); }
  .banner h1 { margin: 0 0 6px; font-size: 1.9rem; }
  .banner p { margin: 0 0 12px; opacity: .92; }
  .hint { display: flex; align-items: center; flex-wrap: wrap; gap: 8px; font-size: .9rem; }
  .key { background: #fff; color: var(--brand-deep); font-weight: 700; padding: 3px 10px; border-radius: 7px; }
  .wave { display: flex; align-items: center; gap: 6px; height: 90px; flex: none; }
  .wave span { display: block; width: 6px; border-radius: 3px; background: #fff; }
  .head { justify-content: space-between; margin: 0 0 6px; }
  .recent { display: flex; gap: 14px; padding: 8px 0; border-top: 1px solid var(--line); }
  .recent:first-of-type { border-top: 0; }
  .time { width: 46px; flex: none; font-variant-numeric: tabular-nums; }
  .text { overflow-wrap: anywhere; display: -webkit-box; -webkit-line-clamp: 3; line-clamp: 3; -webkit-box-orient: vertical; overflow: hidden; }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); gap: 12px; margin-bottom: 14px; }
  .tile { margin: 0; padding: 14px 16px; }
  .n { font-size: 1.8rem; font-weight: 700; line-height: 1.2; }
  .n small { font-size: .8rem; font-weight: 400; color: var(--muted); }
  .two { display: grid; grid-template-columns: 1.4fr 1fr; gap: 14px; align-items: start; }
  .two :global(.card) { margin: 0 0 14px; }
  @media (max-width: 760px) { .two { grid-template-columns: 1fr; } .wave { display: none; } }
  .streak { float: right; background: color-mix(in srgb, var(--warn) 15%, transparent); color: var(--warn); border-radius: 999px; padding: 2px 12px; font-size: .85rem; font-weight: 600; }
  .chart { display: flex; gap: 6px; align-items: flex-end; height: 140px; }
  .col { flex: 1; height: 100%; display: flex; flex-direction: column; justify-content: flex-end; align-items: stretch; }
  .bar-v { background: var(--brand); border-radius: 4px 4px 0 0; min-height: 2px; }
  .day { text-align: center; font-size: .7rem; height: 1.2em; white-space: nowrap; }
  .app { margin: 10px 0; }
  .line { display: flex; justify-content: space-between; font-weight: 600; font-size: .9rem; margin-bottom: 3px; }
  .pct { color: var(--accent); }
  .share { height: 6px; background: var(--line); border-radius: 3px; overflow: hidden; }
  .share > div { height: 100%; background: var(--brand); }
</style>
