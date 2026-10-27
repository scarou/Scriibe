<script lang="ts">
  import { settings } from '../lib/settings.svelte';
  import {
    averageWpm, counts, isExact, marks, percent, rejectionText, SETTING_RANGE, typingResult, typingTexts, type TypingResult,
  } from '../lib/typing';

  let textIndex = $state(0);
  let typed = $state('');
  let startedAt = $state<number | null>(null);
  let result = $state<TypingResult | null>(null);
  let results = $state<TypingResult[]>([]);
  let zone: HTMLTextAreaElement | undefined = $state();

  let reference = $derived(typingTexts[textIndex]);
  let letters = $derived(Array.from(reference));
  let state_ = $derived(marks(reference, typed));
  let average = $derived(averageWpm(results));

  function onInput() {
    if (result) return;
    if (startedAt === null && typed.length > 0) startedAt = performance.now();
    if (startedAt !== null && isExact(reference, typed)) finish();
  }

  function finish() {
    if (startedAt === null || result) return;
    result = typingResult(reference, typed, (performance.now() - startedAt) / 1000);
    results = [...results, result];
  }

  function again() {
    textIndex = (textIndex + 1) % typingTexts.length;
    typed = '';
    startedAt = null;
    result = null;
    queueMicrotask(() => zone?.focus());
  }

  async function adopt() {
    if (average !== null) await settings.change({ typingWPM: average });
  }
</script>

<div class="reference" aria-label="Texte à recopier">
  {#each letters as c, i (i)}<span class={state_[i]}>{c}</span>{/each}
</div>
<textarea
  bind:this={zone}
  bind:value={typed}
  oninput={onInput}
  onpaste={(e) => e.preventDefault()}
  ondrop={(e) => e.preventDefault()}
  readonly={result !== null}
  rows="4"
  spellcheck="false"
  autocomplete="off"
  autocapitalize="off"
  placeholder="Recopiez le texte ci-dessus ; le chrono part à la première lettre."
></textarea>
<div class="row">
  {#if !result}
    <button onclick={finish} disabled={startedAt === null}>Terminer</button>
  {:else}
    <button class="primary" onclick={again}>Nouvel essai</button>
  {/if}
</div>

{#if result}
  <div class="result">
    <b>{result.wpm} mots par minute</b> en {Math.round(result.seconds)} s, {percent(result.correct, result.total)} % de lettres justes
    {#if counts(result)}<span class="ok"> — essai retenu</span>{:else}<span class="warn"> — non retenu : {rejectionText(result.rejection!)}</span>{/if}
  </div>
{/if}
{#if average !== null}
  <div class="row">
    <span>Moyenne des essais retenus : <b>{average} mots par minute</b>.</span>
    <button onclick={adopt} disabled={settings.value?.typingWPM === average}>Utiliser comme ma vitesse au clavier</button>
  </div>
{/if}
<div class="muted">Réglage actuel : {settings.value?.typingWPM ?? '…'} mots par minute (entre {SETTING_RANGE.min} et {SETTING_RANGE.max}).</div>

<style>
  .reference { font-size: 1.05rem; line-height: 1.7; padding: 10px; border: 1px dashed var(--line); border-radius: 10px; background: var(--bg); margin-bottom: 8px; }
  .correct { color: var(--ok); }
  .wrong { color: var(--bad); text-decoration: underline; }
  .pending { color: var(--muted); }
  textarea { width: 100%; font: inherit; }
  .result { margin: 8px 0; padding: 8px 10px; border-radius: 10px; background: var(--bg); }
</style>
