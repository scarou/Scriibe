import { describe, expect, it } from 'vitest';
import { joinKeys } from './i18n.svelte';
import { exportWhere } from './exportwhere';

describe('textes selon le système (chantier 2, tâche 2)', () => {
  it('joinKeys saute les libellés vides : pas de « + » en trop sous GNOME sans raccourci de traduction', () => {
    expect(joinKeys('Ctrl + ←', 'Alt')).toBe('Ctrl + ← + Alt');
    expect(joinKeys('Ctrl + ←', '')).toBe('Ctrl + ←');
    expect(joinKeys('Ctrl + ←', undefined)).toBe('Ctrl + ←');
  });

  it('la carte d’export ne parle que du système en cours : Documents sous Linux, la boîte ailleurs', () => {
    expect(exportWhere('linux')).toMatch(/Documents/);
    for (const os of ['macos', 'windows']) {
      expect(exportWhere(os)).not.toMatch(/Documents/);
      expect(exportWhere(os)).not.toBe(exportWhere('linux'));
    }
  });
});
