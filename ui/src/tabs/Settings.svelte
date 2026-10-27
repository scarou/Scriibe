<script lang="ts">
  import { api } from '../lib/api';
  import { base } from '../lib/format';
  import { onMount } from 'svelte';
  import { settings } from '../lib/settings.svelte';
  import { status } from '../lib/status.svelte';

  let s = $derived(status.value);
  let v = $derived(settings.value);
  let devices = $state<{ value: string; label: string }[]>([]);
  let listing = $state(false);
  let keyInput = $state({ deepgram: '', openrouter: '' });
  let modelDraft = $state<string | null>(null);

  const languages = [
    { id: 'fr', label: 'Français' },
    { id: 'en', label: 'Anglais' },
    { id: 'multi', label: 'Plusieurs langues' },
  ];
  const translations = [
    { id: 'en', label: 'Anglais' },
    { id: 'es', label: 'Espagnol' },
    { id: 'de', label: 'Allemand' },
    { id: 'it', label: 'Italien' },
    { id: 'pt', label: 'Portugais (Brésil)' },
  ];

  onMount(() => void settings.load());

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
    if (v && !list.some((d) => d.value === v.llamaDevice)) list.push({ value: v.llamaDevice, label: v.llamaDevice });
    return list;
  });
</script>

<h1>Réglages</h1>

{#if s && v}
  <section class="card">
    <h2>Moteurs</h2>
    <p class="muted">Le local reste la priorité ; le cloud sert de dépannage quand l’ordinateur est trop lent.</p>
    <div class="row">
      <label>Transcription :
        <select value={v.transcriptionEngine} onchange={(e) => settings.change({ transcriptionEngine: e.currentTarget.value as never })}>
          <option value="whisper">Whisper (sur cet ordinateur)</option>
          <option value="deepgram">Deepgram (cloud)</option>
        </select>
      </label>
      {#if v.transcriptionEngine === 'whisper'}
        <label>Modèle Whisper :
          <select value={s.whisper.models.includes(v.whisperModelPath) ? v.whisperModelPath : ''} onchange={(e) => settings.change({ whisperModelPath: e.currentTarget.value })}>
            <option value="">(automatique : le meilleur trouvé)</option>
            {#each s.whisper.models as m (m)}<option value={m}>{base(m)}</option>{/each}
          </select>
        </label>
      {/if}
    </div>
    <div class="row">
      <label>Mise au propre :
        <select value={v.cleanupEngine === 'llamaCpp' || v.cleanupEngine === 'openRouter' ? v.cleanupEngine : 'none'} onchange={(e) => settings.change({ cleanupEngine: e.currentTarget.value as never })}>
          <option value="none">Aucune (transcription littérale)</option>
          <option value="llamaCpp">llama.cpp (sur cet ordinateur)</option>
          <option value="openRouter">OpenRouter (cloud)</option>
        </select>
      </label>
      {#if v.cleanupEngine === 'llamaCpp'}
        <label>Modèle GGUF :
          <select value={s.llama.models.includes(v.llamaCppModel) ? v.llamaCppModel : ''} onchange={(e) => settings.change({ llamaCppModel: e.currentTarget.value })}>
            <option value="">(automatique : le meilleur trouvé)</option>
            {#each s.llama.models as m (m)}<option value={m}>{base(m)}</option>{/each}
          </select>
        </label>
      {/if}
    </div>
    {#if v.cleanupEngine === 'openRouter'}
      <div class="row">
        <label>Modèle OpenRouter :
          <input type="text" style="width: 22em" value={modelDraft ?? v.cleanupModel} oninput={(e) => (modelDraft = e.currentTarget.value)}
            onchange={async (e) => { await settings.change({ cleanupModel: e.currentTarget.value }); modelDraft = null; }} />
        </label>
      </div>
    {/if}
    {#if v.cleanupEngine === 'llamaCpp'}
      <div class="row">
        <label>Carte graphique :
          <select value={v.llamaDevice} onchange={(e) => settings.change({ llamaDevice: e.currentTarget.value })}>
            {#each deviceChoices as d (d.value)}<option value={d.value}>{d.label}</option>{/each}
          </select>
        </label>
        <button onclick={listDevices} disabled={listing}>{listing ? 'Recherche…' : 'Lister les cartes'}</button>
      </div>
      <div class="row">
        <label>Garder le modèle chargé :
          <select value={String(v.llamaIdleMinutes)} onchange={(e) => settings.change({ llamaIdleMinutes: Number(e.currentTarget.value) })}>
            <option value="30">30 min sans dictée (comme Scribe)</option>
            <option value="120">2 h</option>
            <option value="480">8 h</option>
            <option value="0">tant que Scriibe tourne</option>
          </select>
        </label>
      </div>
    {/if}
  </section>

  <section class="card">
    <h2>Dictée</h2>
    <div class="row"><span>Touche : <b>Ctrl droit</b> (Maj en plus : traduire)</span></div>
    <div class="row">
      <label>Langue dictée :
        <select value={v.language} onchange={(e) => settings.change({ language: e.currentTarget.value as never })}>
          {#each languages as l (l.id)}<option value={l.id}>{l.label}</option>{/each}
        </select>
      </label>
      <label>Traduire vers :
        <select value={v.translationLanguage} onchange={(e) => settings.change({ translationLanguage: e.currentTarget.value as never })}>
          {#each translations as l (l.id)}<option value={l.id}>{l.label}</option>{/each}
        </select>
      </label>
    </div>
    <div class="row">
      <label>Micro :
        <select value={v.microphoneUID} onchange={(e) => settings.change({ microphoneUID: e.currentTarget.value })}>
          <option value="">Celui du système</option>
          {#each s.microphone_list as m (m.uid)}<option value={m.uid}>{m.label}</option>{/each}
        </select>
      </label>
    </div>
    <div class="row">
      <label><input type="checkbox" checked={v.pauseMedia} onchange={(e) => settings.change({ pauseMedia: e.currentTarget.checked })} />
        Mettre en pause la musique ou la vidéo pendant la dictée</label>
    </div>
    <div class="row">
      <label><input type="checkbox" checked={v.restoreClipboard} onchange={(e) => settings.change({ restoreClipboard: e.currentTarget.checked })} />
        Remettre le presse-papiers d’avant après le collage</label>
    </div>
    <div class="row">
      <label><input type="checkbox" checked={v.skipWhenClean} onchange={(e) => settings.change({ skipWhenClean: e.currentTarget.checked })} />
        Sauter la mise au propre quand Whisper rend déjà un texte propre (modes Simple et Amusant)</label>
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
