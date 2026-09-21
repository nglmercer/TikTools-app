import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import { defineConfig, devices } from '@playwright/test';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..', '..', '..');

/**
 * Standalone suite for the SonicBoom plugin UI, run against its compiled
 * `ui/dist/` output (`bun run build:sonicboom-ui` first). The UI talks to
 * a fake `window.tiktools` broker installed per test — never to the real
 * host — so the package stays independently testable.
 *
 * Run: `bun run test:sonicboom-ui`
 */
export default defineConfig({
  testDir: './tests',

  fullyParallel: false,
  workers: 1,

  use: {
    baseURL: 'http://127.0.0.1:4173',
    viewport: { width: 1280, height: 900 },
    deviceScaleFactor: 1,
    colorScheme: 'dark',
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
  },

  projects: [
    {
      name: 'chromium',
      use: {
        ...devices['Desktop Chrome'],
        viewport: { width: 1280, height: 900 },
      },
    },
  ],

  webServer: {
    command:
      'bunx vite preview --config plugins/sonicboom/ui/vite.config.ts --host 127.0.0.1 --port 4173 --strictPort',
    url: 'http://127.0.0.1:4173',
    cwd: repoRoot,
    reuseExistingServer: true,
    timeout: 120_000,
  },

  expect: {
    toHaveScreenshot: {
      animations: 'disabled',
      caret: 'hide',
    },
  },
});
