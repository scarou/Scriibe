<script lang="ts">
  import type { Profile } from '../lib/api';
  import { fmt } from '../lib/display.svelte';
  import { t } from '../lib/i18n.svelte';

  let { profile }: { profile: Profile | undefined } = $props();
</script>

{#if profile && profile.levels.length}
  <div class="muted">
    {t('Profil du son (un trait = ¼ de seconde) — pic {peak}', { peak: fmt(profile.peak) })}
    {#if profile.clipped > 0.005}— <span class="bad">{t('saturation : {x} % des échantillons', { x: fmt(profile.clipped * 100) })}</span>{/if}
  </div>
  <div class="profile">
    {#each profile.levels as l, i (i)}
      <i class:hot={l > 0.5} style="height: {Math.max(2, Math.min(100, Math.sqrt(l) * 110))}%" title={fmt(l)}></i>
    {/each}
  </div>
{/if}

<style>
  .profile { display: flex; align-items: flex-end; gap: 2px; height: 46px; margin: 6px 0; }
  .profile > i { flex: 1 1 0; min-width: 2px; background: var(--accent); border-radius: 2px 2px 0 0; }
  .profile > i.hot { background: var(--bad); }
</style>
