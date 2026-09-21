import { expect, test } from '@playwright/test';

import {
  assertNoUnhandledCalls,
  installFakeHost,
} from './fixtures/tiktools-host.ts';
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

test('plugin view launcher is labeled and keyboard reachable', async ({ page }) => {
  const consoleCapture = await installFakeHost(page, webviewPageState());
  await page.goto('/');
  await page.getByRole('button', { name: 'Text to Speech' }).click();

  const open = page.getByRole('button', { name: 'Open plugin view' });
  await expect(open).toBeVisible();

  // Keyboard: the launcher button is reachable and operable by keyboard.
  await open.focus();
  await expect(open).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('button', { name: 'Close plugin view' })).toBeVisible();

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
