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

/**
 * Les identifiants de page : les pages du menu, et les anciens identifiants (l'ancienne page Réglages et ses onglets),
 * que les liens des cartes, de l'assistant et de l'Aide connaissent encore (la tâche 5 de la refonte les remplace).
 */
export type TabId = MainTab | 'settings' | 'modes' | 'install' | 'bench' | 'journal' | 'update';

const oldPages: Record<string, MainTab> = {
  settings: 'dictate',
  modes: 'style',
  install: 'engines',
  bench: 'measures',
  journal: 'measures',
  update: 'data',
};

/** La page où se trouve une carte de l'ancienne page Réglages (les liens visent une carte par son ancre). */
const settingsAnchors: Record<string, MainTab> = {
  dictee: 'dictate',
  texte: 'style',
  utilisateurs: 'profile',
  cles: 'profile',
  moteurs: 'engines',
  theme: 'appearance',
  'logo-anime': 'appearance',
  langue: 'appearance',
  indicateur: 'appearance',
  demarrage: 'data',
  sauvegarde: 'data',
  desinstaller: 'data',
  'reglages-emplacements': 'data',
};

/** La page du menu qui s'ouvre pour un identifiant (ancien ou nouveau) et, pour l'ancien « settings », l'ancre de la carte visée. */
export function resolveTab(id: TabId, anchor?: string): MainTab {
  if (id === 'settings') return (anchor && settingsAnchors[anchor]) || 'dictate';
  return oldPages[id] ?? (id as MainTab);
}
