import { describe, expect, it } from 'vitest';
import { base, fmt, mo, percent } from './format';

describe('format', () => {
  it('écrit les décimales à la française', () => {
    expect(fmt(3.456)).toBe('3,46');
    expect(fmt(2)).toBe('2');
  });
  it('arrondit les tailles en Mo', () => {
    expect(mo(465 * 1048576)).toBe('465 Mo');
  });
  it('garde le nom du fichier, chemin Windows ou Unix', () => {
    expect(base('C:\\Users\\a\\models\\ggml-base.bin')).toBe('ggml-base.bin');
    expect(base('/home/a/m.gguf')).toBe('m.gguf');
    expect(base(null)).toBe('');
  });
  it('ne divise pas par un total inconnu', () => {
    expect(percent(5, 0)).toBe(0);
    expect(percent(50, 200)).toBe(25);
    expect(percent(300, 200)).toBe(100);
  });
});
