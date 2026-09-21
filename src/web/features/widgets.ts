/**
 * Widgets settings helpers: read the Event Gateway plugin settings, derive
 * the loopback OBS URLs (the token travels in the URL fragment, never in the
 * query string), and best-effort probe gateway health.
 */

import type { PluginStatus } from '../../automation/behavior/types.ts';
import type { JsonObject } from '../../automation/types.ts';

export const GATEWAY_PLUGIN_ID = 'tiktools.event-gateway';
export const GATEWAY_DEFAULT_HOST = '127.0.0.1';
export const GATEWAY_DEFAULT_PORT = 17452;

export type WidgetKind = 'follow' | 'gift';

export interface GatewayWidgetConfig {
  host: string;
  port: number;
  token: string | null;
}

export function parseGatewayWidgetConfig(values: JsonObject | undefined): GatewayWidgetConfig {
  let port = GATEWAY_DEFAULT_PORT;
  const rawPort = values?.['port'];
  if (typeof rawPort === 'number' && Number.isInteger(rawPort) && rawPort >= 1 && rawPort <= 65535) {
    port = rawPort;
  }
  const rawToken = values?.['token'];
  const token = typeof rawToken === 'string' && rawToken.trim() !== '' ? rawToken : null;
  return { host: GATEWAY_DEFAULT_HOST, port, token };
}

export function buildWidgetObsUrl(config: GatewayWidgetConfig, widget: WidgetKind): string {
  const base = `http://${config.host}:${config.port}/widgets/${widget}/`;
  if (!config.token) return base;
  return `${base}#token=${encodeURIComponent(config.token)}`;
}

export function gatewayPluginStatus(plugins: PluginStatus[]): PluginStatus | undefined {
  return plugins.find((plugin) => plugin.descriptor.id === GATEWAY_PLUGIN_ID);
}

export type GatewayHealth = 'unknown' | 'checking' | 'ok' | 'unreachable';

/**
 * Best-effort `/health` probe. A failure does not mean the gateway is down:
 * the page origin may simply not be in the gateway's allowed-origins list,
 * in which case the gateway answers 403 to browsers by design.
 */
export async function probeGatewayHealth(
  host: string,
  port: number,
  timeoutMs = 3000,
): Promise<Exclude<GatewayHealth, 'unknown' | 'checking'>> {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    const response = await fetch(`http://${host}:${port}/health`, { signal: controller.signal });
    return response.ok ? 'ok' : 'unreachable';
  } catch {
    return 'unreachable';
  } finally {
    clearTimeout(timer);
  }
}

export async function copyTextToClipboard(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    // Clipboard API needs a secure context; fall back to execCommand.
  }
  try {
    const area = document.createElement('textarea');
    area.value = text;
    area.style.position = 'fixed';
    area.style.opacity = '0';
    document.body.appendChild(area);
    area.select();
    const copied = document.execCommand('copy');
    area.remove();
    return copied;
  } catch {
    return false;
  }
}
