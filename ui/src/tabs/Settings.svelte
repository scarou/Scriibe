<script lang="ts">
  import { notice } from '../lib/dialog.svelte';
  import HelpButton from '../components/HelpButton.svelte';
  import { api } from '../lib/api';
  import { base } from '../lib/format';
  import { onMount, untrack } from 'svelte';
  import KeyPicker from '../components/KeyPicker.svelte';
  import { colors, positions, styles } from '../lib/appearance';
  import { nav } from '../lib/nav.svelte';
  import { t } from '../lib/i18n.svelte';
  import { settings } from '../lib/settings.svelte';
  import { status } from '../lib/status.svelte';

  let s = $derived(status.value);
  let v = $derived(settings.value);
  let devices = $state<{ value: string; label: string }[]>([]);
  let listing = $state(false);
  let keyInput = $state({ deepgram: '', openrouter: '' });
  let modelDraft = $state<string | null>(null);
  let urlDraft = $state<string | null>(null);
  let local = $state<{ models: string[]; status: string }>({ models: [], status: '' });

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

  /** LM Studio et Ollama : le moteur du réglage, la clé de son modèle et celle de son adresse. */
  let localEngine = $derived(
    v?.cleanupEngine === 'lmStudio'
      ? { id: 'lmStudio', name: 'LM Studio', model: 'lmStudioModel', url: 'lmStudioURL' }
      : v?.cleanupEngine === 'ollama'
        ? { id: 'ollama', name: 'Ollama', model: 'ollamaModel', url: 'ollamaURL' }
        : null,
  );
  let localModel = $derived(localEngine && v ? String(v[localEngine.model] ?? '') : '');

  /** La liste des modèles du serveur (réponse tardive d'un autre moteur : ignorée). */
  async function refreshLocal() {
    const engine = localEngine;
    if (!engine) return;
    local = { models: [], status: `Recherche du serveur ${engine.name}…` };
    const out = await api<{ models: string[]; status: string }>('local-models', { engine: engine.id });
    if (localEngine?.id !== engine.id) return;
    local = { models: out.models, status: out.status };
    if (!localModel && out.models.length) await settings.change({ [engine.model]: out.models[0] });
  }

  /** Relit la liste quand le moteur ou son adresse change (et seulement alors, pas à chaque réglage). */
  let localId = $derived(localEngine?.id ?? '');
  let localAddress = $derived(localEngine && v ? String(v[localEngine.url] ?? '') : '');
  $effect(() => {
    void localAddress;
    if (localId) untrack(() => void refreshLocal());
  });

  /** Joue un son au volume des réglages (le serveur le fait jouer par le système). */
  function playSound(name: string) {
    if (name && name !== 'Aucun') void api('sounds/play', { name, volume: v?.soundVolume ?? 0.5 });
  }

  /** « Choisir un fichier… » : la boîte d'ouverture de Windows, pour un modèle Whisper rangé ailleurs. */
  async function chooseWhisperFile() {
    const out = await api<{ ok: boolean; path?: string | null; error?: string }>('pick-file', { kind: 'whisper' });
    if (!out.ok) void notice(out.error ?? 'réglage refusé');
    else if (out.path) await settings.change({ whisperModelPath: out.path });
  }

  async function saveKey(kind: 'deepgram' | 'openrouter') {
    const out = await api('keys', { kind, value: keyInput[kind] });
    keyInput[kind] = '';
    if (!out.ok) void notice(out.error ?? 'réglage refusé');
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

<h1>{t('Réglages')}</h1>

{#if s && v}
  <section class="card" id="langue">
    <h2>{t('Langue de l’interface')}</h2>
    <p class="muted">{t('Celle des menus et des textes de Scriibe. Elle ne change ni la langue que vous dictez ni la langue de traduction.')}</p>
    <label>{t('Langue :')}
      <select value={v.uiLanguage} onchange={(e) => settings.change({ uiLanguage: e.currentTarget.value as never })}>
        <option value="auto">{t('Automatique (celle du système)')}</option>
        <option value="fr">Français</option>
        <option value="en">English</option>
      </select>
    </label>
  </section>

  <section class="card" id="moteurs">
    <h2>Moteurs <HelpButton section="privacy" /></h2>
    <p class="muted">Le local reste la priorité ; le cloud sert de dépannage quand l’ordinateur est trop lent.</p>
    <div class="row">
      <button onclick={() => nav.openAssistant()} title="Mesure l’ordinateur et propose les moteurs qui lui conviennent">Relancer l’assistant</button>
      <button onclick={() => nav.openHelp('start')}>À quoi servent ces choix ?</button>
    </div>
    <div class="row">
      <label>Transcription :
        <select value={v.transcriptionEngine} onchange={(e) => settings.change({ transcriptionEngine: e.currentTarget.value as never })}>
          <option value="whisper">Whisper (sur cet ordinateur)</option>
          <option value="deepgram">Deepgram (cloud)</option>
        </select>
      </label>
      {#if v.transcriptionEngine === 'whisper'}
        <label>Modèle Whisper :
          <select value={v.whisperModelPath} onchange={(e) => settings.change({ whisperModelPath: e.currentTarget.value })}>
            <option value="">(automatique : le meilleur trouvé)</option>
            {#if v.whisperModelPath && !s.whisper.models.includes(v.whisperModelPath)}<option value={v.whisperModelPath}>{base(v.whisperModelPath)}</option>{/if}
            {#each s.whisper.models as m (m)}<option value={m}>{base(m)}</option>{/each}
          </select>
        </label>
        <button onclick={chooseWhisperFile}>Choisir un fichier…</button>
      {:else}
        <label>Modèle Deepgram :
          <select value={v.transcriptionModel} onchange={(e) => settings.change({ transcriptionModel: e.currentTarget.value })}>
            <option value="nova-3">nova-3 (le plus précis)</option>
            <option value="nova-2">nova-2 (secours)</option>
          </select>
        </label>
      {/if}
    </div>
    {#if v.transcriptionEngine === 'whisper'}
      <div class="row">
        <label>Libérer Whisper au repos :
          <select value={String(v.whisperIdleMinutes)} onchange={(e) => settings.change({ whisperIdleMinutes: Number(e.currentTarget.value) })}>
            {#each [0, 5, 10, 30, 60] as m (m)}<option value={String(m)}>{m === 0 ? 'Jamais' : `${m} min`}</option>{/each}
          </select>
        </label>
      </div>
      <p class="muted">
        {v.whisperIdleMinutes === 0
          ? 'Whisper reste en mémoire (environ 1 Go) tant que Scriibe tourne.'
          : `Sans dictée pendant ${v.whisperIdleMinutes} min, Whisper s’arrête et rend sa mémoire ; il revient à l’appui de la touche (environ 1 s, pendant que tu parles).`}
      </p>
    {/if}
    <div class="row">
      <label>Mise au propre :
        <select value={v.cleanupEngine} onchange={(e) => settings.change({ cleanupEngine: e.currentTarget.value as never })}>
          <option value="none">Aucune (transcription littérale)</option>
          <option value="llamaCpp">llama.cpp (sur cet ordinateur)</option>
          <option value="lmStudio">LM Studio (sur cet ordinateur)</option>
          <option value="ollama">Ollama (sur cet ordinateur)</option>
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
    {#if localEngine}
      <div class="row">
        <label>Modèle {localEngine.name} :
          <select value={localModel} onchange={(e) => settings.change({ [localEngine.model]: e.currentTarget.value })}>
            {#if !localModel}<option value="">Choisir un modèle</option>
            {:else if !local.models.includes(localModel)}<option value={localModel}>{localModel}</option>{/if}
            {#each local.models as m (m)}<option value={m}>{m}</option>{/each}
          </select>
        </label>
        <button onclick={refreshLocal}>Actualiser</button>
      </div>
      <p class="muted">{local.status}</p>
      <div class="row">
        <label>Adresse du serveur :
          <input type="text" style="width: 18em" value={urlDraft ?? String(v[localEngine.url] ?? '')} oninput={(e) => (urlDraft = e.currentTarget.value)}
            onchange={async (e) => { await settings.change({ [localEngine.url]: e.currentTarget.value.trim() }); urlDraft = null; }} />
        </label>
      </div>
    {/if}
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

  <section class="card" id="dictee">
    <h2>Dictée <HelpButton section="dictate" /></h2>
    <KeyPicker />
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
      {#each [{ key: 'startSoundName', label: 'Son au début' }, { key: 'endSoundName', label: 'Son à la fin' }] as snd (snd.key)}
        {@const current = String(v[snd.key] ?? 'Aucun')}
        <label>{snd.label} :
          <select value={current} onchange={(e) => { void settings.change({ [snd.key]: e.currentTarget.value }); playSound(e.currentTarget.value); }}>
            <option value="Aucun">Aucun</option>
            {#each s.sounds as n (n)}<option value={n}>{n}</option>{/each}
          </select>
        </label>
        <button onclick={() => playSound(current)} disabled={current === 'Aucun'} title="Écouter">▶</button>
      {/each}
    </div>
    <div class="row">
      <label>Volume des sons :
        <input type="range" min="0.05" max="1" step="0.05" value={v.soundVolume} onchange={async (e) => { await settings.change({ soundVolume: Number(e.currentTarget.value) }); playSound(String(v.startSoundName)); }} />
        {Math.round(v.soundVolume * 100)} %
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
        Sauter la mise au propre quand Whisper rend déjà un texte propre (mode Simple)</label>
    </div>
  </section>

  <section class="card" id="indicateur">
    <h2>Apparence de l’indicateur <HelpButton section="indicator" /></h2>
    <p class="muted">L’icône près de l’horloge change toujours de couleur. La <b>pastille à l’écran</b> (bêta) ajoute l’état de la dictée, le niveau du micro et les alertes, sans jamais prendre le focus.</p>
    <div class="row">
      <label><input type="checkbox" checked={v.overlayEnabled} onchange={(e) => settings.change({ overlayEnabled: e.currentTarget.checked })} />
        Afficher la pastille à l’écran (bêta)</label>
    </div>
    <div class="row">
      <label>Style :
        <select value={v.captureStyle} onchange={(e) => settings.change({ captureStyle: e.currentTarget.value as never })}>
          {#each styles as o (o.id)}<option value={o.id}>{o.label}</option>{/each}
        </select>
      </label>
      <label>Position :
        <select value={v.capturePosition} onchange={(e) => settings.change({ capturePosition: e.currentTarget.value as never })}>
          {#each positions as o (o.id)}<option value={o.id}>{o.label}</option>{/each}
        </select>
      </label>
      <label>Couleur :
        <select value={v.captureColor} onchange={(e) => settings.change({ captureColor: e.currentTarget.value as never })}>
          {#each colors as o (o.id)}<option value={o.id}>{o.label}</option>{/each}
        </select>
      </label>
    </div>
    <div class="row">
      <label>Transparence :
        <input type="range" min="0.3" max="1" step="0.05" value={v.captureOpacity} onchange={(e) => settings.change({ captureOpacity: Number(e.currentTarget.value) })} />
        {Math.round(v.captureOpacity * 100)} %
      </label>
      <button onclick={() => api('overlay/preview', {})} disabled={!v.overlayEnabled}>Aperçu</button>
    </div>
  </section>

  <section class="card" id="cles">
    <h2>Clés du cloud (dépannage) <HelpButton section="privacy" /></h2>
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
