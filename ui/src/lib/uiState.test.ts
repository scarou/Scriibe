import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanUiState, UiStateCache } from './uiState';

/** Un faux programme : garde ce qu'on lui envoie, comme le fichier d'états de Scriibe (et le rend à la lecture suivante). */
function fakeServer(initial: unknown = {}) {
  let kept: any = initial;
  const sent: unknown[] = [];
  const send = async (path: string, body?: unknown) => {
    expect(path).toBe('ui-state');
    if (body === undefined) return kept;
    sent.push(body);
    const b = body as any;
    kept = { ...kept, ...b, folds: { ...kept.folds, ...b.folds } };
    return kept;
  };
  return { send, sent, kept: () => kept };
}

describe('états d’affichage', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it('retrouve après un redémarrage les cartes repliées, la case du banc, la hauteur du Dictionnaire et la langue', async () => {
    const server = fakeServer();
    const first = new UiStateCache(server.send);
    await first.load();
    first.save({ folds: { 'modes-mes-modes': true, utilisateurs: false } });
    first.save({ benchCompare: false });
    first.save({ dictionaryHeight: 420 });
    first.save({ uiLanguage: 'en' });
    await vi.runAllTimersAsync();
    // Nouveau lancement : une autre mémoire, le même fichier.
    const second = new UiStateCache(server.send);
    await second.load();
    expect(second.state).toEqual({ folds: { 'modes-mes-modes': true, utilisateurs: false }, benchCompare: false, dictionaryHeight: 420, uiLanguage: 'en' });
  });

  it('applique le changement en mémoire tout de suite, même sans réponse du programme', async () => {
    const cache = new UiStateCache(() => Promise.reject(new Error('hors ligne')));
    await cache.load();
    expect(cache.state).toEqual({ folds: {} });
    cache.save({ folds: { a: true }, benchCompare: false });
    await vi.runAllTimersAsync();
    expect(cache.state.folds.a).toBe(true);
    expect(cache.state.benchCompare).toBe(false);
  });

  it('null retire une carte ou la hauteur', async () => {
    const cache = new UiStateCache(fakeServer().send);
    cache.save({ folds: { a: true, b: false }, dictionaryHeight: 300 });
    cache.save({ folds: { a: null }, dictionaryHeight: null });
    expect(cache.state).toEqual({ folds: { b: false } });
  });

  it('regroupe en un seul envoi les changements d’un geste continu (glissement de la poignée)', async () => {
    const server = fakeServer();
    const cache = new UiStateCache(server.send);
    for (const h of [300, 320, 340]) cache.save({ dictionaryHeight: h }, 300);
    await vi.advanceTimersByTimeAsync(299);
    expect(server.sent).toEqual([]);
    await vi.advanceTimersByTimeAsync(2);
    expect(server.sent).toEqual([{ dictionaryHeight: 340, folds: {} }]);
  });

  it('flush envoie tout de suite ce qui attend', async () => {
    const server = fakeServer();
    const cache = new UiStateCache(server.send);
    cache.save({ dictionaryHeight: 250 }, 5000);
    cache.flush();
    await vi.advanceTimersByTimeAsync(0);
    expect(server.sent.length).toBe(1);
    await vi.advanceTimersByTimeAsync(6000);
    expect(server.sent.length).toBe(1);
  });

  it('un envoi qui échoue n’empêche pas les suivants', async () => {
    let calls = 0;
    const cache = new UiStateCache(async () => {
      calls++;
      if (calls === 1) throw new Error('non');
      return {};
    });
    cache.save({ benchCompare: true });
    cache.save({ benchCompare: false });
    await vi.runAllTimersAsync();
    expect(calls).toBe(2);
  });

  it('une réponse illisible ou une valeur de mauvais type donne les valeurs par défaut', async () => {
    for (const raw of [null, 'texte', [1], 12, undefined]) expect(cleanUiState(raw)).toEqual({ folds: {} });
    expect(cleanUiState({ folds: { a: 1, b: true }, benchCompare: 'oui', dictionaryHeight: 'haut', uiLanguage: 'de' })).toEqual({ folds: { b: true } });
    const cache = new UiStateCache(async () => 'pas un état');
    await cache.load();
    expect(cache.state).toEqual({ folds: {} });
  });

  it('n’attend pas un programme qui ne répond pas : états par défaut au bout de 2 s, réponse tardive ignorée', async () => {
    let answer: (v: unknown) => void = () => {};
    const cache = new UiStateCache(() => new Promise((resolve) => (answer = resolve)));
    cache.state = { folds: { vieux: true } };
    let done = false;
    const loading = cache.load().then(() => (done = true));
    await vi.advanceTimersByTimeAsync(1999);
    expect(done).toBe(false);
    await vi.advanceTimersByTimeAsync(2);
    await loading;
    expect(done).toBe(true);
    expect(cache.state).toEqual({ folds: {} });
    cache.save({ benchCompare: false });
    answer({ benchCompare: true, folds: { tard: true } });
    await vi.runAllTimersAsync();
    expect(cache.state).toEqual({ folds: {}, benchCompare: false });
  });

  it('une réponse rapide annule le délai (aucun minuteur ne reste)', async () => {
    const cache = new UiStateCache(async () => ({ benchCompare: false }));
    await cache.load();
    expect(cache.state.benchCompare).toBe(false);
    expect(vi.getTimerCount()).toBe(0);
  });

  it('les envois partent à la suite : le second attend la réponse du premier', async () => {
    const calls: string[] = [];
    const releases: (() => void)[] = [];
    const cache = new UiStateCache((_path, body) => {
      calls.push(JSON.stringify(body));
      return new Promise<void>((resolve) => releases.push(resolve));
    });
    cache.save({ benchCompare: true });
    cache.save({ benchCompare: false });
    await vi.advanceTimersByTimeAsync(0);
    expect(calls).toEqual(['{"benchCompare":true}']);
    releases[0]();
    await vi.advanceTimersByTimeAsync(0);
    expect(calls.length).toBe(2);
    expect(calls[1]).toBe('{"benchCompare":false}');
  });

  it('à la fermeture de la page, un changement différé part tout de suite par l’envoi de dernière chance', async () => {
    const server = fakeServer();
    const leaving: unknown[] = [];
    const cache = new UiStateCache(server.send, async (path, body) => {
      leaving.push(body);
      return server.send(path, body);
    });
    await cache.load();
    cache.save({ dictionaryHeight: 500 }, 300);
    expect(server.sent).toEqual([]);
    cache.leave();
    expect(leaving).toEqual([{ dictionaryHeight: 500 }]);
    await vi.runAllTimersAsync();
    expect(server.sent).toEqual([{ dictionaryHeight: 500 }]);
    expect(server.kept().dictionaryHeight).toBe(500);
    cache.leave();
    expect(leaving).toHaveLength(1);
  });

  it('la fermeture n’attend pas un envoi en cours qui ne répond pas, et ne lève rien', async () => {
    const leaving: unknown[] = [];
    const cache = new UiStateCache(
      () => new Promise(() => undefined),
      () => {
        throw new Error('page fermée');
      },
    );
    cache.save({ benchCompare: true });
    cache.save({ benchCompare: false }, 300);
    expect(() => cache.leave()).not.toThrow();
    expect(leaving).toEqual([]);
  });
});
