import { describe, expect, it } from 'vitest';
import { defaultHelpSection, helpSections } from './help';
import { tabs } from './tabs';

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
