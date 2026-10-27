<script lang="ts">
  import { api } from '../lib/api';
  import { base } from '../lib/format';
  import { status } from '../lib/status.svelte';

  let s = $derived(status.value);
  let devices = $state<{ value: string; label: string }[]>([]);
  let listing = $state(false);
  let keyInput = $state({ deepgram: '', openrouter: '' });
  let modelDraft = $state<string | null>(null);

  async function save(change: Record<string, unknown>) {
    const out = await api('settings', change);
    if (!out.ok) alert(out.error);
    await status.refresh();
  }

  async function saveKey(kind: 'deepgram' | 'openrouter') {
    const out = await api('keys', { kind, value: keyInput[kind] });
    keyInput[kind] = '';
    if (!out.ok) alert(out.error);
    await status.refresh();
  }

  async function listDevices() {
    listing = true;
    const out = await api<{ lines: string[] }>('llama/devices');
    for (const line of out.lines) {
      const m = /^(\w+\d+):\s*(.*)$/.exec(line.trim());
      if (m && !devices.some((d) => d.value === m[1])) devices.push({ value: m[1], label: `${m[1]} — ${m[2]}` });
    }
    listing = false;
  }

  let deviceChoices = $derived.by(() => {
    const list = [{ value: '', label: 'Automatique (toutes les cartes)' }, { value: 'none', label: 'Processeur seulement' }, ...devices];
    if (s && !list.some((d) => d.value === s.llama_device)) list.push({ value: s.llama_device, label: s.llama_device });
    return list;
  });
</script>

<h1>Réglages</h1>

{#if s}
  <section class="card">
    <h2>Moteurs</h2>
    <p class="muted">Le local reste la priorité ; le cloud sert de dépannage quand l’ordinateur est trop lent.</p>
    <div class="row">
      <label>Transcription :
        <select value={s.transcription_engine} onchange={(e) => save({ transcription: e.currentTarget.value })}>
          <option value="whisper">Whisper (sur cet ordinateur)</option>
          <option value="deepgram">Deepgram (cloud)</option>
        </select>
      </label>
      <label>Modèle Whisper :
        <select value={s.whisper.models.includes(s.whisper.chosen) ? s.whisper.chosen : ''} onchange={(e) => save({ whisper_model: e.currentTarget.value })}>
          <option value="">(automatique : le meilleur trouvé)</option>
          {#each s.whisper.models as m (m)}<option value={m}>{base(m)}</option>{/each}
        </select>
      </label>
    </div>
    <div class="row">
      <label>Mise au propre :
        <select value={s.cleanup === 'llamaCpp' || s.cleanup === 'openRouter' ? s.cleanup : 'none'} onchange={(e) => save({ cleanup: e.currentTarget.value })}>
          <option value="none">Aucune (transcription littérale)</option>
          <option value="llamaCpp">llama.cpp (sur cet ordinateur)</option>
          <option value="openRouter">OpenRouter (cloud)</option>
        </select>
      </label>
      {#if s.cleanup === 'llamaCpp'}
        <label>Modèle GGUF :
          <select value={s.llama.models.includes(s.llama.chosen) ? s.llama.chosen : ''} onchange={(e) => save({ llama_model: e.currentTarget.value })}>
            <option value="">(automatique : le meilleur trouvé)</option>
            {#each s.llama.models as m (m)}<option value={m}>{base(m)}</option>{/each}
          </select>
        </label>
      {/if}
    </div>
    {#if s.cleanup === 'openRouter'}
      <div class="row">
        <label>Modèle OpenRouter :
          <input type="text" style="width: 22em" value={modelDraft ?? s.cleanup_model} oninput={(e) => (modelDraft = e.currentTarget.value)}
            onchange={async (e) => { await save({ cleanup_model: e.currentTarget.value }); modelDraft = null; }} />
        </label>
      </div>
    {/if}
  </section>

  <section class="card">
    <h2>llama.cpp : carte graphique et mémoire</h2>
    <div class="row">
      <label>Carte graphique :
        <select value={s.llama_device} onchange={(e) => save({ llama_device: e.currentTarget.value })}>
          {#each deviceChoices as d (d.value)}<option value={d.value}>{d.label}</option>{/each}
        </select>
      </label>
      <button onclick={listDevices} disabled={listing}>{listing ? 'Recherche…' : 'Lister les cartes'}</button>
    </div>
    <div class="row">
      <label>Garder le modèle de mise au propre chargé :
        <select value={String(s.llama_idle_minutes)} onchange={(e) => save({ llama_idle_minutes: Number(e.currentTarget.value) })}>
          <option value="30">30 min sans dictée (comme Scribe)</option>
          <option value="120">2 h</option>
          <option value="480">8 h</option>
          <option value="0">tant que Scriibe tourne</option>
        </select>
      </label>
    </div>
  </section>

  <section class="card">
    <h2>Dictée</h2>
    <div class="row"><span>Touche : <b>Ctrl droit</b> (Maj en plus : traduire)</span></div>
    <div class="row">
      <label><input type="checkbox" checked={s.pause_media} onchange={(e) => save({ pause_media: e.currentTarget.checked })} />
        Mettre en pause la musique ou la vidéo pendant la dictée</label>
    </div>
  </section>

  <section class="card">
    <h2>Clés du cloud (dépannage)</h2>
    <p class="muted">Les clés sont gardées dans le Gestionnaire d’identifiants de Windows, jamais affichées ni écrites dans le journal.</p>
    {#each [{ id: 'deepgram', label: 'Deepgram' }, { id: 'openrouter', label: 'OpenRouter' }] as k (k.id)}
      {@const id = k.id as 'deepgram' | 'openrouter'}
      <div class="row">
        <b style="min-width: 7em">{k.label}</b>
        {#if s.keys[id]}<span class="ok">✔ présente</span>{:else}<span class="muted">absente</span>{/if}
        <input type="password" class="grow" placeholder="Colle la clé ici" autocomplete="off" bind:value={keyInput[id]} />
        <button onclick={() => saveKey(id)} disabled={!keyInput[id]}>Enregistrer</button>
      </div>
    {/each}
  </section>

  <section class="card">
    <h2>Emplacements</h2>
    <div class="muted">Réglages : {s.paths.settings}<br />Modèles : {s.paths.models}<br />Journal : {s.paths.log}</div>
  </section>
{:else}
  <p class="muted">Chargement…</p>
{/if}
