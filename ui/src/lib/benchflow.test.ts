import { describe, expect, it } from 'vitest';
import type { BenchSetup, Take, Transcribed } from './api';
import { BenchFlow, cleanButChanged, isoSeconds, newRunId, normalizedText, versions, type BenchResult } from './benchflow';

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

  const skipped = (index: number, modelText?: string): BenchResult => ({
    ...result(index),
    out: {
      raw: 'brut', text: 'Je serai là.', would_skip: true, without_model: 'Je serai là.', compare: true,
      timing: { transcription: 2, cleanup: 0, total: 2.2, outcome: 'skipped' },
    } as unknown as Transcribed,
    ...(modelText === undefined ? {} : { modelText }),
  });

  it('une phrase sautée attend la version du modèle, qui rejoint le passage gardé', () => {
    const flow = new BenchFlow(3);
    flow.start();
    expect(flow.record(skipped(0))).toBe(true);
    expect(flow.comparing).toBe(true);
    flow.redoLast(); // refusé pendant la comparaison
    expect(flow.results.length).toBe(1);
    expect(versions(flow.results[0])).toBeNull();
    expect(flow.attachModel(0, 'Je serai là !', 1.25)).toBe(true);
    expect(flow.comparing).toBe(false);
    const entry = flow.toRun()!.entries[0];
    expect(entry).toMatchObject({ wouldSkip: true, withoutModel: 'Je serai là.', modelText: 'Je serai là !', modelSeconds: 1.25 });
    expect(entry).not.toHaveProperty('modelError');
    expect(flow.attachModel(5, 'x', 1)).toBe(false);
  });

  it('un modèle qui n’a rien écrit garde son erreur, sans texte ni durée', () => {
    const flow = new BenchFlow(2);
    flow.start();
    flow.record(skipped(0));
    flow.attachModel(0, null, 0, 'écarté par le garde-fou');
    expect(flow.toRun()!.entries[0]).toMatchObject({ modelError: 'écarté par le garde-fou' });
    expect(flow.toRun()!.entries[0]).not.toHaveProperty('modelText');
  });

  it('on compte les phrases jugées propres où le modèle écrit autre chose (espaces et apostrophes ignorés)', () => {
    const same = { ...skipped(0, 'Je  serai là.') };
    const different = { ...skipped(1, 'Je serai là !') };
    const modelled: BenchResult = {
      ...result(2),
      out: { text: 'Texte corrigé.', would_skip: true, without_model: 'Texte corigé.', timing: { transcription: 1, cleanup: 1, total: 2, outcome: 'model' } } as unknown as Transcribed,
    };
    const toRedo: BenchResult = { ...result(3), out: { text: 'x', would_skip: false, without_model: 'y', timing: { transcription: 1, cleanup: 1, total: 2, outcome: 'model' } } as unknown as Transcribed };
    expect(cleanButChanged([same, different, modelled, toRedo])).toBe(2);
    expect(normalizedText('l’été\u00A0 ?')).toBe("l'été ?");
  });

  it('« Ne pas attendre » libère la phrase suivante, et une version tardive rejoint son passage mais pas un autre', () => {
    const flow = new BenchFlow(3);
    flow.start();
    const run = flow.runId;
    flow.record(skipped(0));
    expect(flow.comparing).toBe(true);
    expect(flow.giveUpComparing(0)).toBe(true);
    expect(flow.comparing).toBe(false);
    expect(flow.giveUpComparing(0)).toBe(false);
    expect(flow.toRun()!.entries[0]).toHaveProperty('modelError');
    expect(flow.attachModel(0, 'Je serai là !', 30, undefined, run)).toBe(true);
    expect(flow.toRun()!.entries[0]).toMatchObject({ modelText: 'Je serai là !' });

    flow.start(); // nouveau passage : la réponse d'un ancien passage est ignorée
    flow.record(skipped(0));
    expect(flow.attachModel(0, 'ancien', 1, undefined, run)).toBe(false);
    expect(flow.comparing).toBe(true);
  });
});
