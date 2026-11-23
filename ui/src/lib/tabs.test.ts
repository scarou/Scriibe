import { describe, expect, it } from 'vitest';
import { defaultHelpSection, helpSections } from './help';
import { tabGroups, tabs } from './tabs';
import pageHeader from '../components/PageHeader.svelte?raw';
import app from '../App.svelte?raw';
import commandsPage from '../tabs/Commands.svelte?raw';
import dictionary from '../tabs/Dictionary.svelte?raw';
import settingsTab from '../tabs/Settings.svelte?raw';
import keyPicker from '../components/KeyPicker.svelte?raw';
import activation from '../components/commands/ActivationCommands.svelte?raw';
import rewrite from '../components/commands/RewriteCommands.svelte?raw';
import profilePage from '../tabs/Profile.svelte?raw';
import stylePage from '../tabs/StylePage.svelte?raw';
import modesTab from '../tabs/Modes.svelte?raw';
import dashboard from '../tabs/Dashboard.svelte?raw';
import enginesPage from '../tabs/EnginesPage.svelte?raw';
import dataPage from '../tabs/Data.svelte?raw';
import measuresPage from '../tabs/Measures.svelte?raw';
import benchTab from '../tabs/Bench.svelte?raw';
import help from '../tabs/Help.svelte?raw';
import helpEn from '../tabs/HelpEn.svelte?raw';
import helpSearchBox from '../components/HelpSearchBox.svelte?raw';

describe('structure de l’interface', () => {
  it('ui_logic-014 Aide : treize rubriques (une par page du menu, plus Premiers pas, Sur cet ordinateur ou en ligne, Problèmes fréquents, À propos) aux titres distincts, dernière page de la barre latérale, ouverte sur « Premiers pas »', () => {
    expect(helpSections.length).toBe(13);
    expect(new Set(helpSections.map((s) => s.title)).size).toBe(13);
    // Les rubriques de l'Aide suivent les pages du menu, dans le même ordre.
    const pageTitles = tabGroups.flatMap((g) => g.tabs.map((x) => x.label)).filter((l) => ['Dicter', 'Style d’écriture', 'Dictionnaire', 'Commandes vocales', 'Moteurs et modèles', 'Profil', 'Apparence', 'Données et système', 'Mesures et essais'].includes(l));
    expect(helpSections.map((s) => s.title as string).filter((t) => (pageTitles as string[]).includes(t))).toEqual(pageTitles);
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
      expect(src).toMatch(/helpSection = [^;]+;[\s\S]*?window\.scrollTo\(0, 0\)/);
      const narrow = /@media \(max-width: (\d+)px\) \{([\s\S]*?)\n  \}/.exec(src);
      expect(narrow, 'media query de la fenêtre étroite').not.toBeNull();
      expect(Number(narrow![1])).toBeGreaterThanOrEqual(800);
      expect(narrow![2]).toMatch(/flex-direction:\s*column/);
      expect(narrow![2]).toMatch(/overflow-x:\s*auto/);
    });
  }
});

describe('pages Dicter, Profil et Style d’écriture (tâche 3)', () => {
  const between = (src: string, from: string, to: string) => src.slice(src.indexOf(from), src.indexOf(to, src.indexOf(from)));
  const dictation = between(settingsTab, "section === 'dictation'", "section === 'identity'");

  it('Dicter garde touches de dictée et de traduction, langue, traduction, sons, volume, micro, médias, presse-papiers', () => {
    for (const k of ["kinds={['dictation', 'translate']}", 'Langue dictée', 'Traduire vers', 'Son au début', 'Volume des sons', 'Micro :', 'pauseMedia', 'restoreClipboard'])
      expect(dictation, k).toContain(k);
    expect(dictation).not.toContain('firstName');
    expect(dictation).not.toContain('speakerGender');
  });

  it('Profil : utilisateurs, puis prénom et genre, puis « Comptes en ligne » (clés Deepgram et OpenRouter)', () => {
    const order = [...profilePage.matchAll(/<Users \/>|section="(\w+)"/g)].map((m) => m[1] ?? 'users');
    expect(order).toEqual(['users', 'identity', 'keys']);
    const identity = between(settingsTab, "section === 'identity'", "section === 'keys'");
    expect(identity).toContain('firstName');
    expect(identity).toContain('speakerGender');
    expect(settingsTab).toContain("title={t('Comptes en ligne')}");
  });

  it('Style d’écriture : modes, case « Sauter la mise au propre » et fidélité sont dans les cartes de Modes, plus dans Settings', () => {
    expect(stylePage).not.toContain('Settings');
    expect(settingsTab).not.toContain('skipWhenClean');
    expect(modesTab).toContain('skipWhenClean');
    expect(modesTab).toContain('Fidélité');
  });

  it('les cartes de Dicter et de Profil ont plusieurs réglages (pas de carte d’une seule case)', () => {
    const identity = between(settingsTab, "section === 'identity'", "section === 'keys'");
    for (const c of [dictation, identity]) expect((c.match(/<input|<select|<KeyPicker/g) ?? []).length).toBeGreaterThan(1);
  });

  it('le lien « changer » de l’Accueil mène à Profil', () => {
    expect(dashboard).toContain("goto('profile', 'utilisateurs')");
  });
});

describe('pages Moteurs et modèles, Données et système, Mesures et essais (tâche 4)', () => {
  const order = (src: string, re: RegExp) => [...src.matchAll(re)].map((m) => m.slice(1).find(Boolean) ?? m[0]);

  it('Moteurs et modèles : une partie Moteurs, puis une partie Modèles', () => {
    expect(order(enginesPage, /<h2 class="part-title">\{t\('([^']+)'\)\}|<(Settings section="engines"|Install) \/>/g).filter(Boolean))
      .toEqual(['Moteurs', 'Settings section="engines"', 'Modèles', 'Install']);
  });

  it('Données et système : démarrage, sauvegarder et transférer, mise à jour, emplacements, puis la désinstallation en dernier, à part', () => {
    expect(order(dataPage, /<(System part="\w+"|BackupTransfer|Update|Settings section="places") \/>/g))
      .toEqual(['System part="main"', 'BackupTransfer', 'Update', 'Settings section="places"', 'System part="uninstall"']);
    expect(dataPage).toContain("t('Désinstallation')");
  });

  it('Mesures et essais : bancs, temps des dictées, vitesse au clavier, journal en direct, rapport', () => {
    expect(order(measuresPage, /<(Bench|TimingsCard|Journal part="\w+") \/>|id="(dashboard-vitesse-clavier)"/g))
      .toEqual(['Bench', 'TimingsCard', 'dashboard-vitesse-clavier', 'Journal part="live"', 'Journal part="report"']);
    expect(benchTab).not.toContain('<TimingsCard');
    expect(benchTab).not.toContain('Journal et rapport');
  });
});

describe('sélecteurs de touches : aucun vide (correctif de la touche de commande)', () => {
  it('chacune des trois touches (dictée, traduction, commande) est montrée sur une page, et la commande accepte une combinaison', () => {
    for (const row of ["kinds.includes('dictation')", "kinds.includes('translate')", "kinds.includes('command')"]) expect(keyPicker).toContain(row);
    expect(settingsTab).toContain("kinds={['dictation', 'translate']}");
    expect(activation).toContain("kinds={['command']}");
    // La capture de la commande parle d'une combinaison, pas d'une seule touche.
    expect(keyPicker).toContain("kind === 'command' ? t('Appuie sur la touche ou la combinaison à tenir en plus");
  });
});

describe('finitions de mise en page (chantier 2, tâche 1)', () => {
  it('la recherche de l’Aide reste en haut sous l’en-tête, et le sommaire et les ancres comptent sa hauteur', () => {
    expect(helpSearchBox).toMatch(/\.search \{[^}]*position:\s*sticky[^}]*top:\s*var\(--page-head/);
    expect(helpSearchBox).toContain("setProperty('--help-search'");
    for (const src of [help, helpEn]) expect(src).toMatch(/\.toc \{[^}]*top: calc\(var\(--page-head, 0px\) \+ var\(--help-search, 0px\)/);
  });

  it('le défilement au clavier s’arrête sous l’en-tête fixe (scroll-padding-top)', () => {
    expect(app).toMatch(/:global\(html\) \{[^}]*scroll-padding-top:[^;]*--page-head/);
  });

  it('le bandeau d’erreur laisse 20 px sous lui, autant que la remontée de l’en-tête', () => {
    expect(app).toMatch(/main > :global\(\.card\.bad\) \{ margin-bottom: 20px; \}/);
  });

  it('les tuiles de l’Accueil ont la même hauteur d’une ligne à l’autre', () => {
    expect(dashboard).toMatch(/\.grid \{[^}]*grid-auto-rows:\s*1fr/);
  });

  it('menu en fenêtre étroite : une bande qui défile de côté, sans retour à la ligne', () => {
    const narrow = /@media \(max-width: 640px\) \{([\s\S]*?)\n  \}/.exec(app)?.[1] ?? '';
    expect(narrow).toMatch(/flex-wrap:\s*nowrap/);
    expect(narrow).toMatch(/overflow-x:\s*auto/);
  });

  it('menu : nom du menu, page courante annoncée, icônes cachées aux lecteurs d’écran', () => {
    expect(app).toContain("<nav aria-label={t('Menu principal')}>");
    expect(app.match(/aria-current=\{nav\.tab === (tab|helpTab)\.id \? 'page' : undefined\}/g)?.length).toBe(2);
    expect(app.match(/<span class="icon" aria-hidden="true">/g)?.length).toBe(2);
  });

  it('commandes vocales décochées : inert (le clavier n’y entre plus), pas seulement grisées', () => {
    expect(commandsPage).toMatch(/<div class:off=\{settings\.value\?\.voiceCommandsEnabled === false\} inert=\{settings\.value\?\.voiceCommandsEnabled === false\}>/);
  });
});

describe('l’écart entre bandeaux d’erreur', () => {
  it('reste de 14 px entre deux bandeaux, 20 px seulement après le dernier', () => {
    expect(app).toMatch(/main > :global\(\.card\.bad\) \{ margin-bottom: 20px; \}/);
    expect(app).toMatch(/main > :global\(\.card\.bad:has\(\+ \.card\.bad\)\) \{ margin-bottom: 14px; \}/);
  });
});
