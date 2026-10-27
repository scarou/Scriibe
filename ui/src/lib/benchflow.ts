import type { BenchSetup, Take, Timing, Transcribed } from './api';

/** Une phrase lue pendant un passage, avec ce que la chaîne en a fait. */
export interface BenchResult {
  index: number;
  category: string;
  text: string;
  out: Transcribed;
  take: Take;
  translated: boolean;
  setup: BenchSetup;
}

/** Un passage tel que le serveur le garde (même format que Scribe). */
export interface RunPayload {
  id: string;
  date: string;
  setup: BenchSetup;
  settingsChanged: boolean;
  entries: { category: string; text: string; raw: string; pasted: string; timing: Timing }[];
}

const sameSetup = (a: BenchSetup, b: BenchSetup) => JSON.stringify(a) === JSON.stringify(b);

/** Un identifiant écrit comme `UUID.uuidString` de Swift (majuscules). */
export function newRunId(): string {
  return crypto.randomUUID().toUpperCase();
}

/** Une date ISO à la seconde, comme les fichiers de Scribe. */
export function isoSeconds(date: Date): string {
  return date.toISOString().replace(/\.\d{3}Z$/, 'Z');
}

/**
 * Le déroulement d'un passage du banc (port de `ReadingBench` : phrase suivante, « Passer », « Relire », « Recommencer »).
 * Pas de dépendance à l'interface : testé tel quel.
 */
export class BenchFlow {
  phraseCount: number;
  running = false;
  current = 0;
  results: BenchResult[] = [];
  runId = newRunId();
  runDate = isoSeconds(new Date());

  constructor(phraseCount: number) {
    this.phraseCount = phraseCount;
  }

  /** Nouveau passage ; le précédent reste dans l'archive. */
  start(): void {
    this.runId = newRunId();
    this.runDate = isoSeconds(new Date());
    this.results = [];
    this.current = 0;
    this.running = true;
  }

  stop(): void {
    this.running = false;
  }

  /** Phrase suivante sans la lire. */
  skip(): void {
    if (!this.running) return;
    this.current += 1;
    if (this.current >= this.phraseCount) this.running = false;
  }

  /** Relire la dernière phrase lue : son résultat est retiré. */
  redoLast(): void {
    const last = this.results.pop();
    if (!last) return;
    this.current = last.index;
    this.running = true;
  }

  /** Une phrase lue. Ignorée si la phrase a changé entre-temps (Passer, Relire, Recommencer pendant le traitement). */
  record(result: BenchResult): boolean {
    if (!this.running || result.index !== this.current || this.current >= this.phraseCount) return false;
    this.results = [...this.results.filter((r) => r.index !== result.index), result];
    this.current += 1;
    if (this.current >= this.phraseCount) this.running = false;
    return true;
  }

  /** Le passage tel qu'il est gardé ; `null` tant qu'aucune phrase n'est lue. Réglages de la première phrase : s'ils diffèrent pour une
   * autre, le passage le note. */
  toRun(): RunPayload | null {
    if (this.results.length === 0) return null;
    const setup = this.results[0].setup;
    return {
      id: this.runId,
      date: this.runDate,
      setup,
      settingsChanged: this.results.some((r) => !sameSetup(r.setup, setup)),
      entries: this.results.map((r) => ({
        category: r.category,
        text: r.text,
        raw: r.out.raw ?? '',
        pasted: r.out.text ?? '',
        timing: r.out.timing as Timing,
      })),
    };
  }
}
