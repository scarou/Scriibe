<script lang="ts">
  import { confirmBox, notice } from '../lib/dialog.svelte';
  import HelpButton from '../components/HelpButton.svelte';
  import { onMount } from 'svelte';
  import { api, type DownloadProgress } from '../lib/api';
  import HubSearch from '../components/HubSearch.svelte';
  import InstalledModels from '../components/InstalledModels.svelte';
  import { base, mo, percent } from '../lib/format';
  import { status } from '../lib/status.svelte';

  interface Item {
    id: string;
    label: string;
    detail: string;
    kind: 'program' | 'whisper' | 'llama';
    progress: { state: 'idle' | 'running' | 'done' | 'failed'; done?: number; total?: number; error?: string };
  }
  interface Resolved {
    ok: boolean;
    error?: string;
    name?: string;
    size?: number;
    url?: string;
    dest?: string;
  }

  let s = $derived(status.value);
  let items = $state<Item[]>([]);
  let chosen = $state<Record<string, DownloadProgress>>({});
  let resolved = $state<Record<string, Resolved>>({});
  let seeing = $state<string | null>(null);

  async function load() {
    const out = await api<{ items: Item[]; chosen: Record<string, DownloadProgress> }>('downloads');
    items = out.items;
    chosen = out.chosen;
  }

  async function see(id: string) {
    seeing = id;
    resolved[id] = await api<Resolved>('downloads/resolve', { id });
    seeing = null;
  }

  async function go(id: string) {
    const r = resolved[id];
    if (!(await confirmBox(`Télécharger ${r.name} (${mo(r.size ?? 0)})\ndepuis ${r.url}\nvers ${r.dest} ?`, { confirm: 'Télécharger' }))) return;
    const out = await api('downloads/start', { id, confirmed: true });
    if (!out.ok) await notice(out.error);
    delete resolved[id];
    await load();
    await status.refresh();
  }

  /** Un fichier choisi sur Hugging Face : on montre le nom, la taille réelle et la source, puis on demande confirmation. */
  async function downloadChosen(id: string) {
    const r = await api<Resolved>('downloads/resolve', { id });
    if (!r.ok) {
      await notice(r.error ?? 'fichier introuvable');
      return;
    }
    if (!(await confirmBox(`Télécharger ${r.name} (${mo(r.size ?? 0)})\ndepuis ${r.url}\nvers ${r.dest} ?`, { confirm: 'Télécharger' }))) return;
    const out = await api('downloads/start', { id, confirmed: true });
    if (!out.ok) await notice(out.error);
    await load();
    await status.refresh();
  }

  onMount(() => {
    void load();
    const timer = setInterval(() => void load(), 1500);
    return () => clearInterval(timer);
  });

  let rows = $derived(
    s
      ? [
          { label: 'whisper-server', ok: !!s.whisper.binary, detail: s.whisper.binary ?? 'à installer ci-dessous' },
          { label: 'Modèle Whisper', ok: s.whisper.models.length > 0, detail: s.whisper.models.map(base).join(', ') || 'à installer ci-dessous' },
          { label: 'llama-server', ok: !!s.llama.binary, detail: s.llama.binary ?? 'à installer ci-dessous — seulement si la mise au propre est activée' },
          { label: 'Modèle de mise au propre (GGUF)', ok: s.llama.models.length > 0, detail: s.llama.models.map(base).join(', ') || 'à installer ci-dessous' },
          { label: 'Micro', ok: s.microphones.length > 0, detail: s.microphones.join(' | ') || 'aucun micro détecté' },
          { label: 'Touche globale', ok: !s.keys_error, detail: s.keys_error ?? 'Ctrl droit (Maj : traduire)' },
        ]
      : [],
  );
</script>

<h1>Installation</h1>

<section class="card">
  <h2>État de l’installation <HelpButton section="start" /></h2>
  <table>
    <tbody>
      {#each rows as r (r.label)}
        <tr><td class={r.ok ? 'ok' : 'bad'}>{r.ok ? '✔' : '✘'}</td><td>{r.label}</td><td class="muted">{r.detail}</td></tr>
      {/each}
    </tbody>
  </table>
</section>

{#snippet offer(it: Item)}
  {@const p = it.progress}
  {@const r = resolved[it.id]}
  <div class="item">
    <b>{it.label}</b>
    <div class="muted">{it.detail}</div>
    {#if p.state === 'running'}
      <div class="bar"><div style="width: {percent(p.done ?? 0, p.total ?? 0)}%"></div></div>
      <span class="muted">{mo(p.done ?? 0)} / {mo(p.total ?? 0)} ({percent(p.done ?? 0, p.total ?? 0)} %)</span>
    {:else if p.state === 'done'}
      <span class="ok">✔ installé</span>
    {:else if r?.ok}
      <div class="muted">{r.name} — <b>{mo(r.size ?? 0)}</b><br />Source : {r.url}<br />Destination : {r.dest}</div>
      <button class="primary" onclick={() => go(it.id)}>Télécharger ({mo(r.size ?? 0)})</button>
    {:else}
      <button onclick={() => see(it.id)} disabled={seeing === it.id}>{seeing === it.id ? 'Recherche…' : 'Voir la taille et la source'}</button>
      {#if r && !r.ok}<div class="bad">{r.error}</div>{/if}
      {#if p.state === 'failed'}<div class="bad">Échec : {p.error}</div>{/if}
    {/if}
  </div>
{/snippet}

<section class="card">
  <h2>Les programmes</h2>
  <p class="muted">Rien n’est téléchargé sans votre accord : « Voir » affiche le nom, la taille réelle et la source, puis il faut confirmer.</p>
  {#each items.filter((i) => i.kind === 'program') as it (it.id)}{@render offer(it)}{/each}
</section>

<InstalledModels />

<section class="card">
  <h2>Modèles recommandés</h2>
  <p class="muted">Une courte liste pour démarrer, choisie lors des premiers essais. D’autres modèles se trouvent dans la carte suivante.</p>
  {#each items.filter((i) => i.kind !== 'program') as it (it.id)}{@render offer(it)}{/each}
</section>

<HubSearch {chosen} download={downloadChosen} />

<style>
  .item { margin: 12px 0; }
</style>
