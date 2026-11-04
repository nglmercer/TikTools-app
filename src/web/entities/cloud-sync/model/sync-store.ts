import { defineStore } from 'pinia';
import { ref } from 'vue';
import { errorMessage } from '@/shared/api/control-client.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import type { SyncNowResult, SyncStatus } from '../api/sync-types.ts';

export const useSyncStore = defineStore('cloud-sync', () => {
  const client = useControlClient();
  const status = ref<SyncStatus | null>(null);
  const loading = ref(false);
  const working = ref(false);
  const error = ref<string | null>(null);

  async function refresh(): Promise<void> {
    loading.value = true;
    try {
      status.value = await client.call<SyncStatus>('sync.status', {});
      error.value = status.value.lastError ?? null;
    } catch (failure) {
      error.value = errorMessage(failure);
    } finally {
      loading.value = false;
    }
  }

  async function setEnabled(enabled: boolean): Promise<void> {
    if (working.value) return;
    working.value = true;
    error.value = null;
    try {
      status.value = await client.call<SyncStatus>(
        enabled ? 'sync.enable' : 'sync.disable',
        {},
      );
      error.value = status.value.lastError ?? null;
    } catch (failure) {
      error.value = errorMessage(failure);
    } finally {
      working.value = false;
    }
  }

  async function syncNow(): Promise<SyncNowResult | null> {
    if (working.value) return null;
    working.value = true;
    error.value = null;
    try {
      const outcome = await client.call<SyncNowResult>('sync.now', {});
      await refresh();
      return outcome;
    } catch (failure) {
      error.value = errorMessage(failure);
      return null;
    } finally {
      working.value = false;
    }
  }

  async function resolve(key: string, keepLocal: boolean): Promise<void> {
    if (working.value) return;
    working.value = true;
    error.value = null;
    try {
      status.value = await client.call<SyncStatus>('sync.resolve', {
        key,
        keepLocal,
      });
      error.value = status.value.lastError ?? null;
    } catch (failure) {
      error.value = errorMessage(failure);
    } finally {
      working.value = false;
    }
  }

  return { status, loading, working, error, refresh, setEnabled, syncNow, resolve };
});
