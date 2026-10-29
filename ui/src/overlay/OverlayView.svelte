<script lang="ts" module>
  import type { CaptureColor, CapturePosition, CaptureStyle } from '../lib/appearance';

  export interface View {
    visible: boolean;
    state: 'hidden' | 'listening' | 'working' | 'done' | 'message';
    message: string | null;
    translation: string | null;
    mic: 'ok' | 'weak' | 'silent';
    level: number;
    /** « windows », « macos » ou « linux » : la couleur de la marque (absent dans les illustrations de l'Aide). */
    os?: string;
    appearance: { enabled: boolean; style: CaptureStyle; position: CapturePosition; color: CaptureColor; opacity: number };
  }
</script>

<script lang="ts">
  import { caption, gradient, listeningOpacity, listeningText } from '../lib/appearance';
  import { t } from '../lib/i18n.svelte';

  /** `inline` : posé dans une page (illustration de l'Aide) au lieu de remplir la fenêtre de la pastille. */
  let { view, levels = [], inline = false }: { view: View; levels?: number[]; inline?: boolean } = $props();

  let v = $derived(view);
  let fill = $derived(gradient(v.appearance.color));
  let note = $derived(caption(v.mic, v.translation, t));
  let align = $derived(v.appearance.position === 'left' ? 'flex-start' : v.appearance.position === 'right' ? 'flex-end' : 'center');
  let opacity = $derived(v.state === 'listening' ? listeningOpacity(v.mic, v.appearance.opacity) : v.state === 'message' ? 1 : v.appearance.opacity);
  let bars = [0.45, 0.8, 1, 0.7, 0.4];
</script>


{#if v.state !== 'hidden'}
  <div class="stage" class:inline style="align-items: {align}">
    <div class="wrap" style="align-items: {align}; opacity: {opacity}">
      {#if v.state === 'message'}
        <div class="capsule" style="background: {fill}">
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
  .stage.inline { position: static; padding: 8px; font-size: 13px; }
  .wrap { display: flex; flex-direction: column; gap: 6px; transition: opacity 0.15s ease-out; }
  .capsule { display: inline-flex; align-items: center; gap: 10px; height: 40px; padding: 0 18px; border-radius: 999px; box-shadow: 0 4px 10px rgba(0, 0, 0, 0.18); white-space: nowrap; max-width: 100%; }
  .capsule.compact { height: 28px; padding: 0 12px; }
  .text.soft { opacity: 0.85; }
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
