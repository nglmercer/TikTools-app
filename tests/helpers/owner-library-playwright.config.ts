import { defineConfig } from '@playwright/test';

/** Rust supplies an isolated packaged Owner adapter and actual IPC host. */
export default defineConfig({
  testDir: '../e2e', testMatch: 'owner-library-recovery.spec.ts', workers: 1, fullyParallel: false,
  maxFailures: 1, reporter: 'line', timeout: 140_000,
  outputDir: '/home/meme/.cache/tiktools-test-runs/owner-library-playwright',
  use: { viewport: { width: 1360, height: 960 }, colorScheme: 'dark', testIdAttribute: 'data-test',
    trace: 'retain-on-failure', screenshot: 'only-on-failure' },
});
