import { effectScope } from 'vue';
import { flushPromises } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import type { SyncCapabilities } from '@/entities/cloud-sync/api/sync-adapter.ts';
import { allCapabilities, fakeAdapter } from '../testing/fake-adapter.ts';
import { useSyncChanges } from './use-sync-changes.ts';
import { useSyncPanel } from './use-sync-panel.ts';

async function panel(capabilities: Partial<SyncCapabilities>) {
  const { adapter, spies } = fakeAdapter(capabilities);
  const scope = effectScope();
  const state = scope.run(() => useSyncPanel(adapter, useSyncChanges(adapter)))!;
  await flushPromises();
  return { state, spies };
}

describe('useSyncPanel', () => {
  it('never runs an operation the adapter does not declare', async () => {
    const { state, spies } = await panel({});
    await state.push();
    await state.syncAll();
    await state.fetchChanges();
    state.ask({ kind: 'forcePush' });
    state.ask({ kind: 'resetAll' });
    state.ask({ kind: 'markReviewed' });
    state.ask({ kind: 'revert', operationId: 'op1' });
    expect(state.confirmation.value).toBeNull();
    expect(spies.push).not.toHaveBeenCalled();
    expect(spies.sync).not.toHaveBeenCalled();
    expect(spies.fetch).not.toHaveBeenCalled();
    expect(state.sendable.value).toEqual([]);
  });

  it('keeps the destructive operation pending until confirmed and drops it on cancel', async () => {
    const { state, spies } = await panel(allCapabilities);
    state.ask({ kind: 'forcePull' });
    expect(state.confirmation.value).toEqual({ kind: 'forcePull' });
    state.cancel();
    await state.confirm();
    expect(spies.forcePull).not.toHaveBeenCalled();
    state.ask({ kind: 'forcePull' });
    await state.confirm();
    expect(spies.forcePull).toHaveBeenCalledOnce();
    expect(state.confirmation.value).toBeNull();
  });

  it('keeps mark as reviewed pending until confirmed', async () => {
    const { state, spies } = await panel({ markReviewed: true });
    state.ask({ kind: 'markReviewed' });
    expect(state.confirmation.value).toEqual({ kind: 'markReviewed' });
    state.cancel();
    await state.confirm();
    expect(spies.markReviewed).not.toHaveBeenCalled();
    state.ask({ kind: 'markReviewed' });
    const before = spies.list.mock.calls.length;
    await state.confirm();
    expect(spies.markReviewed).toHaveBeenCalledOnce();
    expect(spies.list.mock.calls.length).toBe(before + 1);
    expect(state.confirmation.value).toBeNull();
  });

  it('resets every change one by one when the adapter has no resetAll', async () => {
    const { adapter, spies } = fakeAdapter(allCapabilities);
    delete (adapter as { resetAll?: unknown }).resetAll;
    const scope = effectScope();
    const state = scope.run(() => useSyncPanel(adapter, useSyncChanges(adapter)))!;
    await flushPromises();
    state.ask({ kind: 'resetAll' });
    await state.confirm();
    expect(spies.reset.mock.calls.map(([id]) => id)).toEqual(['e1', 'e2', 'e3', 'e4']);
  });

  it('reports a failed operation and refreshes the list afterwards', async () => {
    const { state, spies } = await panel(allCapabilities);
    spies.sync.mockRejectedValueOnce(new Error('network down'));
    const before = spies.list.mock.calls.length;
    await state.syncAll();
    expect(state.failure.value).toBe('network down');
    expect(spies.list.mock.calls.length).toBe(before + 1);
    expect(state.busy.value).toBe(false);
  });
});

describe('reviewed native record actions', () => {
  it('hides send/apply/resolve for a blocked value, while requiring confirmation for withdrawal', async () => {
    const { adapter, spies } = fakeAdapter({ push: true, apply: true, withdraw: true, resolve: true }, [{ id: 'secret-review', name: 'ordinary', kind: 'variable', status: 'conflict', side: 'conflict', canSend: false, canApply: false, canWithdraw: true, canResolveLocal: false, canResolveCloud: false, valuesHidden: true }]);
    const scope = effectScope();
    const state = scope.run(() => useSyncPanel(adapter, useSyncChanges(adapter)))!;
    await flushPromises();
    expect(state.sendable.value).toEqual([]);
    state.ask({ kind: 'apply', id: 'secret-review', name: 'ordinary' });
    expect(state.confirmation.value).toBeNull();
    await state.resolve('secret-review', { choice: 'local' });
    await state.resolve('secret-review', { choice: 'cloud' });
    expect(spies.resolve).not.toHaveBeenCalled();
    state.ask({ kind: 'withdraw', id: 'secret-review', name: 'ordinary' });
    expect(spies.withdraw).not.toHaveBeenCalled();
    state.cancel();
    await state.confirm();
    expect(spies.withdraw).not.toHaveBeenCalled();
    state.ask({ kind: 'withdraw', id: 'secret-review', name: 'ordinary' });
    await state.confirm();
    expect(spies.withdraw).toHaveBeenCalledExactlyOnceWith('secret-review');
    scope.stop();
  });
  it('does not adopt a comparison that disappeared before confirmation', async () => {
    const { adapter, spies } = fakeAdapter({ apply: true }, [{ id: 'cloud-review', name: 'port', kind: 'variable', status: 'added', side: 'remote', canApply: true }]);
    const scope = effectScope();
    const state = scope.run(() => useSyncPanel(adapter, useSyncChanges(adapter)))!;
    await flushPromises();
    state.ask({ kind: 'apply', id: 'cloud-review', name: 'port' });
    expect(spies.apply).not.toHaveBeenCalled();
    state.changes.value = [];
    await state.confirm();
    expect(spies.apply).not.toHaveBeenCalled();
    scope.stop();
  });
});
