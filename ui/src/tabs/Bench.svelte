<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type Phrase, type Take, type Transcribed } from '../lib/api';
  import { bench } from '../lib/bench.svelte';
  import { fmt } from '../lib/format';
  import ProfileBars from '../components/ProfileBars.svelte';

  let index = $state(0);
  let recording = $state(false);
  let working = $state(false);
  let translate = $state(false);
  let level = $state(0);
  let seconds = $state(0);
  let take = $state<Take | null>(null);
  let out = $state<Transcribed | null>(null);
  let audio = $state('');
  let poll: ReturnType<typeof setInterval> | undefined;

  let phrase = $derived<Phrase | undefined>(bench.phrases[index]);

  function go(delta: number) {
    const n = bench.phrases.length;
    index = (index + n + delta) % n;
    take = null;
    out = null;
    audio = '';
  }

  async function toggle() {
    if (!recording) {
      const started = await api('record/start', {});
      if (!started.ok) {
        alert(started.error);
        return;
      }
      recording = true;
      take = null;
      out = null;
      audio = '';
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
      take = await api<Take>('record/stop', {});
      audio = 'api/record/audio?' + Date.now();
      out = await api<Transcribed>('transcribe', { phrase: index, translate });
      working = false;
      if (phrase && take && out && !out.error) bench.add({ index, category: phrase.category, text: phrase.text, out, take, translated: translate });
    }
  }

  onMount(() => {
    if (!bench.phrases.length) void api<{ phrases: Phrase[] }>('phrases').then((r) => (bench.phrases = r.phrases));
    return () => clearInterval(poll);
  });
</script>

<h1>Banc d’essai</h1>

<section class="card">
  <h2>Dictée d’essai (les 17 phrases du banc de Scribe)</h2>
  <p class="muted">
    Lisez la phrase à voix haute, à 20–30 cm de l’écran, puis cliquez sur « Terminer ». Le résultat s’affiche tout de suite ; rien n’est collé
    ni gardé dans l’historique.
  </p>
  {#if phrase}
    <div class="row">
      <button onclick={() => go(-1)} disabled={recording || working}>←</button>
      <span>{index + 1} / {bench.phrases.length}</span>
      <button onclick={() => go(1)} disabled={recording || working}>→</button>
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
    <div class="row"><label><input type="checkbox" bind:checked={translate} /> Traduire (comme Ctrl + Maj)</label></div>
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
            <div class="bad"><b>Rien n’a été collé.</b></div>
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
  {/if}
</section>

<style>
  .phrase { font-size: 1.25rem; font-weight: 600; margin: 8px 0; padding: 12px; border-radius: 10px; background: var(--bg); border: 1px dashed var(--line); }
  .meter { height: 10px; border-radius: 5px; background: var(--line); overflow: hidden; flex: 1 1 200px; }
  .meter > div { height: 100%; background: var(--ok); transition: width 0.1s; }
  .result { margin-top: 10px; padding: 10px; border-radius: 10px; background: var(--bg); }
</style>
