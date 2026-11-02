export type Theme = 'auto' | 'light' | 'dark';

/** La fenêtre est-elle sombre ? Le choix de l'utilisateur, à défaut l'apparence du système. */
export function isDark(theme: Theme | undefined, systemDark: boolean): boolean {
  return theme === 'dark' || (theme !== 'light' && systemDark);
}

/** Impose l'apparence à la page (`data-theme` sur la racine, lu par `app.css`) ; « auto » laisse faire le système. */
export function applyTheme(root: HTMLElement, theme: Theme | undefined): void {
  if (theme === 'light' || theme === 'dark') root.dataset.theme = theme;
  else delete root.dataset.theme;
}
