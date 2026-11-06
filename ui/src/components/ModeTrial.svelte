<script lang="ts">
  import { api } from '../lib/api';
  import { t } from '../lib/i18n.svelte';
  import Card from './Card.svelte';

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
        error = out.error ?? t('échec');
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

<Card id="essai-mode" accent title={t('Essai : {label}', { label })}>
  <div class="row">
    <span class="grow"></span>
    <button onclick={onclose}>{running ? t('Arrêter') : t('Fermer')}</button>
  </div>
  <p class="muted">
    {running ? t('Essai en cours : {done} / {total} phrases', { done, total: samples.length || 12 }) : t('{done} phrases essayées · {rejected} écartée(s) par le garde-fou · {failed} contrôle(s) échoué(s)', { done, rejected, failed: failedChecks })}
  </p>
  {#if error}<p class="bad">{t('Échec : {error}', { error })}</p>{/if}
  {#each samples as sample, i (i)}
    {@const r = rows[i]}
    <div class="sample">
      <div class="muted">{i + 1}. {sample}</div>
      {#if r}
        {#if r.sent}<div class="muted">{t('envoyé au modèle : {text}', { text: r.sent })}</div>{/if}
        <div class="out">{r.output}</div>
        <div class="muted">
          {t('{s} s · dictée reprise {kept} % · sortie tirée de la dictée {own} % ·', { s: r.seconds.toFixed(1), kept: pct(r.kept_words), own: pct(r.own_words) })}
          <b class={r.accepted ? 'ok' : 'bad'}>{r.accepted ? t('gardée') : t('écartée')}</b>
          {#each r.checks as c (c.label)}<span class={c.ok ? 'ok' : 'bad'}> · {c.ok ? '✔' : '✘'} {t(c.label)}</span>{/each}
        </div>
      {/if}
    </div>
  {/each}
</Card>

<style>
  .sample { padding: 8px 0; border-bottom: 1px solid var(--line); }
  .sample:last-child { border-bottom: 0; }
  .out { margin: 2px 0; }
</style>
