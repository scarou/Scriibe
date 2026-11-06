<script lang="ts">
  import { notice } from '../lib/dialog.svelte';
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import { missing, needLabels, type Assistant } from '../lib/assistant';
  import { settings } from '../lib/settings.svelte';
  import { status } from '../lib/status.svelte';
  import { t } from '../lib/i18n.svelte';
  import Card from './Card.svelte';

  let { onclose, ongoto }: { onclose: () => void; ongoto: (tab: 'install' | 'settings') => void } = $props();

  let data = $state<Assistant | null>(null);
  let choice = $state('');
  let applied = $state(false);
  let s = $derived(status.value);
  let chosen = $derived(data?.chains.find((c) => c.id === choice));
  let lacks = $derived(chosen && s ? missing(chosen, s) : null);

  async function apply() {
    const out = await api('assistant/apply', { chain: choice });
    if (!out.ok) {
      await notice(out.error);
      return;
    }
    applied = true;
    await settings.load();
    await status.refresh();
  }

  async function skip() {
    await api('assistant/skip', {});
    onclose();
  }

  onMount(async () => {
    data = await api<Assistant>('assistant');
    choice = data.recommended;
  });
</script>

<Card id="assistant-premier-lancement" accent title={t('Assistant de premier lancement')}>
  {#if !data}
    <p class="muted">{t('Mesure de l’ordinateur…')}</p>
  {:else}
    <p>
      {t('Votre ordinateur :')} <b>{t('{n} cœurs', { n: data.machine.physical_cores })}</b>, <b>{t('{n} Go', { n: data.machine.ram_gb })}</b> {t('de mémoire')}{#if data.machine.gpus.length},
        {#each data.machine.gpus as g, i (g.name)}{i ? ' + ' : ''}{g.name}{g.vram_gb ? ` (${t('{n} Go', { n: g.vram_gb })})` : ''}{/each}{:else if !data.machine.gpus_known}
        ; {t('cartes graphiques non détectées')}{/if}.
      {#if data.machine.capable_gpu}{t('Une vraie carte graphique est présente.')}{:else}{t('Pas de carte graphique assez puissante : on calcule sur le processeur.')}{/if}
    </p>

    {#if !applied}
      <p class="muted">{t('Voici les chaînes possibles. Les temps annoncés sont ceux qu’on a mesurés ; ce qui n’a pas été mesuré est dit.')}</p>
      {#each data.chains as c (c.id)}
        <label class="chain" class:picked={choice === c.id}>
          <input type="radio" name="chain" value={c.id} bind:group={choice} />
          <span>
            <b>{t(c.title)}</b>{#if c.id === data.recommended}<span class="tag ok">{t('conseillée')}</span>{/if}<br />
            <span class="muted">{t(c.detail)}</span><br />
            <span class="timing">⏱ {t(c.timing)}</span>
          </span>
        </label>
      {/each}
      <div class="row">
        <button class="primary" onclick={apply} disabled={!choice}>{t('Appliquer ce choix')}</button>
        <button onclick={skip}>{t('Garder mes réglages actuels')}</button>
      </div>
    {:else if chosen && lacks}
      <p class="ok">{t('✔ Réglages appliqués : « {title} ».', { title: t(chosen.title) })}</p>
      {#if lacks.items.length || lacks.key}
        <p>{t('Il reste à faire :')}</p>
        <ul>
          {#each lacks.items as id (id)}<li>{t('installer {item}', { item: t(needLabels[id] ?? id) })}</li>{/each}
          {#if lacks.key}<li>{t('enregistrer votre clé OpenRouter (onglet Réglages)')}</li>{/if}
        </ul>
        <div class="row">
          {#if lacks.items.length}<button class="primary" onclick={() => ongoto('install')}>{t('Aller à l’installation')}</button>{/if}
          {#if lacks.key}<button onclick={() => ongoto('settings')}>{t('Aller aux Réglages')}</button>{/if}
          <button onclick={onclose}>{t('Fermer')}</button>
        </div>
      {:else}
        <p>{t('Tout est en place : maintenez Ctrl droit, parlez, relâchez.')}</p>
        <div class="row"><button class="primary" onclick={onclose}>{t('Fermer')}</button></div>
      {/if}
    {/if}
  {/if}
</Card>

<style>
  .chain { display: flex; gap: 10px; align-items: flex-start; padding: 10px; border: 1px solid var(--line); border-radius: 10px; margin: 8px 0; cursor: pointer; }
  .chain.picked { border-color: var(--accent); background: var(--bg); }
  .timing { font-size: 0.85rem; }
  .tag.ok { margin-left: 8px; }
</style>
