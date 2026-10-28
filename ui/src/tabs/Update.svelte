<script lang="ts">
  import { confirmBox, notice } from '../lib/dialog.svelte';
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import { mo, percent } from '../lib/format';

  interface UpdateState {
    windows: boolean;
    has_token: boolean;
    current: string | null;
    repo: string;
    progress: { state: 'idle' | 'running' | 'installed' | 'failed'; done?: number; total?: number; error?: string };
  }
  interface Found {
    ok: boolean;
    error?: string;
    up_to_date: boolean;
    sha: string;
    title: string;
    date: string;
    size: number;
    run: number;
    source: string;
  }

  let u = $state<UpdateState | null>(null);
  let token = $state('');
  let found = $state<Found | null>(null);
  let searching = $state(false);

  async function load() {
    u = await api<UpdateState>('update');
  }

  async function saveToken() {
    const out = await api('update/token', { token });
    token = '';
    if (!out.ok) await notice(out.error);
    await load();
  }

  async function check() {
    searching = true;
    found = await api<Found>('update/check', {});
    searching = false;
  }

  async function install() {
    if (!found) return;
    if (!(await confirmBox(`Installer la version ${found.sha} (${mo(found.size)})\ndepuis ${found.source} ?\n\nScriibe va redémarrer.`, { confirm: 'Installer et redémarrer' }))) return;
    const out = await api('update/install', { confirmed: true });
    if (!out.ok) await notice(out.error);
    found = null;
    await load();
  }

  onMount(() => {
    void load();
    const timer = setInterval(() => void load(), 1500);
    return () => clearInterval(timer);
  });
</script>

<h1>Mise à jour de Scriibe</h1>

{#if u}
  <section class="card">
    <div class="muted">
      Version installée : {u.current ?? 'compilée à la main (inconnue)'}{u.windows ? '' : ' — la mise à jour ne marche que sous Windows'}
    </div>

    {#if u.windows && !u.has_token}
      <p class="muted">
        Pour récupérer les nouvelles versions sur GitHub (dépôt privé), Scriibe a besoin d’un jeton en lecture seule. Il est gardé dans le
        Gestionnaire d’identifiants de Windows, jamais affiché ni écrit dans le journal.
      </p>
      <div class="row">
        <input type="password" class="grow" placeholder="Colle le jeton GitHub ici" autocomplete="off" bind:value={token} />
        <button onclick={saveToken} disabled={!token}>Enregistrer le jeton</button>
      </div>
    {/if}

    {#if u.windows && u.has_token}
      <div class="row"><button onclick={check} disabled={searching}>Chercher une nouvelle version</button></div>
      {#if searching}<span class="muted">Recherche…</span>{/if}
      {#if found}
        {#if !found.ok}
          <div class="bad">{found.error}</div>
        {:else}
          <div class="muted">
            Dernière version : <b>{found.sha}</b> — « {found.title} »<br />
            Construite le {found.date} · <b>{mo(found.size)}</b><br />
            Source : {found.source} (exécution n° {found.run})
          </div>
          {#if found.up_to_date}
            <div class="ok">✔ Vous avez déjà cette version.</div>
          {:else}
            <div class="row"><button class="primary" onclick={install}>Installer et redémarrer</button></div>
          {/if}
        {/if}
      {/if}
    {/if}

    {#if u.progress.state === 'running'}
      <div class="bar"><div style="width: {percent(u.progress.done ?? 0, u.progress.total ?? 0)}%"></div></div>
      <span class="muted">Téléchargement {mo(u.progress.done ?? 0)} / {mo(u.progress.total ?? 0)} ({percent(u.progress.done ?? 0, u.progress.total ?? 0)} %)</span>
    {:else if u.progress.state === 'installed'}
      <div class="ok">✔ Nouvelle version installée. Scriibe redémarre : une nouvelle fenêtre s’ouvre dans quelques secondes (celle-ci peut être fermée).</div>
    {:else if u.progress.state === 'failed'}
      <div class="bad">Échec : {u.progress.error}</div>
    {/if}
  </section>
{/if}
