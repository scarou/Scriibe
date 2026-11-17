import { describe, expect, it } from 'vitest';
import { findMatches, foldQuery } from './helpsearch';

describe('recherche dans l’Aide', () => {
  it('ignore la casse et les accents, dans les deux sens', () => {
    expect(findMatches('Écris un Mail à Éric', foldQuery('ecris'))).toEqual([[0, 5]]);
    expect(findMatches('ecris un mail', foldQuery('ÉCRIS'))).toEqual([[0, 5]]);
    expect(findMatches('Reformulé et reformule', foldQuery('reformule'))).toEqual([[0, 9], [13, 22]]);
  });

  it('ignore le type d’apostrophe et les espaces insécables', () => {
    expect(findMatches('l’Aide', foldQuery("l'aide"))).toEqual([[0, 6]]);
    expect(findMatches('« Terminer »', foldQuery('« terminer'))).toEqual([[0, 10]]);
    expect(findMatches('Ctrl droit', foldQuery('ctrl droit'))).toEqual([[0, 10]]);
  });

  it('ne cherche pas avec moins de deux lettres, et ne trouve rien d’absent', () => {
    expect(foldQuery(' a ')).toBe('');
    expect(findMatches('bonjour', foldQuery('xyz'))).toEqual([]);
  });

  it('marche aussi en anglais', () => {
    expect(findMatches('Open the Dictionary', foldQuery('DICTIONARY'))).toEqual([[9, 19]]);
  });
});
