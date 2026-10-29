import { describe, expect, it } from 'vitest';
import { base, compte, fmt, mo, percent, spoken, taille, when } from './format';

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


describe('tailles et nombres courts', () => {
  it('dit les Mo, puis les Go à partir de 1 Go', () => {
    expect(taille(147_951_465)).toBe('141 Mo');
    expect(taille(1_073_741_824)).toBe('1,0 Go');
    expect(taille(2_490_000_000)).toBe('2,3 Go');
  });

  it('abrège les nombres de téléchargements', () => {
    expect([compte(950), compte(1200), compte(12_345), compte(1_200_000), compte(3_000_000)]).toEqual(['950', '1,2 k', '12 k', '1,2 M', '3 M']);
  });
});

describe('formats en anglais', () => {
  it('met un point aux décimales et les unités en anglais', () => {
    expect(fmt(3.456, 'en')).toBe('3.46');
    expect(mo(465 * 1048576, 'en')).toBe('465 MB');
    expect(taille(2_490_000_000, 'en')).toBe('2.3 GB');
    expect(taille(147_951_465, 'en')).toBe('141 MB');
    expect(compte(12_300, 'en')).toBe('12 k');
    expect(compte(1_200_000, 'en')).toBe('1.2 M');
    expect(compte(2_000, 'en')).toBe('2 k');
    expect(compte(2_000)).toBe('2 k');
    expect(compte(1_250)).toBe('1,3 k');
  });
  it('écrit les dates avec « at »', () => {
    expect(when('2026-10-27T14:05:00', 'en')).toBe('27/10/2026 at 14:05');
  });
});
