import vue from '@vitejs/plugin-vue';
import { resolve } from 'node:path';
import { defineConfig } from 'vite';
export default defineConfig({ root: import.meta.dirname, base: './', plugins: [vue()], build: { outDir: resolve(import.meta.dirname, '../../../dist/widgets/ranking'), emptyOutDir: true, sourcemap: false } });
