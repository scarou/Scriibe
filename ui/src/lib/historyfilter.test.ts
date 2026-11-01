import { describe, expect, it } from 'vitest';
import { matchesHistory, type HistoryFilter } from './historyfilter';

const now = new Date(2026, 9, 22, 15, 0);
const item = (day: number, app = 'Mail', text = 'Bonjour Marie') => ({ date: new Date(2026, 9, day, 9, 30).toISOString(), app, text, raw: text.toLowerCase() });
const base: HistoryFilter = { period: 'all', from: '', to: '', app: '', search: '' };

describe('filtres de l’historique', () => {
  it('sans filtre, tout passe', () => {
    expect(matchesHistory(item(1), base, now)).toBe(true);
  });

  it('« aujourd’hui » et « 7 jours » comptent le jour même', () => {
    expect(matchesHistory(item(22), { ...base, period: 'today' }, now)).toBe(true);
    expect(matchesHistory(item(21), { ...base, period: 'today' }, now)).toBe(false);
    expect(matchesHistory(item(16), { ...base, period: '7' }, now)).toBe(true);
    expect(matchesHistory(item(15), { ...base, period: '7' }, now)).toBe(false);
  });

  it('« 30 jours » remonte jusqu’à 29 jours avant aujourd’hui', () => {
    expect(matchesHistory({ ...item(1), date: new Date(2026, 8, 23, 8).toISOString() }, { ...base, period: '30' }, now)).toBe(true);
    expect(matchesHistory({ ...item(1), date: new Date(2026, 8, 22, 8).toISOString() }, { ...base, period: '30' }, now)).toBe(false);
  });

  it('des dates choisies : bornes comprises, une seule borne possible', () => {
    const f: HistoryFilter = { ...base, period: 'custom', from: '2026-10-10', to: '2026-10-12' };
    expect([9, 10, 12, 13].map((d) => matchesHistory(item(d), f, now))).toEqual([false, true, true, false]);
    expect(matchesHistory(item(5), { ...base, period: 'custom', from: '', to: '2026-10-12' }, now)).toBe(true);
    expect(matchesHistory(item(20), { ...base, period: 'custom', from: '2026-10-10', to: '' }, now)).toBe(true);
  });

  it('l’appli et les mots clés se combinent, sans tenir compte des majuscules', () => {
    expect(matchesHistory(item(3, 'Notes'), { ...base, app: 'Mail' }, now)).toBe(false);
    expect(matchesHistory(item(3), { ...base, app: 'Mail', search: 'MARIE bonjour' }, now)).toBe(true);
    expect(matchesHistory(item(3), { ...base, search: 'pierre' }, now)).toBe(false);
  });
});
