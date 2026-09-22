import vue from '@vitejs/plugin-vue';
import { resolve } from 'node:path';
import { defineConfig } from 'vite';

/**
 * Standalone build for the Chat Overlay widget. Produces static assets the
 * event gateway serves at `/widgets/chat/` for OBS Browser Sources.
 * Separate entry and output from the main TikTools bundle; relative base so
 * assets resolve under the gateway prefix.
 *
 * Run: `bun run build:widget:chat`
 */
export default defineConfig({
  root: import.meta.dirname,
  base: './',
  plugins: [vue()],
  build: {
    outDir: resolve(import.meta.dirname, '../../../dist/widgets/chat'),
    emptyOutDir: true,
    sourcemap: false,
  },
});
