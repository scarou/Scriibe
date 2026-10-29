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
  /** La phrase a été sautée : le modèle écrit sa version à côté, la phrase suivante attend. */
  comparing?: boolean;
  /** Ce que le modèle a écrit pour une phrase sautée. */
  modelText?: string | null;
  modelSeconds?: number | null;
  modelError?: string | null;
}

/** Un passage tel que le serveur le garde (même format que Scribe). */
export interface RunPayload {
  id: string;
  date: string;
  setup: BenchSetup;
  settingsChanged: boolean;
  entries: {
    category: string;
    text: string;
    raw: string;
    pasted: string;
    timing: Timing;
    wouldSkip: boolean;
    withoutModel: string;
    modelText?: string;
    modelSeconds?: number;
    modelError?: string;
  }[];
}

/** Le texte comparé sans les différences d'espaces (insécables, multiples) et d'apostrophes. */
export function normalizedText(text: string): string {
  return text.replace(/\u00A0/g, ' ').replace(/’/g, "'").split(/\s+/).filter(Boolean).join(' ');
}

/**
 * Les deux versions d'une phrase jugée propre : avec modèle, sans modèle (`null` si elle ne l'est pas, ou tant que la comparaison
 * n'est pas revenue). Port de `ReadingBench.Result.versions`.
 */
export function versions(r: BenchResult): { withModel: string; withoutModel: string } | null {
  if (!r.out.would_skip) return null;
  const pasted = r.out.text ?? '';
  switch (r.out.timing?.outcome) {
    case 'skipped':
      return r.modelText == null ? null : { withModel: r.modelText, withoutModel: pasted };
    case 'model':
      return { withModel: pasted, withoutModel: r.out.without_model ?? '' };
    default:
      return null;
  }
}

/** Phrases jugées propres où le modèle écrit autre chose que le texte sans modèle : ce que l'option ferait perdre, ou gagner. */
export function cleanButChanged(results: BenchResult[]): number {
  return results
    .map(versions)
    .filter((v) => v !== null && normalizedText(v.withModel) !== normalizedText(v.withoutModel)).length;
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
    if (this.comparing) return;
    const last = this.results.pop();
    if (!last) return;
    this.current = last.index;
    this.running = true;
  }

  /** Une comparaison avec le modèle tourne : la phrase suivante attend, pour que ses temps ne soient pas faussés. */
  get comparing(): boolean {
    return this.results.some((r) => r.comparing);
  }

  /** On ne veut plus attendre le modèle : la phrase suivante est libre. Sa version, si elle arrive plus tard, rejoindra quand même le passage. */
  giveUpComparing(index: number): boolean {
    const i = this.results.findIndex((r) => r.index === index && r.comparing);
    if (i < 0) return false;
    const next = [...this.results];
    next[i] = { ...next[i], comparing: false, modelError: 'pas attendue : le modèle n’avait pas fini (sa version est ajoutée si elle arrive)' };
    this.results = next;
    return true;
  }

  /** Ce que le modèle a écrit pour la phrase `index` du passage `runId` (ou pourquoi il n'a rien écrit). Un passage recommencé entre-temps l'ignore. */
  attachModel(index: number, text: string | null, seconds: number, error?: string, runId = this.runId): boolean {
    if (runId !== this.runId) return false;
    const i = this.results.findIndex((r) => r.index === index);
    if (i < 0) return false;
    const next = [...this.results];
    next[i] = { ...next[i], comparing: false, modelText: text, modelSeconds: text === null ? null : seconds, modelError: error ?? null };
    this.results = next;
    return true;
  }

  /** Une phrase lue. Ignorée si la phrase a changé entre-temps (Passer, Relire, Recommencer pendant le traitement). */
  record(result: BenchResult): boolean {
    if (!this.running || result.index !== this.current || this.current >= this.phraseCount) return false;
    this.results = [...this.results.filter((r) => r.index !== result.index), { ...result, comparing: result.out.compare === true }];
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
        wouldSkip: r.out.would_skip ?? false,
        withoutModel: r.out.without_model ?? '',
        ...(r.modelText != null ? { modelText: r.modelText } : {}),
        ...(r.modelSeconds != null ? { modelSeconds: r.modelSeconds } : {}),
        ...(r.modelError != null ? { modelError: r.modelError } : {}),
      })),
    };
  }
}
