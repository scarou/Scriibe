import { describe, expect, it } from 'vitest';
import { commandsTransferBlock } from './commandsTransfer';
import custom from '../components/commands/CustomCommands.svelte?raw';
import activation from '../components/commands/ActivationCommands.svelte?raw';

describe('export et import des commandes vocales', () => {
  it('ne s’ouvrent pas dans la carte inerte quand les commandes vocales sont décochées', () => {
    for (const kind of ['commands-export', 'commands-import'] as const) {
      expect(commandsTransferBlock(kind, false, 3)).toContain('décochées');
    }
  });

  it('s’ouvrent quand elles sont cochées (ou pas encore lues)', () => {
    expect(commandsTransferBlock('commands-import', true, 0)).toBeNull();
    expect(commandsTransferBlock('commands-export', true, 2)).toBeNull();
    expect(commandsTransferBlock('commands-export', undefined, 2)).toBeNull();
  });

  it('gardent le message « rien à exporter », et ne concernent que les commandes', () => {
    expect(commandsTransferBlock('commands-export', true, 0)).toContain('Aucune commande');
    expect(commandsTransferBlock('dictionary-export', false, 0)).toBeNull();
  });

  it('la carte les demande par cette garde, et le texte nomme la case qui existe', () => {
    expect(custom).toContain('commandsTransferBlock(kind');
    expect(activation).toContain('Activer les commandes vocales');
  });
});
