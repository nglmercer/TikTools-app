import { readFileSync } from 'node:fs';
import { expect, test } from '@playwright/test';

import {
  assertNoUnhandledCalls,
  installFakeHost,
  waitForFrontend,
} from './fixtures/tiktools-host.ts';
import {
  emptyState,
  sonicboomConnectedState,
  sonicboomDisconnectedState,
} from './fixtures/states.ts';

async function openConnections(page: Parameters<typeof installFakeHost>[0]) {
  const tab = page.getByRole('link', { name: 'Connections' });
  await expect(tab).toBeVisible();
  await tab.click();
  await expect(tab).toHaveAttribute('aria-current', 'page');
}

test('connections page shows empty servers state without plugins', async ({ page }) => {
  const consoleCapture = await installFakeHost(page, emptyState());
  await page.goto('/');
  await waitForFrontend(page);
  await openConnections(page);

  await expect(page.getByText('No server connections yet')).toBeVisible();

  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});

test('server connection success shows latency and stays editable', async ({ page }) => {
  const consoleCapture = await installFakeHost(page, sonicboomConnectedState());
  await page.goto('/');
  await waitForFrontend(page);
  await openConnections(page);

  const server = page.getByRole('button', { name: /SonicBoom Server/ });
  await expect(server).toBeVisible();
  await server.click();

  // The card auto-probes on first expand; the summary collapses to the
  // compact view with an explicit re-test button. Latency surfaces on the
  // server head's accessible name; the compact card shows the target URL.
  await expect(page.getByRole('button', { name: /Connected in 12 ms/ })).toBeVisible();
  await expect(
    page.getByText('Connected to http://127.0.0.1:17842').first(),
  ).toBeVisible();
  await page.getByRole('button', { name: 'Test again' }).click();
  await expect(
    page.getByText('Connected to http://127.0.0.1:17842').first(),
  ).toBeVisible();

  // Settings remain editable after a successful probe.
  await page.getByRole('button', { name: 'Edit settings' }).click();
  const urlField = page.locator('input').first();
  await expect(urlField).toBeEnabled();

  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});

test('server connection failure shows the error without an app-wide crash', async ({
  page,
}) => {
  const consoleCapture = await installFakeHost(page, sonicboomDisconnectedState());
  await page.goto('/');
  await waitForFrontend(page);
  await openConnections(page);

  const server = page.getByRole('button', { name: /SonicBoom Server/ });
  await expect(server).toBeVisible();
  await server.click();

  // Explicit probe; the inline card keeps the form visible and reports the
  // host error on the server head (accessible name) plus a status line.
  // SonicBoom cards label the probe as a connect action.
  await page.getByRole('button', { name: 'Connect SonicBoom' }).click();
  await expect(page.getByRole('button', { name: /connection refused/ })).toBeVisible();
  await expect(page.getByText('connection refused', { exact: false }).first()).toBeVisible();

  // Settings remain editable after a failed probe.
  const urlField = page.locator('input').first();
  await expect(urlField).toBeEnabled();
  await expect(page.getByRole('link', { name: 'Connections' })).toHaveAttribute(
    'aria-current',
    'page',
  );

  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});

test('REPO imports a token through the shared card without showing the secret', async ({ page }) => {
  const { baseHostState } = await import('./fixtures/tiktools-host.ts');
  const { sonicboomStatus, sonicboomDescriptor } = await import('./fixtures/plugins.ts');
  const manifest = JSON.parse(readFileSync(new URL('../../examples/repo-game/plugin.json', import.meta.url), 'utf8'));
  const id = manifest.id;
  const state = baseHostState();
  state.automationSnapshot.plugins = [sonicboomStatus({
    descriptor: sonicboomDescriptor({ id, name: { default: manifest.name },
      supportsTokenProvisioning: false, supportsTokenImport: true, actionTypeIds: [] }),
  })];
  state.automationSnapshot.pluginPages = [{ id: 'connection', pluginId: id,
    title: { default: 'Connection' }, sections: [{ kind: 'connection' }],
    source: { kind: 'plugin', pluginId: id } }];
  state.pluginSettings[id] = { schema: manifest.settings.schema, values: { serverUrl: 'http://127.0.0.1:17453' } };
  state.pluginHealth[id] = { ok: false, latencyMs: 1, error: 'API token missing.' };
  const capture = await installFakeHost(page, state);
  await page.goto('/');
  await waitForFrontend(page);
  await openConnections(page);
  await page.getByRole('button', { name: /R.E.P.O. Game/ }).click();
  const importButton = page.getByRole('button', { name: 'Import token from configuration…' });
  await expect(importButton).toBeVisible();
  await expect(page.getByRole('button', { name: 'Get API token…' })).toHaveCount(0);
  await importButton.click();
  await expect(page.getByText('API token imported and saved.')).toBeVisible();
  await expect(page.getByText('API token imported and saved.')).toHaveClass(/connection-editor__details--ok/);
  await expect(page.getByText('Connected to http://127.0.0.1:17453').first()).toBeVisible();
  await page.getByRole('button', { name: 'Edit settings' }).click();
  await expect(page.locator('input[type="password"]')).toHaveValue('••••••••');
  await assertNoUnhandledCalls(page);
  capture.assertClean();
});

async function repoAutomaticState() {
  const { baseHostState } = await import('./fixtures/tiktools-host.ts');
  const { sonicboomStatus, sonicboomDescriptor } = await import('./fixtures/plugins.ts');
  const manifest = JSON.parse(readFileSync(new URL('../../examples/repo-game/plugin.json', import.meta.url), 'utf8'));
  const state = baseHostState();
  state.automationSnapshot.plugins = [sonicboomStatus({ descriptor: sonicboomDescriptor({
    id: manifest.id, name: { default: manifest.name }, supportsTokenProvisioning: false,
    supportsTokenImport: true, supportsTokenDiscovery: true, actionTypeIds: [],
  }) })];
  state.automationSnapshot.pluginPages = [{ id: 'connection', pluginId: manifest.id,
    title: { default: 'Connection' }, sections: [{ kind: 'connection' }], source: { kind: 'plugin', pluginId: manifest.id } }];
  state.pluginSettings[manifest.id] = { schema: manifest.settings.schema, values: { serverUrl: 'http://127.0.0.1:17453' } };
  state.pluginHealth[manifest.id] = { ok: false, latencyMs: 1, error: 'API token missing.' };
  return state;
}

test('automatic REPO setup chooses a profile and distinguishes API availability from gameplay readiness', async ({ page }) => {
  const state = await repoAutomaticState();
  state.pluginTokenCandidates = { 'repo.game': [
    { id: 'aabbccddeeff0011', label: 'Steam — Local installation' },
    { id: '1122334455667788', label: 'Mod profile: Stream' },
  ] };
  state.pluginSetupEndpoints = { 'repo.game': 'http://127.0.0.1:18053' };
  state.pluginSetupHealth = { 'repo.game': { ok: false, reachable: true, gameplayReady: false, latencyMs: 1, error: 'API reachable; gameplay unavailable.' } };
  const capture = await installFakeHost(page, state);
  await page.goto('/');
  await waitForFrontend(page);
  await openConnections(page);
  await page.getByRole('button', { name: /R.E.P.O. Game/ }).click();
  await page.getByRole('button', { name: 'Connect automatically', exact: true }).click();
  await expect(page.getByText('Steam — Local installation')).toBeVisible();
  await expect(page.getByText('Mod profile: Stream')).toBeVisible();
  await expect(page.getByText('API token imported and saved.')).toHaveCount(0);
  await page.locator('.connection-editor__candidate-list li').filter({ hasText: 'Mod profile: Stream' }).getByRole('button', { name: 'Use this configuration' }).click();
  await expect(page.getByText('API token imported and saved.')).toBeVisible();
  await expect(page.getByText('API token imported and saved.')).toHaveClass(/connection-editor__details--ok/);
  await expect(page.getByText('API connected · gameplay not ready', { exact: true }).first()).toBeVisible();
  await expect(page.locator('input[type="password"]')).toHaveValue('••••••••');
  await expect(page.locator('.connection-editor input').first()).toHaveValue('http://127.0.0.1:18053');
  await assertNoUnhandledCalls(page);
  capture.assertClean();
});

test('REPO setup offers a folder picker when automatic discovery finds nothing', async ({ page }) => {
  const state = await repoAutomaticState();
  const capture = await installFakeHost(page, state);
  await page.goto('/');
  await waitForFrontend(page);
  await openConnections(page);
  await page.getByRole('button', { name: /R.E.P.O. Game/ }).click();
  await page.getByRole('button', { name: 'Connect automatically', exact: true }).click();
  await expect(page.getByText('No configuration found. Select the game folder.')).toBeVisible();
  await page.getByRole('button', { name: 'Select game folder…' }).click();
  await expect(page.getByText('API token imported and saved.')).toBeVisible();
  await expect(page.getByText('API token imported and saved.')).toHaveClass(/connection-editor__details--ok/);
  await expect(page.getByText('Connected to http://127.0.0.1:17453').first()).toBeVisible();
  await assertNoUnhandledCalls(page);
  capture.assertClean();
});
