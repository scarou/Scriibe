<script lang="ts" module>
  import type { CaptureColor, CapturePosition, CaptureStyle } from '../lib/appearance';

  export interface View {
    visible: boolean;
    state: 'hidden' | 'listening' | 'working' | 'done' | 'message';
    message: string | null;
    translation: string | null;
    mic: 'ok' | 'weak' | 'silent';
    level: number;
    /** Une erreur qui reste affichée jusqu'à la croix ou la dictée suivante (le genre choisit le titre) ; `null` sinon. */
    error?: { kind: 'not_recognised' | 'not_done' | 'dictation'; message: string } | null;
    /** « windows », « macos » ou « linux » : la couleur de la marque (absent dans les illustrations de l'Aide). */
    os?: string;
    appearance: { enabled: boolean; style: CaptureStyle; position: CapturePosition; color: CaptureColor; opacity: number };
  }
</script>

<script lang="ts">
  import { caption, gradient, listeningOpacity, listeningText } from '../lib/appearance';
  import { t } from '../lib/i18n.svelte';
  import { api } from '../lib/api';

  /** `inline` : posé dans une page (illustration de l'Aide) au lieu de remplir la fenêtre de la pastille. */
  let { view, levels = [], inline = false }: { view: View; levels?: number[]; inline?: boolean } = $props();

  let v = $derived(view);
  let fill = $derived(gradient(v.appearance.color));
  let note = $derived(caption(v.mic, v.translation, t));
  let align = $derived(v.appearance.position === 'left' ? 'flex-start' : v.appearance.position === 'right' ? 'flex-end' : 'center');
  let opacity = $derived(v.state === 'listening' ? listeningOpacity(v.mic, v.appearance.opacity) : v.state === 'message' ? 1 : v.appearance.opacity);
  let bars = [0.45, 0.8, 1, 0.7, 0.4];

  // Erreur qui reste : titre selon le genre, raison dessous (rien si elle répète le titre). Une erreur de dictée n'a pas de titre.
  let err = $derived(v.state === 'message' ? (v.error ?? null) : null);
  let headline = $derived(err?.kind === 'not_recognised' ? t('Commande non reconnue') : err?.kind === 'not_done' ? t('Commande non faite') : '');
  let detail = $derived(err ? t(err.message) : '');
  let showDetail = $derived(detail !== '' && detail !== headline);

  /** Mesure la capsule d'erreur et la dit à Scriibe : la fenêtre s'y réduit, et c'est alors seulement qu'elle accepte la souris. */
  function measure(node: HTMLElement) {
    if (inline) return {};
    let last = '';
    const send = () => {
      const r = node.getBoundingClientRect();
      const key = `${Math.ceil(r.width) + 1}x${Math.ceil(r.height) + 1}`;
      if (key === last) return;
      last = key;
      void api('overlay/shape', { width: Math.ceil(r.width) + 1, height: Math.ceil(r.height) + 1 }).catch(() => {});
    };
    const observer = new ResizeObserver(send);
    observer.observe(node);
    send();
    return { destroy: () => observer.disconnect() };
  }

  function dismiss() {
    void api('overlay/dismiss', {}).catch(() => {});
  }
</script>


{#if v.state !== 'hidden'}
  <div class="stage" class:inline class:failing={!!err && !inline} style="align-items: {align}">
    <div class="wrap" style="align-items: {align}; opacity: {opacity}">
      {#if err}
        <div class="capsule message failure" style="background: {fill}" use:measure>
          <span class="icon">{err.kind === 'not_recognised' ? '?' : '!'}</span>
          <span class="body">
            {#if headline}<span class="text">{headline}</span>{/if}
            {#if showDetail}<span class="detail" class:soft={!!headline}>{detail}</span>{/if}
          </span>
          <button class="close" type="button" aria-label={t('Fermer')} onclick={dismiss}>✕</button>
        </div>
      {:else if v.state === 'message'}
        <div class="capsule message" style="background: {fill}">
          <span class="icon">!</span><span class="text">{v.message ? t(v.message) : ''}</span>
        </div>
      {:else if v.appearance.style === 'pill'}
        <div class="capsule" style="background: {fill}">
          {#if v.state === 'listening'}
            <span class="bars">{#each bars as b, i (i)}<i style="height: {5 + 15 * b * Math.max(0.15, v.level)}px"></i>{/each}</span>
            <span class="text">{listeningText(v.mic, t)}</span>
            {#if v.translation}<span class="text soft">→ {t(v.translation)}</span>{/if}
          {:else if v.state === 'working'}
            <span class="spinner"></span><span class="text">{v.translation ? t('Traduction en {language}…', { language: t(v.translation) }) : t('Mise au propre…')}</span>
          {:else}
            <span class="icon">✓</span><span class="text">{v.translation ? t('Collé en {language}', { language: t(v.translation) }) : t('Collé')}</span>
          {/if}
        </div>
      {:else if v.appearance.style === 'dot'}
        {#if v.state === 'listening' && note}<div class="capsule compact above" style="background: {fill}"><span class="text">{note}</span></div>{/if}
        {#if v.state === 'listening'}
          <div class="dotbox"><div class="dot" style="background: {fill}; width: {14 + 12 * v.level}px; height: {14 + 12 * v.level}px"></div></div>
        {:else}
          <div class="dotbox"><div class="dot fixed" style="background: {fill}">{#if v.state === 'working'}<span class="spinner small"></span>{:else}<span class="icon small">✓</span>{/if}</div></div>
        {/if}
      {:else}
        {#if v.state === 'listening' && note}<div class="capsule compact above" style="background: {fill}"><span class="text">{note}</span></div>{/if}
        <div class="capsule" style="background: {fill}">
          {#if v.state === 'listening'}
            <span class="curve">{#each levels as l, i (i)}<i style="height: {Math.max(2, 24 * l)}px; opacity: {0.45 + (0.55 * i) / Math.max(1, levels.length)}"></i>{/each}</span>
          {:else if v.state === 'working'}
            <span class="spinner"></span>
          {:else}
            <span class="icon">✓</span>
          {/if}
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .stage { position: fixed; inset: 0; display: flex; flex-direction: column; justify-content: flex-end; padding: 12px; font: 600 13px/1 system-ui, 'Segoe UI', sans-serif; color: #fff; }
  /* Erreur qui reste : la fenêtre est réduite à la capsule (Scriibe la place à l'endroit d'une pastille ordinaire). */
  .stage.failing { padding: 0; }
  .stage.inline { position: static; padding: 8px; font-size: 13px; }
  .wrap { display: flex; flex-direction: column; gap: 6px; transition: opacity 0.15s ease-out; }
  .capsule { display: inline-flex; align-items: center; gap: 10px; height: 40px; padding: 0 18px; border-radius: 999px; box-shadow: 0 4px 10px rgba(0, 0, 0, 0.18); white-space: nowrap; max-width: 100%; }
  .capsule.compact { height: 28px; padding: 0 12px; }
  /* Un message est une phrase entière (« Reformulation impossible : moteur hors de cet ordinateur, sans accord… ») : elle passe à la ligne au lieu d'être coupée. */
  .capsule.message { height: auto; min-height: 40px; padding: 9px 18px; border-radius: 22px; white-space: normal; line-height: 1.3; }
  .capsule.message .icon { flex: none; }
  .text.soft { opacity: 0.85; }
  .failure .body { display: flex; flex-direction: column; gap: 3px; }
  .failure .detail.soft { font-weight: 500; opacity: 0.9; }
  .close { flex: none; width: 24px; height: 24px; margin: -4px -8px -4px 2px; padding: 0; border: 0; border-radius: 50%; background: transparent; color: inherit; font: inherit; font-size: 12px; line-height: 1; cursor: pointer; }
  .close:hover, .close:focus-visible { background: rgba(255, 255, 255, 0.25); outline: none; }
  .icon { font-weight: 800; }
  .bars { display: inline-flex; align-items: center; gap: 3px; height: 20px; }
  .bars i { display: block; width: 3px; border-radius: 2px; background: #fff; transition: height 0.08s ease-out; }
  .curve { display: inline-flex; align-items: center; gap: 1px; width: 140px; height: 24px; }
  .curve i { display: block; flex: 1; min-width: 1.5px; max-width: 3px; border-radius: 1px; background: #fff; }
  .dotbox { width: 26px; height: 26px; display: flex; align-items: center; justify-content: center; }
  .dot { border-radius: 50%; border: 1.5px solid rgba(255, 255, 255, 0.9); box-shadow: 0 2px 4px rgba(0, 0, 0, 0.25); transition: width 0.08s, height 0.08s; }
  .dot.fixed { width: 26px; height: 26px; display: flex; align-items: center; justify-content: center; border: 0; }
  .spinner { width: 13px; height: 13px; border-radius: 50%; border: 2px solid #fff; border-right-color: transparent; animation: turn 0.8s linear infinite; }
  .spinner.small { width: 12px; height: 12px; }
  .icon.small { font-size: 11px; }
  @keyframes turn { to { transform: rotate(360deg); } }
</style>
