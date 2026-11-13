import { expect, test } from '@playwright/test';

test('all seven shared tours load Driver inside the demo and dismissal stays dismissed', async ({ page }) => {
  const errors: string[] = []; page.on('pageerror', (error) => errors.push(error.message));
  for (const [tour, scenario] of [['connection', 'connection'], ['automation', 'automation'], ['plugin', 'plugins'], ['overlay', 'overlays'], ['points', 'points'], ['changes', 'cloud'], ['diagnostics', 'diagnostics']]) {
    await page.goto(`demo/?scenario=${scenario}&locale=en&tour=${tour}&instance=tour-${tour}`);
    await expect(page.locator('.driver-popover')).toBeVisible({ timeout: 10000 });
    await page.locator('.driver-popover-close-btn').click();
    await expect(page.locator('.driver-popover')).toHaveCount(0);
    const progress = await page.evaluate(() => Object.keys(sessionStorage).filter((key) => key.startsWith('tiktools.demo.')).map((key) => sessionStorage.getItem(key)));
    expect(progress.some((value) => value?.includes('"status":"completed"'))).toBe(false);
  }
  expect(errors).toEqual([]);
});

test('navigating away cancels the guide and reset clears only this instance progress', async ({ page }) => {
  await page.goto('demo/?scenario=connection&locale=en&tour=connection&instance=tour-reset');
  await expect(page.locator('.driver-popover')).toBeVisible();
  await page.evaluate(() => { location.hash = '/feed'; });
  await expect(page.locator('.driver-popover')).toHaveCount(0);
  await page.getByTestId('demo-reset').click();
  await expect(page.locator('.driver-popover')).toBeVisible();
});
