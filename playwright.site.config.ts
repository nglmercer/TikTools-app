import { defineConfig, devices } from '@playwright/test';
import { siteConfig } from './scripts/lib/site-config.ts';
const port = Number(process.env.TIKTOOLS_SITE_PORT ?? 4176);
/** One worker per CPU: more only makes the browsers fight for the same two cores. */
const workers = Number(process.env.TIKTOOLS_E2E_WORKERS ?? 2);
/** Tracing and failure screenshots cost about a fifth of a run; opt in with TIKTOOLS_E2E_DEBUG=1 when diagnosing. */
const debug = process.env.TIKTOOLS_E2E_DEBUG === '1';
if (!Number.isInteger(workers) || workers < 1) throw new Error('TIKTOOLS_E2E_WORKERS must be a positive integer.');
export default defineConfig({
  testDir: './tests/site', outputDir: `test-results/site-${port}`, workers, fullyParallel: true, timeout: 30000,
  use: { baseURL: `http://127.0.0.1:${port}${siteConfig().base}`, viewport: { width: 1440, height: 960 }, trace: debug ? 'retain-on-failure' : 'off', screenshot: debug ? 'only-on-failure' : 'off',
    // Tours, feed rows and view changes animate; specs that test the motion itself opt back in with test.use().
    reducedMotion: 'reduce' },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
  webServer: { command: 'bun scripts/task.ts site preview', env: { TIKTOOLS_SITE_PORT: String(port) }, url: `http://127.0.0.1:${port}${siteConfig().base}`, reuseExistingServer: false },
});
