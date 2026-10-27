<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type Dictionary } from '../lib/api';

  let words = $state<string[]>([]);
  let corrections = $state<{ heard: string; written: string }[]>([]);
  let newWord = $state('');
  let heard = $state('');
  let written = $state('');
  let saved = $state('');
  let loaded = false;
  let timer: ReturnType<typeof setTimeout> | undefined;

  function apply(d: Dictionary) {
    words = d.words;
    corrections = d.corrections;
  }

  /** Enregistre un instant après la dernière modification. */
  function changed() {
    if (!loaded) return;
    saved = '';
    clearTimeout(timer);
    timer = setTimeout(async () => {
      apply(await api<Dictionary>('dictionary', { words, corrections }));
      saved = 'Enregistré ✔';
    }, 500);
  }

  function addWord() {
    const w = newWord.trim();
    if (!w) return;
    words = [...words, w];
    newWord = '';
    changed();
  }

  function addCorrection() {
    if (!heard.trim() || !written.trim()) return;
    corrections = [...corrections, { heard: heard.trim(), written: written.trim() }];
    heard = '';
    written = '';
    changed();
  }

  onMount(() => {
    void api<Dictionary>('dictionary').then((d) => {
      apply(d);
      loaded = true;
    });
    return () => clearTimeout(timer);
  });
</script>

<h1>Dictionnaire</h1>
<p class="muted">Ce que Scriibe écrit exactement, même quand Whisper l’entend autrement. {saved}</p>

<section class="card">
  <h2>Mots à écrire exactement</h2>
  <p class="muted">Noms propres, sigles, marques : « Claude Code », « to-do »… Whisper les entend parfois en deux mots ou sans majuscule.</p>
  <div class="row">
    <input type="text" class="grow" placeholder="Ajouter un mot ou une expression" bind:value={newWord} onkeydown={(e) => e.key === 'Enter' && addWord()} />
    <button onclick={addWord} disabled={!newWord.trim()}>Ajouter</button>
  </div>
  <div class="chips">
    {#each words as w, i (w + i)}
      <span class="chip">{w}<button class="x" title="Retirer" onclick={() => { words = words.filter((_, j) => j !== i); changed(); }}>×</button></span>
    {:else}
      <span class="muted">Aucun mot pour l’instant.</span>
    {/each}
  </div>
</section>

<section class="card">
  <h2>Corrections « entendu → écrit »</h2>
  <p class="muted">Quand Whisper entend toujours « cri be », on écrit « Scriibe ». Mot entier, majuscules ignorées.</p>
  <div class="row">
    <input type="text" class="grow" placeholder="Entendu" bind:value={heard} />
    <span>→</span>
    <input type="text" class="grow" placeholder="Écrit" bind:value={written} onkeydown={(e) => e.key === 'Enter' && addCorrection()} />
    <button onclick={addCorrection} disabled={!heard.trim() || !written.trim()}>Ajouter</button>
  </div>
  {#if corrections.length}
    <table>
      <tbody>
        {#each corrections as c, i (c.heard + '→' + c.written + i)}
          <tr>
            <td>{c.heard}</td><td>→</td><td><b>{c.written}</b></td>
            <td style="text-align: right"><button class="x" title="Retirer" onclick={() => { corrections = corrections.filter((_, j) => j !== i); changed(); }}>×</button></td>
          </tr>
        {/each}
      </tbody>
    </table>
  {:else}
    <span class="muted">Aucune correction pour l’instant.</span>
  {/if}
</section>

<style>
  .chips { display: flex; flex-wrap: wrap; gap: 8px; margin-top: 6px; }
  .chip { display: inline-flex; align-items: center; gap: 4px; background: var(--bg); border: 1px solid var(--line); border-radius: 999px; padding: 2px 4px 2px 12px; }
  .x { border: 0; background: transparent; padding: 0 8px; font-size: 1.1rem; color: var(--muted); }
  .x:hover { color: var(--bad); }
</style>
