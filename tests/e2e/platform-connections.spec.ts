import { expect, test } from '@playwright/test';
import { assertNoUnhandledCalls, installFakeHost, waitForFrontend } from './fixtures/tiktools-host.ts';
import { emptyState } from './fixtures/states.ts';

test('connects Twitch through an opaque credential and keeps the token out of browser storage', async ({ page }, info) => {
  const state = emptyState();
  state.livePlatforms = [{ platform: 'twitch', name: 'Twitch', authentication: 'oauth-token', capabilities: { chat: true, subscriptions: true, gifts: true, follows: false, likes: false, shares: false, joins: false, roomStats: false, discovery: false, sendChat: false, moderation: false } }];
  const consoleCapture = await installFakeHost(page, state);
  await page.goto('/');
  await waitForFrontend(page);
  await page.getByRole('link', { name: 'Connections' }).click();
  const panel = page.locator('.base-panel').filter({ has: page.getByRole('heading', { name: 'Live platforms' }) });
  await expect(page.getByRole('heading', { name: 'Twitch' })).toBeVisible();
  await page.getByRole('textbox', { name: 'Channel login' }).fill('channel');
  await page.getByLabel('Twitch OAuth token', { exact: true }).fill('fixture-token');
  await panel.getByRole('button', { name: 'Connect', exact: true }).click();
  await expect(panel.getByText('channel — Connected')).toBeVisible();
  await expect(page.getByLabel('Twitch OAuth token', { exact: true })).toHaveValue('');
  expect(await page.evaluate(() => JSON.stringify(localStorage))).not.toContain('fixture-token');
  await page.screenshot({ path: info.outputPath('platform-connections.png'), fullPage: true });
  await panel.getByRole('button', { name: 'Disconnect', exact: true }).click();
  await expect(panel.getByText('channel — Connected')).toHaveCount(0);
  await assertNoUnhandledCalls(page);
  consoleCapture.assertClean();
});
