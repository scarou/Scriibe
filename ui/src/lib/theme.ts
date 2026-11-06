export type Theme = 'auto' | 'light' | 'dark';

/**
 * Le système de la machine, deviné d'après le navigateur de la fenêtre. La couleur de la marque en dépend (`data-os` sur la racine, lu par
 * `app.css` : bleu Windows, turquoise macOS, violet Linux) ; la réponse du programme (`/api/status`) la confirme ensuite, mais elle peut se
 * faire attendre (par exemple tant que macOS demande l'accès au trousseau) : sans cette devinette, la page restait bleue en attendant.
 */
export function guessOs(userAgent: string): 'macos' | 'windows' | 'linux' {
  if (/Macintosh|Mac OS X/i.test(userAgent)) return 'macos';
  if (/Linux|X11/i.test(userAgent) && !/Android/i.test(userAgent)) return 'linux';
  return 'windows';
}

/** La fenêtre est-elle sombre ? Le choix de l'utilisateur, à défaut l'apparence du système. */
export function isDark(theme: Theme | undefined, systemDark: boolean): boolean {
  return theme === 'dark' || (theme !== 'light' && systemDark);
}

/** Impose l'apparence à la page (`data-theme` sur la racine, lu par `app.css`) ; « auto » laisse faire le système. */
export function applyTheme(root: HTMLElement, theme: Theme | undefined): void {
  if (theme === 'light' || theme === 'dark') root.dataset.theme = theme;
  else delete root.dataset.theme;
}
