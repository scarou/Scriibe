import type { Phrase, Take, Transcribed } from './api';

export interface BenchResult {
  index: number;
  category: string;
  text: string;
  out: Transcribed;
  take: Take;
  translated: boolean;
}

/** Les résultats du banc de la session : le rapport les reprend. */
class BenchStore {
  phrases = $state<Phrase[]>([]);
  results = $state<BenchResult[]>([]);

  add(result: BenchResult): void {
    this.results = [...this.results.filter((r) => r.index !== result.index), result].sort((a, b) => a.index - b.index);
  }
}

export const bench = new BenchStore();
