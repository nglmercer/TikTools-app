import { expect, test } from '@playwright/test';
import { toolbarCopy } from '../../src/web/demo/toolbar-copy.ts';
import { demoTourScenarios } from '../../src/web/demo/tours.ts';

test('tutorial selection prepares each matching scenario and reports dismissal without completion', async ({ page }) => {
  for (const id of Object.keys(demoTourScenarios) as Array<keyof typeof demoTourScenarios>) {
    await page.goto(`demo/?scenario=connection&locale=en&instance=select-${id}`);
    await expect(page.getByTestId('demo-guide')).toBeEnabled();
    await page.getByRole('combobox', { name: 'Guided walkthrough', exact: true }).click();
    await page.getByRole('option', { name: toolbarCopy.en.tourTitles[id], exact: true }).click();
    await page.getByTestId('demo-guide').click();
    await expect(page.locator('.driver-popover')).toBeVisible();
    expect(new URL(page.url()).searchParams.get('scenario')).toBe(demoTourScenarios[id]);
    await expect(page.getByTestId('demo-guide-status')).toHaveAttribute('data-status', 'active');
    if (id === 'changes') {
      await expect(page).toHaveURL(/#\/automations\?view=variables$/);
      await page.locator('.driver-popover-next-btn').click();
      await expect(page.locator('.driver-popover-title')).toHaveText('Review Changes');
      await expect(page.locator('[data-guide="changes"]')).toBeVisible();
    }
    await page.locator('.driver-popover-close-btn').click();
    await expect(page.getByTestId('demo-guide-status')).toHaveAttribute('data-status', 'dismissed');
    await expect(page.getByTestId('demo-guide')).toBeEnabled();
  }
});

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
