import { expect, test } from '@playwright/test';

import {
  assertNoUnhandledCalls,
  installFakeHost,
} from './fixtures/tiktools-host.ts';
import { optionFailureState, ttsPageState } from './fixtures/states.ts';

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

test('TTS inputs are labeled and keyboard reachable', async ({ page }) => {
  const consoleCapture = await installFakeHost(page, ttsPageState());
  await page.goto('/');
  await page.getByRole('button', { name: 'Text to Speech' }).click();

  await expect(page.getByLabel('Default voice')).toBeVisible();
  await expect(page.getByLabel('Volume')).toBeVisible();
  await expect(page.getByLabel('Voice', { exact: true })).toBeVisible();
  await expect(page.getByLabel('Text', { exact: true })).toBeVisible();

  // Keyboard: tab reaches an interactive control and focus stays visible.
  await page.keyboard.press('Tab');
  const focused = page.locator(':focus');
  await expect(focused).toBeVisible();

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
