import { describe, expect, it, vi } from 'vitest';
import type { LiveEvent } from '../../../../../../src/automation/behavior/types.ts';
import type { SectionSnapshot } from '@/entities/cloud-sync/lib/snapshot-store.ts';
import { liveEvent } from '../../../../support/automation.ts';
import { propertyOf } from '../../../../support/json.ts';
import { createAutomationSyncAdapter } from '../../../../../../src/web/features/manage-automations/model/automation-sync-adapter.ts';
import { defined } from '../../../../../shared/index.ts';

const event = (id: string, name: string, enabled = true): LiveEvent => liveEvent({ id, name, enabled, trigger: 'chat' });

function setup(initial: LiveEvent[], loaded = true) {
  let records = initial;
  const stored = new Map<string, SectionSnapshot>();
  const deps = {
    kind: 'event' as const,
    records: () => records,
    loaded: () => loaded,
    read: vi.fn(async (_kind: 'event' | 'action', id: string) => ({ record: records.find((record) => record.id === id) ?? null, revision: '3', host: null })),
    save: vi.fn(async () => {}),
    remove: vi.fn(async () => {}),
    t: (key: string) => key,
    snapshots: { read: (scope: string) => stored.get(scope) ?? null, write: (scope: string, snapshot: SectionSnapshot) => void stored.set(scope, snapshot) },
  };
  return { adapter: createAutomationSyncAdapter(deps), deps, set: (next: LiveEvent[]) => { records = next; }, stored };
}

describe('automation sync adapter', () => {
  it('declares reset and mark as reviewed only, and needs no Cloud account, because nothing travels to Cloud yet', () => {
    const { adapter } = setup([]);
    expect(adapter.capabilities).toMatchObject({
      reset: true, markReviewed: true, push: false, fetch: false, sync: false, force: false, history: false, merge: false,
    });
    expect(adapter.requiresCloud).toBe(false);
    expect(adapter.scope.id).toBe('events');
  });

  it('starts a snapshot the first time and reports no changes', async () => {
    const { adapter, stored } = setup([event('a', 'A')]);
    expect(await adapter.list()).toEqual([]);
    expect(Object.keys(defined(stored.get('automation-events')))).toEqual(['a']);
  });

  it('does not treat a host that has not delivered its definitions as deleting everything', async () => {
    const { adapter, stored } = setup([], false);
    expect(await adapter.list()).toEqual([]);
    expect(stored.size).toBe(0);
  });

  it('lists modified, added and deleted records against the snapshot as local changes', async () => {
    const { adapter, set } = setup([event('a', 'A'), event('b', 'B')]);
    await adapter.list();
    set([event('a', 'A renamed'), event('c', 'C')]);
    const changes = await adapter.list();
    expect(changes.map((change) => [change.id, change.status, change.side])).toEqual([
      ['a', 'modified', 'local'],
      ['c', 'added', 'local'],
      ['b', 'deleted', 'local'],
    ]);
    const modified = defined(changes[0]);
    expect(propertyOf(modified.before, 'name')).toBe('A');
    expect(propertyOf(modified.after, 'name')).toBe('A renamed');
  });

  it('resets a modified or deleted record by saving the snapshot version', async () => {
    const { adapter, deps, set } = setup([event('a', 'A'), event('b', 'B')]);
    await adapter.list();
    set([event('a', 'A renamed')]);
    await defined(adapter.reset)('a');
    expect(deps.save).toHaveBeenLastCalledWith(expect.objectContaining({ id: 'a', name: 'A' }), { revision: '3', create: false, host: null });
    await defined(adapter.reset)('b');
    expect(deps.save).toHaveBeenLastCalledWith(expect.objectContaining({ id: 'b' }), { revision: '3', create: true, host: null });
  });

  it('resets an added record by removing it', async () => {
    const { adapter, deps, set } = setup([event('a', 'A')]);
    await adapter.list();
    set([event('a', 'A'), event('c', 'C')]);
    await defined(adapter.reset)('c');
    expect(deps.remove).toHaveBeenCalledWith('event', 'c', '3', null);
    expect(deps.save).not.toHaveBeenCalled();
  });

  it('resets everything that changed', async () => {
    const { adapter, deps, set } = setup([event('a', 'A')]);
    await adapter.list();
    set([event('a', 'A2'), event('c', 'C')]);
    await defined(adapter.resetAll)();
    expect(deps.save).toHaveBeenCalledTimes(1);
    expect(deps.remove).toHaveBeenCalledTimes(1);
  });

  it('marks the section as reviewed by advancing the reference to the current records', async () => {
    const { adapter, set, stored } = setup([event('a', 'A'), event('b', 'B')]);
    await adapter.list();
    set([event('a', 'A renamed'), event('c', 'C')]);
    expect(await adapter.list()).toHaveLength(3);
    await defined(adapter.markReviewed)();
    expect(await adapter.list()).toEqual([]);
    expect(Object.keys(defined(stored.get('automation-events'))).sort()).toEqual(['a', 'c']);
    expect(propertyOf(defined(stored.get('automation-events')).a, 'name')).toBe('A renamed');
  });

  it('uses the advance hook when the store has one, so readers compare again', async () => {
    const advance = vi.fn();
    const { adapter, deps } = setup([event('a', 'A')]);
    const custom = createAutomationSyncAdapter({ ...deps, snapshots: { ...deps.snapshots, advance } });
    await custom.list();
    await defined(custom.markReviewed)();
    expect(advance).toHaveBeenCalledWith('automation-events', expect.objectContaining({ a: expect.objectContaining({ name: 'A' }) }));
    expect(adapter.watchSource).toBeTypeOf('function');
  });

  it('changes its watch source when the reference is advanced even though no record changed', async () => {
    const { deps } = setup([event('a', 'A')]);
    // The default store is shared and reactive; the count badge watches this source.
    const adapter = createAutomationSyncAdapter({ ...deps, snapshots: undefined });
    const referenceOf = (): string => {
      const source = defined(adapter.watchSource)();
      if (!Array.isArray(source)) throw new Error('Expected the watch source to be a list');
      return JSON.stringify(source.slice(1));
    };
    const before = referenceOf();
    await adapter.list();
    await defined(adapter.markReviewed)();
    expect(referenceOf()).not.toBe(before);
    localStorage.removeItem('tiktools.sync.snapshot.automation-events');
  });

  it('refuses to advance before the host has delivered its definitions', async () => {
    const { adapter, stored } = setup([], false);
    await expect(defined(adapter.markReviewed)()).rejects.toThrow('syncChanges.actionFailed');
    expect(stored.size).toBe(0);
  });

  it('still resets to the new reference after it was advanced', async () => {
    const { adapter, deps, set } = setup([event('a', 'A')]);
    await adapter.list();
    set([event('a', 'A2')]);
    await defined(adapter.markReviewed)();
    set([event('a', 'A3')]);
    await defined(adapter.reset)('a');
    expect(deps.save).toHaveBeenLastCalledWith(expect.objectContaining({ id: 'a', name: 'A2' }), expect.anything());
  });
});
