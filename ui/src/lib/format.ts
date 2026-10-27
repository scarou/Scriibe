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
