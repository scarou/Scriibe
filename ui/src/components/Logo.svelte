<script lang="ts">
  import { featherPath } from '../lib/logo';

  /** Le symbole, d'un seul ton (la couleur du texte) ; `size` : hauteur en pixels. */
  let { size = 22 }: { size?: number } = $props();
</script>

<svg viewBox="18 8 64 91" height={size} width={(size * 64) / 91} role="img" aria-label="Scriibe" fill="currentColor" fill-rule="evenodd">
  <path d={featherPath} />
</svg>
