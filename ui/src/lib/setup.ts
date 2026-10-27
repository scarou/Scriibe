import type { BenchSetup } from './api';

/** Les réglages d'un passage en une ligne (comme `BenchSetup.summary` de Scribe) : version, moteurs, mode, micro. */
export function setupSummary(s: BenchSetup): string {
  return [s.version, s.transcription, s.cleanup, s.mode, s.skipWhenClean ? 'sauter si propre' : '', s.microphone].filter((x) => x).join(' · ');
}
