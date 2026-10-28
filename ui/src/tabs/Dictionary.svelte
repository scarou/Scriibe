<script lang="ts">
  import HelpButton from '../components/HelpButton.svelte';
  import { onMount } from 'svelte';
  import { api, type Dictionary } from '../lib/api';
  import { settings } from '../lib/settings.svelte';

  let words = $state<string[]>([]);
  let corrections = $state<{ heard: string; written: string }[]>([]);
  let newWord = $state('');
  let heard = $state('');
  let written = $state('');
  let saved = $state('');
  let loaded = false;
  let signs = $state<{ said: string; written: string }[]>([]);
  let v = $derived(settings.value);

  const bullets = [
    { id: 'dash', symbol: '-', label: 'Tiret  -' },
    { id: 'dot', symbol: '•', label: 'Pastille  •' },
    { id: 'star', symbol: '*', label: 'Étoile  *' },
  ];
  let symbol = $derived(bullets.find((b) => b.id === v?.listBullet)?.symbol ?? '-');
  let listSayings = $derived([
    { said: 'tiret · puce', example: `${symbol} du lait` },
    { said: 'point un · point 2…', example: '1. 2. 3.' },
    { said: 'numéro suivant', example: 'le numéro d’après' },
  ]);

  /** Les retours à la ligne et espaces insécables ne se voient pas : on les montre. */
  function shown(written: string): string {
    if (written === '\n') return '↵';
    if (written === '\n\n') return '¶';
    if (written === '.\n') return '. ↵';
    return written.replaceAll('\u00a0', '');
  }
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
    void settings.load();
    void api<{ signs: { said: string; written: string }[] }>('signs').then((out) => (signs = out.signs));
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
  <h2>Mots à écrire exactement <HelpButton section="dictionary" /></h2>
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
  <h2>Corrections « entendu → écrit » <HelpButton section="dictionary" /></h2>
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

<section class="card">
  <h2>Signes dictés <HelpButton section="dictionary" /></h2>
  <p class="muted">Toujours reconnus, avec leurs variantes (« ouvre » ou « ouvrez », « ferme » ou « fermez la parenthèse », « retour » ou « passe à la ligne »…). Pour un signe à toi, ajoute une correction : « trois points d’exclamation » → !!!</p>
  <div class="chips">
    {#each signs as sign (sign.said)}
      <span class="sign">{sign.said} → <b>{shown(sign.written)}</b></span>
    {/each}
  </div>
  <h3>Listes</h3>
  <p class="muted">Pour dicter une liste : dis « retour à la ligne », puis « tiret » ou « puce » devant chaque élément, ou « point un », « numéro suivant »… Reconnus seulement en début de ligne : « un tiret entre les deux mots » reste du texte.</p>
  <div class="chips">
    {#each listSayings as saying (saying.said)}
      <span class="sign">{saying.said} → <b>{saying.example}</b></span>
    {/each}
  </div>
  {#if v}
    <div class="row">
      <label>Caractère devant chaque élément :
        <select value={v.listBullet} onchange={(e) => settings.change({ listBullet: e.currentTarget.value as never })}>
          {#each bullets as b (b.id)}<option value={b.id}>{b.label}</option>{/each}
        </select>
      </label>
    </div>
  {/if}
</section>

<style>
  .sign { background: var(--bg); border: 1px solid var(--line); border-radius: 999px; padding: 2px 12px; }
  h3 { margin: 14px 0 4px; font-size: 1rem; }
  .chips { display: flex; flex-wrap: wrap; gap: 8px; margin-top: 6px; }
  .chip { display: inline-flex; align-items: center; gap: 4px; background: var(--bg); border: 1px solid var(--line); border-radius: 999px; padding: 2px 4px 2px 12px; }
  .x { border: 0; background: transparent; padding: 0 8px; font-size: 1.1rem; color: var(--muted); }
  .x:hover { color: var(--bad); }
</style>
