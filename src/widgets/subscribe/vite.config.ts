import vue from '@vitejs/plugin-vue';
import { resolve } from 'node:path';
import { defineConfig } from 'vite';

/**
 * Standalone build for the Subscribe Alert widget. Produces static assets the
 * event gateway serves at `/widgets/subscribe/` for OBS Browser Sources.
 * Separate entry and output from the main TikTools bundle; relative base so
 * assets resolve under the gateway prefix.
 *
 * Run: `bun run build:widget:subscribe`
 */
export default defineConfig({
  root: import.meta.dirname,
  base: './',
  plugins: [vue()],
  build: {
    outDir: resolve(import.meta.dirname, '../../../dist/widgets/subscribe'),
    emptyOutDir: true,
    sourcemap: false,
  },
});
