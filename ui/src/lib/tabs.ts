/** Les pages de la barre latérale, dans l'ordre : l'Aide est la dernière (comme dans Scribe). */
export const tabs = [
  { id: 'dashboard', label: 'Tableau de bord', icon: '▦' },
  { id: 'history', label: 'Historique', icon: '🕘' },
  { id: 'modes', label: 'Modes', icon: '✎' },
  { id: 'dictionary', label: 'Dictionnaire', icon: 'Aa' },
  { id: 'settings', label: 'Réglages', icon: '⚙' },
  { id: 'install', label: 'Installation', icon: '⬇' },
  { id: 'bench', label: 'Banc d’essai', icon: '🎤' },
  { id: 'update', label: 'Mise à jour', icon: '⟳' },
  { id: 'journal', label: 'Journal et rapport', icon: '☰' },
  { id: 'help', label: 'Aide', icon: '?' },
] as const;

export type TabId = (typeof tabs)[number]['id'];
