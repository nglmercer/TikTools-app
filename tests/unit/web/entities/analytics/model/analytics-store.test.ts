import { describe, expect, it } from 'vitest';
import { createApp } from 'vue';
import { createFakeControlClient, storePlugins } from '@/shared/lib/testing/fake-control-client.ts';
import { useAnalyticsStore } from '../../../../../../src/web/entities/analytics/model/analytics-store.ts';

function setup(handler: Parameters<typeof createFakeControlClient>[0]) {
  const { client, calls } = createFakeControlClient(handler);
  const app = createApp({});
  for (const plugin of storePlugins(client)) app.use(plugin);
  return { store: app.runWithContext(() => useAnalyticsStore()), calls };
}

describe('useAnalyticsStore', () => {
  it('requests the span for the normalized creator', async () => {
    const summary = { creatorUniqueId: 'luna', startDay: 1, endDay: 7 };
    const { store, calls } = setup(() => ({ summary }));
    await store.load('@luna ', { startDay: 1, endDay: 7 });
    expect(calls[0]).toMatchObject({
      method: 'analytics.summary',
      params: { creatorUniqueId: 'luna', startDay: 1, endDay: 7, limit: 100, tzOffsetSecs: expect.any(Number) },
    });
    expect(store.summary).toEqual(summary);
    expect(calls[1]).toMatchObject({
      method: 'analytics.summary',
      params: { creatorUniqueId: 'luna', startDay: -6, endDay: 0 },
    });
    expect(store.loading).toBe(false);
  });

  it('keeps only the latest response', async () => {
    const resolvers: Array<(value: unknown) => void> = [];
    const { store } = setup(() => new Promise((resolve) => resolvers.push(resolve)));
    // Each load fetches the current span and the previous comparison span.
    const first = store.load('a', { startDay: 1, endDay: 1 });
    const second = store.load('a', { startDay: 2, endDay: 2 });
    resolvers[2]!({ summary: { startDay: 2 } });
    resolvers[3]!({ summary: null });
    await second;
    resolvers[0]!({ summary: { startDay: 1 } });
    resolvers[1]!({ summary: null });
    await first;
    expect(store.summary).toEqual({ startDay: 2 });
  });

  it('reports failures', async () => {
    const { store } = setup(() => {
      throw new Error('host down');
    });
    await store.load('a', { startDay: 1, endDay: 1 });
    expect(store.error).toBe('host down');
    expect(store.loading).toBe(false);
  });
});
