<script lang="ts">
  import HelpButton from '../components/HelpButton.svelte';
  import { onMount } from 'svelte';
  import { api, type HistoryItem } from '../lib/api';
  import { fmt, when } from '../lib/format';
  import { outcomeLabels } from '../lib/timings';
  import { settings } from '../lib/settings.svelte';

  let items = $state<HistoryItem[]>([]);
  let apps = $state<string[]>([]);
  let app = $state('');
  let search = $state('');
  let copied = $state('');

  let keepText = $state(true);
  let keepDays = $state<number | null>(null);
  let preview = $state('');

  let shown = $derived(
    items.filter((d) => (!app || d.app === app) && (!search || (d.text + ' ' + d.raw).toLowerCase().includes(search.toLowerCase()))),
  );

  async function load() {
    const out = await api<{ items: HistoryItem[]; apps: string[] }>('history');
    items = out.items;
    apps = out.apps;
  }

  async function copy(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      copied = 'Copié ✔';
    } catch {
      copied = 'Copie refusée';
    }
    setTimeout(() => (copied = ''), 2000);
  }

  async function remove(id: string) {
    await api('history/delete', { id });
    await load();
  }

  async function clearAll() {
    if (!confirm('Effacer tout l’historique, y compris les chiffres du tableau de bord ? C’est définitif.')) return;
    await api('history/clear', { confirmed: true });
    await load();
  }

  async function previewChange() {
    const out = await api<{ purged: number; erased: number }>('history/preview', { keep_text: keepText, keep_days: keepDays });
    const parts = [];
    if (out.purged) parts.push(`${out.purged} dictée(s) plus anciennes seraient supprimées (leurs totaux restent)`);
    if (out.erased) parts.push(`le texte de ${out.erased} dictée(s) serait effacé`);
    preview = parts.join(' ; ') || 'Rien ne serait supprimé.';
  }

  async function applyChange() {
    await previewChange();
    if (!confirm(`${preview}\n\nAppliquer ces réglages ? C’est définitif.`)) return;
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

<h1>Historique</h1>
<p class="muted">Gardé sur cet ordinateur seulement. {copied}</p>

<section class="card">
  <div class="row">
    <input type="text" class="grow" placeholder="Chercher dans les textes" bind:value={search} />
    <select bind:value={app}>
      <option value="">Toutes les applis</option>
      {#each apps as a (a)}<option value={a}>{a}</option>{/each}
    </select>
  </div>
  {#each shown as d (d.id)}
    <div class="item">
      <div class="head">
        <span class="muted">{when(d.date)} · {d.app} · {d.words} mots · {fmt(d.duration)} s{d.translation ? ' · traduit en ' + d.translation : ''}</span>
        <span>
          {#if d.has_text}<button onclick={() => copy(d.text)}>Copier</button>{/if}
          <button onclick={() => remove(d.id)}>Supprimer</button>
        </span>
      </div>
      {#if d.has_text}
        <div class="text">{d.text}</div>
        {#if d.raw && d.raw !== d.text}<details><summary class="muted">Ce que Whisper a entendu</summary><div class="text muted">{d.raw}</div></details>{/if}
      {:else}
        <div class="muted">Texte non gardé (réglage « Garder le texte des dictées »).</div>
      {/if}
      {#if d.mode || d.timing}
        <div class="muted">
          {#if d.mode}Mode : {d.mode}{#if d.timing} — {outcomeLabels[d.timing.outcome] ?? d.timing.outcome}{/if}{/if}
          {#if d.timing}{d.mode ? ' · ' : ''}transcription {fmt(d.timing.transcription)} s · mise au propre {fmt(d.timing.cleanup)} s · total {fmt(d.timing.total)} s{/if}
        </div>
      {/if}
    </div>
  {:else}
    <p class="muted">Aucune dictée{items.length ? ' ne correspond' : ' pour l’instant'}.</p>
  {/each}
</section>

<section class="card" id="conservation">
  <h2>Conservation <HelpButton section="privacy" /></h2>
  <div class="row"><label><input type="checkbox" bind:checked={keepText} onchange={previewChange} /> Garder le texte des dictées (sinon seule la fiche chiffrée reste)</label></div>
  <div class="row">
    <label>Garder l’historique :
      <select value={keepDays === null ? '' : String(keepDays)} onchange={(e) => { keepDays = e.currentTarget.value === '' ? null : Number(e.currentTarget.value); void previewChange(); }}>
        <option value="">sans limite</option>
        <option value="2">2 jours</option><option value="7">7 jours</option><option value="30">30 jours</option><option value="90">90 jours</option><option value="365">1 an</option>
      </select>
    </label>
  </div>
  {#if preview}<div class="muted">{preview}</div>{/if}
  <div class="row">
    <button class="primary" onclick={applyChange}>Appliquer</button>
    <button onclick={clearAll}>Effacer tout l’historique</button>
  </div>
</section>

<style>
  .item { padding: 10px 0; border-bottom: 1px solid var(--line); }
  .head { display: flex; justify-content: space-between; gap: 8px; flex-wrap: wrap; align-items: center; }
  .text { white-space: pre-wrap; margin: 4px 0; }
</style>
