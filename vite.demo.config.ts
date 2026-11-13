import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';
import { resolve } from 'node:path';
import { siteConfig } from './scripts/lib/site-config.ts';
import { buildProvenance } from './scripts/lib/build-provenance.ts';

const root = resolve(import.meta.dirname, 'src/web');
export default defineConfig({
  root: resolve(root, 'demo'), base: `${siteConfig().base}demo/`, plugins: [vue(), { name: 'demo-provenance', generateBundle() { this.emitFile({ type: 'asset', fileName: '.build-info.json', source: JSON.stringify(buildProvenance('demo')) + '\n' }); } }],
  publicDir: resolve(import.meta.dirname, 'docs/content/public/demo-assets'),
  resolve: { alias: { '@': root } },
  define: { 'import.meta.env.VITE_TIKTOOLS_PUBLIC_UI_KIT': JSON.stringify('1'), __VUE_I18N_FULL_INSTALL__: true, __VUE_I18N_LEGACY_API__: false, __INTLIFY_PROD_DEVTOOLS__: false },
  build: { outDir: resolve(import.meta.dirname, 'dist/demo'), emptyOutDir: true, sourcemap: false },
  server: { host: '127.0.0.1', port: 4174, strictPort: true },
});
