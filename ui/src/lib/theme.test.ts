import { describe, expect, it } from 'vitest';
import { applyTheme, guessOs, isDark } from './theme';

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

describe('système deviné avant la réponse du programme', () => {
  it('reconnaît macOS, Windows et Linux d’après le navigateur de la fenêtre', () => {
    expect(guessOs('Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko)')).toBe('macos');
    expect(guessOs('Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Edg/130.0')).toBe('windows');
    expect(guessOs('Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15 (KHTML, like Gecko)')).toBe('linux');
    expect(guessOs('')).toBe('windows');
  });
});
