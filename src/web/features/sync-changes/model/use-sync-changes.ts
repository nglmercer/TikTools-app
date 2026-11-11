import { computed, onScopeDispose, shallowRef, watch, type Ref } from 'vue';
import { errorMessage } from '@/shared/api/control-client.ts';
import type { SyncAdapter, SyncChange } from '@/entities/cloud-sync/api/sync-adapter.ts';

/** The pending elements of one section. Shared by the toolbar button (its count) and the panel. */
export function useSyncChanges(adapter: SyncAdapter, enabled: Ref<boolean> | undefined = undefined) {
  const changes = shallowRef<SyncChange[]>([]);
  const loading = shallowRef(false);
  const error = shallowRef<string | null>(null);
  let generation = 0;

  async function refresh(): Promise<void> {
    if (enabled && !enabled.value) { changes.value = []; return; }
    const ticket = ++generation;
    loading.value = true;
    try {
      const next = await adapter.list();
      if (ticket !== generation) return;
      changes.value = next;
      error.value = null;
    } catch (failure) {
      if (ticket === generation) error.value = errorMessage(failure);
    } finally {
      if (ticket === generation) loading.value = false;
    }
  }

  const stop = watch(() => [adapter.watchSource?.(), enabled?.value], () => { void refresh(); }, { immediate: true });
  onScopeDispose(() => { stop(); generation += 1; });

  return { changes, loading, error, refresh, count: computed(() => changes.value.length) };
}

export type SyncChanges = ReturnType<typeof useSyncChanges>;
