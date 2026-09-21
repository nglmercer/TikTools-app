import { expect, test } from '@playwright/test';

import {
  assertNoUnhandledCalls,
  installFakeHost,
} from './fixtures/tiktools-host.ts';
import { installPluginUiOverride, routePluginFixture } from './fixtures/plugin-frame.ts';
import {
  connectedCreatorState,
  connectionPageState,
  emptyState,
  formPageState,
  listPageState,
  sonicboomConnectedState,
  sonicboomDisconnectedState,
  ttsPageState,
  webviewPageState,
} from './fixtures/states.ts';

/**
 * Deterministic screenshot coverage. Every visual test first asserts
 * semantic state, then captures. The clock is frozen and all fixture data
 * is fixed, so baselines stay stable across runs.
 */

async function freezeClock(page: Parameters<typeof installFakeHost>[0]) {
  await page.clock.install({ time: new Date('2026-01-15T12:00:00Z') });
}

test('main feed empty state', async ({ page }) => {
  const consoleCapture = await installFakeHost(page, emptyState());
  await page.goto('/');
  await freezeClock(page);
  await expect(page.getByText('No live events yet.')).toBeVisible();
  await expect(page).toHaveScreenshot('feed-empty.png', { fullPage: true });
  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});

test('connected creator feed', async ({ page }) => {
  const consoleCapture = await installFakeHost(page, connectedCreatorState());
  await page.goto('/');
  await freezeClock(page);
  await expect(page.getByRole('navigation', { name: 'Main Navigation' })).toBeVisible();
  await expect(page).toHaveScreenshot('feed-connected.png', { fullPage: true });
  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});

test('connections page', async ({ page }) => {
  const consoleCapture = await installFakeHost(page, sonicboomConnectedState());
  await page.goto('/');
  await freezeClock(page);
  await page.getByRole('button', { name: 'Connections' }).click();
  await expect(page.getByText('Server connections')).toBeVisible();
  await expect(page).toHaveScreenshot('connections.png', { fullPage: true });
  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});

test('plugins page', async ({ page }) => {
  const consoleCapture = await installFakeHost(page, sonicboomConnectedState());
  await page.goto('/');
  await freezeClock(page);
  await page.getByRole('button', { name: 'Plugins' }).click();
  await expect(page.getByRole('heading', { name: 'Plugins' }).first()).toBeVisible();
  await expect(page).toHaveScreenshot('plugins.png', { fullPage: true });
  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});

test('plugin declarative form', async ({ page }) => {
  const consoleCapture = await installFakeHost(page, formPageState());
  await page.goto('/');
  await freezeClock(page);
  await page.getByRole('button', { name: 'Server settings' }).click();
  await expect(page.getByRole('heading', { name: 'Server settings' }).first()).toBeVisible();
  await expect(page).toHaveScreenshot('plugin-form.png', { fullPage: true });
  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});

test('plugin dynamic list', async ({ page }) => {
  const consoleCapture = await installFakeHost(page, listPageState());
  await page.goto('/');
  await freezeClock(page);
  await page.getByRole('button', { name: 'Voices' }).click();
  await expect(page.getByText('M1').first()).toBeVisible();
  await expect(page).toHaveScreenshot('plugin-list.png', { fullPage: true });
  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});

test('legacy plugin panel note', async ({ page }) => {
  const consoleCapture = await installFakeHost(page, ttsPageState());
  await page.goto('/');
  await freezeClock(page);
  await page.getByRole('button', { name: 'Text to Speech' }).click();
  await expect(page.getByText('This panel moved to the plugin view.')).toBeVisible();
  await expect(page).toHaveScreenshot('plugin-legacy-note.png', { fullPage: true });
  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});

test('plugin inline frame', async ({ page }) => {
  const consoleCapture = await installFakeHost(page, webviewPageState());
  await installPluginUiOverride(page);
  await routePluginFixture(page);
  await page.goto('/');
  await freezeClock(page);
  await page.getByRole('button', { name: 'Text to Speech' }).click();
  const frame = page.frameLocator('.plg-frame');
  await expect(frame.locator('#server')).toHaveText('http://127.0.0.1:17842', { timeout: 10_000 });
  await expect(page).toHaveScreenshot('plugin-inline.png', { fullPage: true });
  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});

test('plugin connection error', async ({ page }) => {
  const consoleCapture = await installFakeHost(page, sonicboomDisconnectedState());
  await page.goto('/');
  await freezeClock(page);
  await page.getByRole('button', { name: 'Connections' }).click();
  await page.getByRole('button', { name: /SonicBoom Server/ }).click();
  await page.getByRole('button', { name: 'Test connection' }).click();
  await expect(page.getByRole('button', { name: /connection refused/ })).toBeVisible();
  await expect(page.getByText('Connection failed.').first()).toBeVisible();
  await expect(page).toHaveScreenshot('plugin-connection-error.png', { fullPage: true });
  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});

test('plugin connection success', async ({ page }) => {
  const consoleCapture = await installFakeHost(page, sonicboomConnectedState());
  await page.goto('/');
  await freezeClock(page);
  await page.getByRole('button', { name: 'Connections' }).click();
  await page.getByRole('button', { name: /SonicBoom Server/ }).click();
  await expect(page.getByRole('button', { name: /Connected in 12 ms/ })).toBeVisible();
  await expect(
    page.getByText('Connected to http://127.0.0.1:17842').first(),
  ).toBeVisible();
  await page.getByRole('button', { name: 'Test again' }).click();
  await expect(
    page.getByText('Connected to http://127.0.0.1:17842').first(),
  ).toBeVisible();
  await expect(page).toHaveScreenshot('plugin-connection-success.png', { fullPage: true });
  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});

test('light theme inline frame', async ({ page }) => {
  const consoleCapture = await installFakeHost(page, webviewPageState(), { theme: 'light' });
  await installPluginUiOverride(page);
  await routePluginFixture(page);
  await page.goto('/');
  await freezeClock(page);
  await page.getByRole('button', { name: 'Text to Speech' }).click();
  const frame = page.frameLocator('.plg-frame');
  await expect(frame.locator('#server')).toHaveText('http://127.0.0.1:17842', { timeout: 10_000 });
  await expect(page).toHaveScreenshot('plugin-inline-light.png', { fullPage: true });
  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});

test('Spanish locale inline frame', async ({ page }) => {
  const consoleCapture = await installFakeHost(page, webviewPageState(), { locale: 'es' });
  await installPluginUiOverride(page);
  await routePluginFixture(page);
  await page.goto('/');
  await freezeClock(page);
  await page.getByRole('button', { name: 'Text to Speech' }).click();
  await expect(page.getByRole('button', { name: 'Abrir en ventana separada' })).toBeVisible();
  const frame = page.frameLocator('.plg-frame');
  await expect(frame.locator('#server')).toHaveText('http://127.0.0.1:17842', { timeout: 10_000 });
  await expect(page).toHaveScreenshot('plugin-inline-es.png', { fullPage: true });
  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});

test('connection-only plugin page is hidden from nav', async ({ page }) => {
  const consoleCapture = await installFakeHost(page, connectionPageState());
  await page.goto('/');
  await freezeClock(page);
  await expect(page.getByRole('button', { name: 'Connection', exact: true })).toHaveCount(0);
  await page.getByRole('button', { name: 'Connections' }).click();
  await expect(page.getByText('Server connections')).toBeVisible();
  await expect(page).toHaveScreenshot('connections-only-hidden.png', { fullPage: true });
  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});
