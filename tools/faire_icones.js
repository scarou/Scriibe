#!/usr/bin/env node
// Fabrique les icônes de Scriibe depuis les SVG de logo/ (Playwright pour tracer, `tools/faire_icones.py` pour assembler).
// Usage : node tools/faire_icones.js <dossier-de-sortie>   puis   python3 tools/faire_icones.py <dossier-de-sortie>
const { chromium } = require('playwright');
const fs = require('fs');
const path = require('path');

const SIZES = [16, 24, 32, 48, 64, 128, 256, 512];
(async () => {
  const out = process.argv[2];
  if (!out) {
    console.error('usage : node tools/faire_icones.js <dossier>');
    process.exit(2);
  }
  fs.mkdirSync(out, { recursive: true });
  const browser = await chromium.launch();
  const page = await browser.newPage();
  const logo = path.join(__dirname, '..', 'logo');
  const jobs = [];
  for (const os of ['windows', 'macos', 'linux']) jobs.push({ svg: `scriibe-icone-${os}.svg`, name: `icone-${os}`, sizes: os === 'windows' ? SIZES : [512, 1024] });
  // La plume blanche seule, pour le masque de l'icône de la zone de notification.
  jobs.push({ svg: 'scriibe-symbole.svg', name: 'plume', sizes: [32], white: true, plain: true });
  for (const job of jobs) {
    let svg = fs.readFileSync(path.join(logo, job.svg), 'utf8');
    if (job.white) svg = svg.replace(/currentColor/g, '#ffffff');
    for (const size of job.sizes) {
      // La plume seule occupe 70 % de la hauteur : elle tient dans le disque coloré de l'icône de la zone de notification.
      const h = job.plain ? Math.round(size * 0.7) : size;
      const w = job.plain ? Math.round((h * 64) / 91) : size;
      await page.setViewportSize({ width: size, height: size });
      await page.setContent(`<html><body style="margin:0;background:transparent"><div style="width:${size}px;height:${size}px;display:flex;align-items:center;justify-content:center">${svg.replace(/width="\d+" height="\d+"/, `width="${w}" height="${h}"`)}</div></body></html>`);
      await page.screenshot({ path: path.join(out, `${job.name}-${size}.png`), omitBackground: true });
    }
  }
  await browser.close();
  console.log('PNG écrits dans', out);
})();
