import { computed } from 'vue';
import { useSyncStore } from '@/entities/cloud-sync/model/sync-store.ts';
import { useGlobalsStore } from '@/entities/runtime-global/model/globals-store.ts';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { i18n } from '@/shared/i18n/i18n.ts';
import { createVariableSyncAdapter, VARIABLE_SYNC_METHODS } from './variable-sync-adapter.ts';

export function useVariableSyncAdapter() {
  const globals = useGlobalsStore();
  const sync = useSyncStore();
  const capabilities = useCapabilityStore();
  return {
    supported: computed(() => VARIABLE_SYNC_METHODS.every((method) => capabilities.supports(method) === true)),
    adapter: createVariableSyncAdapter({
      client: useControlClient(), supports: (method) => capabilities.supports(method),
      revision: () => globals.revision, status: () => sync.status,
      refreshStatus: () => sync.refresh(), reloadLocal: () => globals.loadTyped(),
      t: (key) => i18n.global.t(key),
    }),
  };
}
