import { describe, expect, it } from 'vitest';
import type { BenchSetup, Take, Transcribed } from './api';
import { BenchFlow, isoSeconds, newRunId, type BenchResult } from './benchflow';

const setup = (version = '1'): BenchSetup => ({ version, transcription: 'Whisper · ggml-base', cleanup: 'Sans modèle', mode: 'Simple', modeDetail: '', skipWhenClean: false, microphone: 'Micro' });

const result = (index: number, s = setup()): BenchResult => ({
  index, category: 'Propre', text: `phrase ${index}`, translated: false, setup: s,
  take: { duration: 3 } as Take,
  out: { raw: 'brut', text: 'collé', timing: { transcription: 2, cleanup: 0, total: 2.2, outcome: 'none' } } as unknown as Transcribed,
});

describe('déroulement du banc', () => {
  it('une phrase après l’autre, le passage s’arrête à la dernière', () => {
    const flow = new BenchFlow(3);
    expect(flow.record(result(0))).toBe(false); // pas démarré
    flow.start();
    expect(flow.running && flow.current === 0).toBe(true);
    expect(flow.record(result(0))).toBe(true);
    expect(flow.record(result(0))).toBe(false); // la phrase a changé entre-temps
    expect(flow.record(result(1))).toBe(true);
    expect(flow.record(result(2))).toBe(true);
    expect(flow.running).toBe(false);
  });

  it('« Passer » avance sans résultat ; « Relire » retire le dernier résultat et revient à sa phrase', () => {
    const flow = new BenchFlow(3);
    flow.start();
    flow.record(result(0));
    flow.skip();
    expect(flow.current).toBe(2);
    expect(flow.results.length).toBe(1);
    flow.redoLast();
    expect(flow.current).toBe(0);
    expect(flow.results.length).toBe(0);
    expect(flow.toRun()).toBeNull();
    flow.redoLast(); // rien à relire : sans effet
    expect(flow.current).toBe(0);
  });

  it('le passage garde les réglages de la première phrase et note un changement en cours de route', () => {
    const flow = new BenchFlow(3);
    flow.start();
    flow.record(result(0, setup('a')));
    expect(flow.toRun()?.settingsChanged).toBe(false);
    flow.record(result(1, setup('b')));
    const run = flow.toRun()!;
    expect(run.setup.version).toBe('a');
    expect(run.settingsChanged).toBe(true);
    expect(run.entries.map((e) => e.pasted)).toEqual(['collé', 'collé']);
    flow.redoLast();
    expect(flow.toRun()?.settingsChanged).toBe(false); // oublié après « Relire » de la phrase en cause
  });

  it('un nouveau passage a un nouvel identifiant, écrit comme un UUID de Swift', () => {
    const flow = new BenchFlow(2);
    const first = flow.runId;
    flow.start();
    expect(flow.runId).not.toBe(first);
    expect(flow.runId).toMatch(/^[0-9A-F]{8}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{12}$/);
    expect(newRunId()).toMatch(/^[0-9A-F-]{36}$/);
    expect(isoSeconds(new Date('2026-10-27T14:05:09.123Z'))).toBe('2026-10-27T14:05:09Z');
  });
});
