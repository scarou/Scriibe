import { describe, expect, it } from 'vitest';
import { dialog } from './dialog.svelte';

describe('boîtes de dialogue de l’interface', () => {
  it('une confirmation rend vrai ou faux selon le bouton', async () => {
    const yes = dialog.confirm('Quitter Scriibe ?', { confirm: 'Quitter' });
    expect(dialog.current).toMatchObject({ message: 'Quitter Scriibe ?', confirm: 'Quitter', danger: false });
    dialog.answer(true);
    expect(await yes).toBe(true);
    expect(dialog.current).toBeNull();

    const no = dialog.confirm('Tout effacer ?', { danger: true });
    expect(dialog.current?.danger).toBe(true);
    dialog.answer(false);
    expect(await no).toBe(false);
  });

  it('un avertissement n’a qu’un bouton et une nouvelle boîte refuse la précédente', async () => {
    const first = dialog.confirm('Première ?');
    const notice = dialog.notice('Réglage refusé');
    expect(await first).toBe(false);
    expect(dialog.current).toMatchObject({ message: 'Réglage refusé', confirm: null });
    dialog.answer(true);
    expect(await notice).toBe(true);
  });
});
