import { describe, expect, it } from 'vitest';
import { connectionStatusLabel, connectionTone, isConnectionBusy } from './connection-status.ts';

describe('connection status', () => {
  it('maps statuses to visual tones', () => {
    expect(connectionTone('connected')).toBe('online');
    expect(connectionTone('retrying')).toBe('busy');
    expect(connectionTone('error')).toBe('error');
    expect(connectionTone('idle')).toBe('offline');
  });

  it('includes the retry attempt when the host reports it', () => {
    expect(connectionStatusLabel('retrying', 2, 5)).toEqual({
      key: 'connection.status.retryingAttempt',
      params: { attempt: 2, max: 5 },
    });
    expect(connectionStatusLabel('retrying')).toEqual({ key: 'connection.status.retrying' });
    expect(connectionStatusLabel('disconnected')).toEqual({ key: 'connection.status.disconnected' });
  });

  it('flags in-flight states as busy', () => {
    expect(isConnectionBusy('connecting')).toBe(true);
    expect(isConnectionBusy('connected')).toBe(false);
  });
});


describe('retry delays', () => {
  it('reports the configured wait without assuming the rate-limit attempt budget', () => {
    expect(connectionStatusLabel('retrying', 1, 5, 30500)).toEqual({
      key: 'connection.status.retryingDelay', params: { attempt: 1, seconds: 31 },
    });
    expect(connectionStatusLabel('retrying', 1, 5, Number.NaN).key).toBe('connection.status.retryingAttempt');
  });
});
