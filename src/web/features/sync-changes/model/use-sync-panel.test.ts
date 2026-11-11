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
