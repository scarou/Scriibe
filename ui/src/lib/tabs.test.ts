import { describe, expect, it } from 'vitest';
import { defaultHelpSection, helpSections } from './help';
import { resolveTab, tabGroups, tabs, type TabId } from './tabs';
import pageHeader from '../components/PageHeader.svelte?raw';
import app from '../App.svelte?raw';
import commandsPage from '../tabs/Commands.svelte?raw';
import dictionary from '../tabs/Dictionary.svelte?raw';
import settingsTab from '../tabs/Settings.svelte?raw';
import keyPicker from '../components/KeyPicker.svelte?raw';
import activation from '../components/commands/ActivationCommands.svelte?raw';
import rewrite from '../components/commands/RewriteCommands.svelte?raw';
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

describe('menu de gauche en quatre groupes (refonte de l’interface)', () => {
  it('Utiliser, Ma dictée, Configurer, Avancé, avec leurs pages dans l’ordre voulu, puis l’Aide', () => {
    expect(tabGroups.map((g) => [g.label, g.tabs.map((x) => x.label)])).toEqual([
      ['Utiliser', ['Accueil', 'Historique']],
      ['Ma dictée', ['Dicter', 'Style d’écriture', 'Dictionnaire', 'Commandes vocales']],
      ['Configurer', ['Moteurs et modèles', 'Profil', 'Apparence', 'Données et système']],
      ['Avancé', ['Mesures et essais']],
    ]);
    expect(tabs.length).toBe(12);
    expect(tabs.some((x) => x.label === 'Réglages')).toBe(false);
  });

  it('chaque page du menu s’ouvre dans App.svelte', () => {
    for (const tab of tabs) expect(app, tab.id).toContain(`nav.tab === '${tab.id}'`);
  });

  it('les anciens identifiants de page et d’ancre des Réglages mènent à la page qui a pris leur place', () => {
    const cases: [TabId, string | undefined, string][] = [
      ['settings', 'dictee', 'dictate'], ['settings', 'texte', 'style'], ['settings', 'utilisateurs', 'profile'], ['settings', 'cles', 'profile'],
      ['settings', 'moteurs', 'engines'], ['settings', 'indicateur', 'appearance'], ['settings', 'langue', 'appearance'],
      ['settings', 'sauvegarde', 'data'], ['settings', 'demarrage', 'data'], ['settings', undefined, 'dictate'],
      ['modes', undefined, 'style'], ['install', undefined, 'engines'], ['bench', undefined, 'measures'], ['journal', undefined, 'measures'], ['update', undefined, 'data'],
      ['history', 'conservation', 'history'], ['dictionary', undefined, 'dictionary'],
    ];
    for (const [id, anchor, page] of cases) expect(resolveTab(id, anchor), `${id} ${anchor ?? ''}`).toBe(page);
  });
});

describe('page Commandes vocales (tâche 2)', () => {
  it('sept sections dans l’ordre : activation, intégrées, sites, extraits, applications, commandes, reformulation', () => {
    const order = [...commandsPage.matchAll(/<(\w+Commands) \/>/g)].map((m) => m[1]);
    expect(order).toEqual(['ActivationCommands', 'BuiltinCommands', 'SiteCommands', 'SnippetCommands', 'AppCommands', 'CustomCommands', 'RewriteCommands']);
  });

  it('le Dictionnaire ne garde plus les commandes vocales', () => {
    expect(dictionary).not.toMatch(/VoiceCommands|commandes-vocales/);
  });

  it('la touche de commande et l’accord de reformulation sont sur la page Commandes vocales, plus dans Dicter ni Moteurs', () => {
    expect(activation).toContain("kinds={['command']}");
    expect(activation).toContain('voiceCommandsEnabled');
    expect(rewrite).toContain('rewriteConsent');
    expect(settingsTab).not.toContain('rewriteConsent');
    expect(settingsTab).toContain("kinds={['dictation', 'translate']}");
    expect(keyPicker).toMatch(/kinds\.includes\('command'\)/);
  });

  it('la page dit que les commandes ne marchent pas encore partout', () => {
    expect(activation).toMatch(/ne marchent pas encore partout/);
  });
});

describe('en-tête de page fixe', () => {
  it('l’en-tête reste en haut de la fenêtre quand on fait défiler, sur toutes les pages qui en ont un', () => {
    expect(pageHeader).toMatch(/\.page-header \{[^}]*position:\s*sticky[^}]*top:\s*0/);
    expect(pageHeader).toMatch(/--page-head/);
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
