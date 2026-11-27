import { testMessage } from '../../src/web/shared/i18n/test-message.ts';

const t = (key: string): string => testMessage('en', key);
import { expect, test } from '@playwright/test';
import { createWriteStream } from 'node:fs';
import { assertNoUnhandledCalls, installFakeHost, waitForFrontend } from './fixtures/tiktools-host.ts';
import { emptyState } from './fixtures/states.ts';

test('100 actual Vue root remounts return listeners, timers and DOM to baseline', async ({ page }, testInfo) => {
  test.setTimeout(120_000);
  const capture = await installFakeHost(page, emptyState());
  await page.goto('/');
  await waitForFrontend(page);
  const cdp = await page.context().newCDPSession(page);
  await cdp.send('HeapProfiler.enable');
  const snapshot = async (name: string): Promise<void> => {
    // Disposed RPC chains need a browser turn to settle; fixture call
    // history is deliberately removed from retained-heap measurements.
    await page.waitForTimeout(250);
    await page.evaluate(() => {
      const scope = window as unknown as { __TIKTOOLS_E2E_STATE__: { calls: unknown[] } };
      scope.__TIKTOOLS_E2E_STATE__.calls = [];
    });
    await cdp.send('HeapProfiler.collectGarbage');
    const path = testInfo.outputPath(`${name}.heapsnapshot`);
    const file = createWriteStream(path);
    const onChunk = ({ chunk }: { chunk: string }): void => { file.write(chunk); };
    cdp.on('HeapProfiler.addHeapSnapshotChunk', onChunk);
    await cdp.send('HeapProfiler.takeHeapSnapshot', { reportProgress: false });
    cdp.off('HeapProfiler.addHeapSnapshotChunk', onChunk);
    await new Promise<void>((resolve, reject) => { file.on('error', reject); file.end(resolve); });
    await testInfo.attach(name, { path, contentType: 'application/json' });
  };
  // A remount removes the old root's diagnostics before the new root publishes its own.
  const diagnostics = async () => {
    await page.waitForFunction(() => typeof (window as unknown as Record<string, unknown>)['__tiktools_frontend_diagnostics__'] === 'function');
    return page.evaluate(() => {
    const scope = window as unknown as { __tiktools_frontend_diagnostics__(): {
      domNodes: number; ownedTimers: number; iframeCount: number;
      control: { pending: number; topicSubscriptions: number; pushSubscriptions: number };
      live: { frames: number };
    } };
    return scope.__tiktools_frontend_diagnostics__();
    });
  };
  await expect.poll(async () => (await diagnostics()).control.pending).toBe(0);
  const baseline = await diagnostics();
  await snapshot('warmup');
  for (let i = 0; i < 100; i += 1) {
    await page.evaluate(() => {
      const scope = window as unknown as { __tiktools_benchmark_controls__: { setTab(tab: string): void }; __tiktools_remount__(): void };
      scope.__tiktools_remount__();
    });
    await expect.poll(async () => (await diagnostics()).control.pending).toBe(0);
    if (i === 49) await snapshot('after-50-roots');
  }
  await snapshot('after-100-roots');
  const final = await diagnostics();
  expect(final.control.topicSubscriptions).toBe(baseline.control.topicSubscriptions);
  expect(final.control.pushSubscriptions).toBe(baseline.control.pushSubscriptions);
  expect(final.domNodes).toBe(baseline.domNodes);
  expect(final.ownedTimers).toBe(0);
  expect(final.iframeCount).toBe(0);
  expect(final.live.frames).toBe(0);
  await testInfo.attach('diagnostics', { body: JSON.stringify({ baseline, final }), contentType: 'application/json' });
  await assertNoUnhandledCalls(page);
  capture.assertClean();
});

test('100 Feed to Plugins navigation cycles actually mount both views and release presentation work', async ({ page }) => {
  test.setTimeout(120_000);
  const capture = await installFakeHost(page, emptyState());
  await page.goto('/');
  await waitForFrontend(page);
  await expect(page.locator('.live-feed__stream')).toBeVisible();
  const read = () => page.evaluate(() => (window as unknown as {
    __tiktools_frontend_diagnostics__(): {
      ownedTimers: number; iframeCount: number; domNodes: number;
      live: { presentationActive: boolean; frames: number };
      control: { topicSubscriptions: number; pushSubscriptions: number; pending: number };
    };
  }).__tiktools_frontend_diagnostics__());
  await expect.poll(async () => (await read()).control.pending).toBe(0);
  // Warm the lazy route's development styles before measuring retained DOM.
  await page.getByRole('link', { name: t('navigation.plugins'), exact: true }).click();
  await expect(page.locator('.plugins-page')).toBeVisible();
  await page.getByRole('link', { name: t('navigation.feed'), exact: true }).click();
  await expect(page.locator('.live-feed__stream')).toBeVisible();
  await expect.poll(async () => (await read()).control.pending).toBe(0);
  const baseline = await read();
  for (let i = 0; i < 100; i += 1) {
    await page.evaluate(() => (window as unknown as {
      __tiktools_benchmark_controls__: { setTab(tab: string): void };
    }).__tiktools_benchmark_controls__.setTab('plugins'));
    await expect(page.locator('.plugins-page')).toBeVisible();
    await expect(page.locator('.live-feed__stream')).toHaveCount(0);
    // Visibility gating is retired: streams stay active on every tab so no chat history is skipped.
    expect((await read()).live.presentationActive).toBe(true);
    await page.evaluate(() => (window as unknown as {
      __tiktools_benchmark_controls__: { setTab(tab: string): void };
    }).__tiktools_benchmark_controls__.setTab('feed'));
    await expect(page.locator('.live-feed__stream')).toBeVisible();
    await expect(page.locator('.plugins-page')).toHaveCount(0);
  }
  await expect.poll(async () => (await read()).control.pending).toBe(0);
  const final = await read();
  expect(final.control.topicSubscriptions).toBe(baseline.control.topicSubscriptions);
  expect(final.control.pushSubscriptions).toBe(baseline.control.pushSubscriptions);
  expect(final.domNodes).toBe(baseline.domNodes);
  expect(final.ownedTimers).toBe(0);
  expect(final.iframeCount).toBe(0);
  expect(final.live.frames).toBe(0);
  await assertNoUnhandledCalls(page);
  capture.assertClean();
});
