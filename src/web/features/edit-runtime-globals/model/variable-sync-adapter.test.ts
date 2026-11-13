import { describe, expect, it, vi } from 'vitest';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import { createVariableSyncAdapter, type VariableComparison } from './variable-sync-adapter.ts';

function row(partial: Partial<VariableComparison> = {}): VariableComparison {
  return { review: 'reviewed-first', name: 'port', local: { type: 'number', value: 8 }, cloud: null, cloudDeleted: false, localSecret: false, blockedLocal: false, pendingStatus: null, canSend: true, canApply: false, canWithdraw: false, ...partial };
}
function setup(rows = [row()], options: { support?: boolean } = { support: true }) {
  let comparisons = rows;
  const { client, calls } = createFakeControlClient((method) => {
    if (method === 'sync.v2.variables.review') return { changes: comparisons };
    if (method === 'sync.v2.variables.apply') return null;
    if (method === 'sync.v2.variables.send' || method === 'sync.v2.variables.withdraw' || method === 'sync.v2.variables.fetch') return { conflicts: [] };
    throw new Error(`unexpected ${method}`);
  });
  const reloadLocal = vi.fn(async () => {});
  const refreshStatus = vi.fn(async () => {});
  const adapter = createVariableSyncAdapter({ client, supports: () => options.support, revision: () => 'local-r1', status: () => null, reloadLocal, refreshStatus, t: (key) => key });
  return { adapter, calls, reloadLocal, refreshStatus, replace: (next: VariableComparison[]) => { comparisons = next; } };
}

describe('native Variables Changes adapter', () => {
  it('offers reviewed send/adopt/withdraw, with no whole-workspace sync or field merge', () => {
    const { adapter } = setup();
    expect(adapter.capabilities).toMatchObject({ push: true, fetch: true, apply: true, withdraw: true, resolve: true, sync: false, merge: false, reset: false, force: false, history: false, autoSync: false });
  });
  it('retains explicit types and compares both existing versions without inventing a base', async () => {
    const { adapter } = setup([row({ cloud: { type: 'text', value: '8' }, canApply: true, canWithdraw: true })]);
    const [change] = await adapter.list();
    expect(change).toMatchObject({ id: 'reviewed-first', side: 'conflict', after: { type: 'number', value: 8 }, remote: { type: 'text', value: '8' }, canResolveLocal: true, canResolveCloud: true });
    expect(change?.base).toBeUndefined();
  });
  it('never projects secret or blocked values, even if a host incorrectly offers send/apply', async () => {
    const { adapter } = setup([row({ localSecret: true, local: { type: 'secret', value: 'PRIVATE' }, cloud: { type: 'text', value: 'OLD_PRIVATE' }, canApply: true, canWithdraw: true })]);
    const result = await adapter.list();
    expect(JSON.stringify(result)).not.toContain('PRIVATE');
    expect(result[0]).toMatchObject({ valuesHidden: true, canSend: false, canApply: false, canResolveLocal: false, canResolveCloud: false, canWithdraw: true });
  });
  it('does not read or fall back on older or undiscovered hosts', async () => {
    for (const support of [false, undefined]) {
      const { adapter, calls } = setup([], { support });
      expect(adapter.unavailableReason?.()).toBe('syncChanges.unsupported');
      expect(await adapter.list()).toEqual([]);
      expect(calls).toHaveLength(0);
    }
  });
  it('sends the displayed reference after an independent counter read, never a page value', async () => {
    const fixture = setup();
    const shown = await fixture.adapter.list();
    fixture.replace([row({ review: 'new-comparison' })]);
    await fixture.adapter.list();
    await fixture.adapter.push!([shown[0]!.id]);
    expect(fixture.calls.at(-1)).toMatchObject({ method: 'sync.v2.variables.send', params: { review: 'reviewed-first' } });
    expect(fixture.calls.some((call) => call.method === 'sync.v2.now' || call.method === 'sync.v2.section' || call.method === 'globals.typed.replace')).toBe(false);
  });
  it('fetches only into the cache and withdraws without replacing local values', async () => {
    const { adapter, calls, reloadLocal, refreshStatus } = setup();
    await adapter.fetch!();
    await adapter.withdraw!('secret-reference');
    expect(calls.map((call) => call.method)).toEqual(['sync.v2.variables.fetch', 'sync.v2.variables.withdraw']);
    expect(reloadLocal).not.toHaveBeenCalled();
    expect(refreshStatus).toHaveBeenCalledTimes(2);
  });
  it('adopts the reviewed cached value through the native RPC and reloads local state', async () => {
    const { adapter, calls, reloadLocal } = setup();
    await adapter.apply!('cloud-reference');
    expect(calls).toEqual([{ method: 'sync.v2.variables.apply', params: { review: 'cloud-reference' } }]);
    expect(reloadLocal).toHaveBeenCalledOnce();
  });
  it('resolves by native send or native adoption and rejects field merges', async () => {
    const { adapter, calls } = setup();
    await adapter.resolve!('one', { choice: 'local' });
    await adapter.resolve!('two', { choice: 'cloud' });
    await expect(adapter.resolve!('three', { choice: 'merged', value: {} })).rejects.toThrow();
    expect(calls.map((call) => call.method)).toEqual(['sync.v2.variables.send', 'sync.v2.variables.apply']);
  });
});
