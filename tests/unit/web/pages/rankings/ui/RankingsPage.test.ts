import { testMessage } from '@/shared/i18n/test-message.ts';
import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { afterEach, describe, expect, it } from 'vitest';
import { createFakeControlClient, storePlugins } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import RankingsPage from '../../../../../../src/web/pages/rankings/ui/RankingsPage.vue';
import { EVENT_KIND, RPC, TOPIC, defined } from '../../../../../shared/index.ts';

enableAutoUnmount(afterEach);
const t = (key: string): string => testMessage('en', key);
const definition = {
  schemaVersion: 1,
  id: 'top',
  creatorId: 'creator',
  roomId: 'room',
  groupBy: 'user',
  window: { type: 'session' },
  sources: [{ source: EVENT_KIND.tiktokLike, metric: 'like.count', weight: 1, enabled: true }],
  limit: 10,
  capacity: 100000,
};
const snapshot = {
  rankingId: 'top',
  schemaVersion: 1,
  creatorId: 'creator',
  roomId: 'room',
  revision: 1,
  integrityStatus: 'unknown',
  paused: false,
  entries: [{ key: '42', label: 'Alice', rank: 1, score: 5 }],
  totalScore: 5,
  totalParticipants: 1,
};
async function setup(paused = false) {
  const { client, calls } = createFakeControlClient((method) => {
    if (method === RPC.automationList) return { actions: [] };
    if (method === RPC.competitionsList) return { competitions: [] };
    if (method === RPC.rankingsList) return { rankings: [definition] };
    if (method === 'rankings.metrics') return {
      metrics: [
        { source: EVENT_KIND.tiktokLike, metric: 'like.count', description: 'Accepted likes', unit: 'likes' },
        { source: EVENT_KIND.tiktokGift, metric: 'gift.diamonds', description: 'Diamonds', unit: 'diamonds' },
      ],
    };
    if (method === RPC.rankingsSnapshot) return { ...snapshot, paused };
    return { ok: true };
  });
  const wrapper = mountWithPlugins(RankingsPage, { global: { plugins: storePlugins(client) } });
  await flushPromises();
  return { wrapper, calls };
}
function button(wrapper: Awaited<ReturnType<typeof setup>>['wrapper'], name: string) {
  return defined(wrapper.findAll('button').find((item) => item.text() === name));
}
describe('rankings management', () => {
  it('does not replace a pushed new instance with an older in-flight response', async () => {
    let resolveSnapshot!: (value: unknown) => void;
    let publish!: (data: unknown) => void;
    const { client } = createFakeControlClient((method) => {
      if (method === RPC.rankingsList) return { rankings: [definition] };
      if (method === 'rankings.metrics') return { metrics: [] };
      if (method === RPC.competitionsList) return { competitions: [] };
      if (method === RPC.automationList) return { actions: [] };
      if (method === RPC.rankingsSnapshot) return new Promise((resolve) => { resolveSnapshot = resolve; });
      return { ok: true };
    });
    client.onTopic = <T>(topic: string, callback: (data: T) => void) => {
      if (topic === TOPIC.rankingUpdated) publish = (data) => Reflect.apply(callback, undefined, [data]);
      return () => {};
    };
    const wrapper = mountWithPlugins(RankingsPage, { global: { plugins: storePlugins(client) } });
    await flushPromises();
    await wrapper.get('[data-item="top"]').trigger('click');
    publish({ snapshot: { ...snapshot, instanceId: 'b'.repeat(32), entries: [{ key: '43', label: 'Bob', rank: 1, score: 5 }] } });
    resolveSnapshot({ ...snapshot, instanceId: 'a'.repeat(32), revision: 20 });
    await flushPromises();
    expect(wrapper.text()).toContain('Bob — 5');
    expect(wrapper.text()).not.toContain('Alice — 5');
  });
  it('loads an authoritative snapshot and requests a scoped OBS URL', async () => {
    const { wrapper, calls } = await setup();
    await wrapper.get('[data-item="top"]').trigger('click');
    await flushPromises();
    expect(wrapper.text()).toContain('Alice — 5');
    const visualization = defined(wrapper.findAll('select').find((item) => item.find('option[value="donut"]').exists()));
    await visualization.setValue('donut');
    const percentages = defined(wrapper.findAll('label').find((label) => label.text().includes(t('rankingsPage.detail.showPercentages'))));
    await percentages.get('input').setValue(true);
    await button(wrapper, t('rankingsPage.detail.copyUrl')).trigger('click');
    await flushPromises();
    expect(calls.find((call) => call.method === 'widgets.copyObsUrl')?.params).toEqual({
      widget: 'ranking',
      rankingId: 'top',
      visualization: 'donut',
      showLabels: true,
      showPercentages: true,
      showLegend: true,
    });
  });
  it('creates a backend definition from the metric controls', async () => {
    const { wrapper, calls } = await setup();
    await button(wrapper, t('rankingsPage.list.new')).trigger('click');
    const inputs = wrapper.findAll('input');
    await defined(inputs[0]).setValue('new-rank');
    await defined(inputs[1]).setValue('creator');
    await defined(inputs[2]).setValue('room');
    await button(wrapper, t('rankingsPage.form.addMetric')).trigger('click');
    await wrapper.get('form').trigger('submit');
    await flushPromises();
    expect(calls.find((call) => call.method === RPC.rankingsCreate)?.params).toMatchObject({
      id: 'new-rank',
      creatorId: 'creator',
      roomId: 'room',
      sources: [{ source: EVENT_KIND.tiktokLike, metric: 'like.count', weight: 1, enabled: true }],
    });
  });
  it('edits a paused definition and preserves scores when reset is not selected', async () => {
    const { wrapper, calls } = await setup(true);
    await wrapper.get('[data-item="top"]').trigger('click');
    await flushPromises();
    await button(wrapper, t('rankingsPage.detail.edit')).trigger('click');
    const topEntries = defined(wrapper.findAll('label').find((label) => label.text().startsWith(t('rankingsPage.form.topEntries'))));
    await wrapper.get(`input[id="${topEntries.attributes('for')}"]`).setValue(5);
    await wrapper.get('form').trigger('submit');
    await flushPromises();
    expect(calls.find((call) => call.method === 'rankings.update')?.params).toEqual({ definition: { ...definition, limit: 5 }, reset: false });
  });
  it('configures bounded per-user contribution rules', async () => {
    const { wrapper, calls } = await setup();
    await button(wrapper, t('rankingsPage.list.new')).trigger('click');
    const inputs = wrapper.findAll('input');
    await defined(inputs[0]).setValue('limited');
    await defined(inputs[1]).setValue('creator');
    await defined(inputs[2]).setValue('room');
    await button(wrapper, t('rankingsPage.form.addMetric')).trigger('click');
    await defined(wrapper.findAll('label').find((label) => label.text().includes(t('rankingsPage.form.limitUsers')))).get('input').setValue(true);
    const maximum = defined(wrapper.findAll('label').find((label) => label.text().startsWith('Maximum contributions')));
    await wrapper.get(`input[id="${maximum.attributes('for')}"]`).setValue(3);
    const cooldown = defined(wrapper.findAll('label').find((label) => label.text().startsWith('Cooldown per user')));
    await wrapper.get(`input[id="${cooldown.attributes('for')}"]`).setValue(500);
    await wrapper.get('form').trigger('submit');
    await flushPromises();
    expect(calls.find((call) => call.method === RPC.rankingsCreate)?.params).toMatchObject({
      userLimits: { maxContributions: 3, cooldownMs: 500, maxUsers: 10000 },
    });
  });
  it('configures per-gift multipliers on a gift source', async () => {
    const { wrapper, calls } = await setup();
    await button(wrapper, t('rankingsPage.list.new')).trigger('click');
    const inputs = wrapper.findAll('input');
    await defined(inputs[0]).setValue('gifts');
    await defined(inputs[1]).setValue('creator');
    await defined(inputs[2]).setValue('room');
    const metric = defined(wrapper.findAll('select').find((item) => item.find(`option[value='${JSON.stringify(['tiktok.gift', 'gift.diamonds'])}']`).exists()));
    await metric.setValue(JSON.stringify([EVENT_KIND.tiktokGift, 'gift.diamonds']));
    await button(wrapper, t('rankingsPage.form.addMetric')).trigger('click');
    await button(wrapper, t('rankingsPage.form.addGiftOverride')).trigger('click');
    const giftId = wrapper.get('.gift-rule label').attributes('for');
    await wrapper.get(`input[id="${giftId}"]`).setValue('42');
    await wrapper.find('.gift-rule input[type="number"]').setValue(3);
    await wrapper.get('form').trigger('submit');
    await flushPromises();
    expect(calls.find((call) => call.method === RPC.rankingsCreate)?.params).toMatchObject({
      sources: [{ source: EVENT_KIND.tiktokGift, giftMultipliers: { '42': 3 } }],
    });
  });
});
