/** L'apparence de l'indicateur à l'écran : mêmes styles, positions et couleurs que Scribe (`Models.swift`). */
export type CaptureStyle = 'pill' | 'dot' | 'meter';
export type CapturePosition = 'left' | 'center' | 'right';
export type CaptureColor = 'scribe' | 'gray' | 'red' | 'blue' | 'black';

export const styles: { id: CaptureStyle; label: string }[] = [
  { id: 'pill', label: 'Pastille avec texte' },
  { id: 'dot', label: 'Point' },
  { id: 'meter', label: 'Vumètre' },
];

export const positions: { id: CapturePosition; label: string }[] = [
  { id: 'left', label: 'À gauche' },
  { id: 'center', label: 'Au centre' },
  { id: 'right', label: 'À droite' },
];

/** Teinte claire puis foncée du dégradé. */
export const colors: { id: CaptureColor; label: string; hex: [string, string] }[] = [
  // La couleur de la marque : turquoise sur Mac (comme Scribe), bleu sur Windows, violet sur Linux (variables CSS `--brand*`, voir app.css).
  { id: 'scribe', label: 'Couleur de Scriibe', hex: ['var(--brand)', 'var(--brand-deep)'] },
  { id: 'gray', label: 'Gris', hex: ['#8E8E93', '#6E6E73'] },
  { id: 'red', label: 'Rouge', hex: ['#E5484D', '#C62F35'] },
  { id: 'blue', label: 'Bleu', hex: ['#3B82F6', '#2563EB'] },
  { id: 'black', label: 'Noir', hex: ['#2C2C2E', '#1C1C1E'] },
];

export function gradient(color: CaptureColor): string {
  const [light, dark] = (colors.find((c) => c.id === color) ?? colors[0]).hex;
  return `linear-gradient(to right, ${light}, ${dark})`;
}

/** Le texte de la pastille pendant l'écoute : l'alerte de micro d'abord, sinon « J'écoute… ». */
export function listeningText(mic: 'ok' | 'weak' | 'silent'): string {
  return mic === 'silent' ? 'Je n’entends rien…' : mic === 'weak' ? 'Je t’entends à peine…' : 'J’écoute…';
}

/** La légende des styles sans texte (point, vumètre) : l'alerte de micro et la langue de traduction ; `null` s'il n'y a rien à dire. */
export function caption(mic: 'ok' | 'weak' | 'silent', translation: string | null): string | null {
  const warning = mic === 'silent' ? 'Je n’entends rien…' : mic === 'weak' ? 'Je t’entends à peine…' : null;
  const parts = [warning, translation ? `→ ${translation}` : null].filter((p): p is string => p !== null);
  return parts.length ? parts.join('  ') : null;
}

/** La transparence choisie, sauf pendant un avertissement de micro : il doit rester lisible. */
export function listeningOpacity(mic: 'ok' | 'weak' | 'silent', opacity: number): number {
  return mic === 'ok' ? opacity : 1;
}
