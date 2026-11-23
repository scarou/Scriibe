import { afterEach, describe, expect, it, vi } from 'vitest';
import { api } from './api';
import main from '../main.ts?raw';

describe('envoi qui survit à la fermeture de la page', () => {
  afterEach(() => vi.unstubAllGlobals());

  it('api() ajoute keepalive seulement quand on le demande', async () => {
    const fetchMock = vi.fn(async () => ({ status: 200, json: async () => ({ ok: true }) }));
    vi.stubGlobal('fetch', fetchMock);
    await api('ui-state', { benchCompare: true }, { keepalive: true });
    await api('ui-state', { benchCompare: true });
    const calls = fetchMock.mock.calls as unknown as [string, RequestInit][];
    expect(calls[0][1].keepalive).toBe(true);
    expect(calls[1][1].keepalive).toBeUndefined();
  });

  it('la page vide ses états en attente à pagehide', () => {
    expect(main).toMatch(/addEventListener\('pagehide', \(\) => uiState\.leave\(\)\)/);
  });
});
