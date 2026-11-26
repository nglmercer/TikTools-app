import { tourModules } from '../../src/web/features/guided-help/tours/index.ts';
import type { TourId } from '../../src/web/features/guided-help/model/types.ts';
import { expect, test, type Page } from '@playwright/test';
import { demoTourScenarios, demoTutorialIds } from '../../src/web/demo/tours.ts';
import { tourCatalogs } from '../../src/web/features/guided-help/catalogs/index.ts';
import { tourCategory } from '../../src/web/features/guided-help/model/tutorial-taxonomy.ts';
import { tourRegistry } from '../../src/web/features/guided-help/model/registry.ts';

const tutorialCard = (locale: keyof typeof tourCatalogs, id: TourId) => tourCatalogs[locale].card[id];

async function selectCatalogTour(page: Page, title: string): Promise<void> {
  const dialog = page.locator('dialog[open].tutorial-catalog');
  await dialog.locator('input[type="search"]').fill(title);
  await dialog.locator('.tutorial-catalog__card').filter({ has: page.locator('strong').getByText(title, { exact: true }) }).click();
  await dialog.locator('.tutorial-catalog__start').click();
}

async function launchFromCatalog(page: Page, title: string): Promise<void> {
  await page.getByTestId('demo-guide').click();
  await selectCatalogTour(page, title);
}


async function expectStep(page: Page, id: TourId, step: (typeof tourRegistry)[TourId]['steps'][number]): Promise<void> {
  const copy = (tourCatalogs.en[id] as unknown as Record<string, { title: string }>)[step.id]!;
  await expect(page.locator('.driver-popover-title')).toHaveText(copy.title, { timeout: 10000 });
  await expect(page.locator(`[data-guide="${step.target}"]`)).toBeVisible();
}

// The demo deliberately does not emulate the signed package-installation backend.
// The coordinator capability-skips optional steps behind these methods, so the
// walkthrough assertion must apply the same availability contract.
const methodsNotSimulatedByDemo = new Set(['plugins.packages.install']);

async function walkTour(page: Page, id: TourId): Promise<void> {
  const steps = tourRegistry[id].steps.filter((step) => step.requiredCapabilities.every((method) => !methodsNotSimulatedByDemo.has(method)));
  for (const [index, step] of steps.entries()) {
    await expectStep(page, id, step);
    if (index < steps.length - 1) await page.locator('.driver-popover-next-btn').click();
  }
}

for (const locale of ['zh', 'ja'] as const) test(`${locale}: playback and a selected walkthrough use the localized real interface`, async ({ page }) => {
  await page.goto(`demo/?scenario=connection&locale=${locale}&instance=cjk-tour-${locale}`);
  await expect(page.locator('html')).toHaveAttribute('lang', locale);
  await page.getByTestId('demo-play').click();
  await expect(page.getByTestId('demo-playback-status')).toHaveAttribute('data-status', 'playing');
  await expect(page.getByTestId('demo-event-count')).toContainText('1 / 300');
  await page.getByTestId('demo-play').click();
  await launchFromCatalog(page, tutorialCard(locale, 'overlay').title);
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
    await launchFromCatalog(page, tutorialCard('en', id).title);
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
  const demoIds = new Set<string>(demoTutorialIds);
  const visibleEntries = tourModules.filter((entry) => demoIds.has(entry.id));
  const visibleCategories = new Set(visibleEntries.map((entry) => tourCategory(entry)));
  await expect(page.locator('.tutorial-catalog__category-card')).toHaveCount(visibleCategories.size);
  await expect(page.locator('.tutorial-catalog__category-card').filter({ hasText: 'Account' })).toHaveCount(0);
  const dialog = page.locator('dialog[open].tutorial-catalog');
  const overlayTitle = tutorialCard('en', 'overlayObs').title;
  await dialog.locator('input[type="search"]').fill(overlayTitle);
  await dialog.locator('.tutorial-catalog__card').filter({ has: page.locator('strong').getByText(overlayTitle, { exact: true }) }).click();
  await expect(page.locator('[data-testid="tutorial-demo-frame"]')).toBeVisible();
  await dialog.locator('.tutorial-catalog__start').click();
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
  const connectionTitle = tutorialCard('en', 'connection').title;
  const pointsTitle = tutorialCard('en', 'points').title;
  await expect(recommendation).toContainText(connectionTitle);
  await expect(page.locator('.driver-popover')).toHaveCount(0);
  await page.getByRole('link', { name: 'Points', exact: true }).click();
  await expect(recommendation).toContainText(pointsTitle);
  await expect(page.locator('.driver-popover')).toHaveCount(0);
  await page.getByTestId('demo-guide').click();
  await expect(page.locator('.tutorial-catalog__detail h3')).toHaveText(pointsTitle);
});

test('automation events tour reaches the editor step through the prepare action', async ({ page }) => {
  await page.goto('demo/?scenario=automation&locale=en&instance=prepare-editor');
  await launchFromCatalog(page, tutorialCard('en', 'automationEvents').title);
  await expect(page.locator('.driver-popover-title')).toHaveText(tourCatalogs.en.automationEvents.profiles.title);
  for (const title of [tourCatalogs.en.automationEvents.eventsTab.title, tourCatalogs.en.automationEvents.events.title]) {
    await page.locator('.driver-popover-next-btn').click();
    await expect(page.locator('.driver-popover-title')).toHaveText(title);
  }
  await page.locator('.driver-popover-next-btn').click();
  // The editor only exists after interaction: the tour selects an existing stored record itself.
  await expect(page.locator('.driver-popover-title')).toHaveText(tourCatalogs.en.automationEvents.editor.title, { timeout: 10000 });
  await expect(page.locator('[data-guide="automation-editor"]')).toBeVisible();
  // Backward: the events table only returns after the editor closes — the tour presses back itself.
  await page.locator('.driver-popover-prev-btn').click();
  await expect(page.locator('.driver-popover-title')).toHaveText(tourCatalogs.en.automationEvents.events.title, { timeout: 10000 });
  await expect(page.locator('[data-guide="automation-events"]')).toBeVisible();
  await page.locator('.driver-popover-close-btn').click();
});

test('automation actions tour opens an existing action editor without the type picker', async ({ page }) => {
  await page.goto('demo/?scenario=automation&locale=en&instance=walk-actions');
  await launchFromCatalog(page, tutorialCard('en', 'automationActions').title);
  await walkTour(page, 'automationActions');
  await expect(page.locator('[data-guide="automation-editor"]')).toBeVisible();
  await expect(page.locator('.type-picker')).toHaveCount(0);
  await expect(page.locator('dialog[open]')).toHaveCount(0);
  await page.locator('.driver-popover-close-btn').click();
});



for (const id of ['automationFilters', 'automationCooldowns'] as const) {
  test(`${id} reaches real event editor sections`, async ({ page }) => {
    await page.goto(`demo/?scenario=automation&locale=en&instance=walk-${id}`);
    await launchFromCatalog(page, tutorialCard('en', id).title);
    await walkTour(page, id);
    await expect(page.locator('[data-guide="automation-editor"]')).toBeVisible();
    await page.locator('.driver-popover-close-btn').click();
  });
}

for (const id of ['automationActionMedia', 'automationActionTts', 'automationActionHttp', 'automationActionVars'] as const) {
  test(`${id} reaches the existing action editor steps`, async ({ page }) => {
    await page.goto(`demo/?scenario=automation&locale=en&instance=walk-${id}`);
    await launchFromCatalog(page, tutorialCard('en', id).title);
    await walkTour(page, id);
    await expect(page.locator('[data-guide="action-editor-permissions"]')).toBeVisible();
    await expect(page.locator('.type-picker')).toHaveCount(0);
    await page.locator('.driver-popover-close-btn').click();
  });
}


test('orientation and remaining automation tours can walk every available prepared step', async ({ page }) => {
  test.setTimeout(150000);
  const tours = [
    'automationEventChat', 'automationTemplates',
    'connection', 'connectionRecovery', 'plugin', 'overlay', 'overlayObs',
    'points', 'analytics', 'diagnostics', 'feed', 'settings',
  ] as const;
  for (const id of tours) {
    await page.goto(`demo/?scenario=${demoTourScenarios[id]}&locale=en&instance=walk-full-${id}`);
    await launchFromCatalog(page, tutorialCard('en', id).title);
    await walkTour(page, id);
    await page.locator('.driver-popover-close-btn').click();
  }
});

test('variables and changes tours follow live automations query state', async ({ page }) => {
  await page.goto('demo/?scenario=cloud&locale=en&instance=walk-variables');
  await launchFromCatalog(page, tutorialCard('en', 'automationVariables').title);
  await walkTour(page, 'automationVariables');
  await expect(page).toHaveURL(/#\/automations\?view=variables$/);
  await page.locator('.driver-popover-close-btn').click();

  await page.goto('demo/?scenario=cloud&locale=en&instance=walk-changes');
  await launchFromCatalog(page, tutorialCard('en', 'changes').title);
  await walkTour(page, 'changes');
  await expect(page).toHaveURL(/#\/automations\?view=variables$/);
  await page.locator('.driver-popover-close-btn').click();
});

test('feed inspector tour prepares real event and viewer selections', async ({ page }) => {
  await page.goto('demo/?scenario=feed&locale=en&instance=walk-feed-inspector');
  await page.getByTestId('demo-step').click();
  await launchFromCatalog(page, tutorialCard('en', 'feedInspector').title);
  await walkTour(page, 'feedInspector');
  await expect(page.locator('[data-guide="feed-inspector-details"]')).toHaveCount(0);
  await expect(page.locator('[data-guide="feed-inspector-profile"]')).toBeVisible();
  await page.locator('.driver-popover-close-btn').click();
});

test('plugin setup tour opens the installed plugin details tabs', async ({ page }) => {
  await page.goto('demo/?scenario=plugins&locale=en&instance=walk-plugin-setup');
  await launchFromCatalog(page, tutorialCard('en', 'pluginSetup').title);
  await walkTour(page, 'pluginSetup');
  await expect(page.locator('[data-guide="plugin-details"]')).toBeVisible();
  await expect(page.locator('[data-guide="plugin-connection"]')).toBeVisible();
  await page.locator('.driver-popover-close-btn').click();
});
