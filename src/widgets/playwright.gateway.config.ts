import { defineConfig } from '@playwright/test';

/**
 * Gateway-backed OBS widget tests. Specs boot the real `event-gateway`
 * binary in an installed-package layout (see ./gateway-harness.ts), so
 * origin checks, credential auth, topic restriction, and event delivery
 * run exactly as in production. No shared webServer: each spec file owns
 * its gateway lifetime via beforeAll/afterAll.
 */
export default defineConfig({
  testDir: './tests-gateway',
  // `.e2e.ts` keeps these browser specs out of `bun test`'s scan.
  testMatch: '**/*.e2e.ts',

  fullyParallel: false,
  workers: 1,
  timeout: 90_000,

  use: {
    viewport: { width: 800, height: 600 },
    deviceScaleFactor: 1,
    colorScheme: 'dark',
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
  },
});
