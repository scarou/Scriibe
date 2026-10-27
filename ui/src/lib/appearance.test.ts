import { describe, expect, it } from 'vitest';
import { caption, colors, gradient, listeningOpacity, listeningText } from './appearance';

describe('apparence de l’indicateur', () => {
  it('reprend les cinq couleurs de Scribe', () => {
    expect(colors.map((c) => c.id)).toEqual(['scribe', 'gray', 'red', 'blue', 'black']);
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
