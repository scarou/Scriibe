import { describe, expect, it } from 'vitest';
import { heardKey, saveError, storedDictionary, withCorrection } from './dictionary';

describe('heardKey', () => {
  it('ignore la casse, les espaces multiples et la forme de l’apostrophe', () => {
    expect(heardKey('  Cloud   CODE ')).toBe('cloud code');
    expect(heardKey('l’avion')).toBe(heardKey("L'avion"));
  });
});

describe('withCorrection', () => {
  const list = [
    { heard: 'cloud code', written: 'Claude Code' },
    { heard: 'cri be', written: 'Scriibe' },
  ];

  it('ajoute une correction nouvelle à la fin', () => {
    expect(withCorrection(list, { heard: 'tauri', written: 'Tauri' })).toHaveLength(3);
  });

  it('remplace la valeur d’un « entendu » déjà présent, à la même place, au lieu de doubler la ligne', () => {
    const out = withCorrection(list, { heard: 'Cloud  Code', written: 'Claude code' });
    expect(out).toHaveLength(2);
    expect(out[0]).toEqual({ heard: 'Cloud  Code', written: 'Claude code' });
    expect(out[1]).toBe(list[1]);
  });
});

describe('réponse à un enregistrement', () => {
  it('rend le dictionnaire quand le serveur l’a enregistré', () => {
    const ok = { words: ['Scriibe'], corrections: [{ heard: 'cri be', written: 'Scriibe' }] };
    expect(storedDictionary(ok)).toBe(ok);
    expect(saveError(ok)).toBeUndefined();
  });

  it('ne prend pas une réponse d’erreur pour un dictionnaire (elle casserait la page)', () => {
    const refused = { ok: false as const, error: 'Le fichier dictionary.json est illisible' };
    expect(storedDictionary(refused)).toBeNull();
    expect(saveError(refused)).toBe('Le fichier dictionary.json est illisible');
    expect(storedDictionary({ ok: false })).toBeNull();
  });
});
