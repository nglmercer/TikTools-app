import { defineConfig } from '@playwright/test';
import { resolve } from 'node:path';

const port = Number(process.env.WIDGETS_TEST_PORT ?? 18789);
const baseURL = `http://127.0.0.1:${port}`;
const repositoryRoot = resolve(import.meta.dirname, '..', '..');

/**
 * Browser tests for the built OBS alert widgets. Specs drive the production
 * bundles in `dist/widgets/` (run `bun run build:widgets` first) through
 * the page test hook — no TikTok connection and no gateway needed.
 */
export default defineConfig({
  testDir: './tests',
  // `.e2e.ts` keeps these browser specs out of `bun test`'s scan.
  testMatch: '**/*.e2e.ts',

  fullyParallel: false,
  workers: 1,

  use: {
    baseURL,
    viewport: { width: 800, height: 600 },
    deviceScaleFactor: 1,
    colorScheme: 'dark',
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
  },

  webServer: {
    command: `bun run scripts/serve-widgets.ts --port ${port}`,
    url: `${baseURL}/follow/`,
    cwd: repositoryRoot,
    reuseExistingServer: true,
    timeout: 60_000,
  },
});
