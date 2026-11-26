import { testMessage } from '@/shared/i18n/test-message.ts';
import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import type { AnalyticsSummaryData, AnalyticsTotals } from '../../../../../../src/shared/messages.ts';
import { useConnectionStore } from '@/entities/connection/model/connection-store.ts';
import { systemDay } from '@/entities/analytics/lib/analytics-range.ts';
import type { RpcParams } from '@/shared/api/control-client.ts';
import { createFakeControlClient, createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import AnalyticsPage from '../../../../../../src/web/pages/analytics/ui/AnalyticsPage.vue';

enableAutoUnmount(afterEach);
const t = (key: string): string => testMessage('en', key);

beforeEach(() => {
  localStorage.clear();
});

const totals: AnalyticsTotals = {
  chats: 1520,
  giftEvents: 12,
  gifts: 40,
  diamonds: 900,
  likeEvents: 30,
  likes: 6400,
  joins: 80,
  follows: 9,
  shares: 4,
  peakViewers: 210,
};

function summaryFor(params: RpcParams | undefined, withData = true): AnalyticsSummaryData {
  const startDay = Number(params?.startDay);
  const endDay = Number(params?.endDay);
  return {
    creatorUniqueId: 'luna',
    startDay,
    endDay,
    totals: withData ? totals : { ...totals, ...Object.fromEntries(Object.keys(totals).map((key) => [key, 0])) },
    days: withData ? [{ day: endDay, ...totals }] : [],
    topViewers: withData
      ? [
          { uniqueId: 'pixelpanda', chats: 40, gifts: 2, diamonds: 50, likes: 0, shares: 0, interactions: 42, lastSeen: 0 },
          { uniqueId: 'mango', chats: 12, gifts: 9, diamonds: 400, likes: 0, shares: 0, interactions: 21, lastSeen: 0 },
        ]
      : [],
    sessions: withData ? 3 : 0,
    hours: [],
  };
}

async function mountPage(withData = true) {
  const { client, calls } = createFakeControlClient((_, params) => ({ summary: summaryFor(params, withData) }));
  const stores = createStoreHarness(client);
  stores.use(() => useConnectionStore()).activeCreator = 'luna';
  const wrapper = mountWithPlugins(AnalyticsPage, { attachTo: document.body, global: { plugins: stores.plugins } });
  await flushPromises();
  return { wrapper, calls };
}

describe('AnalyticsPage', () => {
  it('loads the last seven days for the active creator', async () => {
    const { calls } = await mountPage();
    const today = systemDay(Date.now());
    expect(calls[0]?.params).toMatchObject({ creatorUniqueId: 'luna', startDay: today - 6, endDay: today });
  });

  it('shows metric totals, insights and contributors', async () => {
    const { wrapper } = await mountPage();
    const tiles = wrapper.findAll('.kpi-strip__card');
    expect(tiles).toHaveLength(5);
    expect(tiles[0]!.text()).toContain('Chats');
    expect(tiles[0]!.text()).toContain('1,520');
    expect(tiles[1]!.text()).toContain('Gifts');
    expect(tiles[1]!.text()).toContain('40');
    expect(tiles[2]!.text()).toContain('Likes');
    expect(tiles[2]!.text()).toContain('6,400');
    expect(tiles[3]!.text()).toContain('Diamonds');
    expect(tiles[3]!.text()).toContain('900');
    expect(tiles[4]!.text()).toContain(t('analytics.facts.peakViewers'));
    expect(tiles[4]!.text()).toContain('210');
    expect(wrapper.text()).toContain('3 sessions');
    expect(wrapper.findAll('tbody tr')).toHaveLength(2);
  });

  it('switches the charted metric from the activity panel selector', async () => {
    const { wrapper } = await mountPage();
    expect(wrapper.get('svg[role="img"]').attributes('aria-label')).toMatch(/^Chats:/);
    // The metric segmented lives inside the activity panel; Diamonds is its 4th option.
    const panel = wrapper.get('[data-guide="analytics-activity"]');
    await panel.findAll('.base-segmented__option')[3]!.trigger('click');
    expect(wrapper.get('svg[role="img"]').attributes('aria-label')).toMatch(/^Diamonds:/);
  });

  it('requests a new span when the range changes', async () => {
    const { wrapper, calls } = await mountPage();
    const today = systemDay(Date.now());
    const rangeBar = wrapper.get('[data-guide="analytics-range"]');
    await rangeBar.findAll('.base-segmented__option')[0]!.trigger('click');
    await flushPromises();
    expect(calls.at(-2)?.params).toMatchObject({ startDay: today, endDay: today });
    expect(wrapper.text()).toContain(t('analytics.activity.today'));
  });

  it('filters contributors from the panel search', async () => {
    const { wrapper } = await mountPage();
    await wrapper.get('input[type="search"]').setValue('man');
    expect(wrapper.findAll('tbody tr').map((row) => row.text())).toEqual([expect.stringContaining('@mango')]);
  });

  it('explains when there is no data', async () => {
    const { wrapper } = await mountPage(false);
    expect(wrapper.text()).toContain(t('analytics.empty.title'));
    expect(wrapper.find('.kpi-strip').exists()).toBe(false);
  });
});
