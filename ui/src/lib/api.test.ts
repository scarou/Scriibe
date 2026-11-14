import { afterEach, describe, expect, it, vi } from 'vitest';
import { STALE_FLAG, api, getKnownProfile, setKnownProfile, withProfile } from './api';

describe('utilisateur actif envoyé avec les requêtes (étape 4c)', () => {
  afterEach(() => {
    setKnownProfile(undefined);
    vi.unstubAllGlobals();
  });

  it('ajoute l’utilisateur connu aux corps d’objet, sans rien changer d’autre', () => {
    expect(withProfile({ words: ['a'] }, 'AAAA')).toEqual({ words: ['a'], profile: 'AAAA' });
    expect(withProfile({ theme: 'dark' }, null)).toEqual({ theme: 'dark', profile: null });
  });

  it('laisse tels quels les corps sans objet, ceux qui disent déjà un utilisateur, et tout tant que l’utilisateur est inconnu', () => {
    expect(withProfile([1], 'AAAA')).toEqual([1]);
    expect(withProfile(undefined, 'AAAA')).toBeUndefined();
    expect(withProfile({ profile: 'BBBB' }, 'AAAA')).toEqual({ profile: 'BBBB' });
    expect(withProfile({ a: 1 }, undefined)).toEqual({ a: 1 });
  });

  it('api() envoie l’utilisateur connu dans les requêtes POST, pas dans les lectures', async () => {
    const sent: { url: string; body?: string }[] = [];
    vi.stubGlobal('fetch', async (url: string, init?: RequestInit) => {
      sent.push({ url, body: init?.body as string | undefined });
      return { status: 200, json: async () => ({ ok: true }) };
    });
    setKnownProfile('AAAA');
    await api('dictionary', { words: ['x'], corrections: [] });
    await api('dictionary');
    expect(JSON.parse(sent[0].body!)).toEqual({ words: ['x'], corrections: [], profile: 'AAAA' });
    expect(sent[1].body).toBeUndefined();
  });

  it('une réponse 409 « stale » pose la marque et recharge la page, un autre 409 non', async () => {
    const store = new Map<string, string>();
    vi.stubGlobal('sessionStorage', { setItem: (k: string, v: string) => store.set(k, v), getItem: (k: string) => store.get(k) ?? null });
    const reload = vi.fn();
    vi.stubGlobal('location', { reload });
    vi.stubGlobal('fetch', async () => ({ status: 409, json: async () => ({ ok: false, error: 'autre chose' }) }));
    await api('record/start', {});
    expect(reload).not.toHaveBeenCalled();
    vi.stubGlobal('fetch', async () => ({ status: 409, json: async () => ({ ok: false, stale: true }) }));
    const out = await api('dictionary', { words: [], corrections: [] });
    expect(out.stale).toBe(true);
    expect(reload).toHaveBeenCalledTimes(1);
    expect(store.get(STALE_FLAG)).toBe('1');
  });

  it('l’utilisateur connu se lit et s’efface', () => {
    expect(getKnownProfile()).toBeUndefined();
    setKnownProfile(null);
    expect(getKnownProfile()).toBeNull();
  });
});
