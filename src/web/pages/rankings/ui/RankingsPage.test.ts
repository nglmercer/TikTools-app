import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { afterEach, describe, expect, it } from 'vitest';
import { createFakeControlClient, storePlugins } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import RankingsPage from './RankingsPage.vue';

enableAutoUnmount(afterEach);
const definition = { schemaVersion: 1, id: 'top', creatorId: 'creator', roomId: 'room', groupBy: 'user', window: { type: 'session' }, sources: [{ source: 'tiktok.like', metric: 'like.count', weight: 1, enabled: true }], limit: 10, capacity: 100000 };
const snapshot = { rankingId: 'top', schemaVersion: 1, creatorId: 'creator', roomId: 'room', revision: 1, integrityStatus: 'unknown', paused: false, entries: [{ key: '42', label: 'Alice', rank: 1, score: 5 }], totalScore: 5, totalParticipants: 1 };
async function setup(paused = false) {
  const { client, calls } = createFakeControlClient((method) => {
    if (method === 'automation.list') return { actions: [] };
    if (method === 'competitions.list') return { competitions: [] };
    if (method === 'rankings.list') return { rankings: [definition] };
    if (method === 'rankings.metrics') return { metrics: [{ source: 'tiktok.like', metric: 'like.count', description: 'Accepted likes', unit: 'likes' }, { source: 'tiktok.gift', metric: 'gift.diamonds', description: 'Diamonds', unit: 'diamonds' }] };
    if (method === 'rankings.snapshot') return { ...snapshot, paused };
    return { ok: true };
  });
  const wrapper = mountWithPlugins(RankingsPage, { global: { plugins: storePlugins(client) } });
  await flushPromises();
  return { wrapper, calls };
}
function button(wrapper: Awaited<ReturnType<typeof setup>>['wrapper'], name: string) {
  return wrapper.findAll('button').find((item) => item.text() === name)!;
}
describe('rankings management', () => {
  it('does not replace a pushed new instance with an older in-flight response', async () => {
    let resolveSnapshot!: (value: unknown) => void;
    let publish!: (data: unknown) => void;
    const { client } = createFakeControlClient((method) => {
      if (method === 'rankings.list') return { rankings: [definition] };
      if (method === 'rankings.metrics') return { metrics: [] };
      if (method === 'competitions.list') return { competitions: [] };
      if (method === 'automation.list') return { actions: [] };
      if (method === 'rankings.snapshot') return new Promise((resolve) => { resolveSnapshot = resolve; });
      return { ok: true };
    });
    client.onTopic = (topic, callback) => { if (topic === 'ranking.updated') publish = callback as (data: unknown) => void; return () => {}; };
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
    const visualization = wrapper.findAll('select').find((item) => item.find('option[value="donut"]').exists())!;
    await visualization.setValue('donut');
    const percentages = wrapper.findAll('label').find((label) => label.text().includes('Show percentages'))!;
    await percentages.get('input').setValue(true);
    await button(wrapper, 'Copy OBS URL').trigger('click');
    await flushPromises();
    expect(calls.find((call) => call.method === 'widgets.copyObsUrl')?.params).toEqual({ widget: 'ranking', rankingId: 'top', visualization: 'donut', showLabels: true, showPercentages: true, showLegend: true });
  });
  it('creates a backend definition from the metric controls', async () => {
    const { wrapper, calls } = await setup();
    await button(wrapper, 'New ranking').trigger('click');
    const inputs = wrapper.findAll('input');
    await inputs[0]!.setValue('new-rank');
    await inputs[1]!.setValue('creator');
    await inputs[2]!.setValue('room');
    await button(wrapper, 'Add metric').trigger('click');
    await wrapper.get('form').trigger('submit');
    await flushPromises();
    expect(calls.find((call) => call.method === 'rankings.create')?.params).toMatchObject({ id: 'new-rank', creatorId: 'creator', roomId: 'room', sources: [{ source: 'tiktok.like', metric: 'like.count', weight: 1, enabled: true }] });
  });
  it('edits a paused definition and preserves scores when reset is not selected', async () => {
    const { wrapper, calls } = await setup(true);
    await wrapper.get('[data-item="top"]').trigger('click');
    await flushPromises();
    await button(wrapper, 'Edit definition').trigger('click');
    const topEntries = wrapper.findAll('label').find((label) => label.text().startsWith('Top entries'))!;
    await topEntries.get('input').setValue(5);
    await wrapper.get('form').trigger('submit'); await flushPromises();
    expect(calls.find((call) => call.method === 'rankings.update')?.params).toEqual({ definition: { ...definition, limit: 5 }, reset: false });
  });
  it('configures bounded per-user contribution rules', async () => {
    const { wrapper, calls } = await setup();
    await button(wrapper, 'New ranking').trigger('click');
    const inputs = wrapper.findAll('input');
    await inputs[0]!.setValue('limited'); await inputs[1]!.setValue('creator'); await inputs[2]!.setValue('room');
    await button(wrapper, 'Add metric').trigger('click');
    await wrapper.findAll('label').find((label) => label.text().includes('Limit contributions per user'))!.get('input').setValue(true);
    await wrapper.findAll('label').find((label) => label.text().startsWith('Maximum contributions'))!.get('input').setValue(3);
    await wrapper.findAll('label').find((label) => label.text().startsWith('Cooldown per user'))!.get('input').setValue(500);
    await wrapper.get('form').trigger('submit'); await flushPromises();
    expect(calls.find((call) => call.method === 'rankings.create')?.params).toMatchObject({ userLimits: { maxContributions: 3, cooldownMs: 500, maxUsers: 10000 } });
  });
  it('configures per-gift multipliers on a gift source', async () => {
    const { wrapper, calls } = await setup();
    await button(wrapper, 'New ranking').trigger('click');
    const inputs = wrapper.findAll('input');
    await inputs[0]!.setValue('gifts'); await inputs[1]!.setValue('creator'); await inputs[2]!.setValue('room');
    const metric = wrapper.findAll('select').find((item) => item.find(`option[value='${JSON.stringify(['tiktok.gift', 'gift.diamonds'])}']`).exists())!;
    await metric.setValue(JSON.stringify(['tiktok.gift', 'gift.diamonds']));
    await button(wrapper, 'Add metric').trigger('click');
    await button(wrapper, 'Add gift override').trigger('click');
    await wrapper.find('.gift-rule label').get('input').setValue('42');
    await wrapper.find('.gift-rule input[type="number"]').setValue(3);
    await wrapper.get('form').trigger('submit'); await flushPromises();
    expect(calls.find((call) => call.method === 'rankings.create')?.params).toMatchObject({ sources: [{ source: 'tiktok.gift', giftMultipliers: { '42': 3 } }] });
  });
});
