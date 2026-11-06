<script lang="ts">
  import Card from '../components/Card.svelte';
  import { notice } from '../lib/dialog.svelte';
  import HelpButton from '../components/HelpButton.svelte';
  import { api } from '../lib/api';
  import { base } from '../lib/format';
  import { onMount, untrack } from 'svelte';
  import KeyPicker from '../components/KeyPicker.svelte';
  import { colors, positions, styles } from '../lib/appearance';
  import { nav } from '../lib/nav.svelte';
  import { keyName, t, tx } from '../lib/i18n.svelte';
  import { settings } from '../lib/settings.svelte';
  import { status } from '../lib/status.svelte';

  /** L'onglet des Réglages dont cette page montre les cartes (SettingsPage choisit). */
  let { section }: { section: 'dictation' | 'engines' | 'appearance' } = $props();

  let s = $derived(status.value);
  let osName = $derived(status.os);
  let v = $derived(settings.value);
  let devices = $state<{ value: string; label: string }[]>([]);
  let listing = $state(false);
  let keyInput = $state({ deepgram: '', openrouter: '' });
  let modelDraft = $state<string | null>(null);
  let urlDraft = $state<string | null>(null);
  let local = $state<{ models: string[]; status: string }>({ models: [], status: '' });

  const languages = [
    { id: 'fr', label: tx('Français') },
    { id: 'en', label: tx('Anglais') },
    { id: 'multi', label: tx('Plusieurs langues') },
  ];
  const translations = [
    { id: 'en', label: tx('Anglais') },
    { id: 'es', label: tx('Espagnol') },
    { id: 'de', label: tx('Allemand') },
    { id: 'it', label: tx('Italien') },
    { id: 'pt', label: tx('Portugais (Brésil)') },
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
    local = { models: [], status: t('Recherche du serveur {name}…', { name: engine.name }) };
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
    if (!out.ok) void notice(out.error ?? t('réglage refusé'));
    else if (out.path) await settings.change({ whisperModelPath: out.path });
  }

  async function saveKey(kind: 'deepgram' | 'openrouter') {
    const out = await api('keys', { kind, value: keyInput[kind] });
    keyInput[kind] = '';
    if (!out.ok) void notice(out.error ?? t('réglage refusé'));
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
    const list = [{ value: '', label: t('Automatique (toutes les cartes)') }, { value: 'none', label: t('Processeur seulement') }, ...devices];
    if (v && !list.some((d) => d.value === v.llamaDevice)) list.push({ value: v.llamaDevice, label: v.llamaDevice });
    return list;
  });
</script>

{#if s && v}
  {#if section === 'appearance'}
  <Card help="start" id="theme" title={t('Apparence de la fenêtre')} intro={t('Claire, sombre, ou celle du système (Automatique). Elle ne change que la fenêtre de Scriibe.')}>
    <label>{t('Apparence :')}
      <select value={v.theme} onchange={(e) => settings.change({ theme: e.currentTarget.value as never })}>
        <option value="auto">{t('Automatique (apparence du système)')}</option>
        <option value="light">{t('Claire')}</option>
        <option value="dark">{t('Sombre')}</option>
      </select>
    </label>
  </Card>

  <Card help="dictate" id="langue" title={t('Langue de l’interface')} intro={t('Celle des menus et des textes de Scriibe. Elle ne change ni la langue que vous dictez ni la langue de traduction.')}>
    <label>{t('Langue :')}
      <select value={v.uiLanguage} onchange={(e) => settings.change({ uiLanguage: e.currentTarget.value as never })}>
        <option value="auto">{t('Automatique (celle du système)')}</option>
        <option value="fr">Français</option>
        <option value="en">English</option>
      </select>
    </label>
  </Card>
  {/if}

  {#if section === 'engines'}
  <Card id="moteurs" title={t('Moteurs')} help="privacy" intro={t('Où passent votre voix et votre texte : « Local » veut dire sur cet ordinateur, sans Internet ; « Cloud », chez un service en ligne. Le local reste la priorité ; le cloud sert de dépannage quand l’ordinateur est trop lent.')}>
    <p class="summary"><b>{t('Ce qui tourne quand vous dictez :')}</b>
      {t('transcription')} {v.transcriptionEngine === 'whisper' ? t('Local') : t('Cloud')}
      · {t('mise au propre')} {v.cleanupEngine === 'none' ? t('aucune') : v.cleanupEngine === 'openRouter' ? t('Cloud') : t('Local')}</p>
    <div class="row">
      <button onclick={() => nav.openAssistant()} title={t('Mesure l’ordinateur et propose les moteurs qui lui conviennent')}>{t('Relancer l’assistant')}</button>
      <button onclick={() => nav.openHelp('start')}>{t('À quoi servent ces choix ?')}</button>
    </div>
    <p class="muted">{t('La transcription est le moteur qui transforme votre voix en texte. Whisper tourne sur cet ordinateur (votre voix reste sur cet ordinateur) ; Deepgram est un service en ligne, utile sur un ordinateur peu puissant.')}</p>
    <div class="row">
      <label>{t('Transcription :')}
        <select value={v.transcriptionEngine} onchange={(e) => settings.change({ transcriptionEngine: e.currentTarget.value as never })}>
          <option value="whisper">{t('Whisper (sur cet ordinateur)')}</option>
          <option value="deepgram">{t('Deepgram (cloud)')}</option>
        </select>
      </label>
      {#if v.transcriptionEngine === 'whisper'}
        <label>{t('Modèle Whisper :')}
          <select value={v.whisperModelPath} onchange={(e) => settings.change({ whisperModelPath: e.currentTarget.value })}>
            <option value="">{t('(automatique : le meilleur trouvé)')}</option>
            {#if v.whisperModelPath && !s.whisper.models.includes(v.whisperModelPath)}<option value={v.whisperModelPath}>{base(v.whisperModelPath)}</option>{/if}
            {#each s.whisper.models as m (m)}<option value={m}>{base(m)}</option>{/each}
          </select>
        </label>
        <button onclick={chooseWhisperFile}>{t('Choisir un fichier…')}</button>
      {:else}
        <label>{t('Modèle Deepgram :')}
          <select value={v.transcriptionModel} onchange={(e) => settings.change({ transcriptionModel: e.currentTarget.value })}>
            <option value="nova-3">{t('nova-3 (le plus précis)')}</option>
            <option value="nova-2">{t('nova-2 (secours)')}</option>
          </select>
        </label>
      {/if}
    </div>
    {#if v.transcriptionEngine === 'whisper'}
      <div class="row">
        <label>{t('Libérer Whisper au repos :')}
          <select value={String(v.whisperIdleMinutes)} onchange={(e) => settings.change({ whisperIdleMinutes: Number(e.currentTarget.value) })}>
            {#each [0, 5, 10, 30, 60] as m (m)}<option value={String(m)}>{m === 0 ? t('Jamais') : t('{n} min', { n: m })}</option>{/each}
          </select>
        </label>
      </div>
      <p class="muted">
        {v.whisperIdleMinutes === 0
          ? t('Whisper reste en mémoire (environ 1 Go) tant que Scriibe tourne.')
          : t('Sans dictée pendant {n} min, Whisper s’arrête et rend sa mémoire ; il revient à l’appui de la touche (environ 1 s, pendant que tu parles).', { n: v.whisperIdleMinutes })}
      </p>
    {/if}
    <p class="muted">{t('La mise au propre est le modèle d’IA qui applique le mode choisi : il retire les « euh » et ponctue (voir l’onglet Modes). llama.cpp, LM Studio ou Ollama le font tourner sur cet ordinateur ; OpenRouter donne accès en ligne à de plus gros modèles.')}</p>
    <div class="row">
      <label>{t('Mise au propre :')}
        <select value={v.cleanupEngine} onchange={(e) => settings.change({ cleanupEngine: e.currentTarget.value as never })}>
          <option value="none">{t('Aucune (transcription littérale)')}</option>
          <option value="llamaCpp">{t('llama.cpp (sur cet ordinateur)')}</option>
          <option value="lmStudio">{t('LM Studio (sur cet ordinateur)')}</option>
          <option value="ollama">{t('Ollama (sur cet ordinateur)')}</option>
          <option value="openRouter">{t('OpenRouter (cloud)')}</option>
        </select>
      </label>
      {#if v.cleanupEngine === 'llamaCpp'}
        <label>{t('Modèle GGUF :')}
          <select value={s.llama.models.includes(v.llamaCppModel) ? v.llamaCppModel : ''} onchange={(e) => settings.change({ llamaCppModel: e.currentTarget.value })}>
            <option value="">{t('(automatique : le meilleur trouvé)')}</option>
            {#each s.llama.models as m (m)}<option value={m}>{base(m)}</option>{/each}
          </select>
        </label>
      {/if}
    </div>
    {#if localEngine}
      <div class="row">
        <label>{t('Modèle {name} :', { name: localEngine.name })}
          <select value={localModel} onchange={(e) => settings.change({ [localEngine.model]: e.currentTarget.value })}>
            {#if !localModel}<option value="">{t('Choisir un modèle')}</option>
            {:else if !local.models.includes(localModel)}<option value={localModel}>{localModel}</option>{/if}
            {#each local.models as m (m)}<option value={m}>{m}</option>{/each}
          </select>
        </label>
        <button onclick={refreshLocal}>{t('Actualiser')}</button>
      </div>
      <p class="muted">{local.status}</p>
      <div class="row">
        <label>{t('Adresse du serveur :')}
          <input type="text" style="width: 18em" value={urlDraft ?? String(v[localEngine.url] ?? '')} oninput={(e) => (urlDraft = e.currentTarget.value)}
            onchange={async (e) => { await settings.change({ [localEngine.url]: e.currentTarget.value.trim() }); urlDraft = null; }} />
        </label>
      </div>
    {/if}
    {#if v.cleanupEngine === 'openRouter'}
      <div class="row">
        <label>{t('Modèle OpenRouter :')}
          <input type="text" style="width: 22em" value={modelDraft ?? v.cleanupModel} oninput={(e) => (modelDraft = e.currentTarget.value)}
            onchange={async (e) => { await settings.change({ cleanupModel: e.currentTarget.value }); modelDraft = null; }} />
        </label>
      </div>
    {/if}
    {#if v.cleanupEngine === 'llamaCpp'}
      <div class="row">
        <label>{t('Carte graphique :')}
          <select value={v.llamaDevice} onchange={(e) => settings.change({ llamaDevice: e.currentTarget.value })}>
            {#each deviceChoices as d (d.value)}<option value={d.value}>{d.label}</option>{/each}
          </select>
        </label>
        <button onclick={listDevices} disabled={listing}>{listing ? t('Recherche…') : t('Lister les cartes')}</button>
      </div>
      <div class="row">
        <label>{t('Garder le modèle chargé :')}
          <select value={String(v.llamaIdleMinutes)} onchange={(e) => settings.change({ llamaIdleMinutes: Number(e.currentTarget.value) })}>
            <option value="30">{t('30 min sans dictée (comme Scribe)')}</option>
            <option value="120">{t('2 h')}</option>
            <option value="480">{t('8 h')}</option>
            <option value="0">{t('tant que Scriibe tourne')}</option>
          </select>
        </label>
      </div>
    {/if}
  </Card>
  {/if}

  {#if section === 'dictation'}
  <Card id="dictee" title={t('Dictée')} help="dictate" intro={t('Comment vous dictez : vous maintenez la touche, vous parlez, vous relâchez, et le texte se colle là où est votre curseur. Les sons confirment le début et la fin, sans regarder l’écran.')}>
    {#if s?.gnome_shortcut}
      <p class="muted">{t('Sous GNOME (Wayland), vous dictez avec un raccourci de GNOME : {shortcut}. GNOME n’accepte pas une touche seule comme Ctrl droit : il faut une combinaison avec une touche ordinaire (une lettre, une flèche, Espace). Pour le changer : Paramètres de GNOME → Applications → Scriibe → Raccourcis clavier, puis relancez Scriibe.', { shortcut: keyName(s.gnome_shortcut) })}</p>
      <p class="muted">{s.gnome_translate_shortcut ? t('Pour dicter et traduire : {shortcut} (même endroit pour le changer).', { shortcut: keyName(s.gnome_translate_shortcut) }) : t('Pas encore de raccourci de traduction : relancez Scriibe, GNOME vous en fera choisir un.')}</p>
    {:else}
      <KeyPicker />
    {/if}
    <div class="setting">
      <label>{t('Langue dictée :')}
        <select value={v.language} onchange={(e) => settings.change({ language: e.currentTarget.value as never })}>
          {#each languages as l (l.id)}<option value={l.id}>{t(l.label)}</option>{/each}
        </select>
      </label>
    </div>
    <div class="setting">
      <label>{t('Traduire vers :')}
        <select value={v.translationLanguage} onchange={(e) => settings.change({ translationLanguage: e.currentTarget.value as never })}>
          {#each translations as l (l.id)}<option value={l.id}>{t(l.label)}</option>{/each}
        </select>
      </label>
    </div>
    <div class="settings-group">
      {#each [{ key: 'startSoundName', label: tx('Son au début') }, { key: 'endSoundName', label: tx('Son à la fin') }] as snd (snd.key)}
        {@const current = String(v[snd.key] ?? 'Aucun')}
        <div class="setting">
        <label>{t(snd.label)} :
          <select value={current} onchange={(e) => { void settings.change({ [snd.key]: e.currentTarget.value }); playSound(e.currentTarget.value); }}>
            <option value="Aucun">{t('Aucun')}</option>
            {#each s.sounds as n (n)}<option value={n}>{n}</option>{/each}
          </select>
        </label>
        <button onclick={() => playSound(current)} disabled={current === 'Aucun'} title={t('Écouter')}>▶</button>
        </div>
      {/each}
    </div>
    <div class="setting">
      <label>{t('Volume des sons :')}
        <input type="range" min="0.05" max="1" step="0.05" value={v.soundVolume} onchange={async (e) => { await settings.change({ soundVolume: Number(e.currentTarget.value) }); playSound(String(v.startSoundName)); }} />
        {Math.round(v.soundVolume * 100)} %
      </label>
    </div>
    <div class="setting">
      <label>{t('Micro :')}
        <select value={v.microphoneUID} onchange={(e) => settings.change({ microphoneUID: e.currentTarget.value })}>
          <option value="">{t('Celui du système')}</option>
          {#each s.microphone_list as m (m.uid)}<option value={m.uid}>{m.label}</option>{/each}
        </select>
      </label>
    </div>
    <div class="setting">
      <label><input type="checkbox" checked={v.pauseMedia} onchange={(e) => settings.change({ pauseMedia: e.currentTarget.checked })} />
        {t('Mettre en pause la musique ou la vidéo pendant la dictée')}</label>
    </div>
    <div class="setting">
      <label><input type="checkbox" checked={v.restoreClipboard} onchange={(e) => settings.change({ restoreClipboard: e.currentTarget.checked })} />
        {t('Remettre le presse-papiers d’avant après le collage')}</label>
    </div>
  </Card>
  {/if}

  {#if section === 'dictation'}
  <Card help="modes" id="texte" title={t('Ce que devient ton texte')} intro={t('Après la transcription, un modèle d’IA retire les « euh » et ponctue, selon le mode choisi (onglet Modes). Le moteur de cette mise au propre se choisit dans l’onglet Moteurs.')}>
    <div class="row">
      <label><input type="checkbox" checked={v.skipWhenClean} onchange={(e) => settings.change({ skipWhenClean: e.currentTarget.checked })} />
        {t('Sauter la mise au propre quand Whisper rend déjà un texte propre (mode Simple)')}</label>
    </div>
  </Card>
  {/if}

  {#if section === 'appearance'}
  <Card id="indicateur" title={t('Apparence de l’indicateur')} help="indicator" intro={osName === 'macos' ? t('L’icône de la barre des menus change toujours de couleur. La pastille à l’écran (bêta) ajoute l’état de la dictée, le niveau du micro et les alertes, sans jamais prendre le focus.') : osName === 'linux' ? t('Il n’y a pas encore d’icône sous Linux. La pastille à l’écran (bêta) ajoute l’état de la dictée, le niveau du micro et les alertes, sans jamais prendre le focus.') : t('L’icône près de l’horloge change toujours de couleur. La pastille à l’écran (bêta) ajoute l’état de la dictée, le niveau du micro et les alertes, sans jamais prendre le focus.')}>
    <div class="row">
      <label><input type="checkbox" checked={v.overlayEnabled} onchange={(e) => settings.change({ overlayEnabled: e.currentTarget.checked })} />
        {t('Afficher la pastille à l’écran (bêta)')}</label>
    </div>
    <div class="row">
      <label>{t('Style :')}
        <select value={v.captureStyle} onchange={(e) => settings.change({ captureStyle: e.currentTarget.value as never })}>
          {#each styles as o (o.id)}<option value={o.id}>{t(o.label)}</option>{/each}
        </select>
      </label>
      <label>{t('Position :')}
        <select value={v.capturePosition} onchange={(e) => settings.change({ capturePosition: e.currentTarget.value as never })}>
          {#each positions as o (o.id)}<option value={o.id}>{t(o.label)}</option>{/each}
        </select>
      </label>
      <label>{t('Couleur :')}
        <select value={v.captureColor} onchange={(e) => settings.change({ captureColor: e.currentTarget.value as never })}>
          {#each colors as o (o.id)}<option value={o.id}>{t(o.label)}</option>{/each}
        </select>
      </label>
    </div>
    <div class="row">
      <label>{t('Transparence :')}
        <input type="range" min="0.3" max="1" step="0.05" value={v.captureOpacity} onchange={(e) => settings.change({ captureOpacity: Number(e.currentTarget.value) })} />
        {Math.round(v.captureOpacity * 100)} %
      </label>
      <button onclick={() => api('overlay/preview', {})} disabled={!v.overlayEnabled}>{t('Aperçu')}</button>
    </div>
  </Card>
  {/if}

  {#if section === 'engines'}
  <Card id="cles" title={t('Clés du cloud (dépannage)')} help="privacy" intro={t('Seulement pour les services en ligne : une clé est votre identifiant chez Deepgram ou OpenRouter. Avec Whisper et un modèle local, tout reste sur cet ordinateur et aucune clé n’est utile.')}>
    <p class="muted">{osName === 'macos' ? t('Les clés sont gardées dans le trousseau du Mac, jamais affichées ni écrites dans le journal.') : osName === 'linux' ? t('Sous Linux, Scriibe ne range pas les clés : elles se donnent par des variables d’environnement, avant de lancer Scriibe. Elles ne sont jamais affichées ni écrites dans le journal.') : t('Les clés sont gardées dans le Gestionnaire d’identifiants de Windows, jamais affichées ni écrites dans le journal.')}</p>
    {#each [{ id: 'deepgram', label: 'Deepgram' }, { id: 'openrouter', label: 'OpenRouter' }] as k (k.id)}
      {@const id = k.id as 'deepgram' | 'openrouter'}
      <div class="row">
        <b style="min-width: 7em">{k.label}</b>
        {#if s.keys[id]}<span class="ok">{t('✔ présente')}</span>{:else}<span class="muted">{t('absente')}</span>{/if}
        <input type="password" class="grow" placeholder={t('Colle la clé ici')} autocomplete="off" bind:value={keyInput[id]} />
        <button onclick={() => saveKey(id)} disabled={!keyInput[id]}>{t('Enregistrer')}</button>
      </div>
    {/each}
  </Card>
  {/if}

  {#if section === 'engines'}
  <Card help="privacy" id="reglages-emplacements" title={t('Emplacements')}>
    <div class="muted">{t('Réglages : {path}', { path: s.paths.settings })}<br />{t('Modèles : {path}', { path: s.paths.models })}<br />{t('Journal : {path}', { path: s.paths.log })}</div>
  </Card>
  {/if}
{:else}
  <p class="muted">{t('Chargement…')}</p>
{/if}

<style>
  /* Une ligne de réglage comme dans Scribe : le libellé à gauche, la commande à droite, un filet entre deux lignes. */
  .setting { display: flex; align-items: center; gap: 10px; padding: 9px 0; border-top: 1px solid var(--line); }
  .setting label { flex: 1; display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .setting label:has(input[type='checkbox']) { justify-content: flex-start; }
  .settings-group { display: contents; }
  .summary { margin: 4px 0 8px; padding: 6px 10px; border: 1px solid var(--line); border-radius: 8px; background: var(--bg); }
</style>
