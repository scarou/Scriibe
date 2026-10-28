<script lang="ts">
  import HelpButton from '../components/HelpButton.svelte';
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import { bench } from '../lib/bench.svelte';
  import { buildReport } from '../lib/report';
  import { status } from '../lib/status.svelte';

  let lines = $state<string[]>([]);
  let report = $state('Faites quelques essais, puis cliquez sur « Copier le rapport ».');
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
      copied = 'Copié : collez-le dans la conversation.';
    } catch {
      copied = 'Copie refusée : sélectionnez le texte ci-dessous.';
    }
  }

  onMount(() => {
    void loadLog();
    const timer = setInterval(() => void loadLog(), 2000);
    return () => clearInterval(timer);
  });
</script>

<h1>Journal et rapport</h1>

<section class="card">
  <h2>Dictée avec la touche, dans une vraie appli</h2>
  <p>Ouvrez le Bloc-notes, <b>maintenez Ctrl droit</b>, parlez, relâchez. Maj en plus : traduction. Toute autre touche pendant l’appui : annulation.</p>
  <div class="muted">Journal en direct (une ligne par étape, jamais le texte dicté) :</div>
  <pre bind:this={logBox}>{lines.join('\n') || '(vide)'}</pre>
</section>

<section class="card">
  <h2>Rapport à me coller <HelpButton section="measures" /></h2>
  <div class="row"><button class="primary" onclick={copy}>Copier le rapport</button><span class="muted">{copied}</span></div>
  <pre>{report}</pre>
</section>
