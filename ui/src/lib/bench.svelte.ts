import { api, type Phrase } from './api';
import { BenchFlow, type BenchResult } from './benchflow';

export type { BenchResult } from './benchflow';

/** Le passage du banc en cours dans cette session, gardé sur le disque phrase après phrase (comme Scribe) : le rapport le reprend. */
class BenchStore {
  phrases = $state<Phrase[]>([]);
  /** Compteur lu par les accesseurs : chaque changement du déroulement le fait avancer, ce qui met l'interface à jour. */
  private tick = $state(0);
  private flow = new BenchFlow(0);
  /** Le passage a été supprimé dans l'archive : la suite n'est plus gardée. */
  deleted = $state(false);
  /** Incrémenté à chaque enregistrement du passage : l'archive se recharge. */
  saved = $state(0);

  setPhrases(list: Phrase[]): void {
    this.phrases = list;
    this.flow.phraseCount = list.length;
    this.touch();
  }

  private touch(): void {
    this.tick++;
  }

  get running(): boolean {
    this.tick;
    return this.flow.running;
  }

  get current(): number {
    this.tick;
    return this.flow.current;
  }

  get results(): BenchResult[] {
    this.tick;
    return [...this.flow.results].sort((a, b) => a.index - b.index);
  }

  /** Vrai quand il y a un passage (en cours ou fini) à montrer. */
  get active(): boolean {
    this.tick;
    return this.flow.running || this.flow.results.length > 0;
  }

  start(): void {
    this.flow.start();
    this.deleted = false;
    this.touch();
  }

  stop(): void {
    this.flow.stop();
    this.touch();
  }

  skip(): void {
    this.flow.skip();
    this.touch();
  }

  async redoLast(): Promise<void> {
    this.flow.redoLast();
    this.touch();
    await this.persist();
  }

  async record(result: BenchResult): Promise<boolean> {
    const kept = this.flow.record(result);
    this.touch();
    if (kept) await this.persist();
    return kept;
  }

  /** Garde le passage sur le disque ; un passage devenu vide (« Relire » jusqu'à la première phrase) disparaît de l'archive. */
  private async persist(): Promise<void> {
    if (this.deleted) return;
    const run = this.flow.toRun();
    if (!run) {
      await api('bench/discard', { id: this.flow.runId });
    } else {
      const out = await api<{ deleted: boolean }>('bench/save', run);
      if (out.deleted) this.deleted = true;
    }
    this.saved++;
  }
}

export const bench = new BenchStore();
