<script lang="ts">
  import { api, type DownloadProgress, type HubAnswer } from '../lib/api';
  import { notice } from '../lib/dialog.svelte';
  import { percent } from '../lib/format';
  import { compte, taille } from '../lib/display.svelte';
  import { t } from '../lib/i18n.svelte';

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
  <h2>{t('Ajouter un modèle depuis Hugging Face')}</h2>
  <p class="muted">
    {t('Pour essayer un modèle récent : tapez des mots à chercher (« qwen3 instruct », « gemma »), ou le nom d’un dépôt (« propriétaire/nom ») ou son adresse. Scriibe liste alors ses fichiers avec leur vraie taille ; rien n’est téléchargé avant votre confirmation. Pour la mise au propre, choisissez un fichier GGUF ; la quantification « Q4_K_M » est un bon compromis taille / qualité. Une fois installé, le modèle se choisit dans Réglages → Moteurs.')}
  </p>
  <div class="row">
    <select bind:value={kind} aria-label={t('Genre de modèle')}>
      <option value="llama">{t('Mise au propre (GGUF)')}</option>
      <option value="whisper">{t('Transcription (Whisper)')}</option>
    </select>
    <input class="wide" type="text" bind:value={query} placeholder={t('qwen3 instruct   ou   Qwen/Qwen3-4B-GGUF')} onkeydown={(e) => e.key === 'Enter' && look()} />
    <button class="primary" onclick={() => look()} disabled={busy || !query.trim()}>{busy ? t('Recherche…') : t('Chercher')}</button>
  </div>
  <div class="row">
    <span class="muted">{t('Classer les résultats :')}</span>
    <label><input type="radio" bind:group={sort} value="popular" /> {t('les plus téléchargés')}</label>
    <label><input type="radio" bind:group={sort} value="trending" /> {t('tendance')}</label>
    <label><input type="radio" bind:group={sort} value="recent" /> {t('les plus récents')}</label>
  </div>

  {#if answer?.mode === 'search'}
    {#if answer.repos.length}
      <table>
        <tbody>
          {#each answer.repos as r (r.repo)}
            <tr>
              <td><button onclick={() => open(r.repo)}>{r.repo}</button></td>
              <td class="muted">{t('{n} téléchargements · {likes} ♥', { n: compte(r.downloads), likes: compte(r.likes) })}</td>
            </tr>
          {/each}
        </tbody>
      </table>
      <p class="muted">{t('Cliquez sur un dépôt pour voir ses fichiers.')}</p>
    {:else}
      <p class="muted">{t('Aucun résultat : essayez d’autres mots.')}</p>
    {/if}
  {:else if answer?.mode === 'files'}
    <h3>{answer.repo}</h3>
    {#if answer.files.length}
      {#each answer.files as f (f.id)}
        {@const p = chosen[f.id]}
        <div class="file">
          <div>
            <b>{f.name}</b> <span class="muted">— {taille(f.size)}</span>
            {#if f.note}<div class="warn">{t(f.note)}</div>{/if}
          </div>
          {#if p?.state === 'running'}
            <div class="bar"><div style="width: {percent(p.done ?? 0, p.total ?? 0)}%"></div></div>
            <span class="muted">{taille(p.done ?? 0)} / {taille(p.total ?? 0)} ({percent(p.done ?? 0, p.total ?? 0)} %)</span>
          {:else if p?.state === 'done'}
            <span class="ok">{t('✔ installé')}</span>
          {:else}
            <button onclick={() => download(f.id)} disabled={!f.downloadable}>{t('Télécharger')}</button>
            {#if p?.state === 'failed'}<div class="bad">{t('Échec : {error}', { error: p.error ?? '' })}</div>{/if}
          {/if}
        </div>
      {/each}
    {:else}
      <p class="muted">{kind === 'llama' ? t('Aucun fichier de ce genre dans ce dépôt (cherchez un dépôt « …-GGUF »).') : t('Aucun fichier de ce genre dans ce dépôt.')}</p>
    {/if}
  {/if}
</section>

<style>
  .row { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; margin: 8px 0; }
  .wide { flex: 1 1 260px; }
  .file { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 8px; padding: 6px 0; border-bottom: 1px solid var(--line); }
  h3 { margin: 12px 0 4px; font-size: 1rem; }
</style>
