import type { WidgetKind } from '../../../../widgets/sdk/templates.ts';

export const GATEWAY_DEFAULT_HOST = '127.0.0.1';
export const GATEWAY_DEFAULT_PORT = 17452;
export const REDACTED_TOKEN = '••••••••';

export const overlayKinds: readonly WidgetKind[] = ['follow', 'gift', 'chat', 'share', 'subscribe', 'likes', 'ranking'];

export type GatewayState =
  | 'missing'
  | 'disabled'
  | 'stopped'
  | 'starting'
  | 'credential-unavailable'
  | 'assets-missing'
  | 'unreachable'
  | 'ready';

export interface GatewayStatus {
  state: GatewayState;
  port: number;
  error: string | null;
}

const gatewayStates: readonly GatewayState[] = [
  'missing',
  'disabled',
  'stopped',
  'starting',
  'credential-unavailable',
  'assets-missing',
  'unreachable',
  'ready',
];

export function isOverlayKind(value: unknown): value is WidgetKind {
  return typeof value === 'string' && (overlayKinds as readonly string[]).includes(value);
}

export function redactedSourceUrl(port: number, kind: WidgetKind): string {
  return `http://${GATEWAY_DEFAULT_HOST}:${port}/widgets/${kind}/#token=${REDACTED_TOKEN}`;
}

export function canCopySourceUrl(state: GatewayState | null): boolean {
  return state !== null && state !== 'missing' && state !== 'credential-unavailable';
}

export type GatewayTone = 'ok' | 'busy' | 'error' | 'off';

export function gatewayTone(state: GatewayState | null): GatewayTone {
  if (state === 'ready') return 'ok';
  if (state === 'starting' || state === null) return 'busy';
  if (state === 'missing' || state === 'disabled' || state === 'stopped') return 'off';
  return 'error';
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

export function parseGatewayStatus(value: unknown): GatewayStatus {
  if (!isRecord(value)) return { state: 'unreachable', port: GATEWAY_DEFAULT_PORT, error: null };
  const rawState = value['state'];
  const rawPort = value['port'];
  const rawError = value['error'];
  const state = gatewayStates.includes(rawState as GatewayState) ? (rawState as GatewayState) : 'unreachable';
  const port =
    typeof rawPort === 'number' && Number.isInteger(rawPort) && rawPort >= 1 && rawPort <= 65535 ? rawPort : GATEWAY_DEFAULT_PORT;
  const error = typeof rawError === 'string' && rawError.trim() !== '' ? rawError : null;
  return { state, port, error };
}
