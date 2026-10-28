import { describe, expect, it } from 'vitest';
import type { HistoryItem } from './api';
import { average, outcomeLabels, summarize } from './timings';

function item(transcription: number, cleanup: number, total: number, outcome: string): HistoryItem {
  return { id: String(Math.random()), timing: { transcription, cleanup, total, outcome } } as unknown as HistoryItem;
}

describe('temps des dictées', () => {
  it('fait la moyenne, vide : rien', () => {
    expect(average([])).toBeNull();
    expect(average([1, 2, 3])).toBe(2);
  });

  it('sépare les dictées passées par le modèle de celles qui ont été sautées', () => {
    const s = summarize([item(3, 1, 4, 'model'), item(3, 0, 3, 'skipped'), item(1, 1, 2, 'model'), { id: 'x', timing: null } as unknown as HistoryItem]);
    expect(s.count).toBe(3);
    expect(s.transcription).toBeCloseTo(7 / 3);
    expect(s.cleanup).toBe(1);
    expect(s.totalWithModel).toBe(3);
    expect(s.totalSkipped).toBe(3);
    expect(s.skipped).toBe(1);
  });

  it('ne garde que les cinquante dernières et affiche les dix premières', () => {
    const many = Array.from({ length: 80 }, (_, i) => item(i, 0, i, 'model'));
    const s = summarize(many);
    expect(s.count).toBe(50);
    expect(s.rows).toHaveLength(10);
    expect(s.rows[0]).toBe(many[0]);
  });

  it('a un libellé pour chaque décision de Scribe', () => {
    expect(Object.keys(outcomeLabels)).toEqual(['model', 'skipped', 'short', 'none', 'translated', 'fallback', 'rejected', 'failed']);
  });
});
