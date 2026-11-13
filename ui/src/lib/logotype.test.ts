import { describe, expect, it } from 'vitest';
import { logotype, shouldAnimate } from './logotype';

describe('logotype de la barre latérale', () => {
  it('donne les versions animées par défaut et les fixes si on le demande', () => {
    const animated = logotype('macos');
    const fixed = logotype('macos', false);
    expect(animated.light).not.toBe(fixed.light);
    expect(animated.dark).not.toBe(fixed.dark);
    expect(animated.light).not.toBe(animated.dark);
  });
  it('retombe sur Windows pour un système inconnu', () => {
    expect(logotype(undefined)).toEqual(logotype('windows'));
    expect(logotype('autre', false)).toEqual(logotype('windows', false));
  });
});

describe('le curseur clignote-t-il', () => {
  it('oui si la case est cochée ou absente et que le système ne réduit pas les animations', () => {
    expect(shouldAnimate(true, true, false)).toBe(true);
    expect(shouldAnimate(undefined, true, undefined)).toBe(true);
  });
  it('non si la case est décochée, si les réglages ne sont pas lus, ou si le système réduit les animations', () => {
    expect(shouldAnimate(false, true, false)).toBe(false);
    expect(shouldAnimate(undefined, false, false)).toBe(false);
    expect(shouldAnimate(true, true, true)).toBe(false);
  });
});
