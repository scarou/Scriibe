<script lang="ts">
  import Card from '../components/Card.svelte';
  import { confirmBox } from '../lib/dialog.svelte';
  import HelpButton from '../components/HelpButton.svelte';
  import { onMount } from 'svelte';
  import { api, type CustomMode, type Modes } from '../lib/api';
  import { settings } from '../lib/settings.svelte';
  import { t } from '../lib/i18n.svelte';
  import ModeTrial from '../components/ModeTrial.svelte';
  import { watchModes } from '../lib/modesSync';

  let modes = $state<Modes | null>(null);
  let editing = $state<CustomMode | null>(null);
  let styleDrafts = $state<Record<string, string>>({});
  /** L'essai sur les phrases types en cours ou fini : le mode essayé. */
  let trial = $state<{ choice: string; label: string; key: number } | null>(null);
  let trialKey = 0;

  function startTrial(choice: string, label: string) {
    trial = { choice, label, key: ++trialKey };
  }

  const fidelities: { id: CustomMode['fidelity']; label: () => string; help: () => string }[] = [
    { id: 'literal', label: () => t('Littéral'), help: () => t('Garde au moins 90 % de tes mots : seules la ponctuation et de petites corrections sont acceptées.') },
    { id: 'light', label: () => t('Léger'), help: () => t('Garde au moins 75 % de tes mots (dictées de 8 mots ou plus), comme le mode Simple.') },
    { id: 'reformulated', label: () => t('Reformulé'), help: () => t('Reformule les phrases : le texte doit rester proche de ta dictée, mais les mots peuvent changer.') },
    { id: 'free', label: () => t('Libre'), help: () => t('Réécriture libre : la part de mots gardés n’est plus contrôlée (longueur, question, signes et formules de réponse le restent).') },
  ];
  function fidelityLabel(id: CustomMode['fidelity']) {
    return fidelities.find((f) => f.id === id)?.label() ?? id;
  }
  let v = $derived(settings.value);
  let simpleFidelity = $derived(((v?.simpleFidelity as string | undefined) ?? 'light') as CustomMode['fidelity']);
  let usable = $derived(!!v && v.cleanupEngine !== 'none');
  let custom = $derived((v?.customModes as CustomMode[] | undefined) ?? []);
  let overrides = $derived((v?.styleOverrides as Record<string, string> | undefined) ?? {});

  let hidden = $derived((v?.hiddenModes as string[] | undefined) ?? []);
  let menuIds = $derived([...(modes?.groups.flat() ?? []), ...custom.map((m) => m.id)]);
  const shown = (id: string) => !hidden.includes(id);
  /** Le dernier mode affiché ne se décoche pas : le menu en garde toujours un. */
  const lastShown = (id: string) => shown(id) && menuIds.filter(shown).length <= 1;
  async function showInMenu(id: string, on: boolean) {
    await settings.change({ hiddenModes: on ? hidden.filter((h) => h !== id) : [...hidden, id] });
  }

  async function load() {
    modes = await api<Modes>('modes');
  }

  async function choose(choice: string) {
    modes = await api<Modes>('modes/choose', { choice });
    await settings.load();
  }

  async function saveStyle(id: string, text: string) {
    const next = { ...overrides };
    const original = modes?.built_in.find((m) => m.id === id)?.original_style ?? '';
    if (text.trim() === '' || text.trim() === original.trim()) delete next[id];
    else next[id] = text;
    await settings.change({ styleOverrides: next });
    delete styleDrafts[id];
    await load();
  }

  function newMode() {
    editing = { id: crypto.randomUUID().toUpperCase(), name: '', style: '', fidelity: 'light', vouvoie: false, examples: [] };
  }

  async function saveCustom() {
    if (!editing) return;
    const list = custom.some((m) => m.id === editing!.id) ? custom.map((m) => (m.id === editing!.id ? editing! : m)) : [...custom, editing];
    if (await settings.change({ customModes: list })) {
      editing = null;
      await load();
    }
  }

  /** L'export d'un mode : aperçu et avertissement avant que le serveur ne demande où enregistrer. */
  let exporting = $state<{ mode: CustomMode; examples: boolean; busy: boolean; result: string } | null>(null);

  async function exportMode() {
    if (!exporting) return;
    exporting.busy = true;
    exporting.result = '';
    const out = await api<{ ok: boolean; saved?: boolean; path?: string; error?: string }>('modes/export', {
      id: exporting.mode.id,
      examples: exporting.examples,
    });
    exporting.busy = false;
    exporting.result = !out.ok
      ? t(out.error ?? 'enregistrement impossible : l’emplacement choisi n’est pas accessible en écriture')
      : out.saved
        ? t('Enregistré : {path}', { path: out.path ?? '' })
        : t('Export annulé : rien n’a été enregistré.');
  }

  /** L'import d'un mode : le serveur lit et contrôle le fichier, la page montre l'aperçu, rien n'est écrit avant l'accord. */
  type ImportItem = { name: string; fidelity: CustomMode['fidelity']; examples: number; conflict: string | null; added_as: string };
  let importing = $state<{
    items: ImportItem[];
    token: string;
    choices: { include: boolean; replace: boolean }[];
    askPath: boolean;
    path: string;
    busy: boolean;
    message: string;
    done: boolean;
  } | null>(null);

  async function readImport(path?: string) {
    editing = null;
    exporting = null;
    if (!importing) importing = { items: [], token: '', choices: [], askPath: false, path: '', busy: true, message: '', done: false };
    importing.busy = true;
    importing.message = '';
    const out = await api<{ ok: boolean; cancelled?: boolean; ask_path?: boolean; error?: string; token?: string; items?: ImportItem[] }>('modes/import/read', path ? { path } : {});
    if (!importing) return;
    importing.busy = false;
    if (out.ok && out.cancelled) importing = null;
    else if (out.ok && out.items) {
      importing.askPath = false;
      importing.items = out.items;
      importing.token = out.token ?? '';
      importing.choices = out.items.map(() => ({ include: true, replace: false }));
    } else if (out.ask_path) importing.askPath = true;
    else importing.message = t(out.error ?? "le fichier n'a pas pu être lu");
  }

  async function applyImport() {
    if (!importing) return;
    importing.busy = true;
    const out = await api<{ ok: boolean; added?: string[]; replaced?: string[]; error?: string }>('modes/import/apply', { token: importing.token, choices: importing.choices });
    importing.busy = false;
    if (!out.ok) {
      importing.message = t(out.error ?? "le fichier n'a pas pu être lu");
      return;
    }
    importing.items = [];
    importing.done = true;
    importing.message = t('Importé : {added} ajouté(s), {replaced} remplacé(s).', { added: out.added?.length ?? 0, replaced: out.replaced?.length ?? 0 });
    // Le serveur a écrit les réglages : la page les relit, pour ne pas les écraser plus tard.
    await settings.load();
    await load();
  }

  async function closeImport() {
    if (importing && !importing.done) await api('modes/import/cancel', {});
    importing = null;
  }

  async function removeCustom(mode: CustomMode) {
    if (!(await confirmBox(t('Supprimer le mode « {name} » ?', { name: mode.name || t('sans nom') }), { confirm: t('Supprimer'), danger: true }))) return;
    await settings.change({ customModes: custom.filter((m) => m.id !== mode.id) });
    await load();
  }

  onMount(() => {
    void load();
    void settings.load();
    // Le mode changé dans le menu de l'icône apparaît ici sans recharger la page.
    const stop = watchModes(
      () => api<Modes>('modes'),
      (latest) => {
        modes = latest as Modes;
        void settings.load();
      },
      1500,
      () => modes?.choice ?? '',
    );
    return stop;
  });
</script>

{#if modes}
  <p class="muted">{t('Après la transcription, un modèle d’IA retire les « euh » et ponctue, selon le mode (en Littéral, sans modèle : seulement les signes dictés et le dictionnaire). En Simple, « euh, je pense qu’on pourrait, euh, décaler à jeudi » devient « Je pense qu’on pourrait décaler à jeudi. » ; en Soutenu, la phrase est en plus reformulée dans un français soigné.')}</p>
  <p class="muted">{t('Le mode règle le ton de la mise au propre. Mode en cours :')} <b>{t(modes.active_label)}</b>. {t('La consigne de base (ne jamais répondre, garder les personnes) et le garde-fou ne changent pas.')}</p>

  <p class="muted">{t('Chaque mode a une case « Afficher dans le menu de l’icône » : décoché, il disparaît de ce menu (même s’il est en cours) mais reste dans cette page. Au moins un mode reste affiché.')}</p>

  {#if trial}
    {#key trial.key}<ModeTrial choice={trial.choice} label={trial.label} onclose={() => (trial = null)} />{/key}
  {/if}

  {#each modes.groups as group, i (group.join())}
    <Card id={'modes-groupe-' + i} title={i === 0 ? t('Modes de base') : t('Modes professionnels')} help="modes">
      {#each group as id (id)}
        {@const m = modes.built_in.find((x) => x.id === id)!}
        <div class="mode">
          <label class="pick">
            <input type="radio" name="mode" checked={modes.choice === m.id} disabled={!m.available} onchange={() => choose(m.id)} />
            <span><b>{t(m.label)}</b>{#if !m.available}<span class="tag">{t('bientôt : traité comme Simple')}</span>{/if}<br /><span class="muted">{t(m.summary)}</span></span>
          </label>
          <label class="muted menu-box"><input type="checkbox" checked={shown(m.id)} disabled={lastShown(m.id)} onchange={(e) => showInMenu(m.id, e.currentTarget.checked)} /> {t('Afficher dans le menu de l’icône')}</label>
          {#if m.id !== 'literal'}
            <button onclick={() => startTrial(m.id, t(m.label))} disabled={!usable} title={usable ? '' : t('Choisis d’abord un moteur de mise au propre')}>{t('Essayer sur les phrases types')}</button>
          {/if}
          {#if m.id === 'simple'}
            <label class="muted">{t('Fidélité :')}
              <select value={simpleFidelity} onchange={(e) => settings.change({ simpleFidelity: e.currentTarget.value })}>
                {#each fidelities as f (f.id)}<option value={f.id}>{f.label()}</option>{/each}
              </select></label>
          {/if}
          {#if m.has_style}
            <textarea rows="2" class="style" value={styleDrafts[m.id] ?? m.typed_style}
              oninput={(e) => (styleDrafts[m.id] = e.currentTarget.value)} onchange={(e) => saveStyle(m.id, e.currentTarget.value)}></textarea>
            {#if m.modified}<button onclick={() => saveStyle(m.id, '')}>{t('Rétablir la consigne d’origine')}</button>{/if}
          {/if}
        </div>
      {/each}
    </Card>
  {/each}

  <Card id="modes-mes-modes" title={t('Mes modes')} help="modes" intro={t('Un mode, c’est une consigne et quelques exemples donnés au modèle. Pour un usage précis (des SMS, des comptes rendus de séance…), créez votre mode avec deux ou trois exemples dictée → texte attendu : ce sont eux qui donnent le style.')}>
    {#each custom as m (m.id)}
      <div class="mode">
        <label class="pick">
          <input type="radio" name="mode" checked={modes.choice === m.id} onchange={() => choose(m.id)} />
          <span><b>{m.name || t('Mode sans nom')}</b><br /><span class="muted">{fidelityLabel(m.fidelity)} · {t('{n} exemple(s)', { n: m.examples.length })}</span></span>
        </label>
        <label class="muted menu-box"><input type="checkbox" checked={shown(m.id)} disabled={lastShown(m.id)} onchange={(e) => showInMenu(m.id, e.currentTarget.checked)} /> {t('Afficher dans le menu de l’icône')}</label>
        <button onclick={() => startTrial(m.id, m.name || t('Mode sans nom'))} disabled={!usable}>{t('Essayer sur les phrases types')}</button>
        <button onclick={() => (editing = structuredClone($state.snapshot(m)))}>{t('Modifier')}</button>
        <button onclick={() => (exporting = { mode: structuredClone($state.snapshot(m)), examples: true, busy: false, result: '' })}>{t('Exporter')}</button>
        <button onclick={() => removeCustom(m)}>{t('Supprimer')}</button>
      </div>
    {:else}
      <p class="muted">{t('Aucun mode personnel.')}</p>
    {/each}
    <button onclick={newMode}>{t('Créer un mode')}</button>
    <button onclick={() => readImport()} disabled={importing !== null}>{t('Importer un mode…')}</button>
  </Card>

  {#if importing}
    <Card id="mode-import" accent title={t('Importer des modes')} help="modes">
      {#if importing.askPath}
        <p>{t('Sur ce système, indique le chemin du fichier .scriibe.json à importer :')}</p>
        <div class="row"><input type="text" class="grow" bind:value={importing.path} placeholder="/home/…/scriibe-mode-sms.scriibe.json" disabled={importing.busy} />
          <button class="primary" disabled={importing.busy || !importing.path.trim()} onclick={() => readImport(importing!.path)}>{t('Lire le fichier')}</button></div>
      {:else if importing.items.length > 0}
        <p>{t('Ce fichier contient {n} mode(s). Rien n’est modifié tant que tu n’as pas cliqué sur « Importer ». Un mode importé est ajouté à tes modes ; la consigne est un texte donné au modèle : relis-la si le fichier vient de quelqu’un d’autre.', { n: importing.items.length })}</p>
        {#each importing.items as item, i (i)}
          <div class="mode">
            <label class="pick">
              <input type="checkbox" bind:checked={importing.choices[i].include} disabled={importing.busy} />
              <span><b>{item.name}</b><br /><span class="muted">{fidelityLabel(item.fidelity)} · {t('{n} exemple(s)', { n: item.examples })}</span></span>
            </label>
            {#if item.conflict && importing.choices[i].include}
              <label class="muted">{t('Un mode porte déjà ce nom :')}
                <select value={importing.choices[i].replace ? 'replace' : 'add'} disabled={importing.busy}
                  onchange={(e) => (importing!.choices[i].replace = e.currentTarget.value === 'replace')}>
                  <option value="add">{t('l’ajouter sous le nom « {name} »', { name: item.added_as })}</option>
                  <option value="replace">{t('remplacer le mode « {name} » existant', { name: item.conflict })}</option>
                </select></label>
              {#if importing.choices[i].replace}<p class="warn">{t('Remplacer efface la consigne et les exemples actuels de ce mode.')}</p>{/if}
            {/if}
          </div>
        {/each}
      {:else if importing.busy}
        <p class="muted">{t('Chargement…')}</p>
      {/if}
      {#if importing.message}<p role="status" class:warn={!importing.done}><b>{importing.message}</b></p>{/if}
      <div class="row">
        <span class="grow"></span>
        <button onclick={closeImport}>{importing.done ? t('Fermer') : t('Annuler')}</button>
        {#if importing.items.length > 0}
          <button class="primary" disabled={importing.busy || !importing.choices.some((c) => c.include)} onclick={applyImport}>{t('Importer')}</button>
        {/if}
      </div>
    </Card>
  {/if}

  {#if exporting}
    <Card id="mode-export" accent title={t('Exporter le mode « {name} »', { name: exporting.mode.name || t('Mode sans nom') })} help="modes">
      <p>{t('Le fichier (.scriibe.json) contient ce mode : sa consigne, son cran de fidélité et, si la case est cochée, ses exemples. Une boîte te demande où l’enregistrer (sous Linux, il est placé dans le dossier Documents).')}</p>
      <p><b>{t('Contenu : 1 mode, {n} exemple(s)', { n: exporting.examples ? exporting.mode.examples.length : 0 })}</b></p>
      {#if exporting.mode.examples.length > 0}
        <div class="row"><label><input type="checkbox" bind:checked={exporting.examples} disabled={exporting.busy} /> {t('Inclure les exemples')}</label></div>
      {/if}
      <p class="warn">{t('Attention : la consigne et les exemples (dictée → texte attendu) peuvent contenir du texte personnel : noms, adresses, phrases réellement dictées. Le fichier n’est pas chiffré : relis-le avant de le partager.')}</p>
      {#if exporting.result}<p role="status"><b>{exporting.result}</b></p>{/if}
      <div class="row">
        <span class="grow"></span>
        <button onclick={() => (exporting = null)}>{t('Fermer')}</button>
        <button class="primary" disabled={exporting.busy} onclick={exportMode}>{t('Enregistrer le fichier')}</button>
      </div>
    </Card>
  {/if}

  {#if editing}
    <Card id="mode-edition" accent title={custom.some((m) => m.id === editing!.id) ? t('Modifier le mode') : t('Nouveau mode')} help="modes">
      <div class="row"><label>{t('Nom :')} <input type="text" bind:value={editing.name} style="width: 22em" /></label></div>
      <div class="row"><label style="flex: 1">{t('Consigne de style :')}<br />
        <textarea rows="3" style="width: 100%" bind:value={editing.style} placeholder={t('Écris comme un ingénieur : phrases courtes, précises.')}></textarea></label></div>
      <div class="row"><label>{t('Fidélité à ta dictée :')}
        <select bind:value={editing.fidelity}>
          {#each fidelities as f (f.id)}<option value={f.id}>{f.label()}</option>{/each}
        </select></label></div>
      <p class="muted">{fidelities.find((f) => f.id === editing!.fidelity)?.help()}</p>
      {#if editing.fidelity === 'reformulated' || editing.fidelity === 'free'}
        <div class="row"><label><input type="checkbox" bind:checked={editing.vouvoie} /> {t('Vouvoie le destinataire')}</label></div>
      {/if}
      <h2>{t('Exemples')}</h2>
      <p class="muted">{t('Ce sont eux qui donnent le style au modèle : une dictée, puis le texte attendu.')}</p>
      {#each editing.examples as ex, i (ex.id)}
        <div class="row">
          <textarea rows="2" class="grow" placeholder={t('Dictée')} bind:value={ex.dictation}></textarea>
          <textarea rows="2" class="grow" placeholder={t('Texte attendu')} bind:value={ex.expected}></textarea>
          <button onclick={() => (editing!.examples = editing!.examples.filter((_, j) => j !== i))}>×</button>
        </div>
      {/each}
      <div class="row">
        <button onclick={() => editing!.examples.push({ id: crypto.randomUUID().toUpperCase(), dictation: '', expected: '' })}>{t('Ajouter un exemple')}</button>
        <span class="grow"></span>
        <button onclick={() => (editing = null)}>{t('Annuler')}</button>
        <button class="primary" onclick={saveCustom}>{t('Enregistrer')}</button>
      </div>
    </Card>
  {/if}
{:else}
  <p class="muted">{t('Chargement…')}</p>
{/if}

<style>
  .mode { display: flex; flex-wrap: wrap; gap: 8px; align-items: flex-start; padding: 8px 0; border-bottom: 1px solid var(--line); }
  .mode:last-child { border-bottom: 0; }
  .pick { display: flex; gap: 10px; align-items: flex-start; flex: 1 1 320px; cursor: pointer; }
  .style { width: 100%; }
  .warn { color: var(--warn); }
  .menu-box { display: flex; gap: 6px; align-items: center; white-space: nowrap; }
</style>
