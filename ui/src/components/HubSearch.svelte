<script lang="ts">
  import { api, type DownloadProgress, type HubAnswer } from '../lib/api';
  import { notice } from '../lib/dialog.svelte';
  import { compte, percent, taille } from '../lib/format';

  interface Props {
    /** Les téléchargements choisis ici, par identifiant. */
    chosen: Record<string, DownloadProgress>;
    /** Confirme puis lance le téléchargement (nom, taille réelle et source sont montrés dans la confirmation). */
    download: (id: string) => Promise<void>;
  }
  let { chosen, download }: Props = $props();

  let kind = $state<'llama' | 'whisper'>('llama');
  let sort = $state<'popular' | 'trending' | 'recent'>('popular');
  let query = $state('');
  let busy = $state(false);
  let answer = $state<HubAnswer | null>(null);

  async function look(text = query) {
    if (!text.trim()) return;
    busy = true;
    try {
      const out = await api<HubAnswer & { error?: string }>('hub/look', { query: text, kind, sort });
      if ('error' in out && out.error) await notice(out.error);
      else answer = out;
    } finally {
      busy = false;
    }
  }

  function open(repo: string) {
    query = repo;
    void look(repo);
  }
</script>

<section class="card">
  <h2>Ajouter un modèle depuis Hugging Face</h2>
  <p class="muted">
    Pour essayer un modèle récent : tapez des mots à chercher (« qwen3 instruct », « gemma »), ou le nom d’un dépôt (« propriétaire/nom ») ou son adresse.
    Scriibe liste alors ses fichiers avec leur <b>vraie taille</b> ; rien n’est téléchargé avant votre confirmation. Pour la mise au propre, choisissez un
    fichier <b>GGUF</b> ; la quantification « Q4_K_M » est un bon compromis taille / qualité. Une fois installé, le modèle se choisit dans Réglages → Moteurs.
  </p>
  <div class="row">
    <select bind:value={kind} aria-label="Genre de modèle">
      <option value="llama">Mise au propre (GGUF)</option>
      <option value="whisper">Transcription (Whisper)</option>
    </select>
    <input class="wide" type="text" bind:value={query} placeholder="qwen3 instruct   ou   Qwen/Qwen3-4B-GGUF" onkeydown={(e) => e.key === 'Enter' && look()} />
    <button class="primary" onclick={() => look()} disabled={busy || !query.trim()}>{busy ? 'Recherche…' : 'Chercher'}</button>
  </div>
  <div class="row">
    <span class="muted">Classer les résultats :</span>
    <label><input type="radio" bind:group={sort} value="popular" /> les plus téléchargés</label>
    <label><input type="radio" bind:group={sort} value="trending" /> tendance</label>
    <label><input type="radio" bind:group={sort} value="recent" /> les plus récents</label>
  </div>

  {#if answer?.mode === 'search'}
    {#if answer.repos.length}
      <table>
        <tbody>
          {#each answer.repos as r (r.repo)}
            <tr>
              <td><button onclick={() => open(r.repo)}>{r.repo}</button></td>
              <td class="muted">{compte(r.downloads)} téléchargements · {compte(r.likes)} ♥</td>
            </tr>
          {/each}
        </tbody>
      </table>
      <p class="muted">Cliquez sur un dépôt pour voir ses fichiers.</p>
    {:else}
      <p class="muted">Aucun résultat : essayez d’autres mots.</p>
    {/if}
  {:else if answer?.mode === 'files'}
    <h3>{answer.repo}</h3>
    {#if answer.files.length}
      {#each answer.files as f (f.id)}
        {@const p = chosen[f.id]}
        <div class="file">
          <div>
            <b>{f.name}</b> <span class="muted">— {taille(f.size)}</span>
            {#if f.note}<div class="warn">{f.note}</div>{/if}
          </div>
          {#if p?.state === 'running'}
            <div class="bar"><div style="width: {percent(p.done ?? 0, p.total ?? 0)}%"></div></div>
            <span class="muted">{taille(p.done ?? 0)} / {taille(p.total ?? 0)} ({percent(p.done ?? 0, p.total ?? 0)} %)</span>
          {:else if p?.state === 'done'}
            <span class="ok">✔ installé</span>
          {:else}
            <button onclick={() => download(f.id)} disabled={!f.downloadable}>Télécharger</button>
            {#if p?.state === 'failed'}<div class="bad">Échec : {p.error}</div>{/if}
          {/if}
        </div>
      {/each}
    {:else}
      <p class="muted">Aucun fichier de ce genre dans ce dépôt{kind === 'llama' ? ' (cherchez un dépôt « …-GGUF »)' : ''}.</p>
    {/if}
  {/if}
</section>

<style>
  .row { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; margin: 8px 0; }
  .wide { flex: 1 1 260px; }
  .file { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 8px; padding: 6px 0; border-bottom: 1px solid var(--line); }
  h3 { margin: 12px 0 4px; font-size: 1rem; }
</style>
