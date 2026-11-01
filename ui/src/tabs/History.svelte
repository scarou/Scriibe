<script lang="ts">
  import { confirmBox } from '../lib/dialog.svelte';
  import HelpButton from '../components/HelpButton.svelte';
  import { onMount } from 'svelte';
  import { api, type HistoryItem } from '../lib/api';
  import { fmt, when } from '../lib/display.svelte';
  import { t } from '../lib/i18n.svelte';
  import { outcomeLabels } from '../lib/timings';
  import { settings } from '../lib/settings.svelte';
  import { matchesHistory, type Period } from '../lib/historyfilter';

  let items = $state<HistoryItem[]>([]);
  let apps = $state<string[]>([]);
  let app = $state('');
  let search = $state('');
  let period = $state<Period>('all');
  let from = $state('');
  let to = $state('');
  let copied = $state('');

  let keepText = $state(true);
  let keepDays = $state<number | null>(null);
  let preview = $state('');

  let shown = $derived(items.filter((d) => matchesHistory(d, { period, from, to, app, search }, new Date())));
  let filtered = $derived(period !== 'all' || !!app || !!search.trim());

  function resetFilters() {
    period = 'all';
    from = '';
    to = '';
    app = '';
    search = '';
  }

  async function load() {
    const out = await api<{ items: HistoryItem[]; apps: string[] }>('history');
    items = out.items;
    apps = out.apps;
  }

  async function copy(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      copied = t('Copié ✔');
    } catch {
      copied = t('Copie refusée');
    }
    setTimeout(() => (copied = ''), 2000);
  }

  async function remove(id: string) {
    await api('history/delete', { id });
    await load();
  }

  async function clearAll() {
    if (!(await confirmBox(t('Effacer tout l’historique, y compris les chiffres du tableau de bord ? C’est définitif.'), { confirm: t('Tout effacer'), danger: true }))) return;
    await api('history/clear', { confirmed: true });
    await load();
  }

  async function previewChange() {
    const out = await api<{ purged: number; erased: number }>('history/preview', { keep_text: keepText, keep_days: keepDays });
    const parts = [];
    if (out.purged) parts.push(t('{n} dictée(s) plus anciennes seraient supprimées (leurs totaux restent)', { n: out.purged }));
    if (out.erased) parts.push(t('le texte de {n} dictée(s) serait effacé', { n: out.erased }));
    preview = parts.join(' ; ') || t('Rien ne serait supprimé.');
  }

  async function applyChange() {
    await previewChange();
    if (!(await confirmBox(`${preview}\n\n${t('Appliquer ces réglages ? C’est définitif.')}`, { confirm: t('Appliquer'), danger: true }))) return;
    await api('history/apply', { keep_text: keepText, keep_days: keepDays, confirmed: true });
    await settings.load();
    await load();
  }

  onMount(async () => {
    await settings.load();
    keepText = settings.value?.keepDictationText ?? true;
    keepDays = (settings.value?.historyKeepDays as number | null | undefined) ?? null;
    await load();
  });
</script>

<h1>{t('Historique')}</h1>
<p class="muted">{t('Gardé sur cet ordinateur seulement.')} {copied}</p>

<section class="card">
  <p class="muted">{t('Tout ce que vous dictez s’enregistre ici, sur cet ordinateur. Filtrez par mots clés, par date ou par appli ; les filtres se cumulent.')}</p>
  <div class="row">
    <input type="text" class="grow" placeholder={t('Chercher dans les textes')} bind:value={search} />
    <select bind:value={app}>
      <option value="">{t('Toutes les applis')}</option>
      {#each apps as a (a)}<option value={a}>{a}</option>{/each}
    </select>
  </div>
  <div class="row">
    <label>{t('Période :')}
      <select bind:value={period}>
        <option value="all">{t('Tout')}</option>
        <option value="today">{t('Aujourd’hui')}</option>
        <option value="7">{t('7 derniers jours')}</option>
        <option value="30">{t('30 derniers jours')}</option>
        <option value="custom">{t('Dates choisies')}</option>
      </select>
    </label>
    {#if period === 'custom'}
      <label>{t('du')} <input type="date" bind:value={from} max={to || undefined} /></label>
      <label>{t('au')} <input type="date" bind:value={to} min={from || undefined} /></label>
    {/if}
    {#if filtered}
      <span class="muted">{t('{n} dictée(s) sur {total}', { n: shown.length, total: items.length })}</span>
      <button onclick={resetFilters}>{t('Tout afficher')}</button>
    {/if}
  </div>
  {#each shown as d (d.id)}
    <div class="item">
      <div class="head">
        <span class="muted">{t('{date} · {app} · {n} mots · {s} s', { date: when(d.date), app: d.app, n: d.words, s: fmt(d.duration) })}{d.translation ? t(' · traduit en {language}', { language: d.translation }) : ''}</span>
        <span>
          {#if d.has_text}<button onclick={() => copy(d.text)}>{t('Copier')}</button>{/if}
          <button onclick={() => remove(d.id)}>{t('Supprimer')}</button>
        </span>
      </div>
      {#if d.has_text}
        <div class="text">{d.text}</div>
        {#if d.raw && d.raw !== d.text}<details><summary class="muted">{t('Ce que Whisper a entendu')}</summary><div class="text muted">{d.raw}</div></details>{/if}
      {:else}
        <div class="muted">{t('Texte non gardé (réglage « Garder le texte des dictées »).')}</div>
      {/if}
      {#if d.mode || d.timing}
        <div class="muted">
          {#if d.mode}{t('Mode : {mode}', { mode: t(d.mode) })}{#if d.timing}{' — ' + t(outcomeLabels[d.timing.outcome] ?? d.timing.outcome)}{/if}{/if}
          {#if d.timing}{d.mode ? ' · ' : ''}{t('transcription {a} s · mise au propre {b} s · total {c} s', { a: fmt(d.timing.transcription), b: fmt(d.timing.cleanup), c: fmt(d.timing.total) })}{/if}
        </div>
      {/if}
    </div>
  {:else}
    <p class="muted">{items.length ? t('Aucune dictée ne correspond.') : t('Aucune dictée pour l’instant.')}</p>
  {/each}
</section>

<section class="card" id="conservation">
  <h2>{t('Confidentialité de l’historique')} <HelpButton section="privacy" /></h2>
  <p class="muted">{t('Pour que ce que vous dictez ne reste pas sur le disque : par exemple un code ou une adresse dictés dans un message. Vous pouvez ne garder que les statistiques (nombre de mots, durée, appli), ou n’avoir que les derniers jours. Le tableau de bord reste juste dans les deux cas.')}</p>
  <div class="row"><label><input type="checkbox" bind:checked={keepText} onchange={previewChange} /> {t('Garder le texte des dictées (sinon seule la fiche chiffrée reste)')}</label></div>
  <div class="row">
    <label>{t('Garder l’historique :')}
      <select value={keepDays === null ? '' : String(keepDays)} onchange={(e) => { keepDays = e.currentTarget.value === '' ? null : Number(e.currentTarget.value); void previewChange(); }}>
        <option value="">{t('sans limite')}</option>
        <option value="2">{t('{n} jours', { n: 2 })}</option><option value="7">{t('{n} jours', { n: 7 })}</option><option value="14">{t('{n} jours', { n: 14 })}</option><option value="30">{t('{n} jours', { n: 30 })}</option><option value="90">{t('{n} jours', { n: 90 })}</option><option value="365">{t('1 an')}</option>
      </select>
    </label>
  </div>
  {#if preview}<div class="muted">{preview}</div>{/if}
  <div class="row">
    <button class="primary" onclick={applyChange}>{t('Appliquer')}</button>
    <button onclick={clearAll}>{t('Effacer tout l’historique')}</button>
  </div>
</section>

<style>
  .item { padding: 10px 0; border-bottom: 1px solid var(--line); }
  .head { display: flex; justify-content: space-between; gap: 8px; flex-wrap: wrap; align-items: center; }
  .text { white-space: pre-wrap; margin: 4px 0; }
</style>
