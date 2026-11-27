import { testMessage } from '@/shared/i18n/test-message.ts';
import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { nextTick } from 'vue';
import type { ViewerRecord } from '../../../../../../src/shared/messages.ts';
import { useConnectionStore } from '@/entities/connection/model/connection-store.ts';
import { defaultPointsConfig } from '@/entities/points/model/default-config.ts';
import { usePointsStore } from '@/entities/points/model/points-store.ts';
import { createFakeControlClient, createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import type { ConnectionStatus } from '@/types.ts';
import PointsPage from '../../../../../../src/web/pages/points/ui/PointsPage.vue';
import { RPC, defined } from '../../../../../shared/index.ts';

enableAutoUnmount(afterEach);
const t = (key: string): string => testMessage('en', key);

beforeEach(() => {
  localStorage.clear();
});

function viewer(uniqueId: string, points: number): ViewerRecord {
  return {
    uniqueId,
    points,
    level: Math.floor(points / 100) + 1,
    isSubscriber: uniqueId === 'luna',
    totalChats: 0,
    totalCoins: 0,
    totalLikes: 0,
    totalShares: 0,
    firstSeen: 0,
    lastSeen: 0,
  };
}

async function mountPage(status: ConnectionStatus = 'idle') {
  const { client, calls } = createFakeControlClient((method, params) => {
    if (method === RPC.pointsConfigGet) return defaultPointsConfig;
    if (method === RPC.pointsConfigSet) return { ...defaultPointsConfig, ...params };
    if (method === RPC.pointsLeaderboard) return { viewers: [viewer('luna', 420), viewer('mango', 150)] };
    return {};
  });
  const stores = createStoreHarness(client);
  stores.use(() => useConnectionStore()).status = status;
  const wrapper = mountWithPlugins(PointsPage, { attachTo: document.body, global: { plugins: stores.plugins } });
  await wrapper.vm.$nextTick();
  const store = stores.use(() => usePointsStore());
  await store.refresh();
  await flushPromises();
  return { wrapper, calls, store };
}

function button(wrapper: Awaited<ReturnType<typeof mountPage>>['wrapper'], label: string) {
  return defined(wrapper.findAll('button').find((entry) => entry.text() === label));
}

function documentButton(label: string): HTMLButtonElement {
  return defined(Array.from(document.querySelectorAll<HTMLButtonElement>('button')).find((entry) => entry.textContent?.trim() === label));
}

describe('PointsPage', () => {
  it('lists the leaderboard with ranks, subscriber badges and the currency column', async () => {
    const { wrapper } = await mountPage();
    const rows = wrapper.findAll('.leaderboard-table tbody tr');
    expect(rows).toHaveLength(2);
    expect(defined(rows[0]).text()).toContain('#1');
    expect(defined(rows[0]).text()).toContain('@luna');
    expect(defined(rows[0]).text()).toContain(t('points.leaderboard.subscriber'));
    expect(wrapper.findAll('th').map((th) => th.text())).toContain(t('navigation.points'));
    expect(wrapper.text()).toContain('2 viewers');
  });

  it('saves edited rules and disables saving until something changes', async () => {
    const { wrapper, calls } = await mountPage();
    expect(button(wrapper, t('points.rules.save')).attributes('disabled')).toBeDefined();
    await wrapper.get('#points-currency').setValue('Gems');
    await button(wrapper, t('points.rules.save')).trigger('click');
    await flushPromises();
    expect(calls.find((call) => call.method === RPC.pointsConfigSet)?.params).toMatchObject({ currencyName: 'Gems' });
    expect(wrapper.findAll('th').map((th) => th.text())).toContain('Gems');
  });

  it.each<ConnectionStatus>(['connected', 'connecting', 'retrying'])('saves rules during %s without reconnecting', async (status) => {
    const { wrapper, calls } = await mountPage(status);
    expect(wrapper.get('#points-currency').attributes('disabled')).toBeUndefined();
    await wrapper.get('#points-currency').setValue('Live Gems');
    await button(wrapper, t('points.rules.save')).trigger('click');
    await flushPromises();
    expect(calls.filter((call) => call.method === RPC.pointsConfigSet)).toHaveLength(1);
    expect(calls.find((call) => call.method === RPC.pointsConfigSet)?.params).toMatchObject({ currencyName: 'Live Gems' });
    expect(wrapper.findAll('th').map((th) => th.text())).toContain('Live Gems');
    expect(calls.some((call) => call.method.startsWith('live.'))).toBe(false);
    expect(button(wrapper, t('points.actions.resetAll')).attributes('disabled')).toBeDefined();
  });

  it('reloads clean rules from the store and preserves unsaved edits', async () => {
    const { wrapper, store } = await mountPage('connected');
    store.pointsConfig = { ...store.pointsConfig, currencyName: t('feed.profile.stats.coins') };
    await nextTick();
    expect(wrapper.get<HTMLInputElement>('#points-currency').element.value).toBe(t('feed.profile.stats.coins'));
    await wrapper.get('#points-currency').setValue('Draft Gems');
    store.pointsConfig = { ...store.pointsConfig, currencyName: 'Stars' };
    await nextTick();
    expect(wrapper.get<HTMLInputElement>('#points-currency').element.value).toBe('Draft Gems');
    await button(wrapper, t('points.rules.discard')).trigger('click');
    expect(wrapper.get<HTMLInputElement>('#points-currency').element.value).toBe('Stars');
  });

  it('deducts points for a viewer from the row menu', async () => {
    const { wrapper, calls } = await mountPage();
    await wrapper.get('button[aria-label="Actions for @mango"]').trigger('click');
    await nextTick();
    defined(Array.from(document.querySelectorAll<HTMLElement>('[role="menuitem"]'))
      .find((item) => item.textContent?.includes(t('points.adjust.confirmDeduct'))))
      .click();
    await flushPromises();
    expect(document.querySelector('dialog h2')?.textContent).toContain('@mango');
    documentButton(t('points.adjust.confirmDeduct')).click();
    await flushPromises();
    expect(calls.at(-1)).toEqual({ method: RPC.pointsAdjust, params: { uniqueId: 'mango', delta: -50 } });
  });

  it('confirms before resetting every viewer', async () => {
    const { wrapper, calls } = await mountPage();
    await button(wrapper, t('points.actions.resetAll')).trigger('click');
    await flushPromises();
    expect(document.querySelector('dialog[open]')?.textContent).toContain('cannot be undone');
    documentButton(t('points.reset.confirm')).click();
    await flushPromises();
    expect(calls.some((call) => call.method === 'points.reset' && JSON.stringify(call.params) === '{}')).toBe(true);
  });

  it('filters the leaderboard from the search field', async () => {
    const { wrapper } = await mountPage();
    await wrapper.get('input[type="search"]').setValue('man');
    expect(wrapper.findAll('.leaderboard-table tbody tr').map((row) => row.text())).toEqual([expect.stringContaining('@mango')]);
  });
});
