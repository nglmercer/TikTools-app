import { testMessage } from '../../src/web/shared/i18n/test-message.ts';

const t = (key: string): string => testMessage('en', key);
import { expect, test } from '@playwright/test';
import { assertNoUnhandledCalls, emitTopic, installFakeHost, waitForFrontend, writeHostState, readHostState } from './fixtures/tiktools-host.ts';
import { emptyState } from './fixtures/states.ts';
import { SEL } from './support/selectors.ts';

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
  await expect(page.locator(SEL.eventCardGift)).toHaveCount(2);
  await page.getByRole('link', { name: t('navigation.connections'), exact: true }).click();
  await expect(page.locator('.connections-page')).toBeVisible();
  await writeHostState(page, { ...await readHostState(page), feedJournal: [{ cursor: 4, event: gift }, { cursor: 5, event: gift }] });
  await page.getByRole('link', { name: t('navigation.feed'), exact: true }).click();
  // Streams stay active, so the rows missed in the journal are recovered when the next delivery reveals the gap.
  await emitTopic(page, 'live.ui-event', { cursor: 6, event: gift });
  await expect(page.locator(SEL.eventCardGift)).toHaveCount(5);
  await expect(page.locator('.event-card--gift img.event-card__gift')).toHaveCount(0);
  await page.locator(SEL.feedSearch).fill('Rose');
  await expect(page.locator(SEL.eventCard)).toHaveCount(5);
  await assertNoUnhandledCalls(page);
  capture.assertClean();
});

test('opaque emotes render alongside unchanged text and degrade on image failure', async ({ page }) => {
  const capture = await installFakeHost(page, emptyState());
  await page.route('https://cdn.example.test/**', (route) => route.fulfill({ status: 200, contentType: 'image/png', body: 'invalid image bytes' }));
  await page.goto('/');
  await waitForFrontend(page);
  await emitTopic(page, 'live.ui-event', { event: { kind: 'chat', author: 'alice', text: 'Hola 👩🏽‍💻 世界', msgId: '1', emotes: [{ id: '98765', imageUrl: 'https://cdn.example.test/emote.png', packageId: 'creator' }] } });
  await expect(page.locator(SEL.eventCard)).toContainText('Hola 👩🏽‍💻 世界');
  await expect(page.locator('.event-card [role="img"][aria-label="TikTok emote"]')).toBeVisible();
  await emitTopic(page, 'live.ui-event', { event: { kind: 'chat', author: 'alice', text: '', msgId: '2', emotes: [{ id: '98765' }] } });
  await expect(page.locator(SEL.eventCard)).toHaveCount(2);
  await expect(page.locator('.event-card [aria-label="TikTok emote"]')).toHaveCount(2);
  await assertNoUnhandledCalls(page);
  capture.assertClean();
});

test('monitor configuration survives page navigation and stops through the host', async ({ page }) => {
  const capture = await installFakeHost(page, emptyState());
  await page.goto('/');
  await waitForFrontend(page);
  await page.getByRole('link', { name: t('navigation.connections'), exact: true }).click();
  await page.locator('#live-handle').fill('alice');
  // Monitoring is a switch; Connect then starts it instead of connecting directly.
  await page.getByRole('switch', { name: t('connections.monitor.start') }).check();
  await page.getByRole('main').getByRole('button', { name: t('connection.connect'), exact: true }).click();
  await expect(page.getByText('LIVE monitor: Waiting for LIVE')).toBeVisible();
  await expect(page.locator('#live-handle')).toBeDisabled();
  await page.getByRole('link', { name: t('navigation.feed'), exact: true }).click();
  await emitTopic(page, 'live.monitor', { status: { enabled: true, uniqueId: 'alice', phase: 'live', pollIntervalSeconds: 30, checks: 2, failures: 0, lastCheckedAt: Date.now(), nextCheckAt: null, lastError: null } });
  await page.getByRole('link', { name: t('navigation.connections'), exact: true }).click();
  await expect(page.getByText('LIVE monitor: Connected; monitoring remains active')).toBeVisible();
  await page.getByRole('button', { name: t('connections.monitor.stop'), exact: true }).click();
  await expect(page.locator('#live-handle')).toBeEnabled();
  await expect(page.getByRole('switch', { name: t('connections.monitor.start') })).toBeEnabled();
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
  await expect(page.locator(SEL.eventCardGift)).toHaveCount(3);
  await assertNoUnhandledCalls(page);
  capture.assertClean();
});
