import { expect, test } from '@playwright/test';

import { readHostState, installFakeHost, waitForFrontend } from './fixtures/tiktools-host.ts';
import { emptyState, sonicboomConnectedState } from './fixtures/states.ts';
import { testMessage } from '../../src/web/shared/i18n/test-message.ts';

const t = (key: string): string => testMessage('en', key);

/**
 * Every area owns its own settings; Settings keeps only the application-level ones. Old
 * `/settings?section=...` links land on the area that now owns the option.
 */

test('Settings keeps only application sections, with a language picker', async ({ page }) => {
  const consoleCapture = await installFakeHost(page, emptyState());
  await page.goto('/#/settings');
  await waitForFrontend(page);
  await expect(page.getByRole('tab')).toHaveText([t('settingsLayout.labels.general'), t('settingsLayout.labels.account')]);
  await expect(page.getByRole('heading', { level: 1 })).toHaveCount(0);
  await expect(page.getByText(t('packages.sources.allowSideload'))).toHaveCount(0);
  const language = page.getByRole('combobox', { name: t('settings.language.label') });
  await expect(language).toHaveValue(t('menu.languages.en'));
  await language.fill('fra');
  await expect(page.getByRole('option')).toHaveText([new RegExp(t('menu.languages.fr'))]);
  consoleCapture.assertClean();
});

test('the old globals link lands on Automations > Variables and saves through the same contract', async ({ page }) => {
  const state = emptyState();
  state.globals = { commandPort: '46665' };
  const consoleCapture = await installFakeHost(page, state);
  await page.goto('/#/settings?section=globals');
  await waitForFrontend(page);
  await expect(page).toHaveURL(/#\/automations/);
  await expect(page.getByRole('tab', { name: t('automations.library.variables') })).toHaveAttribute('aria-selected', 'true');
  const value = page.getByRole('textbox', { name: t('globals.value') });
  await expect(value).toHaveValue('46665');
  await value.fill('25575');
  await page.getByRole('button', { name: t('globals.save') }).click();
  await expect.poll(async () => (await readHostState(page)).globals?.['commandPort']).toBe('25575');
  consoleCapture.assertClean();
});

test('the old plugin updates link opens the plugin preferences and goes back', async ({ page }) => {
  const consoleCapture = await installFakeHost(page, sonicboomConnectedState());
  await page.goto('/#/settings?section=plugins');
  await waitForFrontend(page);
  await expect(page).toHaveURL(/#\/plugins/);
  await expect(page.getByText(t('packages.sources.allowSideload'))).toBeVisible();
  await page.getByRole('button', { name: t('common.back') }).click();
  await expect(page.getByRole('tab', { name: new RegExp(t('packages.versions.current')) })).toBeVisible();
  await page.getByRole('button', { name: t('syncChanges.more') }).click();
  await page.getByRole('menuitem', { name: t('plugins.catalog.preferences') }).click();
  await expect(page.getByText(t('packages.sources.allowSideload'))).toBeVisible();
  consoleCapture.assertClean();
});

test('Cloud areas appear in their own page only when the host announces them', async ({ page }) => {
  const state = emptyState();
  state.announcedMethods = ['bundles.overlays.create', 'bundles.packages.preflight', 'integrations.connections'];
  await installFakeHost(page, state);
  await page.goto('/#/overlays');
  await waitForFrontend(page);
  await page.getByRole('button', { name: t('overlays.cloud') }).click();
  await expect(page.getByRole('heading', { name: t('overlays.cloud'), level: 2 })).toBeVisible();
  await page.getByRole('button', { name: 'Close' }).click();
  await expect(page.getByRole('heading', { name: t('overlays.widgets') })).toBeVisible();
  await page.goto('/#/connections');
  await expect(page.getByRole('button', { name: t('connections.tabs.webhooks') })).toBeVisible();
});

test('without announcements the Cloud areas stay hidden', async ({ page }) => {
  await installFakeHost(page, emptyState());
  await page.goto('/#/overlays');
  await waitForFrontend(page);
  await expect(page.getByRole('button', { name: t('overlays.cloud'), exact: true })).toHaveCount(0);
  await page.goto('/#/connections');
  await expect(page.getByRole('button', { name: t('connections.tabs.webhooks') })).toHaveCount(0);
});
