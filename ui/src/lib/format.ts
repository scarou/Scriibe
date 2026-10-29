import type { Lang } from './i18n';

/** Un nombre à la française : deux décimales au plus, virgule (en anglais : un point). */
export function fmt(n: number, lang: Lang = 'fr'): string {
  const text = (Math.round(n * 100) / 100).toString();
  return lang === 'en' ? text : text.replace('.', ',');
}

/** Une taille en mégaoctets entiers. */
export function mo(bytes: number, lang: Lang = 'fr'): string {
  return Math.round(bytes / 1048576) + (lang === 'en' ? ' MB' : ' Mo');
}

/** Une taille de fichier à la française : « 148 Mo », « 2,5 Go » (au-dessus de 1 Go). */
export function taille(bytes: number, lang: Lang = 'fr'): string {
  if (bytes < 1073741824) return mo(bytes, lang);
  const gigabytes = (bytes / 1073741824).toFixed(1);
  return lang === 'en' ? gigabytes + ' GB' : gigabytes.replace('.', ',') + ' Go';
}

/** Un nombre d'une dizaine de milliers, court : « 950 », « 12,3 k », « 1,2 M ». */
export function compte(n: number, lang: Lang = 'fr'): string {
  const decimal = (x: string) => (lang === 'en' ? x.replace(/\.0$/, '') : x.replace('.', ',').replace(/,0$/, ''));
  if (n < 1000) return String(n);
  if (n < 1_000_000) return decimal((n / 1000).toFixed(n < 10_000 ? 1 : 0)) + ' k';
  return decimal((n / 1_000_000).toFixed(1)) + ' M';
}

/** Le nom d'un fichier dans un chemin Windows ou Unix. */
export function base(path: string | null | undefined): string {
  return String(path ?? '').split(/[\\/]/).pop() ?? '';
}

/** Pourcentage d'avancement, 0 si le total est inconnu. */
export function percent(done: number, total: number): number {
  return total > 0 ? Math.min(100, Math.round((done / total) * 100)) : 0;
}

/** Un temps en secondes, dit à la française : « 45 s », « 12 min », « 1 h 05 min ». */
export function spoken(seconds: number): string {
  const s = Math.max(0, Math.round(seconds));
  if (s < 60) return `${s} s`;
  const minutes = Math.round(s / 60);
  if (minutes < 60) return `${minutes} min`;
  return `${Math.floor(minutes / 60)} h ${String(minutes % 60).padStart(2, '0')} min`;
}

/** Une date ISO affichée « 27/10/2026 à 14:05 ». */
export function when(iso: string, lang: Lang = 'fr'): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  const two = (n: number) => String(n).padStart(2, '0');
  return `${two(d.getDate())}/${two(d.getMonth() + 1)}/${d.getFullYear()} ${lang === 'en' ? 'at' : 'à'} ${two(d.getHours())}:${two(d.getMinutes())}`;
}
