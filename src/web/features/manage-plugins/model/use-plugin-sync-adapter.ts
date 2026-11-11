import { computed } from 'vue';
import { useSyncStore } from '@/entities/cloud-sync/model/sync-store.ts';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import type { SyncAdapter } from '@/entities/cloud-sync/api/sync-adapter.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { i18n, localizedText } from '@/shared/i18n/i18n.ts';
import { usePluginControls } from './use-plugin-controls.ts';
import { createPluginSyncAdapter } from './plugin-sync-adapter.ts';

/** Sync adapter for the plugin selections and settings shown in the Plugins page. */
export function usePluginSyncAdapter(): SyncAdapter {
  const controls = usePluginControls();
  const sync = useSyncStore();
  const capabilities = useCapabilityStore();
  const plugins = computed(() => controls.behavior.value.plugins);
  return createPluginSyncAdapter({
    client: useControlClient(),
    plugins: () => plugins.value,
    status: () => sync.status,
    refreshStatus: () => sync.refresh(),
    supports: (method) => capabilities.supports(method),
    name: (plugin) => localizedText(plugin.descriptor.name) || plugin.descriptor.id,
    t: (key) => i18n.global.t(key),
  });
}
