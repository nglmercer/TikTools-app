import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { afterEach, expect, it, vi } from 'vitest';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import { createFakeControlClient, storePlugins } from '@/shared/lib/testing/fake-control-client.ts';
import { useTeamsStore } from '@/entities/cloud-teams/model/teams-store.ts';
import TeamInvitations from './TeamInvitations.vue';

enableAutoUnmount(afterEach);
afterEach(() => vi.useRealTimers());
it('requires an unambiguous nickname result and binds the invitation to its account', async () => {
  vi.useFakeTimers();
  const { client, calls } = createFakeControlClient(method => {
    if (method === 'teams.people.search') return { items: [{ id: 'private-account', nickname: 'user', ambiguous: false }, { id: 'other-account', nickname: 'user2', ambiguous: true }] };
    if (method === 'teams.invitations.create') return { id: '3f2c9a1e-8b7d-4c6e-9a0b-1d2e3f4a5b6c', requestedRole: 'observer', expiresAt: 'later', token: 'x'.repeat(43) };
    if (method === 'teams.invitations.list') return { items: [] };
    throw new Error(`unexpected ${method}`);
  });
  const wrapper = mountWithPlugins(TeamInvitations, { attachTo: document.body, global: { plugins: storePlugins(client) } });
  const teams = useTeamsStore();
  teams.selectedId = '3f2c9a1e-8b7d-4c6e-9a0b-1d2e3f4a5b6c';
  await flushPromises();
  const create = () => wrapper.findAll('button').find(button => button.text() === 'Create invitation')!;
  expect(create().attributes('disabled')).toBeDefined();
  const input = wrapper.get('input[role="combobox"]');
  await input.setValue('use');
  await vi.advanceTimersByTimeAsync(350);
  await flushPromises();
  expect(calls.filter(call => call.method === 'teams.people.search')).toHaveLength(1);
  expect(create().attributes('disabled')).toBeDefined();
  const results = [...document.querySelectorAll<HTMLElement>('[role="option"]')];
  expect(results[1]?.getAttribute('aria-disabled')).toBe('true');
  expect(results[0]?.textContent).toContain('user');
  expect(results[0]?.textContent).not.toContain('private-account');
  results[0]!.click();
  await flushPromises();
  await create().trigger('click');
  await flushPromises();
  expect(calls.find(call => call.method === 'teams.invitations.create')?.params).toMatchObject({ recipientId: 'private-account' });
});

it('discards a delayed search when the selected team changes', async () => {
  vi.useFakeTimers();
  let finish!: (value: unknown) => void;
  const { client } = createFakeControlClient(method => {
    if (method === 'teams.people.search') return new Promise(resolve => { finish = resolve; });
    throw new Error(`unexpected ${method}`);
  });
  const wrapper = mountWithPlugins(TeamInvitations, { global: { plugins: storePlugins(client) } });
  await wrapper.get('input[role="combobox"]').setValue('use');
  await vi.advanceTimersByTimeAsync(350);
  useTeamsStore().selectedId = 'different-team';
  await flushPromises();
  finish({ items: [{ id: 'private-account', nickname: 'user', ambiguous: false }] });
  await flushPromises();
  expect(wrapper.findAll('button').find(button => button.text() === 'Create invitation')!.attributes('disabled')).toBeDefined();
});
