import vue from '@vitejs/plugin-vue';
import vueJsx from '@vitejs/plugin-vue-jsx';
import { defineConfig } from 'vite';

/**
 * Standalone build for the SonicBoom plugin UI. Produces static assets in
 * `ui/dist/` consumed as-is by the isolated plugin WebView (desktop) or
 * the sandboxed dev iframe. Never merged into the main TikTools bundle:
 * separate entry, separate output, relative base so assets resolve under
 * `tiktools-plugin://<id>/` as well as dev preview servers.
 *
 * Run: `bun run build:sonicboom-ui`
 */
export default defineConfig({
  root: import.meta.dirname,
  base: './',
  plugins: [vue(), vueJsx()],
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    sourcemap: false,
    chunkSizeWarningLimit: 500,
  },
  // The sandboxed web host mounts this UI at an opaque origin, so the
  // preview server answers cross-origin like every real plugin-asset host
  // must: dev middleware, `vite preview`, and the desktop custom protocol
  // alike. (The desktop protocol is NOT exempt: strict engines such as
  // WebKitGTK CORS-check custom-protocol subresource loads from opaque
  // origins too. Production parity is pinned by the Rust asset-server
  // header tests, not by this preview config.)
  preview: {
    headers: { 'Access-Control-Allow-Origin': '*' },
  },
});
