#!/usr/bin/env node
// Captures de l'interface depuis une session cloud : le programme tourne sur Linux (sans touche ni micro) et la page est pilotée par
// Playwright (`require('playwright')` est installé dans l'environnement cloud).
//
// Usage (depuis le dépôt) :
//   cd ui && npm ci && npm run build && cd ..
//   cargo build -p scriibe-cli
//   SCRIIBE_DATA=/tmp/scriibe-essai target/debug/scriibe lancer --sans-navigateur &     # affiche l'adresse « Interface : http://127.0.0.1:…/<jeton>/ »
//   node tools/captures_interface.js <adresse> <dossier-de-sortie> [clair|sombre] [onglet…]
//
// Sans onglet : tous. L'onglet se désigne par un morceau de son titre (« Réglages », « Banc », « Tableau »…).
const { chromium } = require('playwright');
const fs = require('fs');

(async () => {
  const [url, out, scheme = 'clair', ...tabs] = process.argv.slice(2);
  if (!url || !out) {
    console.error('usage : node tools/captures_interface.js <adresse> <dossier> [clair|sombre] [onglet…]');
    process.exit(2);
  }
  fs.mkdirSync(out, { recursive: true });
  const browser = await chromium.launch();
  const context = await browser.newContext({ viewport: { width: 1100, height: 800 }, colorScheme: scheme === 'sombre' ? 'dark' : 'light' });
  const page = await context.newPage();
  const errors = [];
  page.on('pageerror', (e) => errors.push('erreur de page : ' + e.message));
  page.on('console', (m) => m.type() === 'error' && errors.push('console : ' + m.text()));
  await page.goto(url);
  await page.waitForTimeout(800);
  const labels = tabs.length ? tabs : await page.$$eval('nav button.nav', (bs) => bs.map((b) => b.textContent.trim().replace(/^\S+\s+/, '')).filter((t) => !t.startsWith('Quitter')));
  for (const label of labels) {
    await page.click(`nav button:has-text("${label}")`);
    await page.waitForTimeout(700);
    const file = `${out}/${label.replace(/[^a-z0-9]/gi, '_')}.png`;
    await page.screenshot({ path: file, fullPage: true });
    console.log(file);
  }
  console.log(errors.length ? errors.join('\n') : 'aucune erreur dans la page');
  await browser.close();
})();
