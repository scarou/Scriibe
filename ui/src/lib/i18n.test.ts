import { describe, expect, it } from 'vitest';
import { fill, placeholders, resolveLanguage, translate } from './i18n';
import { en } from './locales/en';

// Le texte de tous les fichiers de la page (sauf les tests et les traductions), lu tel quel.
const files = import.meta.glob(['../**/*.svelte', '../**/*.ts', '!../**/*.test.ts', '!../lib/locales/*'], { query: '?raw', import: 'default', eager: true }) as Record<string, string>;

/** Les textes français passés à `t('…')` dans la page, plus les libellés des onglets (`label: '…'`, traduits par `t(tab.label)`). */
function usedKeys(): Set<string> {
  const keys = new Set<string>();
  for (const [file, text] of Object.entries(files)) {
    for (const m of text.matchAll(/(?<![\w.$])t\(\s*'((?:[^'\\]|\\.)*)'/g)) keys.add(m[1].replace(/\\'/g, "'"));
    if (file.endsWith('/tabs.ts')) for (const m of text.matchAll(/label:\s*'((?:[^'\\]|\\.)*)'/g)) keys.add(m[1]);
  }
  return keys;
}

describe('langue de l’interface', () => {
  it('choisit la langue demandée, sinon celle du système, sinon l’anglais', () => {
    expect(resolveLanguage('fr', ['en-US'])).toBe('fr');
    expect(resolveLanguage('en', ['fr-FR'])).toBe('en');
    expect(resolveLanguage('auto', ['fr-FR', 'en'])).toBe('fr');
    expect(resolveLanguage('auto', ['de-DE', 'en-GB'])).toBe('en');
    expect(resolveLanguage('auto', ['de-DE', 'ja'])).toBe('en');
    expect(resolveLanguage(undefined, [])).toBe('en');
  });

  it('traduit en anglais, laisse le français tel quel et garde le français d’une phrase sans traduction', () => {
    expect(translate('Historique', 'fr')).toBe('Historique');
    expect(translate('Historique', 'en')).toBe('History');
    expect(translate('Une phrase pas encore traduite', 'en')).toBe('Une phrase pas encore traduite');
  });

  it('insère les valeurs entre accolades, dans les deux langues', () => {
    expect(translate('{n} mots', 'fr', { n: 3 })).toBe('3 mots');
    expect(translate('{n} mots', 'en', { n: 3 })).toBe('3 words');
    expect(fill('{a} et {b}', { a: 'x' })).toBe('x et {b}');
    expect(fill('sans valeur')).toBe('sans valeur');
  });

  it('garde dans chaque traduction les mêmes valeurs insérées que le texte français', () => {
    for (const [french, english] of Object.entries(en)) {
      expect(english.trim(), french).not.toBe('');
      expect(placeholders(english), french).toEqual(placeholders(french));
    }
  });

  it('traduit tous les textes de la page, sans traduction orpheline', () => {
    const used = usedKeys();
    expect(used.size, 'la lecture des textes de la page ne trouve rien').toBeGreaterThan(30);
    const missing = [...used].filter((k) => !(k in en));
    const orphans = Object.keys(en).filter((k) => !used.has(k));
    expect(missing, 'textes de la page sans traduction (à ajouter dans locales/en.ts)').toEqual([]);
    expect(orphans, 'traductions que la page n’utilise plus').toEqual([]);
  });
});
