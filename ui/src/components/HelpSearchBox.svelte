<script lang="ts">
  import { tick } from 'svelte';
  import { i18n, t } from '../lib/i18n.svelte';
  import { findMatches } from '../lib/helpsearch';
  import { helpSearch } from '../lib/helpSearch.svelte';

  /**
   * Le champ de recherche de l'Aide : texte intégral, sans IA, hors ligne. Quand on tape, toutes les rubriques sont posées dans la page
   * (`show` dans Help), cette boîte lit leur texte, garde celles qui contiennent le mot (casse et accents ignorés) et le surligne avec le
   * « CSS Custom Highlight » (sans toucher au DOM de Svelte). Un mot coupé par une balise (gras au milieu) n'est pas trouvé.
   */
  const HIGHLIGHT = 'help-search';
  let scrolledFor = '';

  type Highlights = { set(name: string, value: unknown): void; delete(name: string): void };
  const registry = (): Highlights | null => (CSS as unknown as { highlights?: Highlights }).highlights ?? null;

  function clearHighlight() {
    registry()?.delete(HIGHLIGHT);
  }

  /** Lit le texte de chaque rubrique : met à jour les rubriques trouvées et le surlignage. Ne change l'état que s'il y a une différence. */
  function scan(root: HTMLElement, needle: string): Range | null {
    const ranges: Range[] = [];
    const found: string[] = [];
    let count = 0;
    for (const sec of root.querySelectorAll<HTMLElement>('[data-help]')) {
      let hit = false;
      const walker = document.createTreeWalker(sec, NodeFilter.SHOW_TEXT);
      for (let node = walker.nextNode(); node; node = walker.nextNode()) {
        for (const [from, to] of findMatches(node.textContent ?? '', needle)) {
          const range = document.createRange();
          range.setStart(node, from);
          range.setEnd(node, to);
          ranges.push(range);
          hit = true;
          count++;
        }
      }
      if (hit) found.push(sec.dataset.help!);
    }
    if (found.join() !== helpSearch.matches.join()) helpSearch.matches = found;
    if (count !== helpSearch.count) helpSearch.count = count;
    const api = registry();
    if (api && ranges.length) api.set(HIGHLIGHT, new (window as unknown as { Highlight: new (...r: Range[]) => unknown }).Highlight(...ranges));
    else clearHighlight();
    return ranges[0] ?? null;
  }

  $effect(() => {
    const needle = helpSearch.needle;
    void i18n.lang;
    if (!needle) {
      clearHighlight();
      helpSearch.matches = [];
      helpSearch.count = 0;
      scrolledFor = '';
      return;
    }
    let frame = 0;
    let observer: MutationObserver | null = null;
    const run = async (root: HTMLElement) => {
      const first = scan(root, needle);
      if (first && scrolledFor !== needle) {
        scrolledFor = needle;
        await tick(); // les rubriques sans résultat sont encore cachées : attendre que Svelte les ait montrées
        first.startContainer.parentElement?.scrollIntoView({ block: 'center', behavior: 'smooth' });
      }
    };
    const attach = () => {
      const root = document.querySelector<HTMLElement>('[data-help-root]');
      if (!root) {
        frame = requestAnimationFrame(attach);
        return;
      }
      observer = new MutationObserver(() => {
        cancelAnimationFrame(frame);
        frame = requestAnimationFrame(() => run(root));
      });
      observer.observe(root, { childList: true, subtree: true, characterData: true });
      frame = requestAnimationFrame(() => run(root));
    };
    attach();
    return () => {
      cancelAnimationFrame(frame);
      observer?.disconnect();
    };
  });

  $effect(() => () => clearHighlight());

  let height = $state(0);
  // La recherche reste en haut sous l'en-tête : sa hauteur est connue du sommaire et des ancres (voir PageHeader).
  $effect(() => {
    document.documentElement.style.setProperty('--help-search', `${height}px`);
    return () => document.documentElement.style.removeProperty('--help-search');
  });
</script>

<div class="search" bind:clientHeight={height}>
  <input
    type="search"
    bind:value={helpSearch.query}
    placeholder={t('Chercher dans l’Aide…')}
    aria-label={t('Chercher dans l’Aide')}
    onkeydown={(e) => { if (e.key === 'Escape') helpSearch.clear(); }}
  />
  {#if helpSearch.active}
    <span class="muted" role="status">
      {helpSearch.count > 0
        ? t('{n} résultat(s) dans {m} rubrique(s)', { n: helpSearch.count, m: helpSearch.matches.length })
        : t('Aucun résultat pour « {q} »', { q: helpSearch.query.trim() })}
    </span>
    <button onclick={() => helpSearch.clear()}>{t('Effacer')}</button>
  {/if}
</div>

<style>
  .search { position: sticky; top: var(--page-head, 0px); z-index: 6; display: flex; flex-wrap: wrap; align-items: center; gap: 8px; margin: 0; padding: 10px 0 8px; background: var(--bg); }
  .search input { flex: 1 1 14em; max-width: 28em; }
</style>
