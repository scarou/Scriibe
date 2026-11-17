import { tick } from 'svelte';
import { defaultHelpSection, type HelpId } from './help';
import { tabs, type MainTab, type TabId } from './tabs';

function saved<T extends string>(key: string, allowed: readonly { id: T }[], fallback: T): T {
  try {
    const value = sessionStorage.getItem(key);
    if (allowed.some((t) => t.id === value)) return value as T;
  } catch {
    /* le stockage peut être refusé : on commence par le début */
  }
  return fallback;
}

/** L'onglet affiché et la section ouverte de l'Aide ; l'Aide et les boutons « ? » des cartes y passent pour ouvrir le bon endroit. */
class Nav {
  tab = $state<MainTab>(saved('tab', tabs, 'dashboard'));
  helpSection = $state<HelpId>(defaultHelpSection);
  /** Texte sélectionné envoyé par la commande vocale « ajouter au dictionnaire » : le Dictionnaire le met dans « Entendu » puis l'oublie. Jamais gardé ailleurs qu'en mémoire. */
  dictionaryPrefill = $state<string | null>(null);

  /** Ouvre une page du menu ; `anchor` : l'identifiant de la carte à montrer. */
  async go(id: TabId, anchor?: string): Promise<void> {
    const tab: MainTab = id;
    this.tab = tab;
    try {
      sessionStorage.setItem('tab', tab);
    } catch {
      /* sans importance */
    }
    await tick();
    const target = anchor ? document.getElementById(anchor) : null;
    if (target) {
      // Une carte repliée se déplie pour être montrée.
      if (target.dataset.folded === 'true') target.querySelector<HTMLButtonElement>('button.fold')?.click();
      target.scrollIntoView({ behavior: 'smooth', block: 'start' });
    } else window.scrollTo(0, 0);
  }

  /** Demande l'ouverture de l'assistant de premier lancement ; l'Accueil la prend en compte à son affichage. */
  assistantRequested = $state(false);

  /** Relance l'assistant de premier lancement (boutons de l'Accueil, de Moteurs et modèles et de l'Aide). */
  openAssistant(): void {
    this.assistantRequested = true;
    void this.go('dashboard');
  }

  /** Ouvre une section de l'Aide (bouton « ? » d'une carte). */
  openHelp(section: HelpId): void {
    this.helpSection = section;
    void this.go('help');
  }
}

export const nav = new Nav();
