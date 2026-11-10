import { describe, expect, it } from 'vitest';
import { createFakeControlClient, createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
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
    return handler(method, params as Record<string, unknown> | undefined);
  });
  return { stores: createStoreHarness(client), calls };
}

describe('teams store', () => {
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
});
