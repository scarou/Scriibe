/** Un nombre à la française : deux décimales au plus, virgule. */
export function fmt(n: number): string {
  return (Math.round(n * 100) / 100).toString().replace('.', ',');
}

/** Une taille en mégaoctets entiers. */
export function mo(bytes: number): string {
  return Math.round(bytes / 1048576) + ' Mo';
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
export function when(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  const two = (n: number) => String(n).padStart(2, '0');
  return `${two(d.getDate())}/${two(d.getMonth() + 1)}/${d.getFullYear()} à ${two(d.getHours())}:${two(d.getMinutes())}`;
}
