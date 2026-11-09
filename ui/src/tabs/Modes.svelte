<script lang="ts">
  import Card from '../components/Card.svelte';
  import { confirmBox } from '../lib/dialog.svelte';
  import HelpButton from '../components/HelpButton.svelte';
  import { onMount } from 'svelte';
  import { api, type CustomMode, type Modes } from '../lib/api';
  import { settings } from '../lib/settings.svelte';
  import { t } from '../lib/i18n.svelte';
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

  async function removeCustom(mode: CustomMode) {
    if (!(await confirmBox(t('Supprimer le mode « {name} » ?', { name: mode.name || t('sans nom') }), { confirm: t('Supprimer'), danger: true }))) return;
    await settings.change({ customModes: custom.filter((m) => m.id !== mode.id) });
    await load();
  }

  onMount(() => {
    void load();
    void settings.load();
  });
</script>

{#if modes}
  <p class="muted">{t('Après la transcription, un modèle d’IA retire les « euh » et ponctue, selon le mode (en Littéral, sans modèle : seulement les signes dictés et le dictionnaire). En Simple, « euh, je pense qu’on pourrait, euh, décaler à jeudi » devient « Je pense qu’on pourrait décaler à jeudi. » ; en Soutenu, la phrase est en plus reformulée dans un français soigné.')}</p>
  <p class="muted">{t('Le mode règle le ton de la mise au propre. Mode en cours :')} <b>{t(modes.active_label)}</b>. {t('La consigne de base (ne jamais répondre, garder les personnes) et le garde-fou ne changent pas.')}</p>

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
        <button onclick={() => startTrial(m.id, m.name || t('Mode sans nom'))} disabled={!usable}>{t('Essayer sur les phrases types')}</button>
        <button onclick={() => (editing = structuredClone($state.snapshot(m)))}>{t('Modifier')}</button>
        <button onclick={() => removeCustom(m)}>{t('Supprimer')}</button>
      </div>
    {:else}
      <p class="muted">{t('Aucun mode personnel.')}</p>
    {/each}
    <button onclick={newMode}>{t('Créer un mode')}</button>
  </Card>

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
</style>
