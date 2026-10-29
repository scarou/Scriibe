import { resolveLanguage, translate, tx, type Lang, type UiLanguage, type Vars } from './i18n';

const STORAGE_KEY = 'uiLanguage';

function savedChoice(): UiLanguage {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved === 'fr' || saved === 'en' || saved === 'auto') return saved;
  } catch {
    /* le stockage peut être refusé : on suit la langue du système */
  }
  return 'auto';
}

const systemLanguages = (): readonly string[] => (typeof navigator === 'undefined' ? [] : navigator.languages?.length ? navigator.languages : [navigator.language ?? '']);

/** La langue de l'interface : le réglage « Langue de l'interface », gardé aussi dans la page pour éviter un instant de français au démarrage. */
class I18n {
  choice = $state<UiLanguage>(savedChoice());
  lang = $derived<Lang>(resolveLanguage(this.choice, systemLanguages()));

  /** Prend le choix du fichier de réglages. */
  use(choice: UiLanguage | undefined): void {
    const next = choice ?? 'auto';
    this.choice = next;
    try {
      localStorage.setItem(STORAGE_KEY, next);
    } catch {
      /* sans importance */
    }
  }

  /** Pour les dates et les nombres. */
  get locale(): string {
    return this.lang === 'en' ? 'en-GB' : 'fr-FR';
  }
}

export const i18n = new I18n();

export { tx };

/** Le nom d'une touche ou d'une combinaison du programme (« Ctrl gauche + F9 ») dans la langue de l'interface. */
export function keyName(label: string): string {
  return label
    .split(' + ')
    .map((part) => translate(part, i18n.lang))
    .join(' + ');
}

/** Le texte dans la langue de l'interface ; `{nom}` est remplacé par la valeur de `vars`. Se relit tout seul quand la langue change. */
export function t(text: string, vars?: Vars): string {
  return translate(text, i18n.lang, vars);
}
