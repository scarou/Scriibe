/** Les pages du menu de gauche, rangées en quatre groupes ; l'Aide vient en dernier, hors groupe (comme dans Scribe). */
export const tabGroups = [
  {
    label: 'Utiliser',
    tabs: [
      { id: 'dashboard', label: 'Accueil', icon: '▦' },
      { id: 'history', label: 'Historique', icon: '🕘' },
    ],
  },
  {
    label: 'Ma dictée',
    tabs: [
      { id: 'dictate', label: 'Dicter', icon: '🎙',
        description: 'Les réglages de tous les jours : la touche, le micro, la langue et ce que devient ton texte.' },
      { id: 'style', label: 'Style d’écriture', icon: '✎',
        description: 'Comment le texte est mis au propre : un mode par usage, et les tiens.' },
      { id: 'dictionary', label: 'Dictionnaire', icon: 'Aa',
        description: 'Ce que Scriibe écrit exactement, même quand Whisper l’entend autrement.' },
      { id: 'commands', label: 'Commandes vocales', icon: '⌘',
        description: 'Dire à Scriibe quoi faire : entourer, mettre en gras, chercher, ouvrir une appli.' },
    ],
  },
  {
    label: 'Configurer',
    tabs: [
      { id: 'engines', label: 'Moteurs et modèles', icon: '⚙',
        description: 'Où passent ta voix et ton texte, les programmes et les modèles à installer.' },
      { id: 'profile', label: 'Profil', icon: '👤',
        description: 'Qui dicte : les utilisateurs et leurs clés.' },
      { id: 'appearance', label: 'Apparence', icon: '◐',
        description: 'L’apparence de la fenêtre, la langue de l’interface et l’indicateur affiché à l’écran.' },
      { id: 'data', label: 'Données et système', icon: '🗂',
        description: 'Lancer Scriibe à l’ouverture de session, mettre à jour, sauvegarder ou restaurer tes données, et, au besoin, désinstaller Scriibe.' },
    ],
  },
  {
    label: 'Avancé',
    tabs: [{ id: 'measures', label: 'Mesures et essais', icon: '⏱',
        description: 'Mesurer la vitesse et la qualité, essayer la dictée, et le journal à joindre à un rapport.' }],
  },
] as const;

export const helpTab = { id: 'help', label: 'Aide', icon: '?' } as const;

export type MainTab = (typeof tabGroups)[number]['tabs'][number]['id'] | typeof helpTab.id;

/** Toutes les pages, dans l'ordre du menu : l'Aide est la dernière. */
export const tabs: readonly { id: MainTab; label: string; icon: string }[] = [...tabGroups.flatMap((g) => [...g.tabs]), helpTab];

/** La phrase sous le titre d'une page (celles du menu, sauf l'Accueil, l'Historique et l'Aide, qui ont la leur). */
export function tabDescription(id: MainTab): string {
  return (tabs.find((x) => x.id === id) as { description?: string } | undefined)?.description ?? '';
}

/** Les identifiants de page : ceux du menu. Les liens (cartes, assistant, Aide) visent une page et, au besoin, l'id d'une carte. */
export type TabId = MainTab;
