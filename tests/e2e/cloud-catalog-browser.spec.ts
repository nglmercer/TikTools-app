/** Packaged Vue -> native IPC catalog -> loopback Cloud HTTP. Browser responses
 * are never replaced or fabricated by this test. */
import { expect, test, type BrowserContext } from '@playwright/test';
import english from '../../src/web/shared/i18n/locales/en.ts';
import spanish from '../../src/web/shared/i18n/locales/es.ts';

const ownerUrl = process.env['TIKTOOLS_CLOUD_CATALOG_BROWSER_URL'];
let cookies: Awaited<ReturnType<BrowserContext['cookies']>> = [];

for (const locale of ['en', 'es'] as const) {
  test(`packaged Cloud catalog uses exact pinned pagination (${locale})`, async ({ page }, testInfo) => {
    test.skip(!ownerUrl, 'Requires the real Rust cloud_catalog_browser fixture');
    const words = locale === 'en' ? english : spanish;
    const methods: { method: string; params?: Record<string, unknown> }[] = [];
    const errors: string[] = [];
    page.on('pageerror', (error) => errors.push(error.message));
    page.on('request', (request) => {
      const url = new URL(request.url());
      if (!['blob:', 'data:'].includes(url.protocol) && url.host !== new URL(ownerUrl!).host) {
        errors.push('Unexpected external browser request');
      }
    });
    page.on('websocket', (socket) => socket.on('framesent', (frame) => {
      try {
        const sent = JSON.parse(String(frame.payload)) as { method?: unknown; params?: Record<string, unknown> };
        if (typeof sent.method === 'string') methods.push({ method: sent.method, params: sent.params });
      } catch { /* Non-RPC transport frames are irrelevant. */ }
    }));
    await page.setViewportSize(locale === 'es' ? { width: 390, height: 844 } : { width: 1360, height: 960 });
    await page.addInitScript((language) => { localStorage.setItem('tiktok-live-locale', language); }, locale);
    if (cookies.length) await page.context().addCookies(cookies);
    await page.goto(ownerUrl!);
    if (!cookies.length) {
      await page.getByLabel('Pairing code', { exact: true }).fill('948231');
      await page.getByRole('button', { name: 'Connect', exact: true }).click();
    }
    await page.getByRole('link', { name: locale === 'es' ? 'Ajustes' : 'Settings', exact: true }).waitFor({ state: 'attached' });
    cookies = await page.context().cookies();
    await page.goto(`${ownerUrl!}/#/plugins`);
    await page.getByRole('button', { name: words.plugins.views.cloud, exact: true }).click();
    const catalog = page.locator('.cloud-catalog');
    await expect(catalog).toBeVisible();
    await expect(catalog).toContainText('catalog.p00');
    await expect(catalog.locator('.cloud-catalog__plugin')).toHaveCount(50);
    await expect(catalog).toContainText(words.plugins.cloud.trustHint);
    const first = catalog.locator('.cloud-catalog__plugin').first();
    await first.getByRole('button', { name: words.plugins.cloud.showVersions, exact: true }).click();
    await expect(first).toContainText('1.0.0+build00');
    await expect(first.locator('.cloud-catalog__versions li')).toHaveCount(50);
    await first.getByRole('navigation', { name: words.plugins.cloud.versionPages }).getByRole('button', { name: words.plugins.cloud.next, exact: true }).click();
    await expect(first).toContainText('1.0.0+build50');
    await expect(first.locator('.cloud-catalog__versions li')).toHaveCount(1);
    await catalog.getByRole('navigation', { name: words.plugins.cloud.pages }).getByRole('button', { name: words.plugins.cloud.next, exact: true }).click();
    await expect(catalog.locator('.cloud-catalog__plugin')).toHaveCount(1);
    await expect(catalog).toContainText('catalog.p50');
    await catalog.getByRole('button', { name: words.plugins.cloud.showVersions, exact: true }).click();
    await expect(catalog).toContainText(words.plugins.cloud.yanked);
    await expect(catalog).toContainText(words.plugins.cloud.keyRevoked);
    await expect(catalog).toContainText(words.plugins.cloud.publisherUnapproved);
    await expect(catalog).toContainText('Fixture advisory');
    expect(methods.some((entry) => entry.method === 'registry.cloudCatalogSummaries' && typeof entry.params?.['cursor'] === 'string' && entry.params?.['generation'] === '9007199254740993')).toBe(true);
    expect(methods.some((entry) => entry.method === 'registry.cloudCatalogVersions' && typeof entry.params?.['cursor'] === 'string' && entry.params?.['id'] === 'catalog.p00' && entry.params?.['generation'] === '9007199254740993')).toBe(true);
    expect(methods.some((entry) => entry.method === 'registry.cloudCatalog')).toBe(false);
    expect(methods.some((entry) => entry.method === 'plugins.packages.inspect' || entry.method === 'plugins.packages.install')).toBe(false);
    await expect.poll(() => page.evaluate(() => JSON.stringify(Object.entries(localStorage)))).toContain('cloud');
    await page.reload();
    await expect(page.getByRole('radio', { name: words.plugins.views.cloud, exact: true })).toHaveAttribute('aria-checked', 'true');
    await expect(page.locator('.cloud-catalog')).toContainText('catalog.p00');
    if (locale === 'es') {
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1)).toBe(true);
      expect(await page.locator('.cloud-catalog').evaluate((node) => node.scrollWidth <= node.clientWidth + 1)).toBe(true);
    }
    await page.screenshot({ path: testInfo.outputPath(`cloud-catalog-${locale}.png`) });
    expect(errors).toEqual([]);
  });
}
