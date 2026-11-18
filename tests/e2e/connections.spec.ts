import { HOST_MESSAGES } from '../../src/web/shared/testing/host-messages.ts';
import { testMessage } from '../../src/web/shared/i18n/test-message.ts';

const t = (key: string, params?: Record<string, string | number>): string => testMessage('en', key, params);
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
  const tab = page.getByRole('link', { name: t('navigation.connections') });
  await expect(tab).toBeVisible();
  await tab.click();
  await expect(tab).toHaveAttribute('aria-current', 'page');
}

test('connections page shows empty servers state without plugins', async ({ page }) => {
  const consoleCapture = await installFakeHost(page, emptyState());
  await page.goto('/');
  await waitForFrontend(page);
  await openConnections(page);

  await expect(page.getByText(t('connections.servers.emptyTitle'))).toBeVisible();

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

  // The detail opens inside the server panel: the header carries the probe
  // status while the body keeps the connection fields editable.
  await expect(page.getByText(t('connections.servers.connectedIn', { ms: 12 }))).toBeVisible();
  const urlField = page.locator('.plugin-servers__detail input').first();
  await expect(urlField).toBeEnabled();
  await expect(urlField).toHaveValue('http://127.0.0.1:17842');

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

  // Explicit probe; the inline form stays visible and the panel header
  // reports the failure while the body keeps the host error.
  // SonicBoom cards label the probe as a connect action.
  await page.getByRole('button', { name: /Connect SonicBoom/ }).click();
  await expect(page.getByText(t('connections.servers.failed'))).toBeVisible();
  await expect(page.getByText('connection refused', { exact: false }).first()).toBeVisible();

  // Settings remain editable after a failed probe.
  const urlField = page.locator('.plugin-servers__detail input').first();
  await expect(urlField).toBeEnabled();
  await expect(page.getByRole('link', { name: t('navigation.connections') })).toHaveAttribute(
    'aria-current',
    'page',
  );

  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});

test('REPO imports a token through the shared card without showing the secret', async ({ page }) => {
  const { baseHostState } = await import('./fixtures/tiktools-host.ts');
  const { sonicboomStatus, sonicboomDescriptor } = await import('./fixtures/plugins.ts');
  const manifest = JSON.parse(readFileSync(new URL('../../tests/fixtures/plugins/repo-game.plugin.json', import.meta.url), 'utf8'));
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
  state.pluginHealth[id] = { ok: false, latencyMs: 1, error: HOST_MESSAGES.apiTokenMissing };
  const capture = await installFakeHost(page, state);
  await page.goto('/');
  await waitForFrontend(page);
  await openConnections(page);
  await page.getByRole('button', { name: /R.E.P.O. Game/ }).click();
  const importButton = page.getByRole('button', { name: t('plugins.connection.actions.import'), exact: true });
  await expect(importButton).toBeVisible();
  await expect(page.getByRole('button', { name: t('plugins.connection.getToken') })).toHaveCount(0);
  await importButton.click();
  await expect(page.getByText(t('plugins.messages.tokenImported'))).toBeVisible();
  await expect(page.getByText(t('plugins.messages.tokenImported'))).toHaveClass(/connection-editor__details--ok/);
  await expect(page.getByText(t('connections.servers.connectedIn', { ms: 1 }))).toBeVisible();
  await expect(page.locator('.plugin-servers__detail input[type="password"]')).toHaveValue('••••••••');
  await assertNoUnhandledCalls(page);
  capture.assertClean();
});

async function repoAutomaticState() {
  const { baseHostState } = await import('./fixtures/tiktools-host.ts');
  const { sonicboomStatus, sonicboomDescriptor } = await import('./fixtures/plugins.ts');
  const manifest = JSON.parse(readFileSync(new URL('../../tests/fixtures/plugins/repo-game.plugin.json', import.meta.url), 'utf8'));
  const state = baseHostState();
  state.automationSnapshot.plugins = [sonicboomStatus({ descriptor: sonicboomDescriptor({
    id: manifest.id, name: { default: manifest.name }, supportsTokenProvisioning: false,
    supportsTokenImport: true, supportsTokenDiscovery: true, actionTypeIds: [],
  }) })];
  state.automationSnapshot.pluginPages = [{ id: 'connection', pluginId: manifest.id,
    title: { default: 'Connection' }, sections: [{ kind: 'connection' }], source: { kind: 'plugin', pluginId: manifest.id } }];
  state.pluginSettings[manifest.id] = { schema: manifest.settings.schema, values: { serverUrl: 'http://127.0.0.1:17453' } };
  state.pluginHealth[manifest.id] = { ok: false, latencyMs: 1, error: HOST_MESSAGES.apiTokenMissing };
  return state;
}

test('automatic REPO setup chooses a profile and distinguishes API availability from gameplay readiness', async ({ page }) => {
  const state = await repoAutomaticState();
  state.pluginTokenCandidates = { 'repo.game': [
    { id: 'aabbccddeeff0011', label: 'Steam — Local installation' },
    { id: '1122334455667788', label: 'Mod profile: Stream' },
  ] };
  state.pluginSetupEndpoints = { 'repo.game': 'http://127.0.0.1:18053' };
  state.pluginSetupHealth = { 'repo.game': { ok: false, reachable: true, gameplayReady: false, latencyMs: 1, error: HOST_MESSAGES.apiReachableGameplayUnavailable } };
  const capture = await installFakeHost(page, state);
  await page.goto('/');
  await waitForFrontend(page);
  await openConnections(page);
  await page.getByRole('button', { name: /R.E.P.O. Game/ }).click();
  await page.getByRole('button', { name: t('plugins.connection.actions.auto'), exact: true }).click();
  await expect(page.getByText('Steam — Local installation')).toBeVisible();
  await expect(page.getByText('Mod profile: Stream')).toBeVisible();
  await expect(page.getByText(t('plugins.messages.tokenImported'))).toHaveCount(0);
  await page.locator('.connection-editor__candidate-list li').filter({ hasText: 'Mod profile: Stream' }).getByRole('button', { name: t('plugins.connection.useConfiguration') }).click();
  await expect(page.getByText(t('plugins.messages.tokenImported'))).toBeVisible();
  await expect(page.getByText(t('plugins.messages.tokenImported'))).toHaveClass(/connection-editor__details--ok/);
  await expect(page.getByText(t('connections.servers.apiNotReady'), { exact: true }).first()).toBeVisible();
  await expect(page.locator('.plugin-servers__detail input[type="password"]')).toHaveValue('••••••••');
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
  await page.getByRole('button', { name: t('plugins.connection.actions.auto'), exact: true }).click();
  await expect(page.getByRole('button', { name: t('plugins.connection.actions.folder'), exact: true })).toBeVisible();
  await page.getByRole('button', { name: t('plugins.connection.actions.folder'), exact: true }).click();
  await expect(page.getByText(t('plugins.messages.tokenImported'))).toBeVisible();
  await expect(page.getByText(t('plugins.messages.tokenImported'))).toHaveClass(/connection-editor__details--ok/);
  await expect(page.getByText(t('connections.servers.connectedIn', { ms: 1 }))).toBeVisible();
  await assertNoUnhandledCalls(page);
  capture.assertClean();
});
