import { expect, test } from '@playwright/test';

import {
  assertNoUnhandledCalls,
  installFakeHost,
  readHostState,
} from './fixtures/tiktools-host.ts';
import { installPluginUiOverride, routePluginFixture } from './fixtures/plugin-frame.ts';
import { optionFailureState, ttsPageState, webviewPageState } from './fixtures/states.ts';

test('navigation exposes named buttons and the active tab marker', async ({ page }) => {
  const consoleCapture = await installFakeHost(page, ttsPageState());
  await page.goto('/');

  const nav = page.getByRole('navigation', { name: 'Main Navigation' });
  await expect(nav).toBeVisible();
  const buttons = nav.getByRole('button');
  expect(await buttons.count()).toBeGreaterThanOrEqual(7);
  for (const name of [
    'Live Stream Feed',
    'Points System',
    'Connections',
    'Plugins',
    'Text to Speech',
  ]) {
    await expect(nav.getByRole('button', { name })).toBeVisible();
  }
  await expect(nav.getByRole('button', { name: 'Live Stream Feed' })).toHaveAttribute(
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
  await page.getByRole('button', { name: 'Text to Speech' }).click();

  const frame = page.frameLocator('.plg-frame');
  await expect(frame.locator('#server')).toHaveText('http://127.0.0.1:17842', { timeout: 10_000 });

  // Keyboard: the pop-out action is reachable and operable by keyboard.
  const popOut = page.getByRole('button', { name: 'Open in separate window' });
  await popOut.focus();
  await expect(popOut).toBeFocused();
  await page.keyboard.press('Enter');
  await expect
    .poll(async () => (await readHostState(page)).legacyMessages.length)
    .toBe(1);

  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});

test('option errors expose status semantics', async ({ page }) => {
  const consoleCapture = await installFakeHost(page, optionFailureState());
  await page.goto('/');
  await page.getByRole('button', { name: 'Voices' }).click();

  const status = page.getByRole('status');
  await expect(status).toContainText('option source exploded');

  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});
