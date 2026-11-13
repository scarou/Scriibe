import { describe, expect, it } from 'vitest';
import { logotype } from './logotype';

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
