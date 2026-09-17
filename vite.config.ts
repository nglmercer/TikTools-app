import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';
import vueJsx from '@vitejs/plugin-vue-jsx';
import { resolve } from 'node:path';

const root = resolve(process.cwd(), 'src/web');

export default defineConfig({
  root,
  plugins: [vue(), vueJsx()],
  build: {
    outDir: resolve(process.cwd(), 'dist/web'),
    emptyOutDir: true,
    sourcemap: true,
  },
  server: {
    host: '127.0.0.1',
    port: Number(process.env.TIKTOOLS_WEB_PORT ?? 3000),
    // The dev launcher allocates the port and passes the actual URL to the
    // desktop via TIKTOOLS_DEV_URL. Never drift to another port silently.
    strictPort: true,
  },
});
