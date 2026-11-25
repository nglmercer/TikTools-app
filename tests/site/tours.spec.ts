import { expect, test, type Page } from '@playwright/test';
import { testMessage } from '../../src/web/shared/i18n/test-message.ts';
import { demoTourScenarios, demoTutorialIds } from '../../src/web/demo/tours.ts';
import { tourCatalogs } from '../../src/web/features/guided-help/catalogs/index.ts';

async function selectCatalogTour(page: Page, title: string): Promise<void> {
  await page.locator('.tutorial-catalog__card').filter({ has: page.locator('strong').getByText(title, { exact: true }) }).click();
  await page.locator('.tutorial-catalog__detail button').click();
}

async function launchFromCatalog(page: Page, title: string): Promise<void> {
  await page.getByTestId('demo-guide').click();
  await selectCatalogTour(page, title);
}

for (const locale of ['zh', 'ja'] as const) test(`${locale}: playback and a selected walkthrough use the localized real interface`, async ({ page }) => {
  await page.goto(`demo/?scenario=connection&locale=${locale}&instance=cjk-tour-${locale}`);
  await expect(page.locator('html')).toHaveAttribute('lang', locale);
  await page.getByTestId('demo-play').click();
  await expect(page.getByTestId('demo-playback-status')).toHaveAttribute('data-status', 'playing');
  await expect(page.getByTestId('demo-event-count')).toContainText('1 / 300');
  await page.getByTestId('demo-play').click();
  await launchFromCatalog(page, testMessage(locale, 'guides.catalog.tours.overlay.title'));
  await expect(page.locator('.driver-popover-title')).toHaveText(tourCatalogs[locale].overlay.gateway.title);
  await expect(page.locator('.driver-popover-next-btn')).toHaveText(tourCatalogs[locale].controls.next);
  await page.locator('.driver-popover-close-btn').click();
  await expect(page.getByTestId('demo-guide-status')).toHaveAttribute('data-status', 'dismissed');
});

test('tutorial selection prepares each matching scenario and reports dismissal without completion', async ({ page }) => {
  test.setTimeout(120000);
  for (const id of Object.keys(demoTourScenarios) as Array<keyof typeof demoTourScenarios>) {
    await page.goto(`demo/?scenario=connection&locale=en&instance=select-${id}`);
    await expect(page.getByTestId('demo-guide')).toBeEnabled();
    await launchFromCatalog(page, testMessage('en', `guides.catalog.tours.${id}.title`));
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

test('the in-app demo catalog keeps unsupported tours out and delegates scenario preparation', async ({ page }) => {
  await page.goto('demo/?scenario=connection&locale=en&instance=menu-catalog');
  await page.locator('.app-menu-bar').getByRole('menuitem', { name: 'Help', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Guides and tutorials', exact: true }).click();
  await expect(page.locator('.tutorial-catalog__card')).toHaveCount(demoTutorialIds.length);
  await expect(page.locator('.tutorial-catalog__card').filter({ hasText: 'Account and backups' })).toHaveCount(0);
  await selectCatalogTour(page, testMessage('en', 'guides.catalog.tours.overlayObs.title'));
  await expect(page.locator('.driver-popover')).toBeVisible();
  expect(new URL(page.url()).searchParams.get('scenario')).toBe('overlays');
  await page.locator('.driver-popover-close-btn').click();
});

test('shared tours load Driver inside the demo and dismissal stays dismissed', async ({ page }) => {
  const errors: string[] = []; page.on('pageerror', (error) => errors.push(error.message));
  for (const [tour, scenario] of Object.entries(demoTourScenarios)) {
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

test('the recommended tour follows in-app navigation without starting automatically', async ({ page }) => {
  await page.goto('demo/?scenario=connection&locale=en&instance=recommendation');
  const recommendation = page.locator('.demo-toolbar__recommendation');
  const connectionTitle = testMessage('en', 'guides.catalog.tours.connection.title');
  const pointsTitle = testMessage('en', 'guides.catalog.tours.points.title');
  await expect(recommendation).toContainText(connectionTitle);
  await expect(page.locator('.driver-popover')).toHaveCount(0);
  await page.getByRole('link', { name: 'Points', exact: true }).click();
  await expect(recommendation).toContainText(pointsTitle);
  await expect(page.locator('.driver-popover')).toHaveCount(0);
  await page.getByTestId('demo-guide').click();
  await expect(page.locator('.tutorial-catalog__card--active')).toContainText(pointsTitle);
});
