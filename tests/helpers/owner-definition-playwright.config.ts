import { defineConfig } from '@playwright/test';

/** The Rust fixture supplies a real packaged Owner adapter and isolated IPC host. */
export default defineConfig({
  testDir: '../e2e',
  testMatch: 'owner-definition-recovery.spec.ts',
  workers: 1,
  fullyParallel: false,
  maxFailures: 1,
  reporter: 'line',
  timeout: 100_000,
  outputDir: '/home/meme/.cache/tiktools-test-runs/owner-definition-playwright',
  use: {
    viewport: { width: 1360, height: 960 },
    colorScheme: 'dark',
    testIdAttribute: 'data-test',
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
  },
});
