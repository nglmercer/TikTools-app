import { docLocales } from '../../docs/locales.ts';
import { test, expect } from '@playwright/test';

test('demo toolbar shares real controls, contains menus and fits desktop and mobile', async ({ page }) => {
  const errors: string[] = []; page.on('pageerror', (error) => errors.push(error.message));
  await page.goto('demo/?scenario=connection&locale=en');
  await expect(page.getByTestId('demo-more')).toBeEnabled();
  await expect(page.getByTestId('demo-play')).toHaveClass(/base-button/);
  await page.getByTestId('demo-play').click();
  await expect(page.getByTestId('demo-play')).toHaveAttribute('aria-pressed', 'true');
  await page.getByTestId('demo-play').click();
  await expect(page.getByTestId('demo-play')).toHaveAttribute('aria-pressed', 'false');
  await page.getByTestId('demo-more').click();
  await page.getByRole('menuitem', { name: 'Component reference', exact: true }).click();
  await expect(page).toHaveURL(/#\/components\/buttons$/);
  await expect(page.locator('.component-catalog')).toBeVisible();
  await page.getByTestId('demo-more').click();
  await page.getByRole('menuitemcheckbox', { name: 'Simulate an action failure' }).click();
  await page.getByTestId('demo-more').click();
  await expect(page.getByRole('menuitemcheckbox', { name: 'Simulate an action failure' })).toHaveAttribute('aria-checked', 'true');
  await page.getByRole('menuitemcheckbox', { name: 'Simulate an action failure' }).click();
  for (const width of [1440, 1058, 390]) {
    await page.setViewportSize({ width, height: 960 });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    const toolbar = await page.locator('#demo-controls').boundingBox();
    const workspace = await page.locator('#app').boundingBox();
    expect(toolbar && workspace && Math.abs(toolbar.y + toolbar.height - workspace.y) < 2).toBe(true);
    const shell = await page.locator('.app-layout').boundingBox();
    expect(shell && shell.y + shell.height <= 962).toBe(true);
    await expect(page.getByRole('button', { name: /^Diagnostics/ })).toBeInViewport();
    await expect(page.getByTestId('demo-reset')).toBeInViewport();
  }
  await page.getByRole('combobox', { name: 'Scenario', exact: true }).click();
  await page.getByRole('option', { name: 'Points', exact: true }).click();
  await expect(page).toHaveURL(/scenario=points/);
  await expect(page.getByTestId('demo-step')).toBeVisible();
  expect(errors).toEqual([]);
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.screenshot({ path: '/tmp/tiktools-docs-design-demo-desktop.png' });
  await page.setViewportSize({ width: 390, height: 844 });
  await page.screenshot({ path: '/tmp/tiktools-docs-design-demo-mobile.png' });
});

test('product landing pages show the real demo automatically and fit every viewport', async ({ page }) => {
  for (const route of ['', ...docLocales.map((locale) => `${locale}/`)]) {
    await page.goto(route);
    const title = page.locator('h1'); await expect(title).toHaveCount(1);
    expect(await title.evaluate((node) => parseFloat(getComputedStyle(node).fontSize))).toBeGreaterThanOrEqual(32);
    await expect(page.locator('main a[href*="/user/"]').first()).toBeVisible();
    await expect(page.locator('main a[href*="/developer/"]').first()).toBeVisible();
    await expect(page.locator('iframe')).toHaveCount(1);
    await expect(page.frameLocator('.product-stage iframe').locator('.live-feed')).toContainText('Hello TikTools!');
    await page.setViewportSize({ width: 390, height: 844 });
    const layout = await page.evaluate(() => ({ width: document.documentElement.scrollWidth, viewport: innerWidth, overflow: [...document.querySelectorAll('main *')].filter((node) => node.getBoundingClientRect().right > innerWidth + 1).slice(0, 8).map((node) => ({ element: node.tagName, class: node.className, right: node.getBoundingClientRect().right })) }));
    expect(layout.width, JSON.stringify(layout.overflow)).toBeLessThanOrEqual(layout.viewport);
    await page.screenshot({ path: `/tmp/tiktools-docs-design-${route ? route.slice(0, 2) : 'root'}-mobile.png`, fullPage: true });
    await page.setViewportSize({ width: 1440, height: 960 });
    await page.screenshot({ path: `/tmp/tiktools-docs-design-${route ? route.slice(0, 2) : 'root'}-desktop.png`, fullPage: true });
  }
  await page.emulateMedia({ colorScheme: 'dark' });
  await page.goto('');
  await expect(page.locator('html')).toHaveClass(/dark/);
  await page.screenshot({ path: '/tmp/tiktools-docs-design-root-dark.png', fullPage: true });
  await page.goto('es/');
  await expect(page.frameLocator('.product-stage iframe').locator('.live-feed')).toContainText('Hello TikTools!');
  await page.screenshot({ path: '/tmp/tiktools-docs-design-es-dark.png', fullPage: true });
});
