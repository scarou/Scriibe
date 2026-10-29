<script lang="ts">
  import type { HelpId } from '../lib/help';
  import { nav } from '../lib/nav.svelte';
  import { t } from '../lib/i18n.svelte';

  /** Le bouton « ? » d'une carte : ouvre la section de l'Aide qui explique à quoi elle sert. */
  let { section, label = undefined }: { section: HelpId; label?: string } = $props();
  let text = $derived(label ?? t('Aide sur cette carte'));
</script>

<button class="help" type="button" title={text} aria-label={text} onclick={() => nav.openHelp(section)}>?</button>

<style>
  .help { width: 1.5rem; height: 1.5rem; padding: 0; border-radius: 50%; font-size: 0.8rem; line-height: 1; margin-left: 6px; vertical-align: middle; color: var(--muted); }
</style>
