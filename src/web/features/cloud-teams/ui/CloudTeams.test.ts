import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { afterEach, describe, expect, it } from 'vitest';
import { activateLocale } from '@/shared/i18n/i18n.ts';
import { createFakeControlClient, storePlugins } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import GuideButton from '@/features/guided-help/ui/GuideButton.vue';
import CloudTeams from './CloudTeams.vue';
import TeamAccessForms from './TeamAccessForms.vue';

enableAutoUnmount(afterEach);
afterEach(async () => { await activateLocale('en'); });

const team = '3f2c9a1e-8b7d-4c6e-9a0b-1d2e3f4a5b6c';

describe('CloudTeams', () => {
  it('separates owned teams from joining and controller setup', async () => {
    const { client } = createFakeControlClient(() => ({ items: [] }));
    const wrapper = mountWithPlugins(CloudTeams, { global: { plugins: storePlugins(client) } });
    await flushPromises();
    expect(wrapper.findComponent(TeamAccessForms).exists()).toBe(false);
    await wrapper.findAll('[role="tab"]').find(tab => tab.text() === 'Join a team')!.trigger('click');
    expect(wrapper.findComponent(TeamAccessForms).exists()).toBe(true);
    expect(wrapper.find('[data-guide="createTeam"]').exists()).toBe(false);
    await wrapper.findAll('[role="tab"]').find(tab => tab.text() === 'My teams')!.trigger('click');
    expect(wrapper.find('[data-guide="createTeam"]').exists()).toBe(true);
  });

  it('lists teams and shows the team detail sections once a team is selected', async () => {
    const { client } = createFakeControlClient((method) => {
      if (method === 'teams.hosts.list') return { items: [{ id: team, name: 'Stream crew', status: 'active', protocolVersion: 1, revision: 2, online: true }] };
      if (method === 'teams.devices.list') return { items: [] };
      return { items: [] };
    });
    const wrapper = mountWithPlugins(CloudTeams, { global: { plugins: storePlugins(client) } });
    await flushPromises();
    expect(wrapper.text()).toContain('Stream crew');
    const teamButton = wrapper.findAll('button').find((node) => node.text().startsWith('Stream crew'))!;
    await teamButton.trigger('click');
    await flushPromises();
    const text = wrapper.text();
    expect(text).toContain('Invitations');
    expect(text).toContain('Join requests');
    expect(text).toContain('Grants');
    expect(text).toContain('Open sessions');
  });
});


it('restores the session before loading teams and offers a contextual setup guide', async () => {
  let restored = false;
  const { client, calls } = createFakeControlClient((method) => {
    if (method === 'cloud.status') return { state: 'signed-out', origin: 'https://cloud.example', persistence: 'keychain' };
    if (method === 'cloud.restore') { restored = true; return { state: 'signed-in', accountId: 'tester', origin: 'https://cloud.example', persistence: 'keychain' }; }
    if (method === 'teams.hosts.list' || method === 'teams.devices.list') {
      expect(restored).toBe(true);
      return { items: [] };
    }
    if (method === 'teams.host.status') return { state: 'stopped', hostId: null, lastError: null };
    return { items: [] };
  });
  const wrapper = mountWithPlugins(CloudTeams, { global: { plugins: storePlugins(client) } });
  await flushPromises();
  expect(calls.findIndex(call => call.method === 'cloud.restore')).toBeLessThan(calls.findIndex(call => call.method === 'teams.hosts.list'));
  expect(wrapper.getComponent(GuideButton).props('steps')[0]?.description).toBe('Create or select a team below.');
  expect(wrapper.get('[aria-label="Show guide"]').text()).toBe('?');
});
