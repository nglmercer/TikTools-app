import { describe, expect, it } from 'vitest';
import type { PluginStatus } from '../../../../automation/behavior/types.ts';
import type { WorkspaceEntry, WorkspaceValue } from '@/entities/cloud-sync/api/workspace-changes.ts';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import { createPluginSyncAdapter } from './plugin-sync-adapter.ts';

const absent: WorkspaceValue = { state: 'absent', revision: null, value: null, version: null, enabled: null };
const present = (version: string, enabled: boolean, revision = 1): WorkspaceValue => ({
  state: 'present', revision, version, enabled, value: { schemaVersion: 1, pluginId: 'alpha', version, enabled },
});
const plugin = (id: string, version: string, enabled: boolean, installed = true): PluginStatus =>
  ({ descriptor: { id, version, name: id }, installed, enabled, available: true }) as unknown as PluginStatus;

function setup(entries: WorkspaceEntry[], plugins: PluginStatus[], overrides: { supports?: boolean | undefined; selected?: boolean } = {}) {
  const { client, calls } = createFakeControlClient((method) => {
    if (method === 'sync.v2.selected') return overrides.selected ?? true;
    if (method === 'sync.v2.changes.list') return entries;
    if (method === 'sync.v2.now') return { pushed: 1 };
    if (method === 'sync.v2.conflict.review') return { handle: 'h'.repeat(64) };
    if (method === 'sync.v2.conflict.resolve') return 'resolved';
    throw new Error(`unexpected ${method}`);
  });
  let refreshed = 0;
  const adapter = createPluginSyncAdapter({
    client, plugins: () => plugins, status: () => ({ lastSyncedAt: 100, pending: 1 }), refreshStatus: async () => { refreshed += 1; },
    supports: () => ('supports' in overrides ? overrides.supports : true), name: (entry) => entry.descriptor.id.toUpperCase(), t: (key) => key,
  });
  return { adapter, calls, refreshed: () => refreshed };
}

const entry = (partial: Partial<WorkspaceEntry>): WorkspaceEntry => ({ collection: 'plugin-selections', key: 'alpha', remote: absent, pending: null, hasNewer: false, ...partial });

describe('plugin sync adapter', () => {
  it('declares whole-pass sync and conflict resolution, and nothing it cannot do', () => {
    const { adapter } = setup([], []);
    expect(adapter.capabilities).toMatchObject({ sync: true, resolve: true, push: false, fetch: false, merge: false, force: false, reset: false, history: false, fork: false });
  });

  it('lists a queued edit as a local change between the Cloud base and the staged value', async () => {
    const pending = { status: 'queued' as const, base: present('1.0.0', true), local: present('1.0.0', false), changedKeys: null };
    const { adapter } = setup([entry({ remote: present('1.0.0', true), pending })], [plugin('alpha', '1.0.0', false)]);
    const [change] = await adapter.list();
    expect(change).toMatchObject({ id: 'plugin-selections/alpha', kind: 'plugin', name: 'ALPHA', status: 'modified', side: 'local' });
    expect((change!.before as { enabled: boolean }).enabled).toBe(true);
    expect((change!.after as { enabled: boolean }).enabled).toBe(false);
  });

  it('lists an unconfirmed or conflicting edit as a conflict with base, local and Cloud', async () => {
    const pending = { status: 'conflict' as const, base: present('1.0.0', true, 1), local: present('1.0.0', false, 1), changedKeys: null };
    const { adapter } = setup([entry({ remote: present('2.0.0', true, 2), pending })], [plugin('alpha', '1.0.0', false)]);
    const [change] = await adapter.list();
    expect(change).toMatchObject({ status: 'conflict', side: 'conflict' });
    expect((change!.base as { version: string }).version).toBe('1.0.0');
    expect((change!.remote as { version: string }).version).toBe('2.0.0');
  });

  it('lists a staged settings edit and a staged delete', async () => {
    const settings = entry({ collection: 'plugin-settings', pending: { status: 'queued', base: absent, local: { state: 'present', revision: null, value: null, version: null, enabled: null }, changedKeys: ['a'] } });
    const removal = entry({ key: 'beta', remote: present('1.0.0', true), pending: { status: 'queued', base: present('1.0.0', true), local: { state: 'deleted', revision: null, value: null, version: null, enabled: null }, changedKeys: null } });
    const { adapter } = setup([settings, removal], []);
    const changes = await adapter.list();
    expect(changes.map((change) => [change.kind, change.status])).toEqual([['pluginSettings', 'added'], ['plugin', 'deleted']]);
  });

  it('lists a settings change by key names only and never reads or shows a value', async () => {
    const held = (state: 'present' | 'deleted', value: unknown): WorkspaceValue => ({ state, revision: 2, value: value as WorkspaceValue['value'], version: 'LEAK-VERSION', enabled: true });
    // An older host could still send values; the adapter must not surface them.
    const queued = entry({ collection: 'plugin-settings', remote: held('present', { settings: { apiKey: 'LEAK-REMOTE' } }), pending: { status: 'queued', base: held('present', { settings: { apiKey: 'LEAK-BASE' } }), local: held('present', { settings: { apiKey: 'LEAK-LOCAL' } }), changedKeys: ['apiKey', 'volume'] } });
    const clash = entry({ collection: 'plugin-settings', key: 'beta', remote: held('present', { settings: { token: 'LEAK-CLOUD' } }), pending: { status: 'conflict', base: held('present', { settings: { token: 'LEAK-BASE' } }), local: held('deleted', null), changedKeys: ['token'] } });
    const idle = entry({ collection: 'plugin-settings', key: 'gamma', remote: held('present', { settings: { x: 'LEAK-IDLE' } }) });
    const { adapter } = setup([queued, clash, idle], []);
    const changes = await adapter.list();
    expect(changes.map((change) => [change.id, change.kind, change.status, change.side, change.changedKeys])).toEqual([
      ['plugin-settings/alpha', 'pluginSettings', 'modified', 'local', ['apiKey', 'volume']],
      ['plugin-settings/beta', 'pluginSettings', 'conflict', 'conflict', ['token']],
    ]);
    for (const change of changes) {
      expect(change.before).toBeUndefined();
      expect(change.after).toBeUndefined();
      expect(change.remote).toBeUndefined();
      expect(change.base).toBeUndefined();
    }
    expect(JSON.stringify(changes)).not.toContain('LEAK');
  });

  it('lists a Cloud selection that differs from the installed plugin as something to fetch', async () => {
    const { adapter } = setup([entry({ remote: present('2.0.0', true) }), entry({ key: 'gamma', remote: present('1.0.0', true) })], [plugin('alpha', '1.0.0', true)]);
    const changes = await adapter.list();
    expect(changes.map((change) => [change.id, change.side, change.status])).toEqual([['plugin-selections/alpha', 'remote', 'modified'], ['plugin-selections/gamma', 'remote', 'added']]);
  });

  it('ignores records that match the installed plugin and entries it cannot compare', async () => {
    const { adapter } = setup([entry({ remote: present('1.0.0', true) }), entry({ collection: 'preferences', key: 'desktop' }), entry({ collection: 'plugin-settings', remote: present('1', true) })], [plugin('alpha', '1.0.0', true)]);
    expect(await adapter.list()).toEqual([]);
  });

  it('fails with an explanation when v2 is not the selected protocol', async () => {
    const { adapter } = setup([], [], { selected: false });
    await expect(adapter.list()).rejects.toThrow('syncChanges.v2Off');
  });

  it('is unavailable on a host without the workspace read', async () => {
    const { adapter, calls } = setup([], [], { supports: false });
    expect(adapter.unavailableReason?.()).toBe('syncChanges.unsupported');
    expect(await adapter.list()).toEqual([]);
    expect(calls).toEqual([]);
  });

  it('syncs with one v2 pass and refreshes the status', async () => {
    const { adapter, calls, refreshed } = setup([], []);
    await adapter.sync!();
    expect(calls.map((call) => call.method)).toEqual(['sync.v2.now']);
    expect(refreshed()).toBe(1);
  });

  it('resolves a conflict by reviewing it first, mapping the choice to the v2 resolution', async () => {
    const { adapter, calls } = setup([], []);
    await adapter.resolve!('plugin-selections/alpha', { choice: 'cloud' });
    expect(calls.map((call) => [call.method, call.params])).toEqual([
      ['sync.v2.conflict.review', { collection: 'plugin-selections', key: 'alpha' }],
      ['sync.v2.conflict.resolve', { handle: 'h'.repeat(64), choice: 'takeCloud' }],
    ]);
    await adapter.resolve!('plugin-selections/alpha', { choice: 'local' });
    expect(calls.at(-1)!.params).toMatchObject({ choice: 'keepLocal' });
  });

  it('refuses a field-level merge it cannot stage', async () => {
    const { adapter, calls } = setup([], []);
    await expect(adapter.resolve!('plugin-selections/alpha', { choice: 'merged', value: {} })).rejects.toThrow();
    expect(calls).toEqual([]);
  });
});
