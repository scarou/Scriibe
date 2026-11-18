import { i18n } from './i18n.svelte';
import { uiState } from './uiState';

/**
 * Avant d'afficher une fenêtre (la page comme la pastille de l'indicateur) : lit les états gardés par Scriibe puis reprend la copie de la
 * langue (le module de langue est chargé avant cette lecture). Ne lève jamais d'erreur et n'attend pas plus de deux secondes.
 */
export async function restoreUiState(): Promise<void> {
  await uiState.load();
  i18n.restore();
}
