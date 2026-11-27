import { testMessage } from '../../src/web/shared/i18n/test-message.ts';

const t = (key: string, params?: Record<string, string | number>): string => testMessage('en', key, params);
import { expect, test } from '@playwright/test';

import {
  assertNoUnhandledCalls,
  installFakeHost,
  legacyMessagesOf,
  waitForFrontend,
} from './fixtures/tiktools-host.ts';
import { installPluginUiOverride, routePluginFixture } from './fixtures/plugin-frame.ts';
import { optionFailureState, ttsPageState, webviewPageState } from './fixtures/states.ts';

test('navigation exposes named buttons and the active tab marker', async ({ page }) => {
  const consoleCapture = await installFakeHost(page, ttsPageState());
  await page.goto('/');
  await waitForFrontend(page);

  const nav = page.getByRole('navigation', { name: 'Main Navigation' });
  await expect(nav).toBeVisible();
  const buttons = nav.getByRole('link');
  expect(await buttons.count()).toBeGreaterThanOrEqual(7);
  for (const name of [
    t('navigation.feed'),
    t('navigation.points'),
    t('navigation.connections'),
    t('navigation.plugins'),
    'Text to Speech',
  ]) {
    await expect(nav.getByRole('link', { name })).toBeVisible();
  }
  await expect(nav.getByRole('link', { name: t('navigation.feed') })).toHaveAttribute(
    'aria-current',
    'page',
  );

  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});

test('inline plugin frame exposes a keyboard-operable pop-out', async ({ page }) => {
  const consoleCapture = await installFakeHost(page, webviewPageState());
  await installPluginUiOverride(page);
  await routePluginFixture(page);
  await page.goto('/');
  await waitForFrontend(page);
  await page.getByRole('link', { name: 'Text to Speech' }).click();

  const frame = page.frameLocator('.plugin-frame');
  await expect(frame.locator('#server')).toHaveText('http://127.0.0.1:17842', { timeout: 10_000 });

  // Keyboard: the pop-out action is reachable and operable by keyboard.
  const popOut = page.getByRole('button', { name: t('plugins.page.openWindow') });
  await popOut.focus();
  await expect(popOut).toBeFocused();
  await page.keyboard.press('Enter');
  await expect
    .poll(async () => (await legacyMessagesOf(page, 'plugin-ui-open')).length)
    .toBe(1);

  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});

test('option errors expose status semantics', async ({ page }) => {
  const consoleCapture = await installFakeHost(page, optionFailureState());
  await page.goto('/');
  await waitForFrontend(page);
  await page.getByRole('link', { name: 'Voices' }).click();

  const status = page.getByRole('status').filter({ hasText: 'option source exploded' });
  await expect(status).toContainText('option source exploded');

  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});
