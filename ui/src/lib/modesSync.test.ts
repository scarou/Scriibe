import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { watchModes } from './modesSync';

describe('page Modes : suit le mode choisi ailleurs (menu de l’icône)', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it('relit le mode à intervalles réguliers et ne prévient que si le choix a changé', async () => {
    let choice = 'simple';
    let shown = 'simple';
    const seen: string[] = [];
    const stop = watchModes(async () => ({ choice, active_label: choice }), (m) => { seen.push(m.choice); shown = m.choice; }, 1000, () => shown);
    await vi.advanceTimersByTimeAsync(3000);
    expect(seen).toEqual([]);
    choice = 'formal';
    await vi.advanceTimersByTimeAsync(1000);
    expect(seen).toEqual(['formal']);
    await vi.advanceTimersByTimeAsync(3000);
    expect(seen).toEqual(['formal']);
    stop();
    choice = 'tech';
    await vi.advanceTimersByTimeAsync(3000);
    expect(seen).toEqual(['formal']);
  });

  it('une erreur de lecture (serveur occupé) ne casse pas la surveillance', async () => {
    let fail = true;
    const seen: string[] = [];
    const stop = watchModes(
      async () => {
        if (fail) throw new Error('hors ligne');
        return { choice: 'calm', active_label: 'Apaisé' };
      },
      (m) => seen.push(m.choice),
      1000,
      () => 'simple',
    );
    await vi.advanceTimersByTimeAsync(2000);
    fail = false;
    await vi.advanceTimersByTimeAsync(1000);
    expect(seen).toEqual(['calm']);
    stop();
  });
});

describe('page Modes : choix fait dans la page pendant la surveillance', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it('revient au mode du menu même si la page avait déjà changé de mode entre deux lectures', async () => {
    const seen: string[] = [];
    const stop = watchModes(async () => ({ choice: 'simple', active_label: 'Simple' }), (m) => seen.push(m.choice), 1000, () => 'formal');
    await vi.advanceTimersByTimeAsync(1000);
    expect(seen).toEqual(['simple']);
    stop();
  });
});
