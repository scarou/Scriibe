import { tx } from './i18n';
import type { TransferKind } from './nav.svelte';

/**
 * Pourquoi une demande d'export ou d'import des commandes vocales ne peut pas s'ouvrir, ou `null`. Les commandes vocales décochées :
 * la carte « Mes commandes » est grisée et inerte, la demande n'y ouvrirait qu'une carte qui ne répond pas ; on le dit plutôt.
 */
export function commandsTransferBlock(kind: TransferKind, voiceCommandsEnabled: boolean | undefined, customCount: number): string | null {
  if (kind !== 'commands-export' && kind !== 'commands-import') return null;
  if (voiceCommandsEnabled === false) {
    return tx('Les commandes vocales sont décochées : coche « Activer les commandes vocales » en haut de la page Commandes vocales, puis recommence.');
  }
  if (kind === 'commands-export' && customCount === 0) return tx('Aucune commande vocale personnelle à exporter.');
  return null;
}
