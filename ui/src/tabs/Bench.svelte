<script lang="ts">
  import { notice } from '../lib/dialog.svelte';
  import HelpButton from '../components/HelpButton.svelte';
  import { onMount } from 'svelte';
  import { api, type ModelVersion, type Phrase, type Take, type Transcribed } from '../lib/api';
  import { bench } from '../lib/bench.svelte';
  import { cleanButChanged, normalizedText, versions } from '../lib/benchflow';
  import { fmt } from '../lib/format';
  import BenchArchive from '../components/BenchArchive.svelte';
  import TimingsCard from '../components/TimingsCard.svelte';
  import TrialBench from '../components/TrialBench.svelte';
  import ProfileBars from '../components/ProfileBars.svelte';
  import { outcomeLabels } from '../lib/timings';

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
  let summary = $derived.by(() => {
    const results = bench.results;
    const judged = results.filter((r) => r.out.would_skip);
    const skipped = results.filter((r) => r.out.timing?.outcome === 'skipped');
    const avg = (list: typeof results) => (list.length ? list.reduce((n, r) => n + (r.out.timing?.total ?? 0), 0) / list.length : null);
    const lines: string[] = [];
    if (results[0]?.setup.skipWhenClean) lines.push(`${skipped.length} phrase(s) sautée(s) sur ${results.length}.`);
    else if (judged.length) lines.push(`Option « Sauter si déjà propre » désactivée ou sans effet : ${judged.length} phrase(s) sur ${results.length} auraient été sautées.`);
    if (judged.length) lines.push(`Sur les ${judged.length} jugée(s) propre(s), le modèle écrit autre chose pour ${cleanButChanged(results)}.`);
    const totals = [
      avg(skipped) !== null ? `${fmt(avg(skipped)!)} s sans modèle` : '',
      avg(results.filter((r) => r.out.timing?.outcome === 'model')) !== null ? `${fmt(avg(results.filter((r) => r.out.timing?.outcome === 'model'))!)} s avec modèle` : '',
    ].filter(Boolean);
    if (totals.length) lines.push(`Total moyen du relâchement au collage : ${totals.join(' · ')}.`);
    return lines.join(' ');
  });
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
        await notice(started.error);
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
        const kept = await bench.record({ index: phraseIndex, category: p.category, text: p.text, out, take, translated: translate, setup: out.setup });
        // Phrase sautée : le modèle écrit sa version à côté (rien n'est collé), la phrase suivante attend son retour.
        if (kept && out.compare && out.cleanup_input) {
          const m = await api<ModelVersion>('bench/model', { text: out.cleanup_input });
          await bench.attachModel(phraseIndex, m.text ?? null, m.seconds ?? 0, m.error);
        }
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
  {:else if bench.running && bench.comparing}
    <p class="muted">Le modèle écrit sa version de la phrase sautée, pour comparer ; la phrase suivante arrive juste après…</p>
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
    {#if bench.comparing}<p class="muted">Le modèle écrit sa version de la dernière phrase sautée…</p>{/if}
    <p class="ok">✔ Passage fini : {bench.results.length} phrase(s) lue(s).</p>
    <div class="row"><button class="primary" onclick={start} disabled={bench.comparing}>Recommencer un passage</button></div>
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
    <p class="muted">{summary}</p>
    {#each bench.results as r (r.index)}
      {@const failed = r.out.checks.filter((c) => !c.passed)}
      {@const t = r.out.timing}
      <div class="line">
        <div>
          <b>{r.index + 1}. {r.category}</b>
          {#if t}<span class="tag">{outcomeLabels[t.outcome] ?? t.outcome}</span>{/if}
          {#if r.out.would_skip && t?.outcome === 'model'}<span class="tag">aurait été sautée</span>{/if}
          <span class={failed.length ? 'bad' : 'ok'}>{r.out.checks.length - failed.length}/{r.out.checks.length}{failed.length ? ' — ' + failed.map((c) => c.label).join(' ; ') : ''}</span>
          {#if t}<span class="muted"> · transcription {fmt(t.transcription)} s · mise au propre {fmt(t.cleanup)} s · total {fmt(t.total)} s</span>{/if}
        </div>
        <div class="muted">Whisper : {r.out.raw ?? '—'}</div>
        <div class="muted">Collé : {r.out.text ?? '—'}</div>
        {#if r.out.would_skip && t?.outcome === 'model'}
          <div class="muted">Sans modèle : {normalizedText(r.out.without_model ?? '') === normalizedText(r.out.text ?? '') ? 'identique au texte collé' : r.out.without_model}</div>
        {/if}
        {#if r.comparing}
          <div class="muted">Modèle : en cours…</div>
        {:else if r.modelText != null && r.modelSeconds != null}
          <div class="muted">Modèle ({fmt(r.modelSeconds)} s) : {normalizedText(r.modelText) === normalizedText(r.out.text ?? '') ? 'identique au texte collé' : r.modelText}</div>
        {:else if r.modelError}
          <div class="muted">Modèle : {r.modelError}</div>
        {/if}
      </div>
    {/each}
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
  .line { padding: 6px 0; border-bottom: 1px solid var(--line); }
  .result { margin-top: 10px; padding: 10px; border-radius: 10px; background: var(--bg); }
</style>
