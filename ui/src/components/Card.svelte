<script lang="ts">
  import { untrack, type Snippet } from 'svelte';
  import type { HelpId } from '../lib/help';
  import { saveFold, savedFold } from '../lib/folds';
  import { t } from '../lib/i18n.svelte';
  import HelpButton from './HelpButton.svelte';

  /**
   * Une carte repliable, comme dans Scribe : le titre est le bouton (chevron vers le bas : dépliée) et, repliée, la carte ne garde que
   * son titre. `id` sert d'ancre (nav.go) et de clé de mémoire du pli ; `intro` est le texte d'explication sous le titre.
   * Le contenu reste dans la page quand la carte est repliée (un brouillon ou un chrono en cours n'est pas perdu).
   */
  let { title, id, help = undefined, intro = undefined, folded: initiallyFolded = false, children }: {
    title: string;
    id: string;
    help?: HelpId;
    intro?: string;
    /** Repliée tant que l'utilisateur n'a pas choisi. */
    folded?: boolean;
    children: Snippet;
  } = $props();

  let folded = $state(untrack(() => savedFold(id) ?? initiallyFolded));

  function toggle() {
    folded = !folded;
    saveFold(id, folded);
  }
</script>

<section class="card fold-card" {id} data-folded={folded}>
  <div class="head">
    <h2>
      <button class="fold" type="button" aria-expanded={!folded} onclick={toggle} title={folded ? t('Déplier') : t('Replier')}>
        <span class="chevron" class:closed={folded}>⌄</span>{title}
      </button>
    </h2>
    {#if help}<HelpButton section={help} />{/if}
  </div>
  <div class="content" hidden={folded}>
    {#if intro}<p class="intro">{intro}</p>{/if}
    {@render children()}
  </div>
</section>

<style>
  .fold-card { padding: 14px 18px; }
  .head { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
  h2 { margin: 0; flex: 1; font-size: 1.05rem; }
  .fold { all: unset; cursor: pointer; display: flex; align-items: center; gap: 8px; font-weight: 700; width: 100%; }
  .fold:focus-visible { outline: 2px solid var(--accent); border-radius: 6px; }
  .chevron { display: inline-block; width: 1em; color: var(--muted); transition: transform .15s; line-height: 1; }
  .chevron.closed { transform: rotate(-90deg); }
  .content { margin-top: 6px; }
  .intro { color: var(--muted); font-size: .88rem; margin: 0 0 8px; }
</style>
