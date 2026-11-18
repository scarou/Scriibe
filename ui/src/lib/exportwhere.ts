import { t } from './i18n.svelte';

/** Où va le fichier d'un export : une boîte « Enregistrer sous » (Mac, Windows) ou le dossier Documents (Linux, pas de boîte). Le texte ne parle que du système en cours. */
export function exportWhere(os: string): string {
  return os === 'linux' ? t('Il est placé dans le dossier Documents.') : t('Une boîte te demande où l’enregistrer.');
}
