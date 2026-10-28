<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import type { CaptureColor, CapturePosition, CaptureStyle } from '../lib/appearance';
  import OverlayView, { type View } from './OverlayView.svelte';
  import { MeterHistory } from '../lib/logic';

  let view = $state<View | null>(null);
  let levels = $state<number[]>([]);
  const meter = new MeterHistory();
  let last = '';

  async function poll() {
    try {
      const next = await api<View>('overlay');
      if (next.state !== last) {
        meter.setListening(next.state === 'listening');
        last = next.state;
      }
      if (next.state === 'listening') meter.push(next.level);
      levels = [...meter.history];
      view = next;
    } catch {
      /* Scriibe s'arrête : la fenêtre se ferme avec lui. */
    }
  }

  onMount(() => {
    document.documentElement.classList.add('overlay');
    void poll();
    const timer = setInterval(() => void poll(), 45);
    return () => clearInterval(timer);
  });

</script>

{#if view}<OverlayView {view} {levels} />{/if}

<style>
  :global(html.overlay), :global(html.overlay body) { background: transparent !important; overflow: hidden; }
</style>
