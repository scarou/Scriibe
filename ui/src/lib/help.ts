/** Les huit sections de l'Aide (mêmes titres que Scribe) ; la première est ouverte par défaut. */
export const helpSections = [
  { id: 'start', title: 'Premiers pas' },
  { id: 'dictate', title: 'Dicter et traduire' },
  { id: 'modes', title: 'Les modes' },
  { id: 'dictionary', title: 'Dictionnaire et signes' },
  { id: 'indicator', title: 'L’indicateur' },
  { id: 'measures', title: 'Mesurer et comparer' },
  { id: 'privacy', title: 'Sur cet ordinateur ou en ligne' },
  { id: 'problems', title: 'Problèmes fréquents' },
] as const;

export type HelpId = (typeof helpSections)[number]['id'];

export const defaultHelpSection: HelpId = 'start';
