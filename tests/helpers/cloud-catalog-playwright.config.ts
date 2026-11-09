import { defineConfig } from '@playwright/test';

/** The Rust fixture supplies an actual packaged adapter, host and loopback Cloud HTTP service. */
export default defineConfig({
  testDir: '../e2e',
  testMatch: 'cloud-catalog-browser.spec.ts',
  workers: 1,
  fullyParallel: false,
  maxFailures: 1,
  reporter: 'line',
  timeout: 90_000,
  use: {
    viewport: { width: 1360, height: 960 },
    colorScheme: 'dark',
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
  },
});
