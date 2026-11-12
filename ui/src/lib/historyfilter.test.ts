import { describe, expect, it } from 'vitest';
import { groupByDay, isRewritten, matchesHistory, type HistoryFilter } from './historyfilter';

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

  it('groupe par jour, le plus récent d’abord, et dans un jour la plus récente d’abord', () => {
    const at = (day: number, hour: number) => ({ date: new Date(2026, 9, day, hour).toISOString(), id: `${day}-${hour}` });
    const groups = groupByDay([at(20, 9), at(22, 8), at(22, 17), at(20, 18)]);
    expect(groups.map((g) => g.day)).toEqual(['2026-10-22', '2026-10-20']);
    expect(groups.map((g) => g.items.map((i) => i.id))).toEqual([['22-17', '22-8'], ['20-18', '20-9']]);
    expect(groupByDay([])).toEqual([]);
  });

  it('« reformulée » : le modèle a été retenu et le texte a changé', () => {
    const d = (outcome: string, raw: string, text: string, has_text = true) => ({ raw, text, has_text, timing: { outcome } });
    expect(isRewritten(d('model', 'bonjour marie', 'Bonjour Marie.'))).toBe(true);
    expect(isRewritten(d('model', 'Bonjour Marie.', 'Bonjour Marie.'))).toBe(false);
    expect(isRewritten(d('rejected', 'bonjour marie', 'bonjour marie'))).toBe(false);
    for (const outcome of ['skipped', 'short', 'none', 'failed', 'fallback']) expect(isRewritten(d(outcome, 'a b', 'A b.'))).toBe(false);
    expect(isRewritten(d('model', '', '', false))).toBe(true);
    expect(isRewritten({ raw: 'a', text: 'b', timing: null })).toBe(false);
  });

  it('le filtre « reformulées » se cumule avec la période, l’appli et les mots clés', () => {
    const rewritten = { ...item(22), raw: 'bonjour marie', timing: { outcome: 'model' } };
    const plain = { ...item(22), raw: 'Bonjour Marie', timing: { outcome: 'model' } };
    const only: HistoryFilter = { ...base, rewrittenOnly: true };
    expect(matchesHistory(rewritten, only, now)).toBe(true);
    expect(matchesHistory(plain, only, now)).toBe(false);
    expect(matchesHistory(plain, base, now)).toBe(true);
    expect(matchesHistory(rewritten, { ...only, period: 'today', app: 'Mail', search: 'marie' }, now)).toBe(true);
    expect(matchesHistory(rewritten, { ...only, period: 'today', app: 'Notes' }, now)).toBe(false);
    expect(matchesHistory({ ...rewritten, date: new Date(2026, 9, 3).toISOString() }, { ...only, period: 'today' }, now)).toBe(false);
    expect(matchesHistory(rewritten, { ...only, search: 'pierre' }, now)).toBe(false);
  });
});
