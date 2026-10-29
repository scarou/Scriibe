import { en } from './locales/en';
import { enServer } from './locales/en-server';

/** Langues de l'interface. Le français est la source : le texte français sert de clé, une phrase sans traduction reste en français. */
export type Lang = 'fr' | 'en';
export type UiLanguage = 'auto' | Lang;
export type Vars = Record<string, string | number>;

/** La langue réellement utilisée : le choix, ou (en automatique) la première langue du système que l'on sait afficher, sinon l'anglais. */
export function resolveLanguage(choice: UiLanguage | undefined, systemLanguages: readonly string[]): Lang {
  if (choice === 'fr' || choice === 'en') return choice;
  for (const tag of systemLanguages) {
    const base = tag.toLowerCase().split('-')[0];
    if (base === 'fr' || base === 'en') return base;
  }
  return 'en';
}

/** Remplace les `{nom}` d'un texte par les valeurs données (un `{nom}` sans valeur reste tel quel). */
export function fill(text: string, vars?: Vars): string {
  if (!vars) return text;
  return text.replace(/\{(\w+)\}/g, (whole, name: string) => (name in vars ? String(vars[name]) : whole));
}

/** Marque un texte français à traduire plus tard (constante de module, libellé d'une liste) : il reste tel quel, `t(texte)` le traduit à l'affichage. */
export function tx(text: string): string {
  return text;
}

/** Un texte du programme avec ses `{valeurs}` devient un motif qui retrouve les valeurs dans le message reçu. */
function templateOf(key: string): { pattern: RegExp; names: string[] } {
  const names: string[] = [];
  const source = key
    .split(/\{(\w+)\}/)
    .map((part, i) => {
      if (i % 2) {
        names.push(part);
        return '(.+?)';
      }
      return part.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    })
    .join('');
  return { pattern: new RegExp('^' + source + '$', 's'), names };
}

let templates: { pattern: RegExp; names: string[]; english: string }[] | null = null;

/** Un message du programme qui contient une valeur (« Aucun son capté — vérifie le micro (Realtek) ») : traduit d'après son modèle. */
function fromTemplate(text: string): string | null {
  templates ??= Object.entries(enServer)
    .filter(([french]) => /\{\w+\}/.test(french))
    .map(([french, english]) => ({ ...templateOf(french), english }));
  for (const { pattern, names, english } of templates) {
    const found = pattern.exec(text);
    // Les valeurs connues (un nom de langue, un mode…) se traduisent aussi.
    if (found) return fill(english, Object.fromEntries(names.map((name, i) => [name, en[found[i + 1]] ?? enServer[found[i + 1]] ?? found[i + 1]])));
  }
  return null;
}

export function translate(text: string, lang: Lang, vars?: Vars): string {
  if (lang !== 'en') return fill(text, vars);
  return fill(en[text] ?? enServer[text] ?? fromTemplate(text) ?? text, vars);
}

/** Les mots entre accolades d'un texte, dans l'ordre alphabétique : une traduction doit en avoir les mêmes. */
export function placeholders(text: string): string[] {
  return [...text.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();
}
