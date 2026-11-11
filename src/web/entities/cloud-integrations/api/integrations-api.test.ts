import { describe, expect, it } from 'vitest';
import { InvalidIntegrationResponse, parseConnectResult, parseConnections, parseEvents } from './integrations-api.ts';

const id = '3f2c9a1e-8b7d-4c6e-9a0b-1d2e3f4a5b6c';
const secret = 'a'.repeat(64);

describe('integration response parsing', () => {
  it('keeps only the fields the connections list renders', () => {
    const [connection] = parseConnections({
      apiVersion: 3,
      items: [{ id, provider: 'webhook', label: 'Alerts', status: 'active', createdAt: '2026-11-10T00:00:00Z', extra: 'ignored' }],
    });
    expect(connection).toEqual({ id, provider: 'webhook', label: 'Alerts', status: 'active', createdAt: '2026-11-10T00:00:00Z' });
  });

  it('refuses a connection whose identifier is not a UUID', () => {
    expect(() => parseConnections({ items: [{ id: '../x', provider: 'webhook', label: 'a', status: 'active', createdAt: 'now' }] }))
      .toThrow(InvalidIntegrationResponse);
  });

  it('requires a 64 character hex secret on connect', () => {
    const result = parseConnectResult({ connectionId: id, provider: 'webhook', label: 'a', secret, deliveryPath: '/api/v3/x' });
    expect(result.secret).toBe(secret);
    expect(() => parseConnectResult({ connectionId: id, provider: 'webhook', label: 'a', secret: 'short', deliveryPath: '/x' }))
      .toThrow(InvalidIntegrationResponse);
  });

  it('carries the truncation flag and rejects a page without it', () => {
    expect(parseEvents({ items: [{ deliveryId: 'd1', receivedAt: 'now', payload: { ok: true } }], truncated: true }).truncated).toBe(true);
    expect(() => parseEvents({ items: [] })).toThrow(InvalidIntegrationResponse);
  });
});
