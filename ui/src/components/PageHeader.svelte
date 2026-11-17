<script lang="ts">
  import type { Snippet } from 'svelte';

  /** L'en-tête d'une page : le titre, une phrase sous le titre (en contenu) et, à droite, des boutons. Il reste en haut de la fenêtre quand on fait défiler. */
  let { title, actions, children }: { title: string; actions?: Snippet; children?: Snippet } = $props();

  let height = $state(0);
  // Sa hauteur est connue des éléments qui restent aussi en haut (sommaire de l'Aide, filtres de l'Historique) et des ancres.
  $effect(() => {
    document.documentElement.style.setProperty('--page-head', `${height}px`);
    return () => document.documentElement.style.removeProperty('--page-head');
  });
</script>

<header class="page-header" bind:clientHeight={height}>
  <div class="text">
    <h1>{title}</h1>
    {#if children}<p class="muted lead">{@render children()}</p>{/if}
  </div>
  {#if actions}<div class="actions">{@render actions()}</div>{/if}
</header>

<style>
  /* Les marges négatives recouvrent le remplissage de <main> : rien ne défile au-dessus de l'en-tête. */
  .page-header { position: sticky; top: 0; z-index: 6; display: flex; justify-content: space-between; align-items: flex-start; gap: 12px; background: var(--bg); margin: -20px -20px 8px; padding: 20px 20px 4px; }
  .text { min-width: 0; }
  .lead { margin: 0 0 8px; }
  .actions { flex: none; }
</style>
