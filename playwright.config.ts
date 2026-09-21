import { defineConfig, devices } from '@playwright/test';

const webPort = Number(process.env.TIKTOOLS_WEB_PORT ?? 3000);
const baseURL = `http://127.0.0.1:${webPort}`;

/**
 * Browser-level E2E for the Vite/Vue SPA in Chromium.
 *
 * Playwright never drives the Wry shell: native behavior stays covered by
 * Rust tests, while these specs exercise the frontend against a fake host
 * installed at the exact `window.ipc` boundary
 * (`tests/e2e/fixtures/tiktools-host.ts`).
 *
 * Docs: `docs/PLAYWRIGHT.md`.
 */
export default defineConfig({
  testDir: './tests/e2e',

  // One Chromium project owns the canonical screenshot baselines. The dev
  // server is shared (`reuseExistingServer`), so specs run serially.
  fullyParallel: false,
  workers: 1,

  use: {
    baseURL,
    viewport: { width: 1440, height: 900 },
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
        viewport: { width: 1440, height: 900 },
      },
    },
  ],

  webServer: {
    command: 'bun run serve:web',
    url: baseURL,
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
