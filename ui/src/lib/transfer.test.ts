import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { nav } from './nav.svelte';

const sources = import.meta.glob('../**/*.svelte', { query: '?raw', import: 'default', eager: true }) as Record<string, string>;
const source = (name: string) => Object.entries(sources).find(([f]) => f.endsWith(name))![1];

describe('carte « Sauvegarder et transférer » : demandes de transfert', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.spyOn(nav, 'go').mockResolvedValue();
    nav.transferRequest = null;
    nav.transferNotice = '';
  });
  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it('le minuteur d’une demande n’efface pas la demande suivante du même type', async () => {
    await nav.transfer('modes-export');
    await vi.advanceTimersByTimeAsync(3000);
    await nav.transfer('modes-export'); // deuxième clic à 3 s
    await vi.advanceTimersByTimeAsync(1500); // le minuteur du premier (4 s) a sonné
    expect(nav.transferRequest).toBe('modes-export');
    await vi.advanceTimersByTimeAsync(3000); // celui du second (7 s) a sonné
    expect(nav.transferRequest).toBeNull();
  });

  it('une demande non prise est effacée après 4 s', async () => {
    await nav.transfer('dictionary-export');
    await vi.advanceTimersByTimeAsync(4100);
    expect(nav.transferRequest).toBeNull();
  });

  it('une page sans rien à exporter laisse un message, effacé à la demande suivante', async () => {
    await nav.transfer('modes-export');
    nav.transferNothing('Aucun mode personnel à exporter.');
    expect(nav.transferRequest).toBeNull();
    expect(nav.transferNotice).toBe('Aucun mode personnel à exporter.');
    await nav.transfer('dictionary-export');
    expect(nav.transferNotice).toBe('');
  });

  it('le message disparaît seul après 8 s', async () => {
    nav.transferNothing('Le dictionnaire est vide : rien à exporter.');
    await vi.advanceTimersByTimeAsync(8100);
    expect(nav.transferNotice).toBe('');
  });

  it('chaque page (modes, dictionnaire, commandes, utilisateurs) dit qu’elle n’a rien à exporter, et la carte l’affiche', () => {
    for (const f of ['tabs/Modes.svelte', 'tabs/Dictionary.svelte', 'components/commands/CustomCommands.svelte', 'components/Users.svelte']) {
      expect(source(f), f).toMatch(/nav\.transferNothing\(/);
    }
    expect(source('components/BackupTransfer.svelte')).toMatch(/role="status"[^>]*>\{nav\.transferNotice\}/);
  });

  it('chaque saisie du chemin du fichier à importer a un bouton « Annuler »', () => {
    for (const f of ['tabs/Modes.svelte', 'tabs/Dictionary.svelte', 'components/UserImport.svelte', 'components/BackupTransfer.svelte']) {
      const src = source(f);
      const block = src.slice(src.indexOf('{#if ' + (f.includes('Backup') ? 'restoring.askPath' : f.includes('UserImport') ? 'askPath' : 'importing.askPath')));
      const end = block.indexOf('{:else');
      expect(block.slice(0, end > 0 ? end : 900), f).toContain("t('Annuler')");
    }
  });
});
