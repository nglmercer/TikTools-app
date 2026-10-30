import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { afterEach, describe, expect, it } from 'vitest';
import { createFakeControlClient, storePlugins } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import type { RankingDefinition } from '@/entities/ranking/types.ts';
import CompetitionPanel from './CompetitionPanel.vue';

enableAutoUnmount(afterEach);
const ranking: RankingDefinition = { schemaVersion: 1, id: 'vote', creatorId: 'c', roomId: 'r', groupBy: 'option', window: { type: 'session' }, sources: [{ source: 'competition.vote', metric: 'event.count', weight: 1, enabled: true }], limit: 10, capacity: 100 };
const poll = { schemaVersion: 1, id: 'poll', name: 'Poll', kind: 'poll', rankingId: 'vote', options: [], selectionMode: 'highest_score', tiePolicy: 'stable', durationMs: null, voteLimit: 1, allowSwitch: false, minimumParticipants: 1 };
async function setup() {
  const { client, calls } = createFakeControlClient((method) => {
    if (method === 'automation.list') return { actions: [{ id: 'tnt', name: 'TNT', enabled: true }] };
    if (method === 'competitions.list') return { competitions: [poll] };
    if (method === 'competitions.state') return { lifecycle: 'draft', generation: 0, deadlineMs: null, remainingMs: null };
    if (method === 'competitions.results') return { results: [] };
    return { ok: true };
  });
  const wrapper = mountWithPlugins(CompetitionPanel, { props: { rankings: [ranking] }, global: { plugins: storePlugins(client) } });
  await flushPromises();
  return { wrapper, calls };
}
async function openForm(wrapper: Awaited<ReturnType<typeof setup>>['wrapper'], id: string, name: string) {
  await wrapper.findAll('button').find((item) => item.text() === 'New competition')!.trigger('click');
  await wrapper.get('[data-field="id"]').setValue(id);
  await wrapper.get('[data-field="name"]').setValue(name);
}
describe('competition controls', () => {
  it('offers mixed like and share rankings but excludes unsupported sources', async () => {
    const { client } = createFakeControlClient((method) => {
      if (method === 'automation.list') return { actions: [] };
      if (method === 'competitions.list') return { competitions: [] };
      return { ok: true };
    });
    const mixed: RankingDefinition = { ...ranking, id: 'mixed', sources: [...ranking.sources,
      { source: 'tiktok.like', metric: 'like.count', weight: 2, enabled: true },
      { source: 'tiktok.share', metric: 'share.count', weight: 3, enabled: true }] };
    const gift: RankingDefinition = { ...mixed, id: 'unsupported', sources: [...mixed.sources,
      { source: 'tiktok.follow', metric: 'event.count', weight: 1, enabled: true }] };
    const wrapper = mountWithPlugins(CompetitionPanel, { props: { rankings: [mixed, gift] }, global: { plugins: storePlugins(client) } });
    await flushPromises();
    await wrapper.findAll('button').find((item) => item.text() === 'New competition')!.trigger('click');
    expect(wrapper.find('option[value="mixed"]').exists()).toBe(true);
    expect(wrapper.find('option[value="unsupported"]').exists()).toBe(false);
  });
  it('creates a command poll with explicit option routing', async () => {
    const { wrapper, calls } = await setup();
    await openForm(wrapper, 'poll', 'Poll');
    await wrapper.findAll('select').find((item) => item.find('option[value="vote"]').exists())!.setValue('vote');
    const optionFields = wrapper.findAll('fieldset input');
    await optionFields[0]!.setValue('a'); await optionFields[1]!.setValue('A'); await optionFields[2]!.setValue('!a');
    await optionFields[5]!.setValue('b'); await optionFields[6]!.setValue('B'); await optionFields[7]!.setValue('!b');
    await optionFields[4]!.setValue('123, 456');
    await wrapper.find('fieldset select').setValue('tnt');
    await wrapper.get('form').trigger('submit');
    await flushPromises();
    expect(calls.find((call) => call.method === 'competitions.create')?.params).toMatchObject({ id: 'poll', rankingId: 'vote', kind: 'poll', options: [{ id: 'a', commands: ['!a'], giftIds: ['123', '456'], onWin: { actionId: 'tnt', inputs: {} } }, { id: 'b', commands: ['!b'], onWin: null }] });
  });
  it('rejects ambiguous gift bindings before sending a definition', async () => {
    const { wrapper, calls } = await setup();
    await openForm(wrapper, 'poll', 'Poll');
    await wrapper.findAll('select').find((item) => item.find('option[value="vote"]').exists())!.setValue('vote');
    const items = wrapper.findAll('fieldset');
    for (const [index, item] of items.entries()) {
      const values = item.findAll('input');
      await values[0]!.setValue(`option${index}`); await values[1]!.setValue(`Option ${index}`); await values[2]!.setValue(`!option${index}`); await values[4]!.setValue('123');
    }
    await wrapper.get('form').trigger('submit'); await flushPromises();
    expect(wrapper.text()).toContain('unique options, commands, gift IDs');
    expect(calls.some((call) => call.method === 'competitions.create')).toBe(false);
  });
  it('creates a team battle with the same durable option ranking', async () => {
    const { wrapper, calls } = await setup();
    await openForm(wrapper, 'battle', 'Battle');
    await wrapper.get('[data-field="type"]').setValue('team_battle');
    await wrapper.findAll('select').find((item) => item.find('option[value="vote"]').exists())!.setValue('vote');
    const optionFields = wrapper.findAll('fieldset input');
    await optionFields[0]!.setValue('red'); await optionFields[1]!.setValue('Red'); await optionFields[2]!.setValue('!red');
    await optionFields[5]!.setValue('blue'); await optionFields[6]!.setValue('Blue'); await optionFields[7]!.setValue('!blue');
    await wrapper.get('form').trigger('submit'); await flushPromises();
    expect(calls.find((call) => call.method === 'competitions.create')?.params).toMatchObject({ id: 'battle', kind: 'team_battle', rankingId: 'vote' });
  });
  it('creates an option roulette using the selected winner policy', async () => {
    const { wrapper, calls } = await setup();
    await openForm(wrapper, 'wheel', 'Wheel');
    await wrapper.get('[data-field="type"]').setValue('roulette');
    await wrapper.findAll('select').find((item) => item.find('option[value="vote"]').exists())!.setValue('vote');
    const optionFields = wrapper.findAll('fieldset input');
    await optionFields[0]!.setValue('a'); await optionFields[1]!.setValue('A'); await optionFields[2]!.setValue('!a');
    await optionFields[5]!.setValue('b'); await optionFields[6]!.setValue('B'); await optionFields[7]!.setValue('!b');
    await wrapper.get('form').trigger('submit'); await flushPromises();
    expect(calls.find((call) => call.method === 'competitions.create')?.params).toMatchObject({ id: 'wheel', kind: 'roulette', rankingId: 'vote' });
  });
  it('edits a draft without losing additional commands or action inputs', async () => {
    const draft = { ...poll, options: [{ id: 'a', label: 'A', commands: ['!a', '!alpha'], giftIds: [], imageUrl: null, onWin: { actionId: 'tnt', inputs: { amount: 20 } } }, { id: 'b', label: 'B', commands: ['!b'], giftIds: [], imageUrl: null, onWin: null }] };
    const { client, calls } = createFakeControlClient((method) => {
      if (method === 'automation.list') return { actions: [{ id: 'tnt', name: 'TNT', enabled: true }] };
      if (method === 'competitions.list') return { competitions: [draft] };
      if (method === 'competitions.state') return { lifecycle: 'draft', generation: 0, deadlineMs: null, remainingMs: null };
      if (method === 'competitions.results') return { results: [] };
      return { ok: true };
    });
    const wrapper = mountWithPlugins(CompetitionPanel, { props: { rankings: [ranking] }, global: { plugins: storePlugins(client) } });
    await flushPromises();
    await wrapper.get('[data-item="poll"]').trigger('click');
    await flushPromises();
    await wrapper.findAll('button').find((item) => item.text() === 'Edit competition')!.trigger('click');
    await wrapper.get('form').trigger('submit'); await flushPromises();
    expect(calls.find((call) => call.method === 'competitions.update')?.params).toMatchObject({ id: 'poll', options: [{ commands: ['!a', '!alpha'], onWin: { actionId: 'tnt', inputs: { amount: 20 } } }, { commands: ['!b'] }] });
  });
  it('configures typed winner action inputs', async () => {
    const { wrapper, calls } = await setup();
    await openForm(wrapper, 'poll', 'Poll');
    await wrapper.findAll('select').find((item) => item.find('option[value="vote"]').exists())!.setValue('vote');
    const optionFields = wrapper.findAll('fieldset input');
    await optionFields[0]!.setValue('a'); await optionFields[1]!.setValue('A'); await optionFields[2]!.setValue('!a');
    await optionFields[5]!.setValue('b'); await optionFields[6]!.setValue('B'); await optionFields[7]!.setValue('!b');
    const first = wrapper.find('fieldset');
    await first.get('select').setValue('tnt');
    await first.findAll('button').find((item) => item.text() === 'Add action input')!.trigger('click');
    const row = first.get('.action-input');
    await row.findAll('input')[0]!.setValue('amount');
    await row.get('select').setValue('number');
    await row.findAll('input')[1]!.setValue('20');
    await wrapper.get('form').trigger('submit'); await flushPromises();
    expect(calls.find((call) => call.method === 'competitions.create')?.params).toMatchObject({ options: [{ onWin: { actionId: 'tnt', inputs: { amount: 20 } } }, { onWin: null }] });
  });
  it('selects a tied leader with the current competition generation', async () => {
    const { client, calls } = createFakeControlClient((method) => {
      if (method === 'automation.list') return { actions: [] };
      if (method === 'competitions.list') return { competitions: [{ ...poll, tiePolicy: 'manual' }] };
      if (method === 'competitions.state') return { lifecycle: 'finalizing', generation: 3, deadlineMs: null, remainingMs: null };
      if (method === 'competitions.results') return { results: [] };
      if (method === 'competitions.action') return { action: null };
      if (method === 'competitions.candidates') return { generation: 3, candidates: [{ key: 'a', label: 'Alpha', score: 5 }, { key: 'b', label: 'Beta', score: 5 }] };
      return { ok: true };
    });
    const wrapper = mountWithPlugins(CompetitionPanel, { props: { rankings: [ranking] }, global: { plugins: storePlugins(client) } });
    await flushPromises();
    await wrapper.get('[data-item="poll"]').trigger('click'); await flushPromises();
    const candidate = wrapper.findAll('select').find((item) => item.find('option[value="b"]').exists())!;
    await candidate.setValue('b');
    await wrapper.findAll('button').find((item) => item.text() === 'Select winner')!.trigger('click'); await flushPromises();
    expect(calls.find((call) => call.method === 'competitions.selectWinner')?.params).toEqual({ id: 'poll', generation: 3, winnerId: 'b' });
  });
  it('records a manual outcome for an uncertain action without rerunning it', async () => {
    const { client, calls } = createFakeControlClient((method) => {
      if (method === 'automation.list') return { actions: [] };
      if (method === 'competitions.list') return { competitions: [poll] };
      if (method === 'competitions.state') return { lifecycle: 'finished', generation: 1, deadlineMs: null, remainingMs: null };
      if (method === 'competitions.results') return { results: [] };
      if (method === 'competitions.action') return { action: { generation: 1, actionId: 'tnt', status: 'uncertain', reason: 'execution_failed' } };
      return { ok: true };
    });
    const wrapper = mountWithPlugins(CompetitionPanel, { props: { rankings: [ranking] }, global: { plugins: storePlugins(client) } });
    await flushPromises();
    await wrapper.get('[data-item="poll"]').trigger('click'); await flushPromises();
    expect(wrapper.text()).toContain('execution_failed');
    await wrapper.findAll('button').find((item) => item.text() === 'Mark completed')!.trigger('click'); await flushPromises();
    expect(calls.find((call) => call.method === 'competitions.resolveAction')?.params).toEqual({ id: 'poll', generation: 1, resolution: 'completed' });
    expect(calls.some((call) => call.method === 'competitions.finish')).toBe(false);
  });
  it('reads lifecycle and starts through the control API', async () => {
    const { wrapper, calls } = await setup();
    await wrapper.get('[data-item="poll"]').trigger('click'); await flushPromises();
    expect(wrapper.text()).toContain('Draft');
    await wrapper.findAll('button').find((item) => item.text() === 'Copy winner reveal URL')!.trigger('click'); await flushPromises();
    expect(calls.find((call) => call.method === 'widgets.copyObsUrl')?.params).toEqual({ widget: 'ranking', rankingId: 'vote', visualization: 'roulette', competitionId: 'poll' });
    await wrapper.findAll('button').find((item) => item.text() === 'Start')!.trigger('click'); await flushPromises();
    expect(calls.find((call) => call.method === 'competitions.start')?.params).toEqual({ id: 'poll' });
  });
});
