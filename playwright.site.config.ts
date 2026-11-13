import { defineConfig, devices } from '@playwright/test';
import { siteConfig } from './scripts/lib/site-config.ts';
const port = Number(process.env.TIKTOOLS_SITE_PORT ?? 4176);
export default defineConfig({
  testDir: './tests/site', outputDir: `test-results/site-${port}`, workers: 1, fullyParallel: false, timeout: 30000,
  use: { baseURL: `http://127.0.0.1:${port}${siteConfig().base}`, viewport: { width: 1440, height: 960 }, trace: 'retain-on-failure', screenshot: 'only-on-failure' },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
  webServer: { command: 'bun scripts/task.ts site preview', env: { TIKTOOLS_SITE_PORT: String(port) }, url: `http://127.0.0.1:${port}${siteConfig().base}`, reuseExistingServer: false },
});
