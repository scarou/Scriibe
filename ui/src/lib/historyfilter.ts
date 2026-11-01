/** Filtres de l'onglet Historique : période, appli et mots clés (aucun équivalent dans Scribe, voir `docs/ECARTS.md`). */

export type Period = 'all' | 'today' | '7' | '30' | 'custom';

export interface HistoryFilter {
  period: Period;
  /** Bornes d'une période choisie, au format `AAAA-MM-JJ` (jour local, bornes comprises) ; vide : pas de borne. */
  from: string;
  to: string;
  app: string;
  search: string;
}

/** `AAAA-MM-JJ` du jour local de l'instant donné. */
export function dayKey(date: Date): string {
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

/** Première journée affichée pour une période : « 7 jours » compte aujourd'hui (comme le filtre de Scribe). */
function firstDay(period: Period, now: Date, from: string): string {
  if (period === 'custom') return from;
  if (period === 'today') return dayKey(now);
  if (period === '7' || period === '30') {
    const start = new Date(now.getFullYear(), now.getMonth(), now.getDate() - (Number(period) - 1));
    return dayKey(start);
  }
  return '';
}

/** La dictée passe-t-elle les filtres ? Le texte cherché se compare sans tenir compte des majuscules, dans le texte final et le texte brut. */
export function matchesHistory(item: { date: string; app: string; text: string; raw: string }, filter: HistoryFilter, now: Date): boolean {
  if (filter.app && item.app !== filter.app) return false;
  const day = dayKey(new Date(item.date));
  const first = firstDay(filter.period, now, filter.from);
  if (first && day < first) return false;
  const last = filter.period === 'custom' ? filter.to : '';
  if (last && day > last) return false;
  const words = filter.search.trim().toLowerCase().split(/\s+/).filter(Boolean);
  if (words.length === 0) return true;
  const haystack = `${item.text} ${item.raw}`.toLowerCase();
  return words.every((w) => haystack.includes(w));
}
