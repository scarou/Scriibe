import { tick } from 'svelte';
import { defaultHelpSection, type HelpId } from './help';
import { tabs, type TabId } from './tabs';

function savedTab(): TabId {
  try {
    const saved = sessionStorage.getItem('tab');
    if (tabs.some((t) => t.id === saved)) return saved as TabId;
  } catch {
    /* le stockage peut être refusé : on commence par le tableau de bord */
  }
  return 'dashboard';
}

/** L'onglet affiché et la section ouverte de l'Aide ; l'Aide et les boutons « ? » des cartes y passent pour ouvrir le bon endroit. */
class Nav {
  tab = $state<TabId>(savedTab());
  helpSection = $state<HelpId>(defaultHelpSection);

  /** Ouvre un onglet ; `anchor` : l'identifiant de la carte à montrer dans cet onglet. */
  async go(tab: TabId, anchor?: string): Promise<void> {
    this.tab = tab;
    try {
      sessionStorage.setItem('tab', tab);
    } catch {
      /* sans importance */
    }
    await tick();
    const target = anchor ? document.getElementById(anchor) : null;
    if (target) target.scrollIntoView({ behavior: 'smooth', block: 'start' });
    else window.scrollTo(0, 0);
  }

  /** Ouvre une section de l'Aide (bouton « ? » d'une carte). */
  openHelp(section: HelpId): void {
    this.helpSection = section;
    void this.go('help');
  }
}

export const nav = new Nav();
