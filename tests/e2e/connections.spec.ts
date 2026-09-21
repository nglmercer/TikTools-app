import { expect, test } from '@playwright/test';

import {
  assertNoUnhandledCalls,
  installFakeHost,
} from './fixtures/tiktools-host.ts';
import {
  emptyState,
  sonicboomConnectedState,
  sonicboomDisconnectedState,
} from './fixtures/states.ts';

async function openConnections(page: Parameters<typeof installFakeHost>[0]) {
  const tab = page.getByRole('button', { name: 'Connections' });
  await expect(tab).toBeVisible();
  await tab.click();
  await expect(tab).toHaveAttribute('aria-current', 'page');
}

test('connections page shows empty servers state without plugins', async ({ page }) => {
  const consoleCapture = await installFakeHost(page, emptyState());
  await page.goto('/');
  await openConnections(page);

  await expect(page.getByText('No server connections yet')).toBeVisible();

  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});

test('server connection success shows latency and stays editable', async ({ page }) => {
  const consoleCapture = await installFakeHost(page, sonicboomConnectedState());
  await page.goto('/');
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
  await openConnections(page);

  const server = page.getByRole('button', { name: /SonicBoom Server/ });
  await expect(server).toBeVisible();
  await server.click();

  // Explicit probe; the inline card keeps the form visible and reports the
  // host error on the server head (accessible name) plus a status line.
  await page.getByRole('button', { name: 'Test connection' }).click();
  await expect(page.getByRole('button', { name: /connection refused/ })).toBeVisible();
  await expect(page.getByText('Connection failed.').first()).toBeVisible();

  // Settings remain editable after a failed probe.
  const urlField = page.locator('input').first();
  await expect(urlField).toBeEnabled();
  await expect(page.getByRole('button', { name: 'Connections' })).toHaveAttribute(
    'aria-current',
    'page',
  );

  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});
