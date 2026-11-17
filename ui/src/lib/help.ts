/** Les rubriques de l'Aide, dans l'ordre du menu de gauche (une par page, sauf « Premiers pas », « Sur cet ordinateur ou en ligne », « Problèmes fréquents » et « À propos ») ; la première est ouverte par défaut. */
export const helpSections = [
  { id: 'start', title: 'Premiers pas' },
  { id: 'dictate', title: 'Dicter' },
  { id: 'modes', title: 'Style d’écriture' },
  { id: 'dictionary', title: 'Dictionnaire' },
  { id: 'commands', title: 'Commandes vocales' },
  { id: 'engines', title: 'Moteurs et modèles' },
  { id: 'privacy', title: 'Sur cet ordinateur ou en ligne' },
  { id: 'profile', title: 'Profil' },
  { id: 'indicator', title: 'Apparence' },
  { id: 'data', title: 'Données et système' },
  { id: 'measures', title: 'Mesures et essais' },
  { id: 'problems', title: 'Problèmes fréquents' },
  { id: 'about', title: 'À propos' },
] as const;

export type HelpId = (typeof helpSections)[number]['id'];

export const defaultHelpSection: HelpId = 'start';
