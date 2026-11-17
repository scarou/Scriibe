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

/** Les cartes d'export et d'import que la carte « Sauvegarder et transférer » ouvre sur leur page. */
export type TransferKind = 'user-export' | 'user-import' | 'dictionary-export' | 'dictionary-import' | 'modes-export' | 'modes-import' | 'commands-export' | 'commands-import';

const transferTargets: Record<TransferKind, { tab: MainTab; anchor: string }> = {
  'user-export': { tab: 'profile', anchor: 'utilisateurs' },
  'user-import': { tab: 'profile', anchor: 'utilisateurs' },
  'dictionary-export': { tab: 'dictionary', anchor: 'dictionary-export-import' },
  'dictionary-import': { tab: 'dictionary', anchor: 'dictionary-export-import' },
  'modes-export': { tab: 'style', anchor: 'modes-mes-modes' },
  'modes-import': { tab: 'style', anchor: 'modes-mes-modes' },
  'commands-export': { tab: 'commands', anchor: 'commandes-perso' },
  'commands-import': { tab: 'commands', anchor: 'commandes-perso' },
};

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
    let target = anchor ? document.getElementById(anchor) : null;
    // Une carte qui attend la réponse du programme (ex. la mise à jour) n'existe pas encore : on la guette une seconde.
    for (let i = 0; anchor && !target && i < 60; i++) {
      await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
      target = document.getElementById(anchor);
    }
    if (target) {
      // Une carte repliée se déplie pour être montrée.
      if (target.dataset.folded === 'true') target.querySelector<HTMLButtonElement>('button.fold')?.click();
      target.scrollIntoView({ behavior: 'smooth', block: 'start' });
    } else window.scrollTo(0, 0);
  }

  /** Demande d'ouverture d'une carte d'export ou d'import ; la page visée la prend dès qu'elle est prête, puis l'efface. */
  transferRequest = $state<TransferKind | null>(null);

  /** Ouvre la page concernée et y demande la même carte secondaire que ses propres boutons. */
  async transfer(kind: TransferKind): Promise<void> {
    this.transferRequest = kind;
    const { tab, anchor } = transferTargets[kind];
    await this.go(tab, anchor);
    // Une demande que la page n'a pas pu prendre (rien à exporter, données pas lues) ne doit pas rester pour une visite ultérieure.
    setTimeout(() => {
      if (this.transferRequest === kind) this.transferRequest = null;
    }, 4000);
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
