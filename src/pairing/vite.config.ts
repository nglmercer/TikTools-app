import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vite';

/**
 * Static build of the adapter's pairing page. The adapter (Rust) embeds the
 * output with `include_str!`, so the committed files under `generated/` are the
 * shipped artifacts. `task.ts ci` fails when they drift from this source.
 *
 * Output: `pairing.html` (styles inline, served as the unauthenticated page) and
 * `pair.js` (served at `/__local/pair.js`, hence `base`).
 */
export default defineConfig({
  root: fileURLToPath(new URL('.', import.meta.url)),
  base: '/__local/',
  build: {
    outDir: fileURLToPath(new URL('../../crates/tiktools-cli/src/web/generated', import.meta.url)),
    emptyOutDir: true,
    modulePreload: false,
    cssCodeSplit: false,
    sourcemap: false,
    rollupOptions: {
      input: fileURLToPath(new URL('./pairing.html', import.meta.url)),
      output: {
        entryFileNames: 'pair.js',
        chunkFileNames: 'pair-[name].js',
      },
    },
  },
});
