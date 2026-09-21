import { expect, test } from '@playwright/test';

import {
  assertNoUnhandledCalls,
  installFakeHost,
  readHostState,
} from './fixtures/tiktools-host.ts';
import { SONICBOOM_ID, VOICES_SOURCE } from './fixtures/plugins.ts';
import {
  formPageState,
  listPageState,
  optionFailureState,
  textPageState,
} from './fixtures/states.ts';

async function openPluginTab(page: Parameters<typeof installFakeHost>[0], name: string) {
  const tab = page.getByRole('button', { name });
  await expect(tab).toBeVisible();
  await tab.click();
  await expect(tab).toHaveAttribute('aria-current', 'page');
}

test('declarative form page renders and saves settings', async ({ page }) => {
  const consoleCapture = await installFakeHost(page, formPageState());
  await page.goto('/');
  await openPluginTab(page, 'Server settings');

  await expect(page.getByRole('heading', { name: 'Server settings' }).first()).toBeVisible();
  const urlField = page.locator('input').first();
  await expect(urlField).toBeVisible();
  await urlField.fill('http://127.0.0.1:19999');
  await page.getByRole('button', { name: 'Save settings' }).click();

  await expect
    .poll(async () => {
      const state = await readHostState(page);
      return state.calls.filter((call) => call.method === 'plugins.settings.set').length;
    })
    .toBe(1);
  const state = await readHostState(page);
  const saved = state.calls.find((call) => call.method === 'plugins.settings.set');
  expect(saved?.params['pluginId']).toBe(SONICBOOM_ID);
  expect(saved?.params['values']).toMatchObject({ serverUrl: 'http://127.0.0.1:19999' });

  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});

test('dynamic list page renders rows and refreshes', async ({ page }) => {
  const consoleCapture = await installFakeHost(page, listPageState());
  await page.goto('/');
  await openPluginTab(page, 'Voices');

  await expect(page.getByRole('heading', { name: 'Voices' }).first()).toBeVisible();
  await expect(page.getByText('M1').first()).toBeVisible();
  await expect(page.getByText('F2').first()).toBeVisible();

  await page.getByRole('button', { name: 'Refresh' }).click();
  await expect
    .poll(async () => {
      const state = await readHostState(page);
      return state.calls.filter(
        (call) =>
          call.method === 'plugins.options' && call.params['source'] === VOICES_SOURCE,
      ).length;
    })
    .toBeGreaterThanOrEqual(2);

  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});

test('text page renders manifest copy as plain text', async ({ page }) => {
  const consoleCapture = await installFakeHost(page, textPageState());
  await page.goto('/');
  await openPluginTab(page, 'About');

  await expect(page.getByText('SonicBoom renders chat to speech on this machine.')).toBeVisible();

  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});

test('option-source failure shows an error but keeps the page usable', async ({ page }) => {
  const consoleCapture = await installFakeHost(page, optionFailureState());
  await page.goto('/');
  await openPluginTab(page, 'Voices');

  await expect(page.getByRole('heading', { name: 'Voices' }).first()).toBeVisible();
  await expect(page.getByRole('status')).toContainText('option source exploded');
  // Refresh stays available; the rest of the page did not crash.
  await expect(page.getByRole('button', { name: 'Refresh' })).toBeEnabled();

  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});
