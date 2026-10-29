/**
 * Traduction anglaise de l'interface : la clé est le texte français tel qu'il est écrit dans `t('…')`, la valeur sa traduction.
 * `{nom}` marque une valeur insérée ; la traduction doit en avoir les mêmes (`i18n.test.ts` le vérifie, ainsi que l'absence de clé orpheline
 * ou de phrase de la page sans traduction).
 */
export const en: Record<string, string> = {
  // Barre latérale et fenêtre
  'Tableau de bord': 'Dashboard',
  'Historique': 'History',
  'Modes': 'Modes',
  'Dictionnaire': 'Dictionary',
  'Réglages': 'Settings',
  'Installation': 'Installation',
  'Banc d’essai': 'Test bench',
  'Mise à jour': 'Update',
  'Journal et rapport': 'Log and report',
  'Aide': 'Help',
  'Quitter Scriibe': 'Quit Scriibe',
  'Quitter Scriibe ?': 'Quit Scriibe?',
  'Quitter': 'Quit',
  'version {version}': 'version {version}',
  'Scriibe est arrêté. Vous pouvez fermer cette page.': 'Scriibe has stopped. You can close this page.',
  'Scriibe ne répond plus. Relancez-le depuis l’icône de la zone de notification.': 'Scriibe is not responding. Start it again from the notification area icon.',

  // Tableau de bord
  'Relancer l’assistant de premier lancement': 'Run the first-launch assistant again',
  'Mesure l’ordinateur et propose les moteurs qui lui conviennent': 'Measures this computer and suggests the engines that suit it',
  'mots aujourd’hui': 'words today',
  '{change} % par rapport à hier à la même heure': '{change}% compared with yesterday at the same time',
  'mots dictés au total, en {n} dictée(s)': 'words dictated in total, in {n} dictation(s)',
  'mots par minute en moyenne (au clavier : {n})': 'words per minute on average (typing: {n})',
  'gagnées par rapport à la frappe': 'saved compared with typing',
  'jour(s) d’affilée avec une dictée': 'day(s) in a row with a dictation',
  'corrections du dictionnaire cette semaine': 'dictionary corrections this week',
  '14 derniers jours': 'Last 14 days',
  '{n} mots': '{n} words',
  'Dernières dictées': 'Latest dictations',
  'Tout voir': 'See all',
  '{app} · {n} mots': '{app} · {n} words',
  'Texte non gardé (réglage de confidentialité)': 'Text not kept (privacy setting)',
  'Maintiens {key} dans n’importe quelle appli et parle : ta première dictée apparaîtra ici.': 'Hold {key} in any app and speak: your first dictation will appear here.',
  'Où tu dictes': 'Where you dictate',
  'Pas encore de dictée.': 'No dictation yet.',
  '{n} mots · {share} %': '{n} words · {share}%',
  'Ma vitesse au clavier': 'My typing speed',
  'Sert au calcul du temps gagné. Recopiez le texte : seuls les essais complets, à plus de 90 % de lettres justes, comptent.':
    'Used to work out the time saved. Copy the text out: only complete attempts with more than 90% correct letters count.',
  'Chargement…': 'Loading…',

  // Réglages
  'Langue de l’interface': 'Interface language',
  'Celle des menus et des textes de Scriibe. Elle ne change ni la langue que vous dictez ni la langue de traduction.':
    'The language of Scriibe’s menus and texts. It changes neither the language you dictate in nor the translation language.',
  'Langue :': 'Language:',
  'Automatique (celle du système)': 'Automatic (the system language)',
};
