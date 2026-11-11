import { computed, ref, shallowRef } from 'vue';
import { errorMessage } from '@/shared/api/control-client.ts';
import { groupChanges } from '@/entities/cloud-sync/lib/sync-merge.ts';
import type { SyncAdapter, SyncChange, SyncOperation, SyncResolution } from '@/entities/cloud-sync/api/sync-adapter.ts';
import type { SyncChanges } from './use-sync-changes.ts';

export type SyncView = 'changes' | 'history';

/** Everything that needs the person's explicit yes before it runs. */
export type SyncConfirmation =
  | { kind: 'forcePush' }
  | { kind: 'forcePull' }
  | { kind: 'resetAll' }
  | { kind: 'reset'; id: string; name: string }
  | { kind: 'restore'; operationId: string; id: string; name: string }
  | { kind: 'revert'; operationId: string };

/** Panel state and operations over one adapter. The component only renders it. */
export function useSyncPanel(adapter: SyncAdapter, source: SyncChanges) {
  const caps = adapter.capabilities;
  const view = ref<SyncView>('changes');
  const busy = shallowRef(false);
  const failure = shallowRef<string | null>(null);
  const confirmation = shallowRef<SyncConfirmation | null>(null);
  const currentId = shallowRef<string | null>(null);
  /** Local elements the person unticked; everything else is sent. */
  const unchecked = ref<ReadonlySet<string>>(new Set());
  const operations = shallowRef<SyncOperation[]>([]);
  const operationId = shallowRef<string | null>(null);
  const historyChangeId = shallowRef<string | null>(null);
  const historyLoading = shallowRef(false);

  const changes = source.changes;
  const groups = computed(() => groupChanges(changes.value));
  const sendable = computed(() => (caps.push ? changes.value.filter((change) => change.side === 'local') : []));
  const checkedIds = computed(() => sendable.value.filter((change) => !unchecked.value.has(change.id)).map((change) => change.id));
  const allChecked = computed(() => sendable.value.length > 0 && checkedIds.value.length === sendable.value.length);
  const current = computed<SyncChange | null>(() => changes.value.find((change) => change.id === currentId.value) ?? changes.value[0] ?? null);
  const operation = computed(() => operations.value.find((entry) => entry.id === operationId.value) ?? operations.value[0] ?? null);
  const historyChange = computed(() => operation.value?.changes.find((change) => change.id === historyChangeId.value) ?? operation.value?.changes[0] ?? null);
  const counts = computed(() => ({
    local: changes.value.filter((change) => change.side === 'local').length,
    remote: changes.value.filter((change) => change.side === 'remote').length,
    conflict: changes.value.filter((change) => change.side === 'conflict').length,
  }));

  function toggle(id: string): void {
    const next = new Set(unchecked.value);
    if (!next.delete(id)) next.add(id);
    unchecked.value = next;
  }

  function toggleAll(): void {
    unchecked.value = allChecked.value ? new Set(sendable.value.map((change) => change.id)) : new Set();
  }

  async function run(operationFn: (() => Promise<void>) | undefined): Promise<boolean> {
    if (!operationFn || busy.value) return false;
    busy.value = true;
    failure.value = null;
    try {
      await operationFn();
      return true;
    } catch (cause) {
      failure.value = errorMessage(cause);
      return false;
    } finally {
      busy.value = false;
      await source.refresh();
      if (view.value === 'history') await loadHistory();
    }
  }

  const fetchChanges = () => run(caps.fetch ? () => adapter.fetch!() : undefined);
  const syncAll = () => run(caps.sync ? () => adapter.sync!() : undefined);
  const push = () => run(caps.push && checkedIds.value.length ? () => adapter.push!(checkedIds.value) : undefined);
  const resolve = (id: string, resolution: SyncResolution) => run(caps.resolve ? () => adapter.resolve!(id, resolution) : undefined);
  const fork = () => run(caps.fork ? () => adapter.fork!() : undefined);

  /** Destructive operations never run directly: they wait here until confirmed. */
  function ask(next: SyncConfirmation): void {
    if (busy.value) return;
    const allowed: Record<SyncConfirmation['kind'], boolean> = {
      forcePush: caps.force && !!adapter.forcePush,
      forcePull: caps.force && !!adapter.forcePull,
      resetAll: caps.reset && !!(adapter.resetAll ?? adapter.reset),
      reset: caps.reset && !!adapter.reset,
      restore: caps.history && !!adapter.restore,
      revert: caps.history && !!adapter.restore,
    };
    if (allowed[next.kind]) confirmation.value = next;
  }

  function cancel(): void {
    confirmation.value = null;
  }

  async function confirm(): Promise<void> {
    const pending = confirmation.value;
    if (!pending || busy.value) return;
    confirmation.value = null;
    switch (pending.kind) {
      case 'forcePush': await run(() => adapter.forcePush!()); break;
      case 'forcePull': await run(() => adapter.forcePull!()); break;
      case 'resetAll':
        await run(adapter.resetAll
          ? () => adapter.resetAll!()
          : async () => { for (const change of changes.value) await adapter.reset!(change.id); });
        break;
      case 'reset': await run(() => adapter.reset!(pending.id)); break;
      case 'restore': await run(() => adapter.restore!(pending.operationId, pending.id)); break;
      case 'revert': await run(() => adapter.restore!(pending.operationId)); break;
    }
  }

  async function loadHistory(): Promise<void> {
    if (!caps.history || !adapter.history) return;
    historyLoading.value = true;
    try {
      operations.value = await adapter.history();
      failure.value = null;
    } catch (cause) {
      failure.value = errorMessage(cause);
    } finally {
      historyLoading.value = false;
    }
  }

  async function show(next: SyncView): Promise<void> {
    if (next === 'history' && !caps.history) return;
    view.value = next;
    confirmation.value = null;
    if (next === 'history') await loadHistory();
  }

  return {
    view, busy, failure, confirmation, currentId, current, changes, groups, counts, sendable, checkedIds, allChecked,
    operations, operation, operationId, historyChange, historyChangeId, historyLoading, loading: source.loading, loadError: source.error,
    toggle, toggleAll, fetchChanges, syncAll, push, resolve, fork, ask, cancel, confirm, loadHistory, show, refresh: source.refresh,
  };
}

export type SyncPanelState = ReturnType<typeof useSyncPanel>;
