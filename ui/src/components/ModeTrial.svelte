<script lang="ts">
  import { api } from '../lib/api';

  interface Row {
    output: string;
    sent: string | null;
    seconds: number;
    kept_words: number;
    own_words: number;
    accepted: boolean;
    checks: { label: string; ok: boolean }[];
  }

  /** Le mode essayé (identifiant du mode, nom), les phrases, ce qui est rendu pour chacune. */
  let { choice, label, onclose }: { choice: string; label: string; onclose: () => void } = $props();

  let samples = $state<string[]>([]);
  let rows = $state<Record<number, Row>>({});
  let running = $state(true);
  let error = $state('');
  let cancelled = false;

  const pct = (x: number) => Math.round(x * 100);

  /** Les phrases l'une après l'autre : en local, les lancer ensemble fausserait les temps. */
  async function run() {
    samples = (await api<{ samples: string[] }>('trial/samples')).samples;
    for (let i = 0; i < samples.length && !cancelled; i++) {
      const out = await api<Row & { ok: boolean; error?: string }>('trial/sample', { index: i, mode: choice });
      if (cancelled) break;
      if (!out.ok) {
        error = out.error ?? 'échec';
        break;
      }
      rows[i] = out;
    }
    running = false;
  }

  $effect(() => {
    void run();
    return () => (cancelled = true);
  });

  let done = $derived(Object.keys(rows).length);
  let rejected = $derived(Object.values(rows).filter((r) => !r.accepted).length);
  let failedChecks = $derived(Object.values(rows).reduce((n, r) => n + r.checks.filter((c) => !c.ok).length, 0));
</script>

<section class="card trial">
  <div class="row">
    <h2>Essai : {label}</h2>
    <span class="grow"></span>
    <button onclick={onclose}>{running ? 'Arrêter' : 'Fermer'}</button>
  </div>
  <p class="muted">
    {running ? `Essai en cours : ${done} / ${samples.length || 12} phrases` : `${done} phrases essayées · ${rejected} écartée(s) par le garde-fou · ${failedChecks} contrôle(s) échoué(s)`}
  </p>
  {#if error}<p class="bad">Échec : {error}</p>{/if}
  {#each samples as sample, i (i)}
    {@const r = rows[i]}
    <div class="sample">
      <div class="muted">{i + 1}. {sample}</div>
      {#if r}
        {#if r.sent}<div class="muted">envoyé au modèle : {r.sent}</div>{/if}
        <div class="out">{r.output}</div>
        <div class="muted">
          {r.seconds.toFixed(1)} s · dictée reprise {pct(r.kept_words)} % · sortie tirée de la dictée {pct(r.own_words)} % ·
          <b class={r.accepted ? 'ok' : 'bad'}>{r.accepted ? 'gardée' : 'écartée'}</b>
          {#each r.checks as c (c.label)}<span class={c.ok ? 'ok' : 'bad'}> · {c.ok ? '✔' : '✘'} {c.label}</span>{/each}
        </div>
      {/if}
    </div>
  {/each}
</section>

<style>
  .trial { border-color: var(--accent); }
  .sample { padding: 8px 0; border-bottom: 1px solid var(--line); }
  .sample:last-child { border-bottom: 0; }
  .out { margin: 2px 0; }
</style>
