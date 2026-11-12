import { describe, expect, it } from 'vitest';
import { defaultHelpSection, helpSections } from './help';
import { tabs } from './tabs';
import help from '../tabs/Help.svelte?raw';
import helpEn from '../tabs/HelpEn.svelte?raw';

describe('structure de l’interface', () => {
  it('ui_logic-014 Aide : huit sections aux titres distincts, dernière page de la barre latérale, ouverte sur « Premiers pas »', () => {
    expect(helpSections.length).toBe(8);
    expect(new Set(helpSections.map((s) => s.title)).size).toBe(8);
    expect(tabs.at(-1)?.id).toBe('help');
    expect(helpSections.find((s) => s.id === defaultHelpSection)?.title).toBe('Premiers pas');
  });

  it('les pages ont des identifiants et des titres distincts', () => {
    expect(new Set(tabs.map((t) => t.id)).size).toBe(tabs.length);
    expect(new Set(tabs.map((t) => t.label)).size).toBe(tabs.length);
  });
});

describe('Aide : sommaire fixe', () => {
  for (const [f, src] of [['Help', help], ['HelpEn', helpEn]] as const) {
    it(`${f} : le sommaire reste visible (sticky, sans retour à la ligne), seule la carte de texte défile`, () => {
      const css = (sel: string) => new RegExp(`\\n\\s*${sel.replace('.', '\\.')} \\{([^}]*)\\}`).exec(src)?.[1] ?? '';
      expect(css('.toc')).toMatch(/position:\s*sticky/);
      expect(css('.toc')).toMatch(/max-height:[^;]*100vh/);
      expect(css('.toc')).toMatch(/overflow-y:\s*auto/);
      expect(css('.layout')).toMatch(/flex-wrap:\s*nowrap/);
      expect(css('.body')).toMatch(/min-width:\s*0/);
    });

    it(`${f} : un clic sur une section remonte en haut de la page, et en fenêtre étroite le sommaire devient une bande fixe en haut`, () => {
      expect(src).toMatch(/helpSection = [^;]+;\s*window\.scrollTo\(0, 0\)/);
      const narrow = /@media \(max-width: (\d+)px\) \{([\s\S]*?)\n  \}/.exec(src);
      expect(narrow, 'media query de la fenêtre étroite').not.toBeNull();
      expect(Number(narrow![1])).toBeGreaterThanOrEqual(800);
      expect(narrow![2]).toMatch(/flex-direction:\s*column/);
      expect(narrow![2]).toMatch(/overflow-x:\s*auto/);
    });
  }
});
