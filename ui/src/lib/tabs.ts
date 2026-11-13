/** Les pages de la barre latérale, dans l'ordre : l'Aide est la dernière (comme dans Scribe). */
export const tabs = [
  { id: 'dashboard', label: 'Tableau de bord', icon: '▦' },
  { id: 'history', label: 'Historique', icon: '🕘' },
  { id: 'dictionary', label: 'Dictionnaire', icon: 'Aa' },
  { id: 'settings', label: 'Réglages', icon: '⚙' },
  { id: 'help', label: 'Aide', icon: '?' },
] as const;

export type MainTab = (typeof tabs)[number]['id'];

/** Les onglets de la page Réglages (comme dans Scribe, plus « Mise à jour » : Scribe ne s'y met pas à jour lui-même). */
export const settingsTabs = [
  { id: 'dictation', label: 'Dictée', description: 'Les réglages de tous les jours : la touche, le micro, la langue et ce que devient ton texte.' },
  { id: 'engines', label: 'Moteurs', description: 'Où passent ta voix et ton texte, les programmes et les modèles à installer.' },
  { id: 'appearance', label: 'Apparence', description: 'L’apparence de la fenêtre, la langue de l’interface et l’indicateur affiché à l’écran.' },
  { id: 'modes', label: 'Modes', description: 'Comment le texte est mis au propre : un mode par usage, et les tiens.' },
  { id: 'measures', label: 'Mesures et essais', description: 'Mesurer la vitesse et la qualité, essayer la dictée, et le journal à joindre à un rapport.' },
  { id: 'system', label: 'Système', description: 'Lancer Scriibe à l’ouverture de session, sauvegarder ou restaurer tes données, et, au besoin, désinstaller Scriibe.' },
  { id: 'update', label: 'Mise à jour', description: 'Chercher une version plus récente de Scriibe.' },
] as const;

export type SettingsTabId = (typeof settingsTabs)[number]['id'];

/** Les identifiants de page : les pages principales, et les anciennes pages devenues des onglets des Réglages (les liens des cartes et de l'Aide les connaissent). */
export type TabId = MainTab | 'modes' | 'install' | 'bench' | 'update' | 'journal';

const movedToSettings: Record<string, SettingsTabId> = {
  modes: 'modes',
  install: 'engines',
  bench: 'measures',
  journal: 'measures',
  update: 'update',
};

/** Où ouvrir un identifiant de page : la page principale, et l'onglet des Réglages si c'en est un. */
export function resolveTab(id: TabId): { tab: MainTab; settingsTab?: SettingsTabId } {
  const moved = movedToSettings[id];
  return moved ? { tab: 'settings', settingsTab: moved } : { tab: id as MainTab };
}

/** L'onglet des Réglages où se trouve une carte (les liens de l'Aide visent une carte par son ancre). */
const settingsAnchors: Record<string, SettingsTabId> = {
  dictee: 'dictation',
  texte: 'dictation',
  moteurs: 'engines',
  cles: 'engines',
  langue: 'appearance',
  indicateur: 'appearance',
  demarrage: 'system',
  sauvegarde: 'system',
  desinstaller: 'system',
};

export function settingsTabOfAnchor(anchor: string | undefined): SettingsTabId | undefined {
  return anchor ? settingsAnchors[anchor] : undefined;
}
