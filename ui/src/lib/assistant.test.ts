import { describe, expect, it } from 'vitest';
import type { Status } from './api';
import { isInstalled, missing, type AssistantChain } from './assistant';

const status = (over: Partial<Status> = {}): Status =>
  ({
    whisper: { binary: 'C:\\w\\whisper-server.exe', models: ['C:\\m\\ggml-base.bin'], chosen: '' },
    llama: { binary: null, models: [], chosen: '' },
    keys: { deepgram: false, openrouter: false },
    ...over,
  }) as Status;

const cloud: AssistantChain = {
  id: 'cloud-cleanup', title: '', detail: '', timing: '', measured: true,
  needs: ['whisper-server', 'whisper-base', 'whisper-small'], needs_key: 'openrouter',
};

describe('assistant de premier lancement', () => {
  it('reconnaît ce qui est installé', () => {
    const s = status();
    expect(isInstalled('whisper-server', s)).toBe(true);
    expect(isInstalled('whisper-base', s)).toBe(true);
    expect(isInstalled('whisper-small', s)).toBe(false);
    expect(isInstalled('llama-server', s)).toBe(false);
    expect(isInstalled('whisper-server-linux', s)).toBe(true);
    expect(isInstalled('llama-server-linux', s)).toBe(false);
    expect(isInstalled('inconnu', s)).toBe(false);
  });
  it('dit ce qui manque, clé comprise', () => {
    expect(missing(cloud, status())).toEqual({ items: ['whisper-small'], key: true });
    const ready = status({ keys: { deepgram: false, openrouter: true } });
    expect(missing({ ...cloud, needs: ['whisper-server'] }, ready)).toEqual({ items: [], key: false });
  });
});
