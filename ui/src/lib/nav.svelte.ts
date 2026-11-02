import { tick } from 'svelte';
import { defaultHelpSection, type HelpId } from './help';
import { resolveTab, settingsTabOfAnchor, settingsTabs, tabs, type MainTab, type SettingsTabId, type TabId } from './tabs';

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
  /** L'onglet ouvert dans les Réglages. */
  settingsTab = $state<SettingsTabId>(saved('settingsTab', settingsTabs, 'dictation'));
  helpSection = $state<HelpId>(defaultHelpSection);

  /** Ouvre une page (ou l'onglet des Réglages qui a pris la place d'une ancienne page) ; `anchor` : l'identifiant de la carte à montrer. */
  async go(id: TabId, anchor?: string): Promise<void> {
    const { tab, settingsTab } = resolveTab(id);
    this.tab = tab;
    const inSettings = settingsTab ?? (tab === 'settings' ? settingsTabOfAnchor(anchor) : undefined);
    if (inSettings) this.settingsTab = inSettings;
    try {
      sessionStorage.setItem('tab', tab);
      sessionStorage.setItem('settingsTab', this.settingsTab);
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

  /** Change d'onglet dans les Réglages. */
  chooseSettingsTab(id: SettingsTabId): void {
    void this.go('settings');
    this.settingsTab = id;
    try {
      sessionStorage.setItem('settingsTab', id);
    } catch {
      /* sans importance */
    }
  }

  /** Demande l'ouverture de l'assistant de premier lancement ; le Tableau de bord la prend en compte à son affichage. */
  assistantRequested = $state(false);

  /** Relance l'assistant de premier lancement (bouton du Tableau de bord, des Réglages et de l'Aide). */
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
