import { describe, expect, it } from 'vitest';
import { setupSummary } from './setup';

describe('réglages d’un passage', () => {
  it('sont décrits comme dans Scribe', () => {
    expect(
      setupSummary({
        version: '1.12.0 (15)', transcription: 'Whisper · ggml-large-v3-turbo', cleanup: 'LM Studio · gemma-4-e4b-it@q4_0', mode: 'Simple',
        modeDetail: 'x', skipWhenClean: true, microphone: 'Micro du Mac',
      }),
    ).toBe('1.12.0 (15) · Whisper · ggml-large-v3-turbo · LM Studio · gemma-4-e4b-it@q4_0 · Simple · sauter si propre · Micro du Mac');
  });
  it('sautent les champs vides', () => {
    expect(setupSummary({ version: '', transcription: 'Whisper', cleanup: '', mode: '', modeDetail: '', skipWhenClean: false, microphone: '' })).toBe('Whisper');
  });
});
