import vue from '@vitejs/plugin-vue';
import { resolve } from 'node:path';
import { defineConfig } from 'vite';

/**
 * Standalone build for the Follow Alert widget. Produces static assets the
 * event gateway serves at `/widgets/follow/` for OBS Browser Sources.
 * Separate entry and output from the main TikTools bundle; relative base so
 * assets resolve under the gateway prefix.
 *
 * Run: `bun run build:widget:follow`
 */
export default defineConfig({
  root: import.meta.dirname,
  base: './',
  plugins: [vue()],
  build: {
    outDir: resolve(import.meta.dirname, '../../../dist/widgets/follow'),
    emptyOutDir: true,
    sourcemap: false,
  },
});
