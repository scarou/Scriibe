import { uiState } from './uiState';
import { resolveLanguage, translate, tx, type Lang, type UiLanguage, type Vars } from './i18n';

/** La copie du choix gardée dans le fichier d'états de Scriibe (lue avant l'affichage), sinon la langue du système. */
function savedChoice(): UiLanguage {
  return uiState.state.uiLanguage ?? 'auto';
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
    if (uiState.state.uiLanguage !== next) uiState.save({ uiLanguage: next });
  }

  /** Reprend la copie du choix une fois les états lus (ce module est chargé avant leur lecture). */
  restore(): void {
    this.choice = savedChoice();
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

/** Réunit des noms de touches avec « + » en sautant les vides (sous GNOME sans raccourci de traduction, le libellé de traduction est vide). */
export function joinKeys(...labels: (string | undefined)[]): string {
  return labels.filter(Boolean).map((l) => keyName(l as string)).join(' + ');
}

/** Le texte dans la langue de l'interface ; `{nom}` est remplacé par la valeur de `vars`. Se relit tout seul quand la langue change. */
export function t(text: string, vars?: Vars): string {
  return translate(text, i18n.lang, vars);
}
