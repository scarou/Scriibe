import { describe, expect, it } from 'vitest';
import { helpSections } from './help';
import { tabs } from './tabs';

/** Tous les fichiers de l'interface, en texte : on y cherche les liens vers une page ou une carte. */
const sources = import.meta.glob('../**/*.{svelte,ts}', { query: '?raw', import: 'default', eager: true }) as Record<string, string>;
const files = Object.entries(sources).filter(([f]) => !/\.test\.ts$|\/locales\//.test(f));

const pages = new Set<string>(tabs.map((t) => t.id));

/** Les ids de carte : statiques (`id="x"`) et à préfixe (`id={'x-' + …}`). */
const cardIds = new Set<string>();
const cardPrefixes: string[] = [];
for (const [, src] of files) {
  for (const m of src.matchAll(/<Card\b[^>]*?\bid="([^"]+)"/g)) cardIds.add(m[1]);
  for (const m of src.matchAll(/<Card\b[^>]*?\bid=\{'([^']+)'\s*\+/g)) cardPrefixes.push(m[1]);
}
const anchorExists = (a: string) => cardIds.has(a) || cardPrefixes.some((p) => a.startsWith(p));

type Link = { file: string; page: string; anchor?: string };
const links: Link[] = [];
for (const [file, src] of files) {
  // go('page', 'ancre'), goto(…), ongoto(…), {@render go('Libellé', 'page', 'ancre')}
  for (const m of src.matchAll(/\b(?:nav\.go|goto|ongoto)\('([a-z]+)'(?:,\s*'([^']+)')?\)/g)) links.push({ file, page: m[1], anchor: m[2] });
  for (const m of src.matchAll(/\{@render go\('[^']*',\s*'([a-z]+)'(?:,\s*'([^']+)')?\)/g)) links.push({ file, page: m[1], anchor: m[2] });
}

describe('liens vers une page ou une carte', () => {
  it('le test trouve bien des liens (Aide, Accueil, assistant, cartes)', () => {
    expect(links.length).toBeGreaterThan(30);
    expect(cardIds.size).toBeGreaterThan(40);
  });

  it('chaque lien vise une page du menu qui existe', () => {
    for (const l of links) expect(pages.has(l.page), `${l.file} → ${l.page}`).toBe(true);
  });

  it('chaque lien qui vise une carte vise une carte qui existe', () => {
    for (const l of links.filter((x) => x.anchor)) expect(anchorExists(l.anchor!), `${l.file} → ${l.page} #${l.anchor}`).toBe(true);
  });

  it('une carte visée est bien posée sur la page visée (fichier de la page ou composant qu\'elle assemble)', () => {
    const owner = new Map<string, string>();
    for (const [file, src] of files) for (const m of src.matchAll(/<Card\b[^>]*?\bid="([^"]+)"/g)) owner.set(m[1], file);
    const pageFile: Record<string, string[]> = {
      dictate: ['Settings.svelte', 'KeyPicker.svelte'], profile: ['Users.svelte', 'Settings.svelte'], style: ['Modes.svelte'],
      engines: ['Settings.svelte', 'Install.svelte', 'InstalledModels.svelte', 'HubSearch.svelte'], appearance: ['Settings.svelte'],
      data: ['System.svelte', 'BackupTransfer.svelte', 'Update.svelte', 'Settings.svelte'], measures: ['Bench.svelte', 'Journal.svelte', 'Dashboard.svelte', 'TimingsCard.svelte', 'Measures.svelte', 'BenchArchive.svelte', 'TrialBench.svelte'],
      commands: ['commands/'], dictionary: ['Dictionary.svelte'], history: ['History.svelte'], dashboard: ['Dashboard.svelte'],
    };
    for (const l of links.filter((x) => x.anchor && owner.has(x.anchor) && pageFile[x.page])) {
      const f = owner.get(l.anchor!)!;
      expect(pageFile[l.page].some((p) => f.includes(p)), `${l.file} → ${l.page} #${l.anchor} (carte dans ${f})`).toBe(true);
    }
  });

  it('chaque ligne de la carte « Sauvegarder et transférer » vise une page et une carte qui existent, et une page qui la prend en charge', () => {
    const nav = sources['./nav.svelte.ts'];
    const targets = [...nav.matchAll(/'([a-z]+-(?:export|import))': \{ tab: '([a-z]+)', anchor: '([^']+)' \}/g)];
    expect(targets.length).toBe(8);
    for (const [, kind, tab, anchor] of targets) {
      expect(pages.has(tab), kind).toBe(true);
      expect(anchorExists(anchor), `${kind} → #${anchor}`).toBe(true);
      const consumers = files.filter(([, src]) => src.includes(`'${kind}'`) && /nav\.transferRequest/.test(src) && !/nav\.svelte\.ts|BackupTransfer/.test(src));
      expect(consumers.length, `${kind} : aucune page ne prend cette demande`).toBeGreaterThan(0);
    }
    const card = sources['../components/BackupTransfer.svelte'];
    for (const [, kind] of targets) expect(card.includes(`'${kind}'`), `${kind} : aucune ligne`).toBe(true);
  });

  it('chaque bouton « ? » (help="…") et chaque openHelp(…) ouvre une section de l\'Aide qui existe', () => {
    const ids = new Set<string>(helpSections.map((h) => h.id));
    let n = 0;
    for (const [file, src] of files) {
      for (const m of src.matchAll(/<(?:Card|HelpButton)\b[^>]*?\bhelp="([^"]+)"/g)) { n++; expect(ids.has(m[1]), `${file} → help="${m[1]}"`).toBe(true); }
      for (const m of src.matchAll(/openHelp\('([^']+)'\)/g)) { n++; expect(ids.has(m[1]), `${file} → openHelp('${m[1]}')`).toBe(true); }
    }
    expect(n).toBeGreaterThan(40);
  });
});
