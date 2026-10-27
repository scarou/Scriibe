import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// base './' : l'interface est servie sous une adresse secrète (/<jeton>/), les chemins doivent rester relatifs.
export default defineConfig({
  base: './',
  plugins: [svelte()],
  build: { outDir: 'dist', emptyOutDir: true, target: 'es2022' },
  test: { environment: 'node', include: ['src/**/*.test.ts'] },
});
