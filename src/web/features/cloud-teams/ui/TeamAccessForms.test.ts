import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { afterEach, expect, it } from 'vitest';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import { createFakeControlClient, storePlugins } from '@/shared/lib/testing/fake-control-client.ts';
import TeamAccessForms from './TeamAccessForms.vue';

enableAutoUnmount(afterEach);
it('searches approved owner nicknames and sends the internal team id only after selecting a result', async () => {
  const id = '3f2c9a1e-8b7d-4c6e-9a0b-1d2e3f4a5b6c';
  const { client, calls } = createFakeControlClient(method => {
    if (method === 'teams.accessible.list') return { items: [{ id, name: 'Stream crew', ownerNickname: 'user', role: 'observer' }] };
    if (method === 'teams.commands.run') return { commandId: 'test-command', status: 'completed', code: null, result: {} };
    throw new Error(`unexpected ${method}`);
  });
  const wrapper = mountWithPlugins(TeamAccessForms, { attachTo: document.body, global: { plugins: storePlugins(client) } });
  const send = () => wrapper.findAll('button').find(button => button.text() === 'Send')!;
  expect(send().attributes('disabled')).toBeDefined();
  await wrapper.findAll('button').find(button => button.text() === 'Load approved teams')!.trigger('click');
  await flushPromises();
  const search = wrapper.get('input[aria-label="Search nickname"]');
  await search.setValue('use');
  await search.trigger('focus');
  await flushPromises();
  expect(send().attributes('disabled')).toBeDefined();
  const result = document.querySelector('[role="option"]') as HTMLElement;
  expect(result.textContent).toContain('user');
  expect(result.textContent).not.toContain(id);
  result.click();
  await flushPromises();
  await send().trigger('click');
  await flushPromises();
  expect(calls.find(call => call.method === 'teams.commands.run')?.params).toEqual({ hostId: id, operation: 'live.status' });
  expect(wrapper.text()).not.toContain('Team id');
});
