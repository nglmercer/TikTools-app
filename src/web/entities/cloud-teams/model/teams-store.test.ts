import { flushPromises } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import { createFakeControlClient, createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import { useAccountStore } from '@/entities/cloud-account/model/account-store.ts';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import { useTeamsStore } from './teams-store.ts';

const team = '3f2c9a1e-8b7d-4c6e-9a0b-1d2e3f4a5b6c';
const other = '4a3d0b2f-9c8e-4d7f-8b1c-2e3f4a5b6c7d';
const grantId = '5b4e1c3a-0d9f-4e8a-9c2d-3f4a5b6c7d8e';
const token = 'T'.repeat(43);
const hostRow = (id: string, revision = 1) => ({ id, name: 'Team', status: 'active', protocolVersion: 1, revision, online: false });

function host(announced: string[], handler: (method: string, params: Record<string, unknown> | undefined) => unknown = () => ({ items: [] })) {
  const { client, calls } = createFakeControlClient((method, params) => {
    if (method === 'rpc.discover') return { methods: announced.map((name) => ({ name })) };
    const result = handler(method, params as Record<string, unknown> | undefined);
    if (method === 'teams.host.status' && result && typeof result === 'object' && 'items' in result) return { state: 'stopped', hostId: null, lastError: null };
    return result;
  });
  return { stores: createStoreHarness(client), calls };
}

describe('teams store', () => {
  it('reports a failed link read without discarding successfully loaded teams', async () => {
    const { stores } = host(['teams.hosts.list'], (method) => {
      if (method === 'teams.hosts.list') return { items: [hostRow(team)] };
      if (method === 'teams.host.status') throw new Error('link status unavailable');
      return { items: [] };
    });
    await stores.use(() => useCapabilityStore()).refresh('host-a');
    const teams = stores.use(() => useTeamsStore());
    teams.link = { state: 'ready', hostId: team, lastError: null };
    await teams.refresh();
    expect(teams.hosts).toHaveLength(1);
    expect(teams.error).toBe('link status unavailable');
    expect(teams.link.state).toBe('unknown');
  });

  it('does not call team methods the host did not announce', async () => {
    const { stores, calls } = host(['rpc.discover']);
    await stores.use(() => useCapabilityStore()).refresh('host-a');
    await stores.use(() => useTeamsStore()).refresh();
    expect(calls.map((call) => call.method)).toEqual(['rpc.discover']);
  });

  it('loads the team detail and keeps only the newest selection', async () => {
    const { stores } = host(['teams.hosts.list'], (method, params) => {
      if (method === 'teams.hosts.list') return { items: [hostRow(team), hostRow(other)] };
      if (method === 'teams.devices.list') return { items: [] };
      if (params?.hostId === team && method === 'teams.grants.list') return { items: [] };
      if (method.endsWith('.list')) return { items: [] };
      throw new Error(method);
    });
    await stores.use(() => useCapabilityStore()).refresh('host-a');
    const teams = stores.use(() => useTeamsStore());
    await teams.refresh();
    expect(teams.hosts.map((h) => h.id)).toEqual([team, other]);
    await teams.select(team);
    expect(teams.selectedId).toBe(team);
  });

  it('sends the grant revision it read when changing a role', async () => {
    const { stores, calls } = host(['teams.hosts.list'], (method) => {
      if (method === 'teams.hosts.list') return { items: [hostRow(team)] };
      if (method === 'teams.grants.list') return { items: [{ id: grantId, deviceId: grantId, deviceLabel: 'PC', role: 'observer', status: 'active', revision: 7, policy: {} }] };
      return { items: [] };
    });
    await stores.use(() => useCapabilityStore()).refresh('host-a');
    const teams = stores.use(() => useTeamsStore());
    await teams.refresh();
    await teams.select(team);
    const [grant] = teams.grants;
    await teams.changeRole(grant!, 'moderator');
    expect(calls.find((call) => call.method === 'teams.grants.update')?.params)
      .toEqual({ hostId: team, grantId, role: 'moderator', expectedRevision: 7 });
  });

  it('holds an invitation token only until it is dismissed or the store resets', async () => {
    const { stores } = host(['teams.hosts.list'], (method) => {
      if (method === 'teams.hosts.list') return { items: [hostRow(team)] };
      if (method === 'teams.invitations.create') return { id: grantId, requestedRole: 'observer', expiresAt: 'later', token };
      return { items: [] };
    });
    await stores.use(() => useCapabilityStore()).refresh('host-a');
    const teams = stores.use(() => useTeamsStore());
    await teams.refresh();
    await teams.select(team);
    await teams.createInvitation('observer', 900);
    expect(teams.issued?.token).toBe(token);
    teams.dismissInvitation();
    expect(teams.issued).toBeNull();
    await teams.createInvitation('observer', 900);
    teams.reset();
    expect(teams.issued).toBeNull();
  });

  it('starts and stops the host link through its own methods', async () => {
    const { stores, calls } = host(['teams.hosts.list', 'teams.host.start', 'teams.host.stop'], (method) => {
      if (method === 'teams.hosts.list') return { items: [hostRow(team)] };
      if (method === 'teams.host.start') return { state: 'connecting', hostId: team, lastError: null };
      if (method === 'teams.host.stop') return { state: 'stopped', hostId: null, lastError: null };
      return { items: [] };
    });
    await stores.use(() => useCapabilityStore()).refresh('host-a');
    const teams = stores.use(() => useTeamsStore());
    await teams.setHostLink(true);
    expect(teams.link.state).toBe('connecting');
    await teams.setHostLink(false);
    expect(teams.link.state).toBe('stopped');
    expect(calls.map((call) => call.method).filter((m) => m.startsWith('teams.host.'))).toEqual(['teams.host.start', 'teams.host.stop']);
  });

  it('redeems an invitation as a request and never approves it locally', async () => {
    const { stores, calls } = host(['teams.hosts.list', 'teams.invitations.redeem'], (method) => {
      if (method === 'teams.invitations.redeem') return { requestId: grantId, hostId: team, status: 'pending', requestedRole: 'observer' };
      return { items: [] };
    });
    await stores.use(() => useCapabilityStore()).refresh('host-a');
    const teams = stores.use(() => useTeamsStore());
    expect(await teams.redeem(token)).toBe(true);
    expect(teams.lastRedeem?.status).toBe('pending');
    expect(calls.find((call) => call.method === 'teams.invitations.redeem')?.params).toEqual({ token });
    expect(calls.some((call) => call.method === 'teams.grants.update')).toBe(false);
  });

  it('shows an unknown outcome as unknown when no result arrives', async () => {
    const { stores, calls } = host(['teams.hosts.list', 'teams.commands.run'], (method) => {
      if (method === 'teams.commands.run') return { commandId: 'cmd-1', status: 'unknown', code: 'noResult', result: null };
      return { items: [] };
    });
    await stores.use(() => useCapabilityStore()).refresh('host-a');
    const teams = stores.use(() => useTeamsStore());
    expect(await teams.runCommand(team, 'live.status')).toBe(true);
    expect(teams.lastCommand?.status).toBe('unknown');
    expect(calls.find((call) => call.method === 'teams.commands.run')?.params).toEqual({ hostId: team, operation: 'live.status' });
  });

  describe('late replies', () => {
    function deferred<T>() {
      let resolve!: (value: T) => void; let reject!: (failure: unknown) => void;
      const promise = new Promise<T>((done, fail) => { resolve = done; reject = fail; });
      return { promise, resolve, reject };
    }
    const invitation = { id: grantId, requestedRole: 'observer', expiresAt: 'later', token };
    const row = { id: grantId, requestedRole: 'observer', status: 'pending', expiresAt: 'later' };

    it('does not show or repopulate an invitation after the team was closed or another was selected', async () => {
      const created = deferred<unknown>();
      const { stores, calls } = host(['teams.hosts.list'], (method, params) => {
        if (method === 'teams.hosts.list') return { items: [hostRow(team), hostRow(other)] };
        if (method === 'teams.invitations.create') return created.promise;
        if (method === 'teams.invitations.list') return { items: params?.hostId === team ? [row] : [] };
        return { items: [] };
      });
      await stores.use(() => useCapabilityStore()).refresh('host-a');
      const teams = stores.use(() => useTeamsStore());
      await teams.refresh();
      await teams.select(team);
      const pending = teams.createInvitation('observer', 900);
      expect(teams.working).toBe(true);
      await teams.select(other);
      created.resolve(invitation);
      await pending; await flushPromises();
      expect(teams.selectedId).toBe(other);
      expect(teams.issued).toBeNull();
      expect(teams.invitations).toEqual([]);
      expect(teams.working).toBe(false);
      expect(calls.filter((call) => call.method === 'teams.invitations.create')).toHaveLength(1);
    });

    it('does not show an invitation that arrives after the panel closed', async () => {
      const created = deferred<unknown>();
      const { stores } = host(['teams.hosts.list'], (method) => {
        if (method === 'teams.hosts.list') return { items: [hostRow(team)] };
        if (method === 'teams.invitations.create') return created.promise;
        return { items: [] };
      });
      await stores.use(() => useCapabilityStore()).refresh('host-a');
      const teams = stores.use(() => useTeamsStore());
      await teams.refresh();
      await teams.select(team);
      const pending = teams.createInvitation('observer', 900);
      teams.reset();
      created.resolve(invitation);
      await pending;
      expect(teams.issued).toBeNull();
      expect(teams.selectedId).toBeNull();
      expect(teams.error).toBeNull();
      expect(teams.working).toBe(false);
    });

    it('checks the generation again after reading the host link', async () => {
      const link = deferred<unknown>();
      const { stores } = host(['teams.hosts.list'], (method) => {
        if (method === 'teams.hosts.list') return { items: [hostRow(team)] };
        if (method === 'teams.host.status') return link.promise;
        return { items: [] };
      });
      await stores.use(() => useCapabilityStore()).refresh('host-a');
      const teams = stores.use(() => useTeamsStore());
      const refreshing = teams.refresh();
      await flushPromises();
      expect(teams.hosts.map((h) => h.id)).toEqual([team]);
      teams.reset();
      link.resolve({ state: 'connected', hostId: team, lastError: null });
      await refreshing;
      expect(teams.link.state).toBe('stopped');
      expect(teams.loading).toBe(false);
    });

    it('drops the previous account teams and ignores its late failure when the account changes', async () => {
      const first = deferred<unknown>();
      let lists = 0;
      const { stores } = host(['teams.hosts.list'], (method) => {
        if (method === 'teams.hosts.list') return (lists += 1) === 1 ? first.promise : { items: [] };
        return { items: [] };
      });
      await stores.use(() => useCapabilityStore()).refresh('host-a');
      const account = stores.use(() => useAccountStore());
      account.status = { origin: 'https://cloud.example', state: 'signed-in', accountId: 'alice', persistence: 'memory-only' };
      const teams = stores.use(() => useTeamsStore());
      teams.hosts = [hostRow(team) as never];
      const refreshing = teams.refresh();
      account.status = { origin: 'https://cloud.example', state: 'signed-in', accountId: 'bob', persistence: 'memory-only' };
      expect(teams.hosts).toEqual([]);
      first.reject(new Error('alice private failure'));
      await refreshing; await flushPromises();
      expect(teams.hosts).toEqual([]);
      expect(teams.error).toBeNull();
      expect(teams.errorDetail).toBeNull();
      expect(teams.loading).toBe(false);
    });

    it('keeps the activity of a creating operation while the team it opens is loading', async () => {
      const detail = deferred<unknown>();
      const { stores } = host(['teams.hosts.list'], (method) => {
        if (method === 'teams.hosts.create') return hostRow(team);
        if (method === 'teams.invitations.list') return detail.promise;
        return { items: [] };
      });
      await stores.use(() => useCapabilityStore()).refresh('host-a');
      const teams = stores.use(() => useTeamsStore());
      const creating = teams.createTeam('Team');
      await flushPromises();
      expect(teams.working).toBe(true);
      detail.resolve({ items: [] });
      expect(await creating).toBe(true);
      expect(teams.working).toBe(false);
    });
  });
});


it('requires the binding of this PC rather than another active host device', async () => {
  const { stores } = host(['teams.hosts.list'], method => {
    if (method === 'teams.hosts.list') return { items: [hostRow(team)] };
    if (method === 'teams.devices.list') return { items: [
      { id: grantId, hostId: team, label: 'Other PC', kind: 'host', status: 'active', isThisDevice: false },
      { id: other, hostId: team, label: 'This PC', kind: 'host', status: 'active', isThisDevice: true },
    ] };
    return { items: [] };
  });
  await stores.use(() => useCapabilityStore()).refresh('host-a');
  const teams = stores.use(() => useTeamsStore());
  await teams.refresh();
  await teams.select(team);
  expect(teams.localHostDevice?.id).toBe(other);
  await teams.select(other);
  expect(teams.localHostDevice).toBeNull();
});
