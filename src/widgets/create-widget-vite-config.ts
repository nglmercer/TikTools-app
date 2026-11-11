import vue from '@vitejs/plugin-vue';
import { basename, resolve } from 'node:path';
import { defineConfig, type UserConfig } from 'vite';

const WIDGET_KIND = /^[a-z][a-z0-9-]*$/;

/**
 * Vite configuration shared by the standalone OBS widgets under
 * `src/widgets/<kind>/`. Each widget builds its own `index.html` entry into
 * `dist/widgets/<kind>/` as an independent bundle the event gateway serves at
 * `/widgets/<kind>/`; the relative `base` keeps assets resolving under that
 * prefix. Only that widget's output directory is emptied.
 *
 * Loaded by Vite in Node (`vite build --config src/widgets/<kind>/vite.config.ts`),
 * so it must not import anything from Bun. Run all widget builds with
 * `bun scripts/task.ts widgets`.
 *
 * @param widgetRoot Directory of the widget (`import.meta.dirname` of its config).
 * @param kind Widget kind; it must match the widget directory name.
 */
export function createWidgetViteConfig(widgetRoot: string, kind: string): UserConfig {
  if (!WIDGET_KIND.test(kind)) throw new Error(`invalid widget kind: ${JSON.stringify(kind)}`);
  if (basename(widgetRoot) !== kind) {
    throw new Error(`widget kind "${kind}" does not match its directory ${widgetRoot}`);
  }
  return defineConfig({
    root: widgetRoot,
    base: './',
    plugins: [vue()],
    build: {
      outDir: resolve(widgetRoot, '..', '..', '..', 'dist', 'widgets', kind),
      emptyOutDir: true,
      sourcemap: false,
    },
  });
}
