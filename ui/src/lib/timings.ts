import type { HistoryItem } from './api';
import { tx } from './i18n';

/** Ce qu'il est advenu de la mise au propre d'une dictée (mêmes libellés que Scribe, `DictationTiming.Cleanup`). */
export const outcomeLabels: Record<string, string> = {
  model: tx('Modèle'),
  skipped: tx('Sautée (déjà propre)'),
  short: tx('Courte, sans modèle'),
  none: tx('Sans modèle'),
  translated: tx('Traduite'),
  fallback: tx('Traduction écartée'),
  rejected: tx('Écartée par le garde-fou'),
  failed: tx('Échec du modèle'),
};

export function average(values: number[]): number | null {
  return values.length ? values.reduce((a, b) => a + b, 0) / values.length : null;
}

export interface TimingsSummary {
  /** Nombre de dictées mesurées (les 50 dernières au plus). */
  count: number;
  transcription: number | null;
  cleanup: number | null;
  totalWithModel: number | null;
  totalSkipped: number | null;
  skipped: number;
  /** Les dix dernières, pour le tableau. */
  rows: HistoryItem[];
}

/** Les temps des dernières dictées : du relâchement de la touche au collage (port de `TimingsCard`, Scribe). `items` : le plus récent d'abord. */
export function summarize(items: HistoryItem[]): TimingsSummary {
  const recent = items.filter((d) => d.timing).slice(0, 50);
  const timings = recent.map((d) => d.timing!);
  const model = timings.filter((t) => t.outcome === 'model');
  const skipped = timings.filter((t) => t.outcome === 'skipped');
  return {
    count: timings.length,
    transcription: average(timings.map((t) => t.transcription)),
    cleanup: average(model.map((t) => t.cleanup)),
    totalWithModel: average(model.map((t) => t.total)),
    totalSkipped: average(skipped.map((t) => t.total)),
    skipped: skipped.length,
    rows: recent.slice(0, 10),
  };
}
