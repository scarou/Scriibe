import { afterEach, describe, expect, it, vi } from 'vitest';
import { savedFold, saveFold } from './folds';
import { uiState } from './uiState';

describe('cartes repliées', () => {
  afterEach(() => {
    uiState.state = { folds: {} };
    vi.restoreAllMocks();
  });

  it('rend l’état retenu, ou null pour une carte jamais touchée', () => {
    uiState.state = { folds: { a: true, b: false } };
    expect(savedFold('a')).toBe(true);
    expect(savedFold('b')).toBe(false);
    expect(savedFold('c')).toBeNull();
  });

  it('une clé héritée de l’objet (constructor, toString) n’est pas une carte retenue', () => {
    expect(savedFold('constructor')).toBeNull();
    expect(savedFold('toString')).toBeNull();
    expect(savedFold('__proto__')).toBeNull();
  });

  it('retient le pli en mémoire et l’envoie', () => {
    const save = vi.spyOn(uiState, 'save').mockImplementation(() => {});
    saveFold('modes-mes-modes', true);
    expect(save).toHaveBeenCalledWith({ folds: { 'modes-mes-modes': true } });
  });
});
