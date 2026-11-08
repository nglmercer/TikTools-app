import { defineConfig } from '@playwright/test';

/** The Rust fixture supplies a real packaged owner adapter and isolated IPC host. */
export default defineConfig({
  testDir: '../e2e',
  testMatch: 'owner-proposal-review.spec.ts',
  workers: 1,
  fullyParallel: false,
  reporter: 'line',
  timeout: 80_000,
  outputDir: '/home/meme/.cache/tiktools-test-runs/owner-proposal-playwright',
  use: {
    viewport: { width: 1360, height: 960 },
    colorScheme: 'dark',
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
  },
});
