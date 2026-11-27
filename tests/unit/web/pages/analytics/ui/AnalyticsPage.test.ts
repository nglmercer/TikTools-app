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
import { defined } from '../../../../../shared/index.ts';

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
    sessionList: [],
    audience: withData ? { activeViewers: 34, newViewers: 12 } : { activeViewers: 0, newViewers: 0 },
  };
}

function hourRows(seed: Record<number, number>): AnalyticsSummaryData['hours'] {
  return Array.from({ length: 24 }, (_, hour) => ({
    hour,
    chats: seed[hour] ?? 0,
    giftEvents: 0,
    gifts: 0,
    diamonds: 0,
    likeEvents: 0,
    likes: 0,
    joins: 0,
    follows: 0,
    shares: 0,
    peakViewers: 0,
  }));
}

function sessionRows(): AnalyticsSummaryData['sessionList'] {
  return [
    {
      startedAt: Math.floor(Date.UTC(2026, 10, 23, 20, 0) / 1_000),
      endedAt: Math.floor(Date.UTC(2026, 10, 23, 22, 14) / 1_000),
      peakViewers: 4,
      chats: 192,
      giftEvents: 0,
      gifts: 0,
      diamonds: 0,
      likeEvents: 30,
      likes: 1005,
      joins: 23,
      follows: 0,
      shares: 0,
    },
  ];
}

async function mountPage(
  withData = true,
  hours: AnalyticsSummaryData['hours'] = [],
  sessionList: AnalyticsSummaryData['sessionList'] = [],
) {
  const { client, calls } = createFakeControlClient((_, params) => ({
    summary: { ...summaryFor(params, withData), hours, sessionList },
  }));
  const stores = createStoreHarness(client);
  stores.use(() => useConnectionStore()).activeCreator = 'luna';
  const wrapper = mountWithPlugins(AnalyticsPage, { attachTo: document.body, global: { plugins: stores.plugins } });
  await flushPromises();
  return { wrapper, calls };
}


async function openTab(
  wrapper: ReturnType<typeof mountWithPlugins>,
  label: 'overview' | 'contributors' | 'sessions',
): Promise<void> {
  const key = `analytics.tabs.${label}`;
  const tab = wrapper
    .findAll('[role="tab"]')
    .find((node: { text: () => string }) => node.text() === testMessage('en', key));
  expect(tab, `tab ${label}`).toBeTruthy();
  await defined(tab).trigger('click');
  await flushPromises();
}

describe('AnalyticsPage', () => {
  it('loads the last seven days for the active creator', async () => {
    const { calls } = await mountPage();
    const today = systemDay(Date.now());
    expect(calls[0]?.params).toMatchObject({ creatorUniqueId: 'luna', startDay: today - 6, endDay: today });
  });

  it('shows metric totals, insights and contributors', async () => {
    const { wrapper } = await mountPage(true, [], sessionRows());
    // Overview tab is active by default: KPIs and insights live there.
    const tiles = wrapper.findAll('.kpi-strip__card');
    expect(tiles).toHaveLength(5);
    expect(defined(tiles[0]).text()).toContain('Chats');
    expect(defined(tiles[0]).text()).toContain('1,520');
    expect(defined(tiles[1]).text()).toContain('Gifts');
    expect(defined(tiles[1]).text()).toContain('40');
    expect(defined(tiles[2]).text()).toContain('Likes');
    expect(defined(tiles[2]).text()).toContain('6,400');
    expect(defined(tiles[3]).text()).toContain('Diamonds');
    expect(defined(tiles[3]).text()).toContain('900');
    expect(defined(tiles[4]).text()).toContain(t('analytics.facts.peakViewers'));
    expect(defined(tiles[4]).text()).toContain('210');
    const sessionsRow = wrapper
      .findAll('.insights__row')
      .find((row: { text: () => string }) => row.text().includes(t('analytics.insights.sessions')));
    expect(sessionsRow?.text()).toContain('3');
    // The contributors table lives behind its own tab.
    await openTab(wrapper, 'contributors');
    expect(wrapper.findAll('tbody tr')).toHaveLength(2);
    // The retention split summarizes the audience above the ranking.
    expect(wrapper.text()).toContain('34');
    expect(wrapper.text()).toContain('12');
    expect(wrapper.text()).toContain('22');
  });

  it('switches the charted metric from the activity panel selector', async () => {
    const { wrapper } = await mountPage();
    expect(wrapper.get('svg[role="img"]').attributes('aria-label')).toMatch(/^Chats:/);
    // The metric segmented lives inside the activity panel; Diamonds follows the 'all' entry.
    const panel = wrapper.get('[data-guide="analytics-activity"]');
    await defined(panel.findAll('.base-segmented__option')[4]).trigger('click');
    expect(wrapper.get('svg[role="img"]').attributes('aria-label')).toMatch(/^Diamonds:/);
  });

  it('shows a value tooltip when hovering the daily chart', async () => {
    const { wrapper } = await mountPage();
    const svg = wrapper.get<SVGSVGElement>('svg[role="img"]');
    // happy-dom reports zero-size boxes; pretend the svg renders at viewBox size.
    svg.element.getBoundingClientRect = () => new DOMRect(0, 0, 720, 220);
    expect(wrapper.find('.chart-tooltip').exists()).toBe(false);
    await svg.trigger('pointermove', { clientX: 360, clientY: 100 });
    const tooltip = wrapper.get('.chart-tooltip');
    expect(tooltip.text()).toContain(t('analytics.metrics.chats'));
    // The guide line and hover dot appear while the pointer is over the chart…
    expect(svg.find('.daily-chart__guide').exists()).toBe(true);
    await svg.trigger('pointerleave');
    expect(wrapper.find('.chart-tooltip').exists()).toBe(false);
  });

  it('overlays every metric when the all option is selected', async () => {
    const { wrapper } = await mountPage();
    const panel = wrapper.get('[data-guide="analytics-activity"]');
    await defined(panel.findAll('.base-segmented__option')[0]).trigger('click');
    const chart = wrapper.get('svg[role="img"]');
    expect(chart.attributes('aria-label')).toMatch(new RegExp(`^${t('analytics.metrics.all')}:`));
    // One line per metric on the shared scale; the picker itself is the legend
    // (tinted icons), so no separate swatch row exists.
    expect(chart.findAll('.daily-chart__line')).toHaveLength(5);
    expect(chart.find('.daily-chart__previous').exists()).toBe(false);
    expect(panel.find('.activity-panel__legend').exists()).toBe(false);
    // Every metric option carries a tinted icon; 'all' has a neutral layers
    // glyph without a chart color.
    const options = panel.findAll('.base-segmented__option');
    expect(defined(options[1]).find('[style*="color"]').exists()).toBe(true);
    expect(defined(options[0]).find('.base-segmented__icon').exists()).toBe(true);
    expect(defined(options[0]).find('[style*="color"]').exists()).toBe(false);
  });

  it('requests a new span when the range changes', async () => {
    const { wrapper, calls } = await mountPage();
    const today = systemDay(Date.now());
    const rangeBar = wrapper.get('[data-guide="analytics-range"]');
    await defined(rangeBar.findAll('.base-segmented__option')[0]).trigger('click');
    await flushPromises();
    expect(calls.at(-2)?.params).toMatchObject({ startDay: today, endDay: today });
    expect(wrapper.text()).toContain(t('analytics.activity.today'));
  });

  it('shows the hour-of-day pattern for multi-day spans with hourly data', async () => {
    const { wrapper } = await mountPage(true, hourRows({ 21: 30, 22: 12 }));
    expect(wrapper.text()).toContain(t('analytics.activity.pattern'));
    expect(wrapper.text()).toContain(t('analytics.activity.patternHint'));
  });

  it('shows the weekday pattern for multi-day spans', async () => {
    const { wrapper } = await mountPage();
    expect(wrapper.text()).toContain(t('analytics.activity.weekdayPattern'));
    expect(wrapper.text()).toContain(t('analytics.activity.weekdayPatternHint'));
    expect(wrapper.find('.weekday-chart svg').exists()).toBe(true);
  });

  it('hides the hour-of-day pattern when no hourly data exists', async () => {
    const { wrapper } = await mountPage(true, hourRows({}));
    expect(wrapper.text()).not.toContain(t('analytics.activity.pattern'));
  });

  it('filters contributors from the panel search', async () => {
    const { wrapper } = await mountPage();
    await openTab(wrapper, 'contributors');
    await wrapper.get('input[type="search"]').setValue('man');
    expect(wrapper.findAll('tbody tr').map((row) => row.text())).toEqual([expect.stringContaining('@mango')]);
  });

  it('lists broadcasts with duration and engagement in the sessions tab', async () => {
    const { wrapper } = await mountPage(true, [], sessionRows());
    await openTab(wrapper, 'sessions');
    const rows = wrapper.findAll('tbody tr');
    expect(rows).toHaveLength(1);
    expect(defined(rows[0]).text()).toContain('192');
    expect(defined(rows[0]).text()).toContain('2h 14m');
  });

  it('explains when the range has no broadcasts', async () => {
    const { wrapper } = await mountPage();
    await openTab(wrapper, 'sessions');
    expect(wrapper.text()).toContain(t('analytics.sessions.empty'));
  });

  it('explains when there is no data', async () => {
    const { wrapper } = await mountPage(false);
    expect(wrapper.text()).toContain(t('analytics.empty.title'));
    expect(wrapper.find('.kpi-strip').exists()).toBe(false);
  });
});
