import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { nav } from './nav.svelte';
import { dialog } from './dialog.svelte';
import { buildReport } from './report';

const sources = import.meta.glob('../**/*.svelte', { query: '?raw', import: 'default', eager: true }) as Record<string, string>;
const source = (name: string) => Object.entries(sources).find(([f]) => f.endsWith(name))![1];

describe('carte « Sauvegarder et transférer » : demandes de transfert', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.spyOn(nav, 'go').mockResolvedValue();
    nav.transferRequest = null;
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

  it('une page sans rien à exporter ouvre une boîte sur SA page (la carte de départ n’est plus à l’écran) et vide la demande', async () => {
    const notice = vi.spyOn(dialog, 'notice').mockResolvedValue(true);
    await nav.transfer('modes-export');
    nav.transferNothing('Aucun mode personnel à exporter.');
    expect(nav.transferRequest).toBeNull();
    expect(notice).toHaveBeenCalledWith('Aucun mode personnel à exporter.');
  });

  it('chaque page (modes, dictionnaire, commandes, utilisateurs) dit qu’elle n’a rien à exporter', () => {
    for (const f of ['tabs/Modes.svelte', 'tabs/Dictionary.svelte', 'components/commands/CustomCommands.svelte', 'components/Users.svelte']) {
      expect(source(f), f).toMatch(/nav\.transferNothing\(/);
    }
  });

  it('l’Aide : la phrase « jamais écrasé » reste dans la condition Linux ; la touche de traduction vide ne laisse pas de phrase trouée', () => {
    for (const f of ['tabs/Help.svelte', 'tabs/HelpEn.svelte']) {
      const src = source(f);
      for (const m of src.matchAll(/never overwritten|jamais écrasé/g)) {
        const before = src.slice(0, m.index);
        expect(before.lastIndexOf('{#if linux}'), f).toBeGreaterThan(before.lastIndexOf('{/if}'));
      }
      expect(src, f).toMatch(/\{#if translate\}/);
    }
  });

  it('le rapport n’écrit pas « (traduction : ) » quand la touche de traduction est vide', () => {
    const status = { version: '0', hotkey: 'x', dictation_label: 'Alt', translate_label: '', transcription: 'local', cleanup_label: 'c', whisper: { binary: false, models: [] }, llama: { binary: false, models: [] }, microphones: [], llama_device: '', llama_log: { bytes: 0, hints: [], head: [] } };
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    expect(buildReport(status as any, [], [], [])).not.toContain('traduction :');
  });

  it('chaque saisie du chemin du fichier à importer a un bouton « Annuler »', () => {
    for (const f of ['tabs/Modes.svelte', 'tabs/Dictionary.svelte', 'components/UserImport.svelte', 'components/BackupTransfer.svelte']) {
      const src = source(f);
      const block = src.slice(src.indexOf('{#if ' + (f.includes('Backup') ? 'restoring.askPath' : f.includes('UserImport') ? 'askPath' : 'importing.askPath')));
      const end = Math.min(...['{:else if', '{#if restoring.preview}'].map((x) => block.indexOf(x)).filter((i) => i > 0), 900);
      expect(block.slice(0, end), f).toContain("t('Annuler')");
    }
  });
});
