import { testMessage } from '../../src/web/shared/i18n/test-message.ts';
import { expect, test } from '@playwright/test';

import {
  assertNoUnhandledCalls,
  baseHostState,
  installFakeHost,
  waitForFrontend,
  readHostState,
} from './fixtures/tiktools-host.ts';
import { RPC } from '../shared/index.ts';

const t = (key: string): string => testMessage('en', key);

async function openConnections(page: Parameters<typeof installFakeHost>[0]): Promise<void> {
  const tab = page.getByRole('link', { name: t('navigation.connections') });
  await expect(tab).toBeVisible();
  await tab.click();
  await expect(tab).toHaveAttribute('aria-current', 'page');
}

function explorerState() {
  const state = baseHostState();
  state.liveChannels = {
    rooms: [
      { uniqueId: 'creator_one', roomId: '7100000000000000001', nickname: 'One', title: 'Live one', viewers: 1200 },
      { uniqueId: 'creator_two', roomId: '7100000000000000002', nickname: 'Two', title: 'Live two', viewers: 340 },
    ],
  };
  return state;
}

async function discoveryCalls(page: Parameters<typeof installFakeHost>[0], method: string) {
  const host = await readHostState(page);
  return host.calls.filter((call) => call.method === method);
}

test('the explorer stays inert until opened, then connects only the chosen room', async ({ page }) => {
  const capture = await installFakeHost(page, explorerState());
  await page.goto('/');
  await waitForFrontend(page);
  await openConnections(page);

  // Closed by default: nothing has queried TikTok yet.
  expect(await discoveryCalls(page, RPC.liveChannels)).toHaveLength(0);

  await page.getByRole('button', { name: 'Explore', exact: true }).first().click();
  await expect(page.getByText('@creator_one')).toBeVisible();
  await expect(page.getByText('@creator_two')).toBeVisible();
  await expect(page.getByRole('cell', { name: '1,200', exact: true })).toBeVisible();

  const row = page.getByRole('row').filter({ hasText: '@creator_two' });
  await row.getByRole('button', { name: t('connection.connect') }).click();

  await expect.poll(async () => (await discoveryCalls(page, RPC.liveConnect)).length).toBe(1);
  const connect = (await discoveryCalls(page, RPC.liveConnect))[0];
  expect(connect?.params).toMatchObject({
    uniqueId: 'creator_two',
    roomId: '7100000000000000002',
  });

  await assertNoUnhandledCalls(page);
  capture.assertClean();
});

test('a random pick previews a candidate and never connects on its own', async ({ page }) => {
  const capture = await installFakeHost(page, explorerState());
  await page.goto('/');
  await waitForFrontend(page);
  await openConnections(page);
  await page.getByRole('button', { name: 'Explore', exact: true }).first().click();
  await expect(page.getByText('@creator_one')).toBeVisible();

  await page.getByRole('button', { name: t('connections.explore.random') }).click();
  await expect(page.getByText(t('connections.explore.randomPreview'))).toBeVisible();

  // Previewing must not connect by itself.
  expect(await discoveryCalls(page, RPC.liveConnect)).toHaveLength(0);

  await page.getByRole('button', { name: 'Dismiss' }).click();
  await expect(page.getByText(t('connections.explore.randomPreview'))).toHaveCount(0);

  await page.getByRole('button', { name: t('connections.explore.random') }).click();
  await page.locator('.live-explorer__random').getByRole('button', { name: t('connection.connect') }).click();
  await expect.poll(async () => (await discoveryCalls(page, RPC.liveConnect)).length).toBe(1);

  await assertNoUnhandledCalls(page);
  capture.assertClean();
});

test('a sign-in refusal reads as an auth requirement, not an empty list', async ({ page }) => {
  const state = explorerState();
  state.failures['live.channels'] = { code: 'unauthenticated', message: 'login required' };
  const capture = await installFakeHost(page, state);
  await page.goto('/');
  await waitForFrontend(page);
  await openConnections(page);

  await page.getByRole('button', { name: 'Explore', exact: true }).first().click();

  await expect(page.getByText(t('connections.explore.authRequired'))).toBeVisible();
  await expect(page.getByText(t('connections.explore.noMatches'))).toHaveCount(0);

  await assertNoUnhandledCalls(page);
  capture.assertClean();
});

test('an empty page reads as no matches, with a retry affordance', async ({ page }) => {
  const capture = await installFakeHost(page, baseHostState());
  await page.goto('/');
  await waitForFrontend(page);
  await openConnections(page);

  await page.getByRole('button', { name: 'Explore', exact: true }).first().click();

  await expect(page.getByText(t('connections.explore.noMatches'))).toBeVisible();
  await assertNoUnhandledCalls(page);
  capture.assertClean();
});
