<script lang="ts">
  import { onMount } from 'svelte';
  import HelpButton from './HelpButton.svelte';
  import { api, type Settings } from '../lib/api';
  import { settings } from '../lib/settings.svelte';
  import { status } from '../lib/status.svelte';
  import { t } from '../lib/i18n.svelte';

  interface Candidate {
    id: string;
    engine: Settings['cleanupEngine'];
    model: string;
  }
  type Result = { ok: true; text: string; seconds: number } | { ok: false; error: string };

  const ENGINES: { id: Settings['cleanupEngine']; label: string; modelKey: string }[] = [
    { id: 'openRouter', label: 'OpenRouter', modelKey: 'cleanupModel' },
    { id: 'lmStudio', label: 'LM Studio', modelKey: 'lmStudioModel' },
    { id: 'llamaCpp', label: 'llama.cpp', modelKey: 'llamaCppModel' },
    { id: 'ollama', label: 'Ollama', modelKey: 'ollamaModel' },
  ];
  const SUGGESTED = ['anthropic/claude-haiku-4.5', 'openai/gpt-4o-mini', 'google/gemini-2.5-flash'];

  let v = $derived(settings.value);
  let s = $derived(status.value);
  let candidates = $derived((v?.trialCandidates as Candidate[] | undefined) ?? []);
  let text = $state<string | null>(null);
  let results = $state<Record<string, Result>>({});
  let running = $state<string | null>(null);
  let models = $state<Record<string, string[]>>({ lmStudio: [], ollama: [] });

  const base = (path: string) => path.split(/[\\/]/).pop() ?? path;

  function listFor(engine: Settings['cleanupEngine']): string[] {
    if (engine === 'llamaCpp') return s?.llama.models ?? [];
    return models[engine] ?? [];
  }

  async function load() {
    for (const engine of ['lmStudio', 'ollama']) {
      const out = await api<{ models: string[] }>('local-models', { engine });
      models[engine] = out.models;
    }
  }

  async function setCandidates(next: Candidate[]) {
    await settings.change({ trialCandidates: next });
  }

  function defaultModel(engine: Settings['cleanupEngine']): string {
    return listFor(engine)[0] ?? '';
  }

  async function update(id: string, change: Partial<Candidate>) {
    await setCandidates(candidates.map((c) => (c.id === id ? { ...c, ...change } : c)));
  }

  async function add() {
    await setCandidates([...candidates, { id: crypto.randomUUID().toUpperCase(), engine: 'lmStudio', model: defaultModel('lmStudio') }]);
  }

  async function remove(id: string) {
    delete results[id];
    await setCandidates(candidates.filter((c) => c.id !== id));
  }

  /** Les modèles l'un après l'autre : en local, les lancer ensemble fausserait les temps. */
  async function runAll() {
    results = {};
    for (const c of candidates) {
      running = c.id;
      results[c.id] = await api<Result>('trial/candidate', { engine: c.engine, model: c.model, text: text ?? v?.trialText });
    }
    running = null;
  }

  function isActive(c: Candidate): boolean {
    if (!v || !v.cleanupEnabled || v.cleanupEngine !== c.engine) return false;
    const key = ENGINES.find((e) => e.id === c.engine)!.modelKey;
    return v[key] === c.model;
  }

  async function use(c: Candidate) {
    const key = ENGINES.find((e) => e.id === c.engine)!.modelKey;
    await settings.change({ cleanupEnabled: true, cleanupEngine: c.engine, [key]: c.model });
  }

  onMount(() => {
    void settings.load();
    void load();
  });
</script>

{#if v && s}
  <section class="card">
    <div class="row">
      <h2>{t('Banc d’essai de la mise au propre')}</h2>
      <span class="grow"></span>
      <button class="primary" onclick={runAll} disabled={running !== null || !candidates.length || !(text ?? v.trialText).trim()}>
        {running ? t('Essais en cours…') : t('Lancer les essais')}
      </button>
      <HelpButton section="measures" />
    </div>
    <p class="muted">{t('Écris une phrase comme tu la dirais, puis compare plusieurs modèles (mode en cours : {mode}). Pour choisir le modèle de mise au propre : même phrase, textes et temps côte à côte.', { mode: v.rewriteMode === 'literal' ? t('Simple') : String(v.rewriteMode) })}</p>
    <textarea rows="3" style="width: 100%" value={text ?? String(v.trialText)} oninput={(e) => (text = e.currentTarget.value)}
      onchange={(e) => settings.change({ trialText: e.currentTarget.value })}></textarea>
    <div class="row"><button onclick={() => { text = null; void settings.change({ trialText: 'euh donc je voulais te dire que euh le rendez-vous avec le client c’est à 5h non pardon à 6h et faudrait que tu prépares le devis' }); }}>{t('Rétablir l’exemple')}</button></div>

    {#each candidates as c (c.id)}
      <div class="cand">
        <div class="row">
          <select value={c.engine} onchange={(e) => update(c.id, { engine: e.currentTarget.value as Candidate['engine'], model: defaultModel(e.currentTarget.value as Candidate['engine']) })}>
            {#each ENGINES as e (e.id)}<option value={e.id}>{e.label}</option>{/each}
          </select>
          {#if c.engine === 'openRouter'}
            <input type="text" list="suggested" style="width: 18em" placeholder={t('fournisseur/modèle')} value={c.model} onchange={(e) => update(c.id, { model: e.currentTarget.value })} />
            <datalist id="suggested">{#each SUGGESTED as m (m)}<option value={m}></option>{/each}</datalist>
          {:else}
            <select value={c.model} onchange={(e) => update(c.id, { model: e.currentTarget.value })}>
              {#if !listFor(c.engine).includes(c.model)}<option value={c.model}>{c.model ? base(c.model) : t('Choisir un modèle')}</option>{/if}
              {#each listFor(c.engine) as m (m)}<option value={m}>{base(m)}</option>{/each}
            </select>
          {/if}
          {#if running === c.id}<span class="muted">…</span>{/if}
          <span class="grow"></span>
          <button onclick={() => remove(c.id)} title={t('Retirer de la comparaison')}>🗑</button>
        </div>
        {#if results[c.id]}
          {@const r = results[c.id]}
          {#if r.ok}
            <div class="result">
              <div>{r.text}</div>
              <div class="row">
                <b>{r.seconds.toFixed(1)} s</b>
                <span class="grow"></span>
                {#if isActive(c)}<span class="muted">{t('Modèle utilisé')}</span>{:else}<button onclick={() => use(c)}>{t('Utiliser ce modèle')}</button>{/if}
              </div>
            </div>
          {:else}
            <div class="bad">{t('Échec : {error}', { error: r.error })}</div>
          {/if}
        {/if}
      </div>
    {/each}
    <div class="row"><button onclick={add}>{t('+ Ajouter un modèle')}</button></div>
  </section>
{/if}

<style>
  .cand { border: 1px solid var(--line); border-radius: 10px; padding: 10px; margin-top: 10px; }
  .result { background: var(--bg); border-radius: 8px; padding: 10px; margin-top: 8px; }
</style>
