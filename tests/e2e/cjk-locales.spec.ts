import { expect, test } from '@playwright/test';
import { emptyState } from './fixtures/states.ts';
import { installFakeHost, waitForFrontend } from './fixtures/tiktools-host.ts';

for (const { locale, name, label } of [
  { locale: 'zh', name: '简体中文', label: '界面语言' },
  { locale: 'ja', name: '日本語', label: '表示言語' },
]) {
  test(`${locale}: the real app switches language and preserves it after reload`, async ({ page }) => {
    await page.emulateMedia({ reducedMotion: 'reduce' });
    const capture = await installFakeHost(page, emptyState());
    await page.goto('/#/settings');
    await waitForFrontend(page);
    const picker = page.getByRole('combobox', { name: 'Interface language', exact: true });
    await picker.fill(locale);
    await page.getByRole('option', { name: new RegExp(name) }).click();
    await expect(page.locator('html')).toHaveAttribute('lang', locale);
    await expect(page.getByRole('combobox', { name: label, exact: true })).toHaveValue(name);
    expect(await page.evaluate(() => localStorage.getItem('tiktok-live-locale'))).toBe(locale);
    await page.reload();
    await waitForFrontend(page);
    await expect(page.locator('html')).toHaveAttribute('lang', locale);
    const localizedPicker = page.getByRole('combobox', { name: label, exact: true });
    await expect(localizedPicker).toHaveValue(name);
    await page.screenshot({ path: `/tmp/tiktools-${locale}-app-desktop.png` });
    await page.setViewportSize({ width: 390, height: 844 });
    await expect(localizedPicker).toBeInViewport();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    await page.screenshot({ path: `/tmp/tiktools-${locale}-app-mobile.png` });
    await localizedPicker.fill('en');
    await page.getByRole('option', { name: /^English/ }).click();
    await expect(page.locator('html')).toHaveAttribute('lang', 'en');
    capture.assertClean();
  });
}

test('the real startup accepts jp preferences and migrates them to canonical ja', async ({ page }) => {
  const capture = await installFakeHost(page, emptyState(), { locale: 'jp' });
  await page.goto('/#/settings');
  await waitForFrontend(page);
  await expect(page.locator('html')).toHaveAttribute('lang', 'ja');
  await expect(page.getByRole('combobox', { name: '表示言語', exact: true })).toHaveValue('日本語');
  expect(await page.evaluate(() => localStorage.getItem('tiktok-live-locale'))).toBe('ja');
  capture.assertClean();
});
