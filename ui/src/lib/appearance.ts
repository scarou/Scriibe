/** L'apparence de l'indicateur à l'écran : mêmes styles, positions et couleurs que Scribe (`Models.swift`). */
import { tx } from './i18n';

export type CaptureStyle = 'pill' | 'dot' | 'meter';
export type CapturePosition = 'left' | 'center' | 'right';
export type CaptureColor = 'scribe' | 'gray' | 'red' | 'blue' | 'black';

export const styles: { id: CaptureStyle; label: string }[] = [
  { id: 'pill', label: tx('Pastille avec texte') },
  { id: 'dot', label: tx('Point') },
  { id: 'meter', label: tx('Vumètre') },
];

export const positions: { id: CapturePosition; label: string }[] = [
  { id: 'left', label: tx('À gauche') },
  { id: 'center', label: tx('Au centre') },
  { id: 'right', label: tx('À droite') },
];

/** Teinte claire puis foncée du dégradé. */
export const colors: { id: CaptureColor; label: string; hex: [string, string] }[] = [
  // La couleur de la marque : turquoise sur Mac (comme Scribe), bleu sur Windows, violet sur Linux (variables CSS `--brand*`, voir app.css).
  { id: 'scribe', label: tx('Couleur de Scriibe'), hex: ['var(--brand)', 'var(--brand-deep)'] },
  { id: 'gray', label: tx('Gris'), hex: ['#8E8E93', '#6E6E73'] },
  { id: 'red', label: tx('Rouge'), hex: ['#E5484D', '#C62F35'] },
  { id: 'blue', label: tx('Bleu'), hex: ['#3B82F6', '#2563EB'] },
  { id: 'black', label: tx('Noir'), hex: ['#2C2C2E', '#1C1C1E'] },
];

export function gradient(color: CaptureColor): string {
  const [light, dark] = (colors.find((c) => c.id === color) ?? colors[0]).hex;
  return `linear-gradient(to right, ${light}, ${dark})`;
}

/** Le texte de la pastille pendant l'écoute : l'alerte de micro d'abord, sinon « J'écoute… ». */
export function listeningText(mic: 'ok' | 'weak' | 'silent', tr: (text: string) => string = (text) => text): string {
  return tr(mic === 'silent' ? tx('Je n’entends rien…') : mic === 'weak' ? tx('Je t’entends à peine…') : tx('J’écoute…'));
}

/** La légende des styles sans texte (point, vumètre) : l'alerte de micro et la langue de traduction ; `null` s'il n'y a rien à dire. */
export function caption(mic: 'ok' | 'weak' | 'silent', translation: string | null, tr: (text: string) => string = (text) => text): string | null {
  const warning = mic === 'silent' ? tr(tx('Je n’entends rien…')) : mic === 'weak' ? tr(tx('Je t’entends à peine…')) : null;
  const parts = [warning, translation ? `→ ${tr(translation)}` : null].filter((p): p is string => p !== null);
  return parts.length ? parts.join('  ') : null;
}

/** La transparence choisie, sauf pendant un avertissement de micro : il doit rester lisible. */
export function listeningOpacity(mic: 'ok' | 'weak' | 'silent', opacity: number): number {
  return mic === 'ok' ? opacity : 1;
}
