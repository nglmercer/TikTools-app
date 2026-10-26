import type { ConnectionStatus } from '@/types.ts';

export type ConnectionTone = 'online' | 'busy' | 'offline' | 'error';

export interface ConnectionStatusLabel {
  key: string;
  params?: Record<string, number>;
}

export function connectionTone(status: ConnectionStatus): ConnectionTone {
  if (status === 'connected') return 'online';
  if (status === 'connecting' || status === 'retrying') return 'busy';
  if (status === 'error') return 'error';
  return 'offline';
}

export function connectionStatusLabel(
  status: ConnectionStatus,
  attempt?: number | null,
  maxAttempts = 5,
  delayMs?: number | null,
): ConnectionStatusLabel {
  if (status === 'retrying' && typeof attempt === 'number' && typeof delayMs === 'number' && Number.isFinite(delayMs) && delayMs >= 0) {
    return { key: 'connection.status.retryingDelay', params: { attempt, seconds: Math.ceil(delayMs / 1000) } };
  }
  if (status === 'retrying' && typeof attempt === 'number') {
    return { key: 'connection.status.retryingAttempt', params: { attempt, max: maxAttempts } };
  }
  return { key: `connection.status.${status}` };
}

export function isConnectionBusy(status: ConnectionStatus): boolean {
  return status === 'connecting' || status === 'retrying';
}
