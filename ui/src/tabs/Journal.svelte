<script lang="ts">
  import Card from '../components/Card.svelte';
  import HelpButton from '../components/HelpButton.svelte';
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import { bench } from '../lib/bench.svelte';
  import { buildReport } from '../lib/report';
  import { status } from '../lib/status.svelte';
  import { keyName, t, tx } from '../lib/i18n.svelte';

  /** `live` : le journal en direct ; `report` : le rapport à coller (la page Mesures les sépare par d'autres cartes). */
  let { part }: { part: 'live' | 'report' } = $props();

  let lines = $state<string[]>([]);
  let report = $state('');
  let copied = $state('');
  let logBox: HTMLPreElement | undefined = $state();

  async function loadLog(): Promise<string[]> {
    const atEnd = !logBox || logBox.scrollTop + logBox.clientHeight >= logBox.scrollHeight - 20;
    lines = (await api<{ lines: string[] }>('log')).lines;
    if (atEnd && logBox) queueMicrotask(() => logBox && (logBox.scrollTop = logBox.scrollHeight));
    return lines;
  }

  async function copy() {
    await status.refresh();
    const s = status.value;
    if (!s) return;
    const log = await loadLog();
    const devices = s.llama.binary ? (await api<{ lines: string[] }>('llama/devices')).lines : [];
    report = buildReport(s, bench.results, devices, log);
    try {
      await navigator.clipboard.writeText(report);
      copied = t('Copié : colle-le dans la conversation.');
    } catch {
      copied = t('Copie refusée : sélectionne le texte ci-dessous.');
    }
  }

  onMount(() => {
    if (part !== 'live') return;
    void loadLog();
    const timer = setInterval(() => void loadLog(), 2000);
    return () => clearInterval(timer);
  });
</script>

{#if part === 'live'}
<Card help="problems" id="journal-dictee-avec-la-touche-dans-une-vraie-app" title={t('Dictée avec la touche, dans une vraie appli')}>
  {@const app = t(status.os === 'macos' ? tx('TextEdit') : status.os === 'linux' ? tx('un éditeur de texte') : tx('le Bloc-notes'))}
  {@const key = keyName(status.value?.dictation_label ?? '')}
  {#if status.value?.translate_label}
    <p>{t('Ouvre {app}, maintiens {key}, parle, relâche. {translate} en plus : traduction. Toute autre touche pendant l’appui : annulation.', { app, key, translate: keyName(status.value.translate_label) })}</p>
  {:else}
    <p>{t('Ouvre {app}, maintiens {key}, parle, relâche. Toute autre touche pendant l’appui : annulation.', { app, key })}</p>
  {/if}
  <div class="muted">{t('Journal en direct (une ligne par étape, jamais le texte dicté) :')}</div>
  <pre bind:this={logBox}>{lines.join('\n') || t('(vide)')}</pre>
</Card>
{:else}
<Card id="journal-rapport-a-me-coller" title={t('Rapport à me coller')} help="measures">
  <div class="row"><button class="primary" onclick={copy}>{t('Copier le rapport')}</button><span class="muted">{copied}</span></div>
  <pre>{report || t('Fais quelques essais, puis clique sur « Copier le rapport ».')}</pre>
</Card>
{/if}
