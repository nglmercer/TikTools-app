import { expect, test, type Page } from '@playwright/test';
import {
  cjkLocales,
  demoTestId,
  demoUrl,
  guideScopeSelector,
  guideTargetSelector,
  instanceId,
  timeout,
  trackPageErrors,
} from './support/index.ts';
import { DEMO_STORAGE_PREFIX, demoLimits } from '../../src/web/demo/contract.ts';
import { demoTourScenarios, demoTutorialIds } from '../../src/web/demo/tours.ts';
import { tourModules } from '../../src/web/features/guided-help/tours/index.ts';
import type { TourId } from '../../src/web/features/guided-help/model/types.ts';
import { tourCatalogs } from '../../src/web/features/guided-help/catalogs/index.ts';
import { tourCategory } from '../../src/web/features/guided-help/model/tutorial-taxonomy.ts';
import { tourRegistry } from '../../src/web/features/guided-help/model/registry.ts';

const catalogCard = (locale: keyof typeof tourCatalogs, id: TourId) => tourCatalogs[locale].card[id];

/** Driver.js popover of a running tour. */
const driver = {
  popover: '.driver-popover',
  title: '.driver-popover-title',
  next: '.driver-popover-next-btn',
  prev: '.driver-popover-prev-btn',
  close: '.driver-popover-close-btn',
} as const;

/** The demo's tutorial catalog dialog. */
const catalog = {
  dialog: 'dialog[open].tutorial-catalog',
  search: 'input[type="search"]',
  card: '.tutorial-catalog__card',
  start: '.tutorial-catalog__start',
  categoryCard: '.tutorial-catalog__category-card',
  detailTitle: '.tutorial-catalog__detail h3',
} as const;

async function pickCatalogTour(page: Page, title: string): Promise<void> {
  const dialog = page.locator(catalog.dialog);
  await dialog.locator(catalog.search).fill(title);
  await dialog.locator(catalog.card).filter({ has: page.locator('strong').getByText(title, { exact: true }) }).click();
}

async function selectCatalogTour(page: Page, title: string): Promise<void> {
  await pickCatalogTour(page, title);
  await page.locator(catalog.dialog).locator(catalog.start).click();
}

async function launchFromCatalog(page: Page, title: string): Promise<void> {
  await page.getByTestId(demoTestId.guide).click();
  await selectCatalogTour(page, title);
}

async function expectStep(page: Page, id: TourId, step: (typeof tourRegistry)[TourId]['steps'][number]): Promise<void> {
  const copy = (tourCatalogs.en[id] as unknown as Record<string, { title: string }>)[step.id]!;
  await expect(page.locator(driver.title)).toHaveText(copy.title, { timeout: timeout.step });
  await expect(page.locator(guideTargetSelector(step.target))).toBeVisible();
}

// The demo deliberately does not emulate the signed package-installation backend.
// The coordinator capability-skips optional steps behind these methods, so the
// walkthrough assertion must apply the same availability contract.
const methodsNotSimulatedByDemo = new Set(['plugins.packages.install']);
const expectedActionEditorRecord: Partial<Record<TourId, string>> = {
  automationActionHttp: 'Notify webhook',
  automationActionMedia: 'Play applause',
  automationActionTts: 'Speak chat',
};

async function walkTour(page: Page, id: TourId): Promise<void> {
  const steps = tourRegistry[id].steps.filter((step) => step.requiredCapabilities.every((method) => !methodsNotSimulatedByDemo.has(method)));
  for (const [index, step] of steps.entries()) {
    await expectStep(page, id, step);
    const expectedRecord = step.id === 'settings' ? expectedActionEditorRecord[id] : undefined;
    if (expectedRecord) await expect(page.locator(guideScopeSelector('automations'))).toContainText(expectedRecord);
    if (index < steps.length - 1) await page.locator(driver.next).click();
  }
}

for (const locale of cjkLocales) test(`${locale}: playback and a selected walkthrough use the localized real interface`, async ({ page }) => {
  await page.goto(demoUrl({ scenario: 'connection', locale, instance: instanceId('cjk-tour', locale) }));
  await expect(page.locator('html')).toHaveAttribute('lang', locale);
  await page.getByTestId(demoTestId.play).click();
  await expect(page.getByTestId(demoTestId.playbackStatus)).toHaveAttribute('data-status', 'playing');
  await expect(page.getByTestId(demoTestId.eventCount)).toContainText(`1 / ${demoLimits.maxTicks}`);
  await page.getByTestId(demoTestId.play).click();
  await launchFromCatalog(page, catalogCard(locale, 'overlay').title);
  await expect(page.locator(driver.title)).toHaveText(tourCatalogs[locale].overlay.gateway.title);
  await expect(page.locator(driver.next)).toHaveText(tourCatalogs[locale].controls.next);
  await page.locator(driver.close).click();
  await expect(page.getByTestId(demoTestId.guideStatus)).toHaveAttribute('data-status', 'dismissed');
});

for (const id of demoTutorialIds) {
  test(`tutorial selection prepares each matching scenario and reports dismissal without completion: ${id}`, async ({ page }) => {
    await page.goto(demoUrl({ scenario: 'connection', instance: instanceId('select', id) }));
    await expect(page.getByTestId(demoTestId.guide)).toBeEnabled();
    await launchFromCatalog(page, catalogCard('en', id).title);
    await expect(page.locator(driver.popover)).toBeVisible();
    expect(new URL(page.url()).searchParams.get('scenario')).toBe(demoTourScenarios[id]);
    await expect(page.getByTestId(demoTestId.guideStatus)).toHaveAttribute('data-status', 'active');
    if (id === 'changes') {
      await expect(page).toHaveURL(/#\/automations\?view=variables$/);
      await page.locator(driver.next).click();
      await expect(page.locator(driver.title)).toHaveText('Review Changes');
      await expect(page.locator(guideTargetSelector('changes'))).toBeVisible();
    }
    await page.locator(driver.close).click();
    await expect(page.getByTestId(demoTestId.guideStatus)).toHaveAttribute('data-status', 'dismissed');
    await expect(page.getByTestId(demoTestId.guide)).toBeEnabled();
  });
}

test('the in-app demo catalog keeps unsupported tours out and delegates scenario preparation', async ({ page }) => {
  await page.goto(demoUrl({ scenario: 'connection', instance: 'menu-catalog' }));
  await page.locator('.app-menu-bar').getByRole('menuitem', { name: 'Help', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Guides and tutorials', exact: true }).click();
  const demoIds = new Set<string>(demoTutorialIds);
  const visibleEntries = tourModules.filter((entry) => demoIds.has(entry.id));
  const visibleCategories = new Set(visibleEntries.map((entry) => tourCategory(entry)));
  await expect(page.locator(catalog.categoryCard)).toHaveCount(visibleCategories.size);
  await expect(page.locator(catalog.categoryCard).filter({ hasText: 'Account' })).toHaveCount(0);
  const dialog = page.locator(catalog.dialog);
  const overlayTitle = catalogCard('en', 'overlayObs').title;
  await pickCatalogTour(page, overlayTitle);
  await expect(page.locator('[data-testid="tutorial-demo-frame"]')).toBeVisible();
  await dialog.locator(catalog.start).click();
  await expect(page.locator(driver.popover)).toBeVisible();
  expect(new URL(page.url()).searchParams.get('scenario')).toBe('overlays');
  await page.locator(driver.close).click();
});

for (const [tour, scenario] of Object.entries(demoTourScenarios)) {
  test(`shared tours load Driver inside the demo and dismissal stays dismissed: ${tour}`, async ({ page }) => {
    const errors = trackPageErrors(page);
    await page.goto(demoUrl({ scenario, tour, instance: instanceId('tour', tour) }));
    await expect(page.locator(driver.popover)).toBeVisible({ timeout: timeout.step });
    await page.locator(driver.close).click();
    await expect(page.locator(driver.popover)).toHaveCount(0);
    // Each case has its own page, so sessionStorage only holds this tour's progress.
    const progress = await page.evaluate(
      (prefix) => Object.keys(sessionStorage).filter((key) => key.startsWith(prefix)).map((key) => sessionStorage.getItem(key)),
      DEMO_STORAGE_PREFIX,
    );
    expect(progress.some((value) => value?.includes('"status":"completed"'))).toBe(false);
    expect(errors).toEqual([]);
  });
}

test('navigating away cancels the guide and reset clears only this instance progress', async ({ page }) => {
  await page.goto(demoUrl({ scenario: 'connection', tour: 'connection', instance: 'tour-reset' }));
  await expect(page.locator(driver.popover)).toBeVisible();
  await page.evaluate(() => { location.hash = '/feed'; });
  await expect(page.locator(driver.popover)).toHaveCount(0);
  await page.getByTestId(demoTestId.reset).click();
  await expect(page.locator(driver.popover)).toBeVisible();
});

test('the recommended tour follows in-app navigation without starting automatically', async ({ page }) => {
  await page.goto(demoUrl({ scenario: 'connection', instance: 'recommendation' }));
  const recommendation = page.locator('.demo-toolbar__recommendation');
  const connectionTitle = catalogCard('en', 'connection').title;
  const pointsTitle = catalogCard('en', 'points').title;
  await expect(recommendation).toContainText(connectionTitle);
  await expect(page.locator(driver.popover)).toHaveCount(0);
  await page.getByRole('link', { name: 'Points', exact: true }).click();
  await expect(recommendation).toContainText(pointsTitle);
  await expect(page.locator(driver.popover)).toHaveCount(0);
  await page.getByTestId(demoTestId.guide).click();
  await expect(page.locator(catalog.detailTitle)).toHaveText(pointsTitle);
});

test('automation events tour reaches the editor step through the prepare action', async ({ page }) => {
  await page.goto(demoUrl({ scenario: 'automation', instance: 'prepare-editor' }));
  await launchFromCatalog(page, catalogCard('en', 'automationEvents').title);
  await expect(page.locator(driver.title)).toHaveText(tourCatalogs.en.automationEvents.profiles.title);
  for (const title of [tourCatalogs.en.automationEvents.eventsTab.title, tourCatalogs.en.automationEvents.events.title]) {
    await page.locator(driver.next).click();
    await expect(page.locator(driver.title)).toHaveText(title);
  }
  await page.locator(driver.next).click();
  // The editor only exists after interaction: the tour selects an existing stored record itself.
  await expect(page.locator(driver.title)).toHaveText(tourCatalogs.en.automationEvents.editor.title, { timeout: timeout.step });
  await expect(page.locator(guideTargetSelector('automation-editor'))).toBeVisible();
  // Backward: the events table only returns after the editor closes — the tour presses back itself.
  await page.locator(driver.prev).click();
  await expect(page.locator(driver.title)).toHaveText(tourCatalogs.en.automationEvents.events.title, { timeout: timeout.step });
  await expect(page.locator(guideTargetSelector('automation-events'))).toBeVisible();
  await page.locator(driver.close).click();
});

test('automation actions tour opens an existing action editor without the type picker', async ({ page }) => {
  await page.goto(demoUrl({ scenario: 'automation', instance: 'walk-actions' }));
  await launchFromCatalog(page, catalogCard('en', 'automationActions').title);
  await walkTour(page, 'automationActions');
  await expect(page.locator(guideTargetSelector('automation-editor'))).toBeVisible();
  await expect(page.locator('.type-picker')).toHaveCount(0);
  await expect(page.locator('dialog[open]')).toHaveCount(0);
  await page.locator(driver.close).click();
});

for (const id of ['automationFilters', 'automationCooldowns'] as const) {
  test(`${id} reaches real event editor sections`, async ({ page }) => {
    await page.goto(demoUrl({ scenario: 'automation', instance: instanceId('walk', id) }));
    await launchFromCatalog(page, catalogCard('en', id).title);
    await walkTour(page, id);
    await expect(page.locator(guideTargetSelector('automation-editor'))).toBeVisible();
    await page.locator(driver.close).click();
  });
}

for (const id of ['automationActionMedia', 'automationActionTts', 'automationActionHttp', 'automationActionVars'] as const) {
  test(`${id} reaches the existing action editor steps`, async ({ page }) => {
    await page.goto(demoUrl({ scenario: 'automation', instance: instanceId('walk', id) }));
    await launchFromCatalog(page, catalogCard('en', id).title);
    await walkTour(page, id);
    await expect(page.locator(guideTargetSelector('action-editor-permissions'))).toBeVisible();
    await expect(page.locator('.type-picker')).toHaveCount(0);
    await page.locator(driver.close).click();
  });
}

for (const id of [
  'automationEventChat', 'automationTemplates',
  'connection', 'connectionRecovery', 'plugin', 'overlay', 'overlayObs',
  'points', 'analytics', 'diagnostics', 'feed', 'settings',
] as const) {
  test(`orientation and remaining automation tours can walk every available prepared step: ${id}`, async ({ page }) => {
    await page.goto(demoUrl({ scenario: demoTourScenarios[id], instance: instanceId('walk-full', id) }));
    await launchFromCatalog(page, catalogCard('en', id).title);
    await walkTour(page, id);
    await page.locator(driver.close).click();
  });
}

test('variables and changes tours follow live automations query state', async ({ page }) => {
  await page.goto(demoUrl({ scenario: 'cloud', instance: 'walk-variables' }));
  await launchFromCatalog(page, catalogCard('en', 'automationVariables').title);
  await walkTour(page, 'automationVariables');
  await expect(page).toHaveURL(/#\/automations\?view=variables$/);
  await page.locator(driver.close).click();

  await page.goto(demoUrl({ scenario: 'cloud', instance: 'walk-changes' }));
  await launchFromCatalog(page, catalogCard('en', 'changes').title);
  await walkTour(page, 'changes');
  await expect(page).toHaveURL(/#\/automations\?view=variables$/);
  await page.locator(driver.close).click();
});

test('feed inspector tour prepares real event and viewer selections', async ({ page }) => {
  await page.goto(demoUrl({ scenario: 'feed', instance: 'walk-feed-inspector' }));
  await page.getByTestId(demoTestId.step).click();
  await launchFromCatalog(page, catalogCard('en', 'feedInspector').title);
  await walkTour(page, 'feedInspector');
  await expect(page.locator(guideTargetSelector('feed-inspector-details'))).toHaveCount(0);
  await expect(page.locator(guideTargetSelector('feed-inspector-profile'))).toBeVisible();
  await page.locator(driver.close).click();
});

test('plugin setup tour opens the installed plugin details tabs', async ({ page }) => {
  await page.goto(demoUrl({ scenario: 'plugins', instance: 'walk-plugin-setup' }));
  await launchFromCatalog(page, catalogCard('en', 'pluginSetup').title);
  await walkTour(page, 'pluginSetup');
  await expect(page.locator(guideTargetSelector('plugin-details'))).toBeVisible();
  await expect(page.locator(guideTargetSelector('plugin-connection'))).toBeVisible();
  await page.locator(driver.close).click();
});
