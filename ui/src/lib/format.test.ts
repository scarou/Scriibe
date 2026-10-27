import { describe, expect, it } from 'vitest';
import { base, fmt, mo, percent, spoken, when } from './format';

describe('format', () => {
  it('dit les durées', () => {
    expect(spoken(45)).toBe('45 s');
    expect(spoken(12 * 60)).toBe('12 min');
    expect(spoken(3900)).toBe('1 h 05 min');
    expect(spoken(-3)).toBe('0 s');
  });
  it('écrit les dates à la française', () => {
    expect(when('2026-10-27T14:05:00')).toBe('27/10/2026 à 14:05');
    expect(when('pas une date')).toBe('pas une date');
  });
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
