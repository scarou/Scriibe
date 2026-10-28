<script lang="ts">
  import { confirmBox } from '../lib/dialog.svelte';
  import HelpButton from '../components/HelpButton.svelte';
  import { onMount } from 'svelte';
  import { api, type CustomMode, type Modes } from '../lib/api';
  import { settings } from '../lib/settings.svelte';
  import ModeTrial from '../components/ModeTrial.svelte';

  let modes = $state<Modes | null>(null);
  let editing = $state<CustomMode | null>(null);
  let styleDrafts = $state<Record<string, string>>({});
  /** L'essai sur les phrases types en cours ou fini : le mode essayé. */
  let trial = $state<{ choice: string; label: string; key: number } | null>(null);
  let trialKey = 0;

  function startTrial(choice: string, label: string) {
    trial = { choice, label, key: ++trialKey };
  }

  let v = $derived(settings.value);
  let usable = $derived(!!v && v.cleanupEngine !== 'none');
  let custom = $derived((v?.customModes as CustomMode[] | undefined) ?? []);
  let overrides = $derived((v?.styleOverrides as Record<string, string> | undefined) ?? {});

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
    editing = { id: crypto.randomUUID().toUpperCase(), name: '', style: '', rewrites: false, vouvoie: false, examples: [] };
  }

  async function saveCustom() {
    if (!editing) return;
    const list = custom.some((m) => m.id === editing!.id) ? custom.map((m) => (m.id === editing!.id ? editing! : m)) : [...custom, editing];
    if (await settings.change({ customModes: list })) {
      editing = null;
      await load();
    }
  }

  async function removeCustom(mode: CustomMode) {
    if (!(await confirmBox(`Supprimer le mode « ${mode.name || 'sans nom'} » ?`, { confirm: 'Supprimer', danger: true }))) return;
    await settings.change({ customModes: custom.filter((m) => m.id !== mode.id) });
    await load();
  }

  onMount(() => {
    void load();
    void settings.load();
  });
</script>

<h1>Modes</h1>

{#if modes}
  <p class="muted">Le mode règle le ton de la mise au propre. Mode en cours : <b>{modes.active_label}</b>. La consigne de base (ne jamais répondre, garder les personnes) et le garde-fou ne changent pas.</p>

  {#if trial}
    {#key trial.key}<ModeTrial choice={trial.choice} label={trial.label} onclose={() => (trial = null)} />{/key}
  {/if}

  {#each modes.groups as group (group.join())}
    <section class="card">
      {#each group as id (id)}
        {@const m = modes.built_in.find((x) => x.id === id)!}
        <div class="mode">
          <label class="pick">
            <input type="radio" name="mode" checked={modes.choice === m.id} disabled={!m.available} onchange={() => choose(m.id)} />
            <span><b>{m.label}</b>{#if !m.available}<span class="tag">bientôt : traité comme Simple</span>{/if}<br /><span class="muted">{m.summary}</span></span>
          </label>
          {#if m.id !== 'literal'}
            <button onclick={() => startTrial(m.id, m.label)} disabled={!usable} title={usable ? '' : 'Choisis d’abord un moteur de mise au propre'}>Essayer sur les phrases types</button>
          {/if}
          {#if m.has_style}
            <textarea rows="2" class="style" value={styleDrafts[m.id] ?? m.typed_style}
              oninput={(e) => (styleDrafts[m.id] = e.currentTarget.value)} onchange={(e) => saveStyle(m.id, e.currentTarget.value)}></textarea>
            {#if m.modified}<button onclick={() => saveStyle(m.id, '')}>Rétablir la consigne d’origine</button>{/if}
          {/if}
        </div>
      {/each}
    </section>
  {/each}

  <section class="card">
    <h2>Mes modes <HelpButton section="modes" /></h2>
    {#each custom as m (m.id)}
      <div class="mode">
        <label class="pick">
          <input type="radio" name="mode" checked={modes.choice === m.id} onchange={() => choose(m.id)} />
          <span><b>{m.name || 'Mode sans nom'}</b><br /><span class="muted">{m.rewrites ? 'Reformule' : 'Garde tes mots'} · {m.examples.length} exemple(s)</span></span>
        </label>
        <button onclick={() => startTrial(m.id, m.name || 'Mode sans nom')} disabled={!usable}>Essayer sur les phrases types</button>
        <button onclick={() => (editing = structuredClone($state.snapshot(m)))}>Modifier</button>
        <button onclick={() => removeCustom(m)}>Supprimer</button>
      </div>
    {:else}
      <p class="muted">Aucun mode personnel.</p>
    {/each}
    <button onclick={newMode}>Créer un mode</button>
  </section>

  {#if editing}
    <section class="card edit">
      <h2>{custom.some((m) => m.id === editing!.id) ? 'Modifier le mode' : 'Nouveau mode'}</h2>
      <div class="row"><label>Nom : <input type="text" bind:value={editing.name} style="width: 22em" /></label></div>
      <div class="row"><label style="flex: 1">Consigne de style :<br />
        <textarea rows="3" style="width: 100%" bind:value={editing.style} placeholder="Écris comme un ingénieur : phrases courtes, précises."></textarea></label></div>
      <div class="row"><label><input type="checkbox" bind:checked={editing.rewrites} /> Reformule les phrases (sinon garde tes mots, comme Simple)</label></div>
      {#if editing.rewrites}
        <div class="row"><label><input type="checkbox" bind:checked={editing.vouvoie} /> Vouvoie le destinataire</label></div>
      {/if}
      <h2>Exemples</h2>
      <p class="muted">Ce sont eux qui donnent le style au modèle : une dictée, puis le texte attendu.</p>
      {#each editing.examples as ex, i (ex.id)}
        <div class="row">
          <textarea rows="2" class="grow" placeholder="Dictée" bind:value={ex.dictation}></textarea>
          <textarea rows="2" class="grow" placeholder="Texte attendu" bind:value={ex.expected}></textarea>
          <button onclick={() => (editing!.examples = editing!.examples.filter((_, j) => j !== i))}>×</button>
        </div>
      {/each}
      <div class="row">
        <button onclick={() => editing!.examples.push({ id: crypto.randomUUID().toUpperCase(), dictation: '', expected: '' })}>Ajouter un exemple</button>
        <span class="grow"></span>
        <button onclick={() => (editing = null)}>Annuler</button>
        <button class="primary" onclick={saveCustom}>Enregistrer</button>
      </div>
    </section>
  {/if}
{:else}
  <p class="muted">Chargement…</p>
{/if}

<style>
  .mode { display: flex; flex-wrap: wrap; gap: 8px; align-items: flex-start; padding: 8px 0; border-bottom: 1px solid var(--line); }
  .mode:last-child { border-bottom: 0; }
  .pick { display: flex; gap: 10px; align-items: flex-start; flex: 1 1 320px; cursor: pointer; }
  .style { width: 100%; }
  .edit { border-color: var(--accent); }
</style>
