import { en } from './locales/en';

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

export function translate(text: string, lang: Lang, vars?: Vars): string {
  return fill(lang === 'en' ? (en[text] ?? text) : text, vars);
}

/** Les mots entre accolades d'un texte, dans l'ordre alphabétique : une traduction doit en avoir les mêmes. */
export function placeholders(text: string): string[] {
  return [...text.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();
}
