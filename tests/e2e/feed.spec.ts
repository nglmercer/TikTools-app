import { testMessage } from '../../src/web/shared/i18n/test-message.ts';

const t = (key: string, params?: Record<string, string | number>): string => testMessage('en', key, params);
import { expect, test } from '@playwright/test';
import { assertNoUnhandledCalls, emitTopic, installFakeHost, waitForFrontend } from './fixtures/tiktools-host.ts';
import { emptyState } from './fixtures/states.ts';

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
  const emitChat = (index: number) => emitTopic(page, 'live.ui-event', {
    event: { kind: 'chat', author: 'alice', text: `feed message ${index}`, msgId: `feed-${index}` },
  });
  await emitChat(1);
  await expect(page.locator('.event-row')).toHaveCount(1);
  await page.evaluate(() => { (window as typeof window & { feedFormatCalls: number }).feedFormatCalls = 0; });
  await emitChat(2);
  await expect(page.locator('.event-row')).toHaveCount(2);
  expect(await page.evaluate(() => (window as typeof window & { feedFormatCalls: number }).feedFormatCalls)).toBe(1);

  await page.getByRole('menuitem', { name: 'View', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Language', exact: true }).hover();
  await page.getByRole('menuitemcheckbox', { name: 'Español' }).click();
  await expect(page.locator('html')).toHaveAttribute('lang', 'es');
  await expect.poll(() => page.evaluate(() => (window as typeof window & { feedFormatCalls: number }).feedFormatCalls)).toBe(3);
  await page.locator('.live-feed__search input').fill('message 2');
  await expect(page.locator('.event-row')).toHaveCount(1);
  await expect(page.locator('.event-row')).toContainText('feed message 2');
  await page.locator('.live-feed__search input').fill('');
  await expect(page.locator('.event-row')).toHaveCount(2);
  await page.getByRole('button', { name: /Limpiar/ }).click();
  await expect(page.locator('.event-row')).toHaveCount(0);
  await assertNoUnhandledCalls(page);
  capture.assertClean();
});

test('hidden feed makes no commits and resumes with a discontinuity', async ({ page }) => {
  const capture = await installFakeHost(page, emptyState());
  await page.goto('/');
  await waitForFrontend(page);
  await emitTopic(page, 'live.ui-event', { event: { kind: 'chat', author: 'alice', text: 'before', msgId: 'before' } });
  await expect(page.locator('.event-row')).toHaveCount(1);
  await page.evaluate(() => {
    const scope = window as unknown as { __tiktools_benchmark_controls__: { setTab(tab: string): void } };
    scope.__tiktools_benchmark_controls__.setTab('settings');
  });
  await expect(page.locator('.live-feed__stream')).toHaveCount(0);
  const commits = await page.evaluate(() => {
    const scope = window as unknown as { __tiktools_frontend_diagnostics__(): { live: { commits: number } } };
    return scope.__tiktools_frontend_diagnostics__().live.commits;
  });
  await page.evaluate(() => {
    for (let i = 0; i < 10_000; i += 1) {
      window.__webview_on_message__?.(JSON.stringify({ method: 'event', params: { topic: 'live.ui-event',
        data: { event: { kind: 'chat', author: 'alice', text: `hidden ${i}`, msgId: `hidden-${i}` } } } }));
    }
  });
  expect(await page.evaluate(() => {
    const scope = window as unknown as { __tiktools_frontend_diagnostics__(): { live: { commits: number } } };
    return scope.__tiktools_frontend_diagnostics__().live.commits;
  })).toBe(commits);
  await page.evaluate(() => {
    (window as unknown as { __tiktools_benchmark_controls__: { setTab(tab: string): void } }).__tiktools_benchmark_controls__.setTab('feed');
  });
  await expect(page.locator('.event-row')).toHaveCount(2);
  await expect(page.locator('.event-row').last()).toContainText(t('feed.resumed'));
  await assertNoUnhandledCalls(page);
  capture.assertClean();
});

test('image-disabled diagnostic mode removes avatar and gift image requests', async ({ page }) => {
  const capture = await installFakeHost(page, emptyState());
  await page.goto('/');
  await waitForFrontend(page);
  await page.evaluate(() => {
    (window as unknown as { __tiktools_benchmark_controls__: { setImagesEnabled(enabled: boolean): void } }).__tiktools_benchmark_controls__.setImagesEnabled(false);
  });
  await emitTopic(page, 'live.ui-event', { event: { kind: 'gift', author: 'alice', text: 'gift', msgId: 'gift-image-ab',
    avatarUrl: 'https://example.invalid/avatar.png', giftDetails: { name: 'Rose', count: 1, diamonds: 1, imageUrl: 'https://example.invalid/gift.png' } } });
  await expect(page.locator('.event-row')).toHaveCount(1);
  await expect(page.locator('.event-row img')).toHaveCount(0);
  await expect(page.locator('.event-row__text')).toHaveText('gift');
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
  await expect(page.locator('.event-row')).toHaveCount(100);
  const bottomDistance = () => page.locator('.live-feed__stream').evaluate((node) => node.scrollHeight - node.clientHeight - node.scrollTop);
  await expect.poll(bottomDistance).toBeLessThan(2);
  await page.getByRole('button', { name: /auto-scroll/ }).click();
  await page.locator('.live-feed__stream').evaluate((node) => { node.scrollTop = 100; });
  await emitTopic(page, 'live.ui-event', { event: { kind: 'chat', author: 'alice', text: 'paused', msgId: 'paused-scroll' } });
  await expect(page.locator('.event-row')).toHaveCount(101);
  expect(await page.locator('.live-feed__stream').evaluate((node) => node.scrollTop)).toBe(100);
  await page.evaluate(() => (window as unknown as { __tiktools_benchmark_controls__: { setTab(tab: string): void } }).__tiktools_benchmark_controls__.setTab('settings'));
  await expect(page.locator('.live-feed__stream')).toHaveCount(0);
  await page.evaluate(() => (window as unknown as { __tiktools_benchmark_controls__: { setTab(tab: string): void } }).__tiktools_benchmark_controls__.setTab('feed'));
  await expect.poll(() => page.locator('.live-feed__stream').evaluate((node) => node.scrollTop)).toBe(100);
  await page.getByRole('button', { name: /auto-scroll/ }).click();
  await expect.poll(bottomDistance).toBeLessThan(2);
  await assertNoUnhandledCalls(page);
  capture.assertClean();
});

test('pinned feed renders 100 while pause and search expose all 300 buffered events', async ({ page }) => {
  const capture = await installFakeHost(page, emptyState());
  await page.goto('/');
  await waitForFrontend(page);
  await page.evaluate(() => {
    window.__tiktools_receive_batch__?.(Array.from({ length: 300 }, (_, index) => JSON.stringify({ method: 'event', params: { topic: 'live.ui-event',
      data: { event: { kind: 'chat', author: 'alice', text: `buffer ${index}`, msgId: `buffer-${index}` } } } })));
  });
  await expect(page.locator('.event-row')).toHaveCount(100);
  await expect(page.locator('.live-feed__hint')).toContainText(t('feed.windowed', { count: 100 }));
  await page.locator('.live-feed__search input').fill('buffer 0');
  await expect(page.locator('.event-row')).toHaveCount(1);
  await expect(page.locator('.event-row')).toContainText('buffer 0');
  await page.locator('.live-feed__search input').fill('');
  await expect(page.locator('.event-row')).toHaveCount(100);
  await page.getByRole('button', { name: /auto-scroll/ }).click();
  await expect(page.locator('.event-row')).toHaveCount(300);
  await expect(page.locator('.live-feed__hint')).toHaveCount(0);
  await assertNoUnhandledCalls(page);
  capture.assertClean();
});
