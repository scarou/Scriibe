import { describe, expect, it } from 'vitest';
import {
  averageWpm, isComplete, isExact, marks, MINIMUM_SECONDS, percent, typingResult, typingTexts, typingTextsEn, wordCount,
} from './typing';

/** Les contrôles de frappe de `Checks.swift` (lignes 786 à 839 de Scribe). */
const ref = typingTexts[0];
const refWords = wordCount(ref);
const perfect = typingResult(ref, ref, 60);

describe('test de frappe', () => {
  it('texte juste en une minute = ses mots par minute', () => {
    expect(perfect.wpm).toBe(refWords);
    expect(perfect.accuracy).toBe(1);
    expect(perfect.rejection).toBeUndefined();
  });
  it('deux fois plus vite, deux fois plus de mots', () => {
    expect(typingResult(ref, ref, 30).wpm).toBe(refWords * 2);
  });
  it('apostrophes courbes et espaces insécables équivalents', () => {
    expect(typingResult(ref, ref.replaceAll("'", '’').replaceAll(' ', ' '), 60).accuracy).toBe(1);
  });
  it('accents décomposés équivalents', () => {
    expect(isExact(ref, ref.normalize('NFD'))).toBe(true);
  });
  const sloppyText = Array.from(ref).map((c, i) => (i % 4 === 0 ? '#' : c)).join('');
  const sloppy = typingResult(ref, sloppyText, 60);
  it('trop de fautes, essai non retenu', () => {
    expect(sloppy.rejection).toBe('inaccurate');
    expect(sloppy.wpm).toBeLessThan(perfect.wpm);
  });
  it('seuil de 90 % : 9 lettres sur 10 retenues, 8 sur 10 non', () => {
    expect(typingResult('aaaaaaaaaa', 'aaaaaaaaab', 60).rejection).toBeUndefined();
    expect(typingResult('aaaaaaaaaa', 'aaaaaaaabb', 60).rejection).toBe('inaccurate');
    expect(percent(448, 500)).toBe(89);
    expect(percent(145, 250)).toBe(58);
  });
  it('essai de 3 s ou de 11 min non retenu', () => {
    expect(typingResult(ref, ref, 3).rejection).toBe('tooShort');
    expect(typingResult(ref, ref, 660).rejection).toBe('tooLong');
  });
  it('plus de 200 mots/min (frappe automatique) non retenu', () => {
    expect(typingResult(ref, ref, Math.max((refWords * 60) / 250, MINIMUM_SECONDS)).rejection).toBe('tooFast');
    expect(typingResult(ref, ref, (refWords * 60) / 190).rejection).toBeUndefined();
  });
  it('texte pas tapé jusqu’au bout non retenu ; fini seulement quand il est identique', () => {
    expect(typingResult(ref, ref.slice(0, 40), 60).rejection).toBe('incomplete');
    expect(isComplete(ref, ref.slice(0, 40))).toBe(false);
    expect(isComplete(ref, ref)).toBe(true);
    expect(isExact(ref, ref)).toBe(true);
    expect(isExact(ref, ref + '\n')).toBe(false);
    expect(isExact(ref, sloppyText)).toBe(false);
  });
  it('caractères en trop comptés comme des fautes', () => {
    expect(typingResult('abcd', 'abcdefgh', 60).accuracy).toBe(0.5);
  });
  it('lettres justes, fausses, restantes', () => {
    expect(marks('abc', 'ax')).toEqual(['correct', 'wrong', 'pending']);
  });
  it('moyenne des essais retenus seulement', () => {
    expect(averageWpm([perfect, sloppy, typingResult(ref, ref, 30)])).toBe(Math.round(refWords * 1.5));
    expect(averageWpm([sloppy])).toBeNull();
    expect(averageWpm([])).toBeNull();
  });
  it('moyenne bornée à 150 mots/min, comme le réglage', () => {
    expect(averageWpm([typingResult(ref, ref, (refWords * 60) / 180)])).toBe(150);
  });
  it('textes distincts et d’une longueur raisonnable', () => {
    expect(new Set(typingTexts).size).toBe(typingTexts.length);
    for (const t of typingTexts) expect(wordCount(t)).toBeGreaterThanOrEqual(35);
    for (const t of typingTexts) expect(wordCount(t)).toBeLessThanOrEqual(70);
    // Les textes anglais : mêmes bornes, même nombre.
    expect(typingTextsEn.length).toBe(typingTexts.length);
    expect(new Set(typingTextsEn).size).toBe(typingTextsEn.length);
    for (const t of typingTextsEn) expect(wordCount(t)).toBeGreaterThanOrEqual(35);
    for (const t of typingTextsEn) expect(wordCount(t)).toBeLessThanOrEqual(70);
  });
});
