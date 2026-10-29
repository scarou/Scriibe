import texts from './typing-texts.json';
import textsEn from './typing-texts-en.json';
import { tx } from './i18n';

/** Test de vitesse de frappe : port de `TypingTest.swift` (Scribe). La vitesse mesurée peut remplacer le réglage « Ma vitesse au clavier »
 * (`typingWPM`) qui sert au temps gagné du tableau de bord. */

/** Textes courts, sans chiffres ni majuscules rares, avec des accents et quelques apostrophes comme en vraie rédaction.
 * Extraits de `TypingTest.swift` par `tools/extraire_frappe.py`, jamais retapés. */
export const typingTexts: string[] = texts;

/** Les textes en anglais, pour l'interface en anglais : le test de frappe ne passe pas par Whisper, il suit la langue de l'interface (écart voulu, `docs/ECARTS.md`). */
export const typingTextsEn: string[] = textsEn;

/** Sous ce taux de caractères justes, la vitesse n'est pas retenue : taper n'importe quoi vite ne mesure rien. */
export const MINIMUM_ACCURACY = 0.9;
/** Hors de cette durée, le chrono n'a pas de sens (frappe automatique, essai oublié ouvert). */
export const MINIMUM_SECONDS = 12;
export const MAXIMUM_SECONDS = 600;
/** Au-dessus, ce n'est plus de la frappe : essai non retenu (le record du monde est autour de 200). */
export const MAXIMUM_WPM = 200;
/** Bornes du réglage « Ma vitesse au clavier ». */
export const SETTING_RANGE = { min: 10, max: 150 };

export type Mark = 'correct' | 'wrong' | 'pending';
export type Rejection = 'incomplete' | 'inaccurate' | 'tooShort' | 'tooLong' | 'tooFast';

export interface TypingResult {
  wpm: number;
  accuracy: number;
  /** Caractères justes et caractères comptés (texte ou frappe, le plus long) : le pourcentage affiché s'en déduit en entiers. */
  correct: number;
  total: number;
  seconds: number;
  /** Pourquoi l'essai n'est pas retenu pour la moyenne ; absent s'il l'est. */
  rejection?: Rejection;
}

export const counts = (r: TypingResult): boolean => r.rejection === undefined;

const segmenter = new Intl.Segmenter('fr', { granularity: 'grapheme' });

/** Même texte des deux côtés : apostrophes courbes ramenées à l'apostrophe droite, espaces insécables en espaces ; les accents composés
 * ou décomposés se valent (comparaison par caractère affiché, après normalisation). */
export function normalized(text: string): string[] {
  const plain = text.replace(/[’‘]/g, "'").replace(/[  ]/g, ' ');
  return Array.from(segmenter.segment(plain), (s) => s.segment.normalize('NFC'));
}

/** Chaque caractère du texte de référence : juste, faux ou pas encore tapé (comparaison à la même place). */
export function marks(reference: string, typed: string): Mark[] {
  const ref = normalized(reference);
  const got = normalized(typed);
  return ref.map((c, i) => (i < got.length ? (got[i] === c ? 'correct' : 'wrong') : 'pending'));
}

/** Tapé jusqu'au bout (juste ou non) : seul un essai complet peut être retenu. */
export function isComplete(reference: string, typed: string): boolean {
  return normalized(typed).length >= normalized(reference).length;
}

/** Texte identique : l'essai s'arrête alors tout seul. Avec une faute ou un décalage, il continue (on corrige) ou se termine par « Terminer ». */
export function isExact(reference: string, typed: string): boolean {
  const a = normalized(reference);
  const b = normalized(typed);
  return a.length === b.length && a.every((c, i) => c === b[i]);
}

export function wordCount(text: string): number {
  const trimmed = text.trim();
  return trimmed === '' ? 0 : trimmed.split(/\s+/).length;
}

/** Mots par minute « nets » : les mots du texte multipliés par la part de caractères justes, divisés par la durée.
 * Les caractères en trop (tapés au-delà du texte) comptent comme des fautes. */
export function typingResult(reference: string, typed: string, seconds: number): TypingResult {
  const m = marks(reference, typed);
  const total = Math.max(m.length, normalized(typed).length, 1);
  const correct = m.filter((x) => x === 'correct').length;
  const accuracy = correct / total;
  const wpm = seconds > 0 ? Math.round((wordCount(reference) * accuracy) / (seconds / 60)) : 0;
  let rejection: Rejection | undefined;
  if (!isComplete(reference, typed)) rejection = 'incomplete';
  else if (accuracy < MINIMUM_ACCURACY) rejection = 'inaccurate';
  else if (seconds < MINIMUM_SECONDS) rejection = 'tooShort';
  else if (seconds > MAXIMUM_SECONDS) rejection = 'tooLong';
  else if (wpm > MAXIMUM_WPM) rejection = 'tooFast';
  return { wpm, accuracy, correct, total, seconds, rejection };
}

/** Moyenne des essais retenus (null s'il n'y en a pas), dans les bornes du réglage. */
export function averageWpm(results: TypingResult[]): number | null {
  const kept = results.filter(counts);
  if (kept.length === 0) return null;
  const mean = kept.reduce((sum, r) => sum + r.wpm, 0) / kept.length;
  return Math.min(Math.max(Math.round(mean), SETTING_RANGE.min), SETTING_RANGE.max);
}

/** Pourcentage arrondi vers le bas, en entiers : 89,6 % ne s'affiche pas « 90 % » à côté d'un seuil de 90 %. */
export function percent(correct: number, total: number): number {
  return Math.floor((correct * 100) / Math.max(total, 1));
}

/** Pourquoi un essai n'est pas retenu, en français. */
export function rejectionText(r: Rejection): string {
  switch (r) {
    case 'incomplete':
      return tx('texte pas tapé jusqu’au bout');
    case 'inaccurate':
      return 'trop de fautes (il faut au moins 90 % de lettres justes)';
    case 'tooShort':
      return tx('trop court pour être mesuré');
    case 'tooLong':
      return tx('trop long : essai oublié ouvert ?');
    case 'tooFast':
      return 'plus de 200 mots par minute : ce n’est pas de la frappe';
  }
}
