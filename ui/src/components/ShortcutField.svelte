<script lang="ts">
  import { t } from '../lib/i18n.svelte';
  import { shortcutOf } from '../lib/shortcut';

  /** Raccourci de l'appli saisi en l'appuyant sur le clavier (« Mod+Shift+X ») ; `Mod` est ⌘ sur Mac, Ctrl ailleurs. */
  let { value, onset }: { value: string; onset: (shortcut: string) => void } = $props();

  let listening = $state(false);
  let hint = $state('');

  const isMac = typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent);

  function keydown(e: KeyboardEvent) {
    if (e.key === 'Tab') return;
    e.preventDefault();
    e.stopPropagation();
    if (e.key === 'Escape') {
      (e.currentTarget as HTMLInputElement).blur();
      return;
    }
    if (['Meta', 'Control', 'Alt', 'Shift'].includes(e.key)) return;
    const text = shortcutOf(e, isMac);
    if (!text) {
      hint = t('Tiens ⌘, Ctrl ou Alt avec une lettre ou un chiffre.');
      return;
    }
    hint = '';
    onset(text);
    (e.currentTarget as HTMLInputElement).blur();
  }
</script>

<input
  type="text"
  class="shortcut"
  readonly
  value={listening ? t('Appuie sur le raccourci…') : value}
  placeholder="Mod+Shift+X"
  onfocus={() => ((listening = true), (hint = ''))}
  onblur={() => (listening = false)}
  onkeydown={keydown}
/>
{#if hint}<span class="hint">{hint}</span>{/if}

<style>
  input { cursor: pointer; width: 10rem; }
  input:focus { outline: 2px solid var(--accent); }
  .hint { color: var(--bad); }
</style>
