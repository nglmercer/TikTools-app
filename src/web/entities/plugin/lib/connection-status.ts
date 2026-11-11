import type { PluginConnectionState } from '../../../../automation/plugins/declarative.ts';

/** One line for a server plugin's last connection check, shared by the server list and the plugin details. */
export function connectionStatusText(
  connection: PluginConnectionState | undefined,
  t: (key: string, values?: Record<string, unknown>) => string,
): string {
  if (!connection) return t('connections.servers.notConfigured');
  if (connection.ok) return t('connections.servers.connectedIn', { ms: connection.latencyMs });
  if (connection.reachable && connection.gameplayReady === false) return t('connections.servers.apiNotReady');
  return t('connections.servers.failed');
}
