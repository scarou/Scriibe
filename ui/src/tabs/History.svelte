<script lang="ts">
  import Card from '../components/Card.svelte';
  import { confirmBox } from '../lib/dialog.svelte';
  import { onMount } from 'svelte';
  import { api, type HistoryItem } from '../lib/api';
  import { fmt } from '../lib/display.svelte';
  import { i18n, t } from '../lib/i18n.svelte';
  import { outcomeLabels } from '../lib/timings';
  import { settings } from '../lib/settings.svelte';
  import { dayKey, groupByDay, isRewritten, matchesHistory, type Period } from '../lib/historyfilter';

  let items = $state<HistoryItem[]>([]);
  let apps = $state<string[]>([]);
  let app = $state('');
  let search = $state('');
  let period = $state<Period>('all');
  let from = $state('');
  let to = $state('');
  let rewrittenOnly = $state(false);
  let copied = $state('');

  let keepText = $state(true);
  let keepDays = $state<number | null>(null);
  let preview = $state('');

  let shown = $derived(items.filter((d) => matchesHistory(d, { period, from, to, app, search, rewrittenOnly }, new Date())));
  let groups = $derived(groupByDay(shown));
  let filtered = $derived(period !== 'all' || !!app || !!search.trim() || rewrittenOnly);

  const periods: { id: Period; label: () => string }[] = [
    { id: 'today', label: () => t('Aujourd’hui') },
    { id: '7', label: () => t('{n} jours', { n: 7 }) },
    { id: '30', label: () => t('{n} jours', { n: 30 }) },
    { id: 'all', label: () => t('Tout') },
    { id: 'custom', label: () => t('Dates choisies') },
  ];

  /** « Aujourd'hui », « Hier », puis « samedi 31 octobre » : le titre d'un jour du groupe. */
  function dayTitle(key: string): string {
    const now = new Date();
    if (key === dayKey(now)) return t('Aujourd’hui');
    if (key === dayKey(new Date(now.getFullYear(), now.getMonth(), now.getDate() - 1))) return t('Hier');
    const [y, m, d] = key.split('-').map(Number);
    const long = new Date(y, m - 1, d).toLocaleDateString(i18n.locale, { weekday: 'long', day: 'numeric', month: 'long' });
    return long.charAt(0).toUpperCase() + long.slice(1);
  }

  const timeOf = (iso: string) => new Date(iso).toLocaleTimeString(i18n.locale, { hour: '2-digit', minute: '2-digit' });

  /** Déplie ou replie les tuiles des jours, par le bouton de chaque carte (qui garde aussi son pli en mémoire). */
  function setAllFolded(folded: boolean) {
    for (const card of document.querySelectorAll<HTMLElement>('section.fold-card[id^="historique-"]')) {
      if ((card.dataset.folded === 'true') !== folded) card.querySelector<HTMLButtonElement>('button.fold')?.click();
    }
  }

  function resetFilters() {
    period = 'all';
    from = '';
    to = '';
    app = '';
    rewrittenOnly = false;
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

  // Cases à cocher : les dictées choisies, effaçables d'un coup.
  let selected = $state<string[]>([]);
  let selectedShown = $derived(shown.filter((d) => selected.includes(d.id)).map((d) => d.id));
  const isOn = (id: string) => selected.includes(id);

  function setSelected(ids: string[], on: boolean) {
    const rest = selected.filter((id) => !ids.includes(id));
    selected = on ? [...rest, ...ids] : rest;
  }

  /** Efface ces dictées après confirmation (une tuile de jour, ou les cases cochées). */
  async function removeMany(ids: string[], what: string) {
    if (!ids.length) return;
    if (!(await confirmBox(t('Effacer {what} ? C’est définitif.', { what }), { confirm: t('Effacer'), danger: true }))) return;
    await api('history/delete_many', { ids });
    selected = selected.filter((id) => !ids.includes(id));
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

<div class="page-head">
  <div>
    <h1>{t('Historique')}</h1>
    <p class="muted">{t('Gardé sur cet ordinateur seulement.')} {copied}</p>
  </div>
  <button class="danger" onclick={clearAll} disabled={!items.length}>🗑 {t('Tout effacer')}</button>
</div>

<div class="sticky-top">
<Card id="conservation" title={t('Confidentialité de l’historique')} help="privacy" intro={t('Pour que ce que vous dictez ne reste pas sur le disque : par exemple un code ou une adresse dictés dans un message. Vous pouvez ne garder que les statistiques (nombre de mots, durée, appli), ou n’avoir que les derniers jours. Le tableau de bord reste juste dans les deux cas.')}>
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
  </div>
</Card>

<div class="card filters">
  <div class="filter-bar">
    <input type="text" class="search" placeholder={t('Chercher dans les textes')} bind:value={search} />
    <div class="seg" role="group">
      {#each periods as p (p.id)}
        <button class:on={period === p.id} aria-pressed={period === p.id} onclick={() => (period = p.id)}>{p.label()}</button>
      {/each}
    </div>
    <select bind:value={app} aria-label={t('Toutes les applis')}>
      <option value="">{t('Toutes les applis')}</option>
      {#each apps as a (a)}<option value={a}>{a}</option>{/each}
    </select>
    <label class="rewritten"><input type="checkbox" bind:checked={rewrittenOnly} /> {t('Dictées reformulées')}</label>
    {#if groups.length}
      <span class="seg" role="group">
        <button onclick={() => setAllFolded(false)}>{t('Tout déplier')}</button>
        <button onclick={() => setAllFolded(true)}>{t('Tout replier')}</button>
      </span>
    {/if}
  </div>
  {#if shown.length}
    <div class="row">
      <label><input type="checkbox" checked={selectedShown.length === shown.length} indeterminate={selectedShown.length > 0 && selectedShown.length < shown.length} onchange={(e) => setSelected(shown.map((d) => d.id), e.currentTarget.checked)} /> {t('Tout sélectionner')}</label>
      {#if selectedShown.length}
        <span class="muted">{t('{n} dictée(s) cochée(s)', { n: selectedShown.length })}</span>
        <button class="danger" onclick={() => removeMany(selectedShown, t('les {n} dictée(s) cochée(s)', { n: selectedShown.length }))}>🗑 {t('Effacer la sélection')}</button>
      {/if}
    </div>
  {/if}
  {#if period === 'custom' || filtered}
    <div class="row">
      {#if period === 'custom'}
        <label>{t('du')} <input type="date" bind:value={from} max={to || undefined} /></label>
        <label>{t('au')} <input type="date" bind:value={to} min={from || undefined} /></label>
      {/if}
      {#if filtered}
        <span class="muted">{t('{n} dictée(s) sur {total}', { n: shown.length, total: items.length })}</span>
        <button onclick={resetFilters}>{t('Tout afficher')}</button>
      {/if}
    </div>
  {/if}
</div>
</div>

{#each groups as g (g.day)}
  <Card id={'historique-' + g.day} title={dayTitle(g.day) + ' · ' + t('{n} dictée(s) · {words} mots', { n: g.items.length, words: g.items.reduce((sum, d) => sum + d.words, 0) })}>
      <div class="day-bar">
        <label><input type="checkbox" checked={g.items.every((d) => isOn(d.id))} indeterminate={g.items.some((d) => isOn(d.id)) && !g.items.every((d) => isOn(d.id))} onchange={(e) => setSelected(g.items.map((d) => d.id), e.currentTarget.checked)} /> {t('Cocher ce jour')}</label>
        <button class="danger" onclick={() => removeMany(g.items.map((d) => d.id), t('les {n} dictée(s) de ce jour', { n: g.items.length }))}>🗑 {t('Effacer ce jour')}</button>
      </div>
      {#each g.items as d (d.id)}
        <div class="item">
          <input type="checkbox" class="pick" checked={isOn(d.id)} onchange={(e) => setSelected([d.id], e.currentTarget.checked)} aria-label={t('Cocher cette dictée')} />
          <span class="muted time">{timeOf(d.date)}</span>
          <div class="body">
            <div class="meta"><span class="chip">{d.app}</span> {#if isRewritten(d)}<span class="chip rewritten-chip" title={t('Le texte a été changé par la mise au propre.')}>{t('Reformulée')}</span> {/if}<span class="muted">{t('{n} mots · {s} s', { n: d.words, s: fmt(d.duration) })}{d.translation ? t(' · traduit en {language}', { language: d.translation }) : ''}</span></div>
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
          <span class="actions">
            {#if d.has_text}<button onclick={() => copy(d.text)}>{t('Copier')}</button>{/if}
            <button onclick={() => remove(d.id)}>{t('Supprimer')}</button>
          </span>
        </div>
      {/each}
  </Card>
{:else}
  <p class="muted">{items.length ? t('Aucune dictée ne correspond.') : t('Aucune dictée pour l’instant.')}</p>
{/each}

<style>
  .page-head { display: flex; justify-content: space-between; align-items: flex-start; gap: 12px; }
  .page-head p { margin: 0 0 12px; }
  .danger { color: var(--bad); border-color: color-mix(in srgb, var(--bad) 40%, var(--line)); }
  /* Confidentialité et filtres restent en haut pendant le défilement des tuiles ; si la fenêtre est basse, ce bloc défile lui-même. */
  .sticky-top { position: sticky; top: 0; z-index: 5; background: var(--bg); padding-top: 4px; max-height: 70vh; overflow-y: auto; }
  .filters { padding: 12px 14px; }
  .filter-bar { display: flex; flex-wrap: wrap; gap: 10px; align-items: center; }
  .search { flex: 1 1 140px; min-width: 0; }
  .seg { display: inline-flex; border: 1px solid var(--line); border-radius: 10px; overflow: hidden; }
  .seg button { border: 0; border-radius: 0; padding: 6px 12px; background: var(--card); }
  .seg button + button { border-left: 1px solid var(--line); }
  .seg button.on { background: var(--accent); color: var(--accent-ink); font-weight: 600; }
  .item { display: flex; gap: 14px; padding: 12px 0; border-top: 1px solid var(--line); }
  .item:first-child { border-top: 0; }
  .day-bar { display: flex; justify-content: space-between; align-items: center; gap: 10px; flex-wrap: wrap; padding: 4px 0 8px; }
  .pick { flex: none; margin-top: 3px; }
  .time { width: 46px; flex: none; font-variant-numeric: tabular-nums; padding-top: 2px; }
  .body { flex: 1; min-width: 0; }
  .meta { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; }
  .rewritten-chip { background: transparent; border: 1px solid var(--accent); }
  .rewritten { display: inline-flex; align-items: center; gap: 4px; font-size: .9rem; }
  .chip { background: var(--accent-soft); color: var(--accent); font-weight: 600; font-size: .78rem; padding: 1px 9px; border-radius: 999px; }
  .actions { display: flex; gap: 6px; align-items: flex-start; flex: none; }
  .text { white-space: pre-wrap; margin: 4px 0; overflow-wrap: anywhere; }
  @media (max-width: 640px) { .item { flex-wrap: wrap; } .actions { width: 100%; } }
</style>
