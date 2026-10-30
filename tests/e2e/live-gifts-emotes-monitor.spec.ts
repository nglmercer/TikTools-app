import { expect, test } from '@playwright/test';
import { assertNoUnhandledCalls, emitTopic, installFakeHost, waitForFrontend, writeHostState, readHostState } from './fixtures/tiktools-host.ts';
import { emptyState } from './fixtures/states.ts';

test('gift identities, hidden-page backfill, metadata search and broken artwork retain rows', async ({ page }) => {
  const capture = await installFakeHost(page, emptyState());
  await page.goto('/');
  await waitForFrontend(page);
  await emitTopic(page, 'live.connected', { uniqueId: 'creator', roomId: 'room' });
  const gift = { kind: 'gift', author: 'alice', text: 'sent gift', msgId: '0', roomId: 'room', creatorId: 'creator', giftDetails: { giftId: '5655', name: 'Rose', count: 1, diamonds: 1, imageUrl: 'https://cdn.example.test/broken.png' } };
  await page.route('https://cdn.example.test/**', (route) => route.fulfill({ status: 200, contentType: 'image/png', body: 'invalid image bytes' }));
  await emitTopic(page, 'live.ui-event', { cursor: 1, event: { kind: 'chat', author: 'alice', text: 'chat', msgId: '42' } });
  await emitTopic(page, 'live.ui-event', { cursor: 2, event: { ...gift, msgId: '42' } });
  await emitTopic(page, 'live.ui-event', { cursor: 3, event: gift });
  await expect(page.locator('.event-row--gift')).toHaveCount(2);
  await page.getByRole('link', { name: 'Connections', exact: true }).click();
  await expect(page.locator('.connections-page')).toBeVisible();
  await writeHostState(page, { ...await readHostState(page), feedJournal: [{ cursor: 4, event: gift }, { cursor: 5, event: gift }] });
  await page.getByRole('link', { name: 'Live feed', exact: true }).click();
  await expect(page.locator('.event-row--gift')).toHaveCount(4);
  await expect(page.locator('.event-row--gift img.event-row__gift')).toHaveCount(0);
  await page.locator('.live-feed__search input').fill('Rose');
  await expect(page.locator('.event-row')).toHaveCount(4);
  await assertNoUnhandledCalls(page);
  capture.assertClean();
});

test('opaque emotes render alongside unchanged text and degrade on image failure', async ({ page }) => {
  const capture = await installFakeHost(page, emptyState());
  await page.route('https://cdn.example.test/**', (route) => route.fulfill({ status: 200, contentType: 'image/png', body: 'invalid image bytes' }));
  await page.goto('/');
  await waitForFrontend(page);
  await emitTopic(page, 'live.ui-event', { event: { kind: 'chat', author: 'alice', text: 'Hola 👩🏽‍💻 世界', msgId: '1', emotes: [{ id: '98765', imageUrl: 'https://cdn.example.test/emote.png', packageId: 'creator' }] } });
  await expect(page.locator('.event-row')).toContainText('Hola 👩🏽‍💻 世界');
  await expect(page.locator('.event-row [role="img"][aria-label="TikTok emote"]')).toBeVisible();
  await emitTopic(page, 'live.ui-event', { event: { kind: 'chat', author: 'alice', text: '', msgId: '2', emotes: [{ id: '98765' }] } });
  await expect(page.locator('.event-row')).toHaveCount(2);
  await expect(page.locator('.event-row [aria-label="TikTok emote"]')).toHaveCount(2);
  await assertNoUnhandledCalls(page);
  capture.assertClean();
});

test('monitor configuration survives page navigation and stops through the host', async ({ page }) => {
  const capture = await installFakeHost(page, emptyState());
  await page.goto('/');
  await waitForFrontend(page);
  await page.getByRole('link', { name: 'Connections', exact: true }).click();
  await page.locator('#live-handle').fill('alice');
  // Monitoring is a switch; Connect then starts it instead of connecting directly.
  await page.getByRole('switch', { name: 'Monitor and auto-connect' }).check();
  await page.getByRole('main').getByRole('button', { name: 'Connect', exact: true }).click();
  await expect(page.getByText('LIVE monitor: Waiting for LIVE')).toBeVisible();
  await expect(page.locator('#live-handle')).toBeDisabled();
  await page.getByRole('link', { name: 'Live feed', exact: true }).click();
  await emitTopic(page, 'live.monitor', { status: { enabled: true, uniqueId: 'alice', phase: 'live', pollIntervalSeconds: 30, checks: 2, failures: 0, lastCheckedAt: Date.now(), nextCheckAt: null, lastError: null } });
  await page.getByRole('link', { name: 'Connections', exact: true }).click();
  await expect(page.getByText('LIVE monitor: Connected; monitoring remains active')).toBeVisible();
  await page.getByRole('button', { name: 'Stop monitoring', exact: true }).click();
  await expect(page.locator('#live-handle')).toBeEnabled();
  await expect(page.getByRole('switch', { name: 'Monitor and auto-connect' })).toBeEnabled();
  await assertNoUnhandledCalls(page);
  capture.assertClean();
});

test('visible feed fills journal holes without artwork or a page visibility transition', async ({ page }) => {
  const capture = await installFakeHost(page, emptyState());
  await page.goto('/');
  await waitForFrontend(page);
  const gift = { kind: 'gift', author: 'alice', text: 'Uncatalogued gift', msgId: '0' };
  await emitTopic(page, 'live.ui-event', { event: { ...gift, feedCursor: 1 } });
  await writeHostState(page, { ...await readHostState(page), feedJournal: [{ cursor: 2, event: gift }, { cursor: 3, event: gift }] });
  await emitTopic(page, 'live.ui-event', { event: { ...gift, feedCursor: 3 } });
  await expect(page.locator('.event-row--gift')).toHaveCount(3);
  await assertNoUnhandledCalls(page);
  capture.assertClean();
});
