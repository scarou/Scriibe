<script lang="ts">
  import Card from '../components/Card.svelte';
  import { notice } from '../lib/dialog.svelte';
  import HelpButton from '../components/HelpButton.svelte';
  import { onMount } from 'svelte';
  import { api, type ModelVersion, type Phrase, type Take, type Transcribed } from '../lib/api';
  import { bench } from '../lib/bench.svelte';
  import { cleanButChanged, normalizedText, versions } from '../lib/benchflow';
  import { fmt } from '../lib/display.svelte';
  import { t } from '../lib/i18n.svelte';
  import BenchArchive from '../components/BenchArchive.svelte';
  import TimingsCard from '../components/TimingsCard.svelte';
  import TrialBench from '../components/TrialBench.svelte';
  import ProfileBars from '../components/ProfileBars.svelte';
  import { outcomeLabels } from '../lib/timings';
  import { settings } from '../lib/settings.svelte';
  import { nav } from '../lib/nav.svelte';

  let recording = $state(false);
  let working = $state(false);
  let translate = $state(false);
  let level = $state(0);
  let seconds = $state(0);
  let take = $state<Take | null>(null);
  let out = $state<Transcribed | null>(null);
  let audio = $state('');
  let poll: ReturnType<typeof setInterval> | undefined;

  // Comparer les phrases sautées avec le modèle : utile pour juger l'option, mais long avec un modèle lent (processeur) : à choisir.
  const COMPARE_KEY = 'scriibe.bench.compare';
  const savedCompare = (() => {
    try {
      return localStorage.getItem(COMPARE_KEY) !== 'off';
    } catch {
      return true;
    }
  })();
  let compareOn = $state(savedCompare);
  function setCompare(on: boolean) {
    compareOn = on;
    try {
      localStorage.setItem(COMPARE_KEY, on ? 'on' : 'off');
    } catch {
      // Sans stockage du navigateur, le choix ne survit pas au prochain lancement : sans importance.
    }
  }

  let index = $derived(bench.current);
  let phrase = $derived<Phrase | undefined>(bench.phrases[index]);
  let summary = $derived.by(() => {
    const results = bench.results;
    const judged = results.filter((r) => r.out.would_skip);
    const skipped = results.filter((r) => r.out.timing?.outcome === 'skipped');
    const avg = (list: typeof results) => (list.length ? list.reduce((n, r) => n + (r.out.timing?.total ?? 0), 0) / list.length : null);
    const lines: string[] = [];
    if (results[0]?.setup.skipWhenClean) lines.push(t('{n} phrase(s) sautée(s) sur {total}.', { n: skipped.length, total: results.length }));
    else if (judged.length) lines.push(t('Option « Sauter si déjà propre » désactivée ou sans effet : {n} phrase(s) sur {total} auraient été sautées.', { n: judged.length, total: results.length }));
    if (judged.length) lines.push(t('Sur les {n} jugée(s) propre(s), le modèle écrit autre chose pour {changed}.', { n: judged.length, changed: cleanButChanged(results) }));
    const totals = [
      avg(skipped) !== null ? t('{s} s sans modèle', { s: fmt(avg(skipped)!) }) : '',
      avg(results.filter((r) => r.out.timing?.outcome === 'model')) !== null ? t('{s} s avec modèle', { s: fmt(avg(results.filter((r) => r.out.timing?.outcome === 'model'))!) }) : '',
    ].filter(Boolean);
    if (totals.length) lines.push(t('Total moyen du relâchement au collage : {totals}.', { totals: totals.join(' · ') }));
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
        // Phrase sautée : le modèle écrit sa version à côté (rien n'est collé), la phrase suivante attend son retour (sauf si la
        // comparaison est coupée ou si on choisit de ne pas l'attendre).
        const compare = compareOn && out.compare === true && !!out.cleanup_input;
        const runId = bench.runId;
        const kept = await bench.record({ index: phraseIndex, category: p.category, text: p.text, out: { ...out, compare }, take, translated: translate, setup: out.setup });
        if (kept && compare) {
          const m = await api<ModelVersion>('bench/model', { text: out.cleanup_input });
          await bench.attachModel(phraseIndex, m.text ?? null, m.seconds ?? 0, m.error, runId);
        }
      }
    }
  }

  // Les phrases à lire suivent la langue dictée (français ou anglais) : on les relit quand elle change, jamais pendant un passage.
  $effect(() => {
    void settings.value?.language;
    if (!bench.running) void api<{ phrases: Phrase[] }>('phrases').then((r) => bench.setPhrases(r.phrases));
  });

  onMount(() => {
    void settings.load();
    return () => clearInterval(poll);
  });
</script>

<Card id="bench-banc-de-dictee-les-17-phrases-de-scribe" title={t('Banc de dictée (les 17 phrases de Scribe)')} help="measures">
  <p class="muted">
    {t('Lisez chaque phrase à voix haute, à 20–30 cm de l’écran, puis cliquez sur « Terminer ». Le résultat s’affiche tout de suite avec ses contrôles et ses temps ; rien n’est collé ni gardé dans l’historique. Le passage est gardé avec ses réglages pour être comparé plus tard.')}
  </p>

  <div class="row">
    <span class="muted">{t('Les phrases à lire suivent la langue dictée : {language}.', { language: t(({ fr: 'Français', en: 'Anglais', multi: 'Plusieurs langues' } as Record<string, string>)[settings.value?.language ?? 'fr'] ?? 'Français') })}</span>
    <button onclick={() => nav.go('settings', 'dictee')} disabled={bench.running}>{t('Changer la langue dictée')}</button>
  </div>

  <div class="row">
    <label><input type="checkbox" checked={compareOn} onchange={(e) => setCompare(e.currentTarget.checked)} disabled={bench.running} />
      {t('Comparer avec le modèle les phrases sautées (plus long : la phrase suivante attend la réponse du modèle)')}</label>
  </div>

  {#if !bench.running && !finished}
    <div class="row"><button class="primary" onclick={start} disabled={!bench.phrases.length}>{t('Commencer un passage')}</button></div>
  {:else if bench.running && bench.comparing}
    <p class="muted">{t('Le modèle écrit sa version de la phrase sautée, pour comparer ; la phrase suivante arrive juste après… Avec un modèle lent (processeur), cela peut durer une minute ou plus.')}</p>
    <div class="row"><button onclick={() => bench.giveUpComparing(bench.results.find((r) => r.comparing)?.index ?? -1)}>{t('Ne pas attendre')}</button></div>
  {:else if phrase && bench.running}
    <div class="row">
      <span class="tag">{index + 1} / {bench.phrases.length}</span>
      <span class="tag">{t(phrase.category)}</span>
    </div>
    <div class="phrase">{phrase.text}</div>
    <div class="muted">{t(phrase.expect)}</div>
    <div class="row">
      <button class:primary={!recording} class:rec={recording} onclick={toggle} disabled={working}>
        {working ? t('Traitement…') : recording ? t('■ Terminer') : t('● Enregistrer')}
      </button>
      <div class="meter"><div style="width: {Math.round(level * 100)}%"></div></div>
      <span class="muted">{fmt(seconds)} s</span>
    </div>
    <div class="row">
      <label><input type="checkbox" bind:checked={translate} /> {t('Traduire (comme Ctrl + Maj)')}</label>
      <button onclick={skip} disabled={recording || working}>{t('Passer')}</button>
      <button onclick={redo} disabled={recording || working || !bench.results.length}>{t('Relire la dernière')}</button>
      <button onclick={() => { clearTake(); bench.stop(); }} disabled={recording || working}>{t('Arrêter')}</button>
    </div>
  {:else}
    {#if bench.comparing}
      <p class="muted">{t('Le modèle écrit sa version de la dernière phrase sautée…')}</p>
      <div class="row"><button onclick={() => bench.giveUpComparing(bench.results.find((r) => r.comparing)?.index ?? -1)}>{t('Ne pas attendre')}</button></div>
    {/if}
    <p class="ok">{t('✔ Passage fini : {n} phrase(s) lue(s).', { n: bench.results.length })}</p>
    <div class="row"><button class="primary" onclick={start} disabled={bench.comparing}>{t('Recommencer un passage')}</button></div>
  {/if}

  {#if audio}<audio controls src={audio}></audio>{/if}
  {#if take && out}
    <div class="result">
      {#if out.error}
        <span class="bad">{out.error}</span>
      {:else}
        <div class="muted">
          {fmt(take.duration)} s —
          {#if take.verdict === 'correct'}<span class="ok">{t('niveau correct')}</span>{:else}<span class={take.verdict === 'muet' ? 'bad' : 'warn'}>{t('micro {verdict}', { verdict: t(take.verdict) })}</span>{/if}
          ({fmt(take.level)}){take.gain > 1.05 ? t(' — amplifié ×{gain}', { gain: fmt(take.gain) }) : ''}
        </div>
        <ProfileBars profile={out.profile} />
        {#if out.ok}
          <div><b>{t('Whisper a entendu :')}</b> {out.raw}</div>
          <div><b>{t('Serait collé :')}</b> {out.text}</div>
        {:else}
          <div class="bad"><b>{t('Rien n’a été collé : la phrase reste à lire.')}</b></div>
        {/if}
        {#if out.messages.length}<div class="warn">{#each out.messages as m}{m}<br />{/each}</div>{/if}
        {#if out.timing}
          <div class="muted">
            {t('transcription {a} s — mise au propre {b} s ({outcome}) — total après relâche {c} s', { a: fmt(out.timing.transcription), b: fmt(out.timing.cleanup), outcome: out.timing.outcome, c: fmt(out.timing.total) })}
          </div>
        {/if}
        {#each out.checks as c (c.label)}<div class={c.passed ? 'ok' : 'bad'}>{c.passed ? '✔' : '✘'} {t(c.label)}</div>{/each}
      {/if}
    </div>
  {/if}
</Card>

{#if bench.results.length}
  <Card id="bench-ce-passage" title={t('Ce passage')}>
    <div class="muted">{bench.results[0].setup.transcription} · {bench.results[0].setup.cleanup} · {bench.results[0].setup.mode}</div>
    <p class="muted">{summary}</p>
    {#each bench.results as r (r.index)}
      {@const failed = r.out.checks.filter((c) => !c.passed)}
      {@const tm = r.out.timing}
      <div class="line">
        <div>
          <b>{r.index + 1}. {t(r.category)}</b>
          {#if tm}<span class="tag">{t(outcomeLabels[tm.outcome] ?? tm.outcome)}</span>{/if}
          {#if r.out.would_skip && tm?.outcome === 'model'}<span class="tag">{t('aurait été sautée')}</span>{/if}
          <span class={failed.length ? 'bad' : 'ok'}>{r.out.checks.length - failed.length}/{r.out.checks.length}{failed.length ? ' — ' + failed.map((c) => t(c.label)).join(' ; ') : ''}</span>
          {#if tm}<span class="muted"> · {t('transcription {a} s · mise au propre {b} s · total {c} s', { a: fmt(tm.transcription), b: fmt(tm.cleanup), c: fmt(tm.total) })}</span>{/if}
        </div>
        <div class="muted">{t('Whisper : {text}', { text: r.out.raw ?? '—' })}</div>
        <div class="muted">{t('Collé : {text}', { text: r.out.text ?? '—' })}</div>
        {#if r.out.would_skip && tm?.outcome === 'model'}
          <div class="muted">{t('Sans modèle : {text}', { text: normalizedText(r.out.without_model ?? '') === normalizedText(r.out.text ?? '') ? t('identique au texte collé') : (r.out.without_model ?? '') })}</div>
        {/if}
        {#if r.comparing}
          <div class="muted">{t('Modèle : en cours…')}</div>
        {:else if r.modelText != null && r.modelSeconds != null}
          <div class="muted">{t('Modèle ({s} s) : {text}', { s: fmt(r.modelSeconds), text: normalizedText(r.modelText) === normalizedText(r.out.text ?? '') ? t('identique au texte collé') : r.modelText })}</div>
        {:else if r.modelError}
          <div class="muted">{t('Modèle : {error}', { error: t(r.modelError) })}</div>
        {/if}
      </div>
    {/each}
    <p class="muted">{t('Le rapport à copier (onglet « Journal et rapport ») reprend ces résultats.')}</p>
  </Card>
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
