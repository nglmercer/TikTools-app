import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { afterEach, describe, expect, it } from 'vitest';
import { activateLocale } from '@/shared/i18n/i18n.ts';
import { createFakeControlClient, storePlugins } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import CloudTeams from './CloudTeams.vue';

enableAutoUnmount(afterEach);
afterEach(async () => { await activateLocale('en'); });

const team = '3f2c9a1e-8b7d-4c6e-9a0b-1d2e3f4a5b6c';

describe('CloudTeams', () => {
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
