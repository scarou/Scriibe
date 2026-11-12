/** Filtres de l'onglet Historique : période, appli, mots clés et dictées reformulées (aucun équivalent dans Scribe, voir `docs/ECARTS.md`). */

export type Period = 'all' | 'today' | '7' | '30' | 'custom';

export interface HistoryFilter {
  period: Period;
  /** Bornes d'une période choisie, au format `AAAA-MM-JJ` (jour local, bornes comprises) ; vide : pas de borne. */
  from: string;
  to: string;
  app: string;
  search: string;
  /** Ne garder que les dictées reformulées (voir `isRewritten`). */
  rewrittenOnly?: boolean;
}

/** Une dictée est « reformulée » quand la mise au propre par le modèle a été retenue et que le texte final diffère du brut. Écartée par le garde-fou (texte brut collé), sautée, courte ou en échec : non. Texte non gardé (rien à comparer) : reformulée dès que le modèle a répondu. */
export function isRewritten(item: { raw: string; text: string; has_text?: boolean; timing?: { outcome: string } | null }): boolean {
  if (item.timing?.outcome !== 'model') return false;
  return item.has_text === false || item.raw !== item.text;
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
export function matchesHistory(item: { date: string; app: string; text: string; raw: string; has_text?: boolean; timing?: { outcome: string } | null }, filter: HistoryFilter, now: Date): boolean {
  if (filter.app && item.app !== filter.app) return false;
  if (filter.rewrittenOnly && !isRewritten(item)) return false;
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

/** Les dictées groupées par jour local, du plus récent au plus ancien (et, dans un jour, la plus récente d'abord), comme l'Historique de Scribe. */
export function groupByDay<T extends { date: string }>(items: T[]): { day: string; items: T[] }[] {
  const groups = new Map<string, T[]>();
  for (const item of [...items].sort((a, b) => b.date.localeCompare(a.date))) {
    const key = dayKey(new Date(item.date));
    groups.set(key, [...(groups.get(key) ?? []), item]);
  }
  return [...groups].map(([day, list]) => ({ day, items: list }));
}
