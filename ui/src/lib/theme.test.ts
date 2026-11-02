import { describe, expect, it } from 'vitest';
import { applyTheme, isDark } from './theme';

describe('apparence de la fenêtre', () => {
  it('suit le système en automatique, et le choix de l’utilisateur sinon', () => {
    expect(isDark('auto', true)).toBe(true);
    expect(isDark('auto', false)).toBe(false);
    expect(isDark(undefined, true)).toBe(true);
    expect(isDark('light', true)).toBe(false);
    expect(isDark('dark', false)).toBe(true);
  });

  it('pose data-theme pour un choix, l’ôte en automatique', () => {
    const root = { dataset: {} as Record<string, string> } as unknown as HTMLElement;
    applyTheme(root, 'dark');
    expect(root.dataset.theme).toBe('dark');
    applyTheme(root, 'light');
    expect(root.dataset.theme).toBe('light');
    applyTheme(root, 'auto');
    expect(root.dataset.theme).toBeUndefined();
  });
});
