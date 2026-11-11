import { onUnmounted, watch } from 'vue';
import { useConnectionStore } from '@/entities/connection/model/connection-store.ts';
import { useDiscoveryStore } from '@/entities/live-room/model/discovery-store.ts';
import { usePluginsStore } from '@/entities/plugin/model/plugins-store.ts';
import { useAccountStore } from '@/entities/cloud-account/model/account-store.ts';
import { useSyncStore } from '@/entities/cloud-sync/model/sync-store.ts';
import { useCatalogStore } from '@/entities/catalog/model/catalog-store.ts';
import { useTeamsStore } from '@/entities/cloud-teams/model/teams-store.ts';
import { useIntegrationsStore } from '@/entities/cloud-integrations/model/integrations-store.ts';
import { useBundlesStore } from '@/entities/cloud-bundles/model/bundles-store.ts';
import { usePackagesStore } from '@/entities/plugin-package/model/packages-store.ts';
import { useAppUpdaterStore } from '@/features/app-updater/model/updater-store.ts';
import type { CloudFailure } from '@/shared/api/cloud-failure.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { diagnosticLog, diagnosticView, type DiagnosticSource } from '@/shared/diagnostics/log.ts';

/** Mounted once by AppShell, not by error components; navigation never duplicates entries. */
export function useDiagnostics(): void {
  const client = useControlClient();
  const connection = useConnectionStore();
  const discovery = useDiscoveryStore();
  const plugins = usePluginsStore();
  const account = useAccountStore();
  const sync = useSyncStore();
  const updater = useAppUpdaterStore();
  const catalog = useCatalogStore();
  const teams = useTeamsStore();
  const integrations = useIntegrationsStore();
  const bundles = useBundlesStore();
  const packages = usePackagesStore();
  const secrets = (): string[] => [connection.cookie, ...connection.cookie.split(';').map((part) => part.slice(part.indexOf('=') + 1).trim())].filter(Boolean);
  function error(source: DiagnosticSource, code: string, message: string | null | undefined, detail?: CloudFailure | null): void {
    if (message) diagnosticLog.record({ source, code: detail ? `${code}:${detail.code}` : code, message, level: 'error', requestId: detail?.requestId }, secrets());
  }
  watch(() => connection.error, (message) => error('live', 'live.error', message), { immediate: true, flush: 'sync' });
  watch(() => connection.status, (state, previous) => {
    if (state !== previous && state !== 'error') diagnosticLog.record({ source: 'live', code: `live.${state}`, level: 'info', message: state });
  });
  watch(() => connection.monitorError || connection.monitorStatus?.lastError, (message) => error('monitor', 'monitor.error', message), { immediate: true, flush: 'sync' });
  watch(() => discovery.error, (message) => error('discovery', 'discovery.error', message), { immediate: true, flush: 'sync' });
  watch(() => plugins.pluginConnections, (connections, previous) => {
    for (const [id, result] of Object.entries(connections)) {
      if (previous?.[id] === result) continue;
      diagnosticLog.record({
        source: 'plugins', entityId: id, level: result.ok ? 'info' : 'error',
        code: result.ok ? 'plugins.health.ok' : 'plugins.health.failed',
        message: result.error || (result.ok ? 'Connection succeeded' : 'Connection failed'),
        details: [result.details, `Latency: ${result.latencyMs} ms`,
          result.reachable === undefined ? '' : `Reachable: ${result.reachable}`,
          result.gameplayReady === undefined ? '' : `Gameplay ready: ${result.gameplayReady}`].filter(Boolean).join('\n'),
      }, secrets());
    }
  }, { immediate: true, flush: 'sync' });
  watch(() => account.error, (message) => error('account', 'cloud.account', message), { immediate: true, flush: 'sync' });
  watch(() => sync.error, (message) => error('sync', 'cloud.sync', message), { immediate: true, flush: 'sync' });
  // Technical detail of the Cloud surfaces lives here; the panels keep a short, actionable message.
  watch(() => [catalog.error, catalog.errorDetail] as const, ([message, detail]) => error('catalog', 'catalog.error', message, detail), { immediate: true, flush: 'sync' });
  watch(() => [teams.error, teams.errorDetail] as const, ([message, detail]) => error('collaboration', 'teams.error', message, detail), { immediate: true, flush: 'sync' });
  watch(() => teams.link.lastError, (message) => error('relay', 'relay.link', message), { immediate: true, flush: 'sync' });
  watch(() => [integrations.error, integrations.errorDetail] as const, ([message, detail]) => error('integrations', 'integrations.error', message, detail), { immediate: true, flush: 'sync' });
  watch(() => [bundles.error, bundles.errorDetail] as const, ([message, detail]) => error('install', 'bundles.error', message, detail), { immediate: true, flush: 'sync' });
  watch(() => packages.searchError, (message) => error('install', 'registry.search', message), { immediate: true, flush: 'sync' });
  watch(() => packages.updatesError, (message) => error('install', 'plugins.updates', message), { immediate: true, flush: 'sync' });
  watch(() => packages.registryError, (message) => error('install', 'registry.list', message), { immediate: true, flush: 'sync' });
  // Switching or leaving an account drops its history so the next one never sees it.
  watch(() => account.status?.accountId ?? '', (accountId) => { diagnosticLog.setContext(accountId); }, { immediate: true, flush: 'sync' });
  watch(() => updater.error || updater.status?.error?.message, (message) => {
    if (!message) return;
    diagnosticLog.record({ source: 'updates', code: 'updates.status', message,
      level: /unavailable in this host/i.test(message) ? 'info' : 'error' }, secrets());
  }, { immediate: true, flush: 'sync' });
  const offTransport = client.onTransportError((message) => error('transport', 'transport.error', message));
  const offGap = client.onGap((count) => diagnosticLog.record({ source: 'transport', code: 'events.gap', level: 'warning', message: `Event stream resynchronized; reported gap: ${count}` }));
  onUnmounted(() => { offTransport(); offGap(); diagnosticView.open.value = false; diagnosticLog.clear(); });
}
