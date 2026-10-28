<script lang="ts">
  import HelpButton from '../components/HelpButton.svelte';
  import { onMount } from 'svelte';
  import { api, type Phrase, type Take, type Transcribed } from '../lib/api';
  import { bench } from '../lib/bench.svelte';
  import { fmt } from '../lib/format';
  import BenchArchive from '../components/BenchArchive.svelte';
  import TimingsCard from '../components/TimingsCard.svelte';
  import TrialBench from '../components/TrialBench.svelte';
  import ProfileBars from '../components/ProfileBars.svelte';

  let recording = $state(false);
  let working = $state(false);
  let translate = $state(false);
  let level = $state(0);
  let seconds = $state(0);
  let take = $state<Take | null>(null);
  let out = $state<Transcribed | null>(null);
  let audio = $state('');
  let poll: ReturnType<typeof setInterval> | undefined;

  let index = $derived(bench.current);
  let phrase = $derived<Phrase | undefined>(bench.phrases[index]);
  let finished = $derived(!bench.running && bench.results.length > 0);

  function clearTake() {
    take = null;
    out = null;
    audio = '';
  }

  function start() {
    clearTake();
    bench.start();
  }

  function skip() {
    clearTake();
    bench.skip();
  }

  async function redo() {
    clearTake();
    await bench.redoLast();
  }

  async function toggle() {
    if (!bench.running) return;
    if (!recording) {
      const started = await api('record/start', {});
      if (!started.ok) {
        alert(started.error);
        return;
      }
      recording = true;
      clearTake();
      poll = setInterval(async () => {
        const l = await api<{ seconds: number; level: number }>('record/level');
        level = l.level;
        seconds = l.seconds;
      }, 120);
    } else {
      clearInterval(poll);
      recording = false;
      level = 0;
      working = true;
      const phraseIndex = index;
      take = await api<Take>('record/stop', {});
      audio = 'api/record/audio?' + Date.now();
      out = await api<Transcribed>('transcribe', { phrase: phraseIndex, translate });
      working = false;
      const p = bench.phrases[phraseIndex];
      if (p && take && out && !out.error && out.ok && out.timing) {
        await bench.record({ index: phraseIndex, category: p.category, text: p.text, out, take, translated: translate, setup: out.setup });
      }
    }
  }

  onMount(() => {
    if (!bench.phrases.length) void api<{ phrases: Phrase[] }>('phrases').then((r) => bench.setPhrases(r.phrases));
    return () => clearInterval(poll);
  });
</script>

<h1>Banc d’essai</h1>

<section class="card">
  <h2>Banc de dictée (les 17 phrases de Scribe) <HelpButton section="measures" /></h2>
  <p class="muted">
    Lisez chaque phrase à voix haute, à 20–30 cm de l’écran, puis cliquez sur « Terminer ». Le résultat s’affiche tout de suite avec ses contrôles
    et ses temps ; rien n’est collé ni gardé dans l’historique. Le passage est gardé avec ses réglages pour être comparé plus tard.
  </p>

  {#if !bench.running && !finished}
    <div class="row"><button class="primary" onclick={start} disabled={!bench.phrases.length}>Commencer un passage</button></div>
  {:else if phrase && bench.running}
    <div class="row">
      <span class="tag">{index + 1} / {bench.phrases.length}</span>
      <span class="tag">{phrase.category}</span>
    </div>
    <div class="phrase">{phrase.text}</div>
    <div class="muted">{phrase.expect}</div>
    <div class="row">
      <button class:primary={!recording} class:rec={recording} onclick={toggle} disabled={working}>
        {working ? 'Traitement…' : recording ? '■ Terminer' : '● Enregistrer'}
      </button>
      <div class="meter"><div style="width: {Math.round(level * 100)}%"></div></div>
      <span class="muted">{fmt(seconds)} s</span>
    </div>
    <div class="row">
      <label><input type="checkbox" bind:checked={translate} /> Traduire (comme Ctrl + Maj)</label>
      <button onclick={skip} disabled={recording || working}>Passer</button>
      <button onclick={redo} disabled={recording || working || !bench.results.length}>Relire la dernière</button>
      <button onclick={() => { clearTake(); bench.stop(); }} disabled={recording || working}>Arrêter</button>
    </div>
  {:else}
    <p class="ok">✔ Passage fini : {bench.results.length} phrase(s) lue(s).</p>
    <div class="row"><button class="primary" onclick={start}>Recommencer un passage</button></div>
  {/if}

  {#if audio}<audio controls src={audio}></audio>{/if}
  {#if take && out}
    <div class="result">
      {#if out.error}
        <span class="bad">{out.error}</span>
      {:else}
        <div class="muted">
          {fmt(take.duration)} s —
          {#if take.verdict === 'correct'}<span class="ok">niveau correct</span>{:else}<span class={take.verdict === 'muet' ? 'bad' : 'warn'}>micro {take.verdict}</span>{/if}
          ({fmt(take.level)}){take.gain > 1.05 ? ` — amplifié ×${fmt(take.gain)}` : ''}
        </div>
        <ProfileBars profile={out.profile} />
        {#if out.ok}
          <div><b>Whisper a entendu :</b> {out.raw}</div>
          <div><b>Serait collé :</b> {out.text}</div>
        {:else}
          <div class="bad"><b>Rien n’a été collé</b> : la phrase reste à lire.</div>
        {/if}
        {#if out.messages.length}<div class="warn">{#each out.messages as m}{m}<br />{/each}</div>{/if}
        {#if out.timing}
          <div class="muted">
            transcription {fmt(out.timing.transcription)} s — mise au propre {fmt(out.timing.cleanup)} s ({out.timing.outcome}) — total après relâche {fmt(out.timing.total)} s
          </div>
        {/if}
        {#each out.checks as c (c.label)}<div class={c.passed ? 'ok' : 'bad'}>{c.passed ? '✔' : '✘'} {c.label}</div>{/each}
      {/if}
    </div>
  {/if}
</section>

{#if bench.results.length}
  <section class="card">
    <h2>Ce passage</h2>
    <div class="muted">{bench.results[0].setup.transcription} · {bench.results[0].setup.cleanup} · {bench.results[0].setup.mode}</div>
    <table>
      <tbody>
        {#each bench.results as r (r.index)}
          {@const failed = r.out.checks.filter((c) => !c.passed)}
          <tr>
            <td>{r.index + 1}</td>
            <td>{r.category}</td>
            <td class={failed.length ? 'bad' : 'ok'}>{r.out.checks.length - failed.length}/{r.out.checks.length}{failed.length ? ' — ' + failed.map((c) => c.label).join(' ; ') : ''}</td>
            <td class="muted">{r.out.timing ? fmt(r.out.timing.total) + ' s' : ''}</td>
          </tr>
        {/each}
      </tbody>
    </table>
    <p class="muted">Le rapport à copier (onglet « Journal et rapport ») reprend ces résultats.</p>
  </section>
{/if}

<BenchArchive />

<TimingsCard />

<TrialBench />

<style>
  .phrase { font-size: 1.25rem; font-weight: 600; margin: 8px 0; padding: 12px; border-radius: 10px; background: var(--bg); border: 1px dashed var(--line); }
  .meter { height: 10px; border-radius: 5px; background: var(--line); overflow: hidden; flex: 1 1 200px; }
  .meter > div { height: 100%; background: var(--ok); transition: width 0.1s; }
  .result { margin-top: 10px; padding: 10px; border-radius: 10px; background: var(--bg); }
</style>
