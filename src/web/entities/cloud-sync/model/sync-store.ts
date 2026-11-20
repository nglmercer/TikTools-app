import { defineStore } from 'pinia';
import { ref } from 'vue';
import { errorMessage } from '@/shared/api/control-client.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import type { SyncNowResult, SyncStatus } from '../api/sync-types.ts';

export const AUTO_SYNC_INTERVAL_MS = 60_000;
export const AUTO_SYNC_DEBOUNCE_MS = 3_000;

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
  let autoSyncRequested = false;
  let autoSyncStopped = false;
  let autoTimer: ReturnType<typeof setInterval> | null = null;
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  /** Both automatic triggers require the same opt-in, signed-in, idle state. */
  function canAutoSync(): boolean {
    return status.value?.enabled === true && status.value?.signedIn === true && !working.value;
  }

  function runAutoSync(): void {
    if (!canAutoSync()) return;
    void syncNow();
  }

  function ensureAutoTimer(): void {
    if (!autoSyncRequested || autoTimer !== null || status.value?.enabled !== true) return;
    autoTimer = setInterval(runAutoSync, AUTO_SYNC_INTERVAL_MS);
  }

  function clearAutoTimer(): void {
    if (autoTimer === null) return;
    clearInterval(autoTimer);
    autoTimer = null;
  }

  /** Enable the repeating trigger; the app root calls this once when binding the store. */
  function startAutoSync(): void {
    autoSyncRequested = true;
    autoSyncStopped = false;
    ensureAutoTimer();
  }

  /** Disable every automatic trigger and drop a queued local-change run. */
  function stopAutoSync(): void {
    autoSyncRequested = false;
    autoSyncStopped = true;
    clearAutoTimer();
    if (debounceTimer !== null) { clearTimeout(debounceTimer); debounceTimer = null; }
  }

  /** A local change schedules one run shortly after the status refresh settles. */
  function scheduleInvalidatedSync(wasEligible: boolean): void {
    if (autoSyncStopped || !wasEligible || debounceTimer !== null) return;
    debounceTimer = setTimeout(() => {
      debounceTimer = null;
      // invalidate() clears the status while its refresh is in flight; keep the
      // eligibility observed at the time of the local change in that case.
      if (status.value === null ? working.value : !canAutoSync()) return;
      void syncNow();
    }, AUTO_SYNC_DEBOUNCE_MS);
  }

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
      if (next.enabled !== true) clearAutoTimer();
      else ensureAutoTimer();
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
    const wasEligible = canAutoSync();
    status.value = null; error.value = null; loading.value = false;
    if (working.value) refreshAfterWork = true;
    else void refresh();
    scheduleInvalidatedSync(wasEligible);
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
      if (next.enabled !== true) {
        clearAutoTimer();
        if (debounceTimer !== null) { clearTimeout(debounceTimer); debounceTimer = null; }
      } else ensureAutoTimer();
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

  return { status, loading, working, error, refresh, invalidate, setEnabled, syncNow, resolve, startAutoSync, stopAutoSync };
});
