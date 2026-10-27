import { describe, expect, it } from 'vitest';
import type { Status } from './api';
import { buildReport } from './report';

const status = {
  version: '0.1.0 (abc1234)', hotkey: 'RightControl', transcription: 'Whisper', cleanup_label: 'llama.cpp', keys_error: null,
  whisper: { binary: 'C:\\w\\whisper-server.exe', models: ['C:\\m\\ggml-base.bin'], chosen: '' },
  llama: { binary: null, models: [], chosen: '' },
  microphones: ['Micro A'], llama_device: '', llama_log: { bytes: 0, hints: [], head: [], tail: [] },
} as unknown as Status;

describe('rapport', () => {
  it('résume l’état et le journal', () => {
    const r = buildReport(status, [], [], ['une ligne', 'deux lignes']);
    expect(r).toContain('Rapport Scriibe — version 0.1.0 (abc1234)');
    expect(r).toContain('modèles Whisper : ggml-base.bin');
    expect(r).toContain('llama-server : non');
    expect(r).toContain('appareil choisi « automatique »');
    expect(r.endsWith('une ligne\ndeux lignes\n')).toBe(true);
  });
  it('donne le détail d’une phrase du banc', () => {
    const result = {
      index: 2, category: 'Chiffres', text: 'Il est 19h.', translated: false,
      setup: { version: 'v', transcription: 'Whisper', cleanup: 'Sans modèle', mode: 'Simple', modeDetail: '', skipWhenClean: false, microphone: 'Micro' },
      take: { duration: 4.2, verdict: 'correct', level: 0.04, gain: 1 },
      out: {
        raw: 'il est dix-neuf heures', text: 'Il est 19 h.', messages: [], profile: { peak: 0.3, clipped: 0, levels: [0.1, 0.2] },
        timing: { transcription: 2.5, cleanup: 0.8, total: 3.4, outcome: 'cleaned' },
        checks: [{ label: 'heure', passed: true }, { label: 'espace', passed: false }],
      },
    };
    const r = buildReport(status, [result as never], [], []);
    expect(r).toContain('Phrase 3 (Chiffres) : 4,2 s');
    expect(r).toContain('collé    : Il est 19 h.');
    expect(r).toContain('contrôles: 1/2 — échec : espace');
    expect(r).toContain('Banc de dictée — v · Whisper · Sans modèle · Simple · Micro');
  });
});
