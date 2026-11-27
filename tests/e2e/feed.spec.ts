import { testMessage } from '../../src/web/shared/i18n/test-message.ts';

const t = (key: string, params?: Record<string, string | number>): string => testMessage('en', key, params);
import { expect, test } from '@playwright/test';
import { assertNoUnhandledCalls, emitPush, emitTopic, installFakeHost, waitForFrontend } from './fixtures/tiktools-host.ts';
import { emptyState } from './fixtures/states.ts';
import { SEL } from './support/selectors.ts';
import { TOPIC, LIMIT } from '../shared/index.ts';

test('feed appends only format new cards and still update locale, search, and clear', async ({ page }) => {
  const capture = await installFakeHost(page, emptyState());
  await page.addInitScript(() => {
    const target = window as typeof window & { feedFormatCalls: number };
    target.feedFormatCalls = 0;
    const descriptor = Object.getOwnPropertyDescriptor(Intl.DateTimeFormat.prototype, 'format')!;
    Object.defineProperty(Intl.DateTimeFormat.prototype, 'format', {
      ...descriptor,
      get() {
        const format = descriptor.get!.call(this) as (date?: number | Date) => string;
        return (date?: number | Date) => {
          target.feedFormatCalls += 1;
          return format(date);
        };
      },
    });
  });
  await page.goto('/');
  await waitForFrontend(page);
  await expect(page.getByText(t('feed.empty'), { exact: false })).toBeVisible();
  const emitChat = (index: number) => emitTopic(page, TOPIC.liveUiEvent, {
    event: { kind: 'chat', author: 'alice', text: `feed message ${index}`, msgId: `feed-${index}` },
  });
  await emitChat(1);
  await expect(page.locator(SEL.eventCard)).toHaveCount(1);
  await page.evaluate(() => { (window as typeof window & { feedFormatCalls: number }).feedFormatCalls = 0; });
  await emitChat(2);
  await expect(page.locator(SEL.eventCard)).toHaveCount(2);
  expect(await page.evaluate(() => (window as typeof window & { feedFormatCalls: number }).feedFormatCalls)).toBe(1);

  // The native shell menu switches the language by pushing its action.
  await emitPush(page, { type: 'shell-menu-action', id: 'locale:es' });
  await expect(page.locator('html')).toHaveAttribute('lang', 'es');
  await expect.poll(() => page.evaluate(() => (window as typeof window & { feedFormatCalls: number }).feedFormatCalls)).toBe(3);
  await page.locator(SEL.feedSearch).fill('message 2');
  await expect(page.locator(SEL.eventCard)).toHaveCount(1);
  await expect(page.locator(SEL.eventCard)).toContainText('feed message 2');
  await page.locator(SEL.feedSearch).fill('');
  await expect(page.locator(SEL.eventCard)).toHaveCount(2);
  await page.getByRole('button', { name: /Limpiar/ }).click();
  await expect(page.locator(SEL.eventCard)).toHaveCount(0);
  await assertNoUnhandledCalls(page);
  capture.assertClean();
});

test('events keep arriving while another tab is open and are waiting on return', async ({ page }) => {
  // Visibility gating is retired: streams stay active so no chat history is skipped.
  const capture = await installFakeHost(page, emptyState());
  await page.goto('/');
  await waitForFrontend(page);
  await emitTopic(page, TOPIC.liveUiEvent, { event: { kind: 'chat', author: 'alice', text: 'before', msgId: 'before' } });
  await expect(page.locator(SEL.eventCard)).toHaveCount(1);
  const setTab = (tab: string) => page.evaluate((target) => {
    (window as unknown as { __tiktools_benchmark_controls__: { setTab(tab: string): void } }).__tiktools_benchmark_controls__.setTab(target);
  }, tab);
  await setTab('settings');
  await expect(page.locator(SEL.feedStream)).toHaveCount(0);
  for (let index = 0; index < 5; index += 1) {
    await emitTopic(page, TOPIC.liveUiEvent, { event: { kind: 'chat', author: 'alice', text: `hidden ${index}`, msgId: `hidden-${index}` } });
  }
  await setTab('feed');
  await expect(page.locator(SEL.eventCard)).toHaveCount(6);
  await expect(page.locator(SEL.eventCard).last()).toContainText('hidden 4');
  await assertNoUnhandledCalls(page);
  capture.assertClean();
});

test('image-disabled diagnostic mode removes avatar and gift image requests', async ({ page }) => {
  const capture = await installFakeHost(page, emptyState());
  await page.goto('/');
  await waitForFrontend(page);
  await page.evaluate(() => {
    (window as unknown as { __tiktools_benchmark_controls__: { setImagesEnabled(enabled: boolean): void } })
      .__tiktools_benchmark_controls__.setImagesEnabled(false);
  });
  await emitTopic(page, TOPIC.liveUiEvent, { event: { kind: 'gift', author: 'alice', text: 'gift', msgId: 'gift-image-ab',
    avatarUrl: 'https://example.invalid/avatar.png', giftDetails: { name: 'Rose', count: 1, diamonds: 1, imageUrl: 'https://example.invalid/gift.png' } } });
  await expect(page.locator(SEL.eventCard)).toHaveCount(1);
  await expect(page.locator('.event-card img')).toHaveCount(0);
  await expect(page.locator(SEL.eventCardContent)).toHaveText('gift');
  await assertNoUnhandledCalls(page);
  capture.assertClean();
});

test('autoscroll and paused position survive feed navigation', async ({ page }) => {
  const capture = await installFakeHost(page, emptyState());
  await page.goto('/');
  await waitForFrontend(page);
  await page.evaluate(() => {
    const batch = Array.from({ length: 100 }, (_, index) => JSON.stringify({ method: 'event', params: { topic: 'live.ui-event',
      data: { event: { kind: 'chat', author: 'alice', text: `scroll ${index}`, msgId: `scroll-${index}` } } } }));
    window.__tiktools_receive_batch__?.(batch);
  });
  await expect(page.locator(SEL.eventCard)).toHaveCount(LIMIT.feedRenderLimit);
  const bottomDistance = () => page.locator(SEL.feedStream).evaluate((node) => node.scrollHeight - node.clientHeight - node.scrollTop);
  await expect.poll(bottomDistance).toBeLessThan(2);
  await page.getByRole('button', { name: /auto-scroll/ }).click();
  await page.locator(SEL.feedStream).evaluate((node) => { node.scrollTop = 100; });
  await emitTopic(page, TOPIC.liveUiEvent, { event: { kind: 'chat', author: 'alice', text: 'paused', msgId: 'paused-scroll' } });
  await expect(page.locator(SEL.eventCard)).toHaveCount(LIMIT.feedRenderLimit); // the per-type window evicts the oldest card
  // Evicting the oldest card above the reader moves the offset by its height so the visible rows stay put.
  const anchored = await page.locator(SEL.feedStream).evaluate((node) => node.scrollTop);
  expect(anchored).toBeGreaterThan(0);
  expect(anchored).toBeLessThan(100);
  await page.evaluate(() =>
    (window as unknown as { __tiktools_benchmark_controls__: { setTab(tab: string): void } }).__tiktools_benchmark_controls__.setTab('settings'),
  );
  await expect(page.locator(SEL.feedStream)).toHaveCount(0);
  await page.evaluate(() =>
    (window as unknown as { __tiktools_benchmark_controls__: { setTab(tab: string): void } }).__tiktools_benchmark_controls__.setTab('feed'),
  );
  await expect.poll(() => page.locator(SEL.feedStream).evaluate((node) => node.scrollTop)).toBe(anchored);
  await page.getByRole('button', { name: /auto-scroll/ }).click();
  await expect.poll(bottomDistance).toBeLessThan(2);
  await assertNoUnhandledCalls(page);
  capture.assertClean();
});

test('a flood of one event type keeps the newest 100 and search stays within them', async ({ page }) => {
  // Retention is per event type: the live buffer keeps the newest `limit` rows of each category.
  const capture = await installFakeHost(page, emptyState());
  await page.goto('/');
  await waitForFrontend(page);
  await page.getByRole('button', { name: 'View as list' }).click();
  await page.evaluate(() => {
    window.__tiktools_receive_batch__?.(Array.from({ length: 300 }, (_, index) => JSON.stringify({ method: 'event', params: { topic: 'live.ui-event',
      data: { event: { kind: 'chat', author: 'alice', text: `buffer ${index}`, msgId: `buffer-${index}` } } } })));
  });
  await expect(page.locator(SEL.eventRow)).toHaveCount(LIMIT.feedRenderLimit);
  await expect(page.locator('.live-feed__count')).toHaveText(t('feed.count', { count: 100 }));
  await expect(page.locator(SEL.eventRow).last()).toContainText('buffer 299');
  await page.locator(SEL.feedSearch).fill('buffer 299');
  await expect(page.locator(SEL.eventRow)).toHaveCount(1);
  await page.locator(SEL.feedSearch).fill('buffer 0');
  await expect(page.locator(SEL.eventRow)).toHaveCount(0);
  await page.locator(SEL.feedSearch).fill('');
  await expect(page.locator(SEL.eventRow)).toHaveCount(LIMIT.feedRenderLimit);
  await assertNoUnhandledCalls(page);
  capture.assertClean();
});
