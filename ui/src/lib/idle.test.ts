import { describe, expect, it } from 'vitest';
import { idleChoices, idleOptions } from './idle';

describe('durées de veille des moteurs', () => {
  it('propose 1, 2, 5, 10, 15, 30, 60 min puis « Jamais » (0)', () => {
    expect(idleOptions(10)).toEqual([1, 2, 5, 10, 15, 30, 60, 0]);
    expect(idleChoices.at(-1)).toBe(0);
  });
  it('garde un ancien réglage de llama.cpp (2 h, 8 h) dans le menu, avant « Jamais »', () => {
    expect(idleOptions(120)).toEqual([1, 2, 5, 10, 15, 30, 60, 120, 0]);
    expect(idleOptions(480)).toEqual([1, 2, 5, 10, 15, 30, 60, 480, 0]);
  });
});
