import { defineStore } from 'pinia';
import { ref } from 'vue';
import { errorMessage } from '@/shared/api/control-client.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import type { SyncNowResult, SyncStatus } from '../api/sync-types.ts';

export const useSyncStore = defineStore('cloud-sync', () => {
  const client = useControlClient();
  const capabilities = useCapabilityStore();
  const status = ref<SyncStatus | null>(null);
  const loading = ref(false);
  const working = ref(false);
  const error = ref<string | null>(null);
  let generation = 0;
  let invalidationRevision = 0;
  let refreshAfterWork = false;

  async function refresh(): Promise<void> {
    // Absent sync methods mean this host cannot sync; never call them.
    if (capabilities.supports('sync.status') === false) return;
    if (working.value) { refreshAfterWork = true; return; }
    const ticket = ++generation;
    loading.value = true;
    try {
      const next = await client.call<SyncStatus>('sync.status', {});
      if (ticket !== generation) return;
      status.value = next;
      error.value = next.lastError ?? null;
    } catch (failure) {
      if (ticket === generation) error.value = errorMessage(failure);
    } finally {
      if (ticket === generation) loading.value = false;
    }
  }

  /** Discard old data and delayed replies after an advisory context event. */
  function invalidate(): void {
    generation += 1;
    invalidationRevision += 1;
    status.value = null; error.value = null; loading.value = false;
    if (working.value) refreshAfterWork = true;
    else void refresh();
  }

  function beginWork(): number {
    generation += 1;
    loading.value = false;
    working.value = true;
    error.value = null;
    return generation;
  }

  async function finishWork(): Promise<void> {
    working.value = false;
    if (refreshAfterWork) { refreshAfterWork = false; await refresh(); }
  }

  async function setEnabled(enabled: boolean): Promise<void> {
    if (working.value) return;
    const ticket = beginWork();
    try {
      const next = await client.call<SyncStatus>(
        enabled ? 'sync.enable' : 'sync.disable',
        {},
      );
      if (ticket !== generation) return;
      status.value = next;
      error.value = next.lastError ?? null;
    } catch (failure) {
      if (ticket === generation) error.value = errorMessage(failure);
    } finally {
      await finishWork();
    }
  }

  async function syncNow(): Promise<SyncNowResult | null> {
    if (working.value) return null;
    const ticket = beginWork();
    const context = invalidationRevision;
    let outcome: SyncNowResult | null = null;
    try {
      const next = await client.call<SyncNowResult>('sync.now', {});
      if (ticket === generation) { outcome = next; refreshAfterWork = true; }
    } catch (failure) {
      if (ticket === generation) error.value = errorMessage(failure);
    } finally {
      await finishWork();
    }
    return context === invalidationRevision ? outcome : null;
  }

  async function resolve(key: string, keepLocal: boolean): Promise<void> {
    if (working.value) return;
    const ticket = beginWork();
    try {
      const next = await client.call<SyncStatus>('sync.resolve', {
        key,
        keepLocal,
      });
      if (ticket !== generation) return;
      status.value = next;
      error.value = next.lastError ?? null;
    } catch (failure) {
      if (ticket === generation) error.value = errorMessage(failure);
    } finally {
      await finishWork();
    }
  }

  return { status, loading, working, error, refresh, invalidate, setEnabled, syncNow, resolve };
});
