/** La recherche dans l'Aide : texte intégral, sans IA, hors ligne ; la casse, les accents et le type d'apostrophe sont ignorés. */

/** Le texte « plié » (minuscules, sans accents, apostrophes droites, espaces insécables en espaces) et, pour chaque caractère plié, sa place dans le texte d'origine. */
export function fold(text: string): { text: string; map: number[] } {
  let out = '';
  const map: number[] = [];
  for (let i = 0; i < text.length; i++) {
    const base = text[i]
      .normalize('NFD')
      .replace(/[̀-ͯ]/g, '')
      .replace(/[’‘ʼ]/g, "'")
      .replace(/[  ]/g, ' ')
      .toLowerCase();
    for (const c of base) {
      out += c;
      map.push(i);
    }
  }
  return { text: out, map };
}

/** La requête pliée, espaces superflus retirés ; vide si elle est trop courte pour chercher (moins de 2 lettres). */
export function foldQuery(query: string): string {
  const q = fold(query).text.replace(/\s+/g, ' ').trim();
  return q.length >= 2 ? q : '';
}

/** Les occurrences de `needle` (déjà pliée) dans `text`, en positions [début, fin) du texte d'origine. */
export function findMatches(text: string, needle: string): [number, number][] {
  if (!needle) return [];
  const folded = fold(text);
  const found: [number, number][] = [];
  let from = 0;
  for (;;) {
    const at = folded.text.indexOf(needle, from);
    if (at < 0) break;
    found.push([folded.map[at], folded.map[at + needle.length - 1] + 1]);
    from = at + needle.length;
  }
  return found;
}
