import { describe, expect, it } from 'vitest';
import { caption, colors, gradient, listeningOpacity, listeningText, rimStyle } from './appearance';

/** Rapport de contraste WCAG d'une teinte `#RRGGBB` avec le blanc. */
function contrastWithWhite(hex: string): number {
  const [r, g, b] = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255).map((c) => (c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4));
  return 1.05 / (0.2126 * r + 0.7152 * g + 0.0722 * b + 0.05);
}

describe('apparence de l’indicateur', () => {
  it('garde le texte blanc lisible (4,5 : 1) sur les sept couleurs ajoutées, aux deux bouts du dégradé', () => {
    for (const id of ['orange', 'green', 'teal', 'purple', 'pink', 'indigo', 'brown']) {
      for (const hex of colors.find((c) => c.id === id)!.hex) expect(contrastWithWhite(hex), `${id} ${hex}`).toBeGreaterThanOrEqual(4.5);
    }
  });
  it('reprend les cinq couleurs de Scribe, suivies de sept autres', () => {
    expect(colors.map((c) => c.id)).toEqual(['scribe', 'gray', 'red', 'blue', 'black', 'orange', 'green', 'teal', 'purple', 'pink', 'indigo', 'brown']);
    expect(gradient('red')).toBe('linear-gradient(to right, #E5484D, #C62F35)');
  });
  it('dit « J’écoute… » ou l’alerte du micro', () => {
    expect(listeningText('ok')).toBe('J’écoute…');
    expect(listeningText('weak')).toBe('Je t’entends à peine…');
    expect(listeningText('silent')).toBe('Je n’entends rien…');
  });
  it('compose la légende des styles sans texte', () => {
    expect(caption('ok', null)).toBeNull();
    expect(caption('weak', 'anglais')).toBe('Je t’entends à peine…  → anglais');
    expect(caption('ok', 'espagnol')).toBe('→ espagnol');
  });
  it('garde l’avertissement de micro lisible malgré la transparence', () => {
    expect(listeningOpacity('ok', 0.4)).toBe(0.4);
    expect(listeningOpacity('silent', 0.4)).toBe(1);
  });
});

describe('le liseré de la pastille sur fond sombre', () => {
  it('est posé sur indigo, violet et noir, et sur aucune autre couleur', () => {
    for (const id of ['indigo', 'purple', 'black'] as const) expect(rimStyle(id)).toContain('--edge: rgba(255, 255, 255');
    for (const c of colors.filter((c) => !['indigo', 'purple', 'black'].includes(c.id))) expect(rimStyle(c.id)).toBe('');
  });
});
