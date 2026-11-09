import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { createRemoteBrowserTransport } from './remote-browser-transport.ts';
import { createControlClient } from './control-client.ts';

const session = { csrfToken: 'csrf-test', identity: { clientKind: 'browser', principalId: 'principal-a', sessionId: 'session-a', grantRevision: '3' } };
const negotiation = { protocolVersion: 1, principalId: 'principal-a', sessionId: 'session-a', grantRevision: '3', hostId: 'host-a', hostInstanceId: 'instance-a', connectionId: 'connection-a', capabilities: ['live.read'], subscriptionDomains: ['workflows'], serverTime: 1700000000000, features: { subscriptions: true }, resourceRestrictions: {}, limits: { frameBytes: 4096, paramsBytes: 2048, requestsPerSecond: 1, requestBurst: 1, pendingCalls: 4 } };
class Socket {
  readyState = 0;
  bufferedAmount = 0;
  onopen?: () => void;
  onmessage?: (event: { data: unknown }) => void;
  onclose?: () => void;
  onerror?: () => void;
  sent: string[] = [];
  constructor(private readonly reply: Record<string, unknown> = negotiation) {}
  open() { this.readyState = 1; this.onopen?.(); }
  send(raw: string) {
    this.sent.push(raw);
    const request = JSON.parse(raw) as { id: string; method: string };
    if (request.method === 'collaboration.hello') {
      queueMicrotask(() => this.message({ jsonrpc: '2.0', id: request.id, result: this.reply }));
    }
  }
  message(value: unknown) { this.onmessage?.({ data: JSON.stringify(value) }); }
  close() { if (this.readyState !== 3) { this.readyState = 3; this.onclose?.(); } }
}
const ok = (value: unknown): Response => ({ ok: true, json: async () => value }) as Response;
const flush = async (): Promise<void> => { for (let i = 0; i < 6; i++) await Promise.resolve(); };
beforeEach(() => vi.stubGlobal('location', { protocol: 'https:', origin: 'https://host.test' }));
afterEach(() => vi.unstubAllGlobals());

test('uses a same-origin session and single-use ticket, negotiates identity, and adapts replies', async () => {
  const fetcher = vi.fn().mockResolvedValueOnce(ok(session)).mockResolvedValueOnce(ok({}));
  const socket = new Socket();
  const factory = vi.fn(() => socket as unknown as WebSocket);
  const transport = createRemoteBrowserTransport({ fetch: fetcher as unknown as typeof fetch, socket: factory });
  const receive = vi.fn(), disconnect = vi.fn(), reconnect = vi.fn();
  transport.attach(receive, disconnect, reconnect);
  const connecting = transport.connect(); await flush(); socket.open();
  const hello = await connecting;
  expect(hello.sessionId).toBe('session-a');
  expect(hello.capabilities).toEqual(['live.read']);
  expect(Object.isFrozen(hello)).toBe(true);
  expect(factory).toHaveBeenCalledWith('wss://host.test/remote/v1/ws');
  expect(fetcher.mock.calls[0]?.[0]).toBe('/remote/v1/session');
  expect(fetcher.mock.calls[1]?.[0]).toBe('/remote/v1/ws-ticket');
  expect(fetcher.mock.calls[1]?.[1]).toMatchObject({ method: 'POST', credentials: 'same-origin', mode: 'same-origin', redirect: 'error', headers: { 'x-csrf-token': 'csrf-test' } });
  expect(JSON.parse(socket.sent[0]!)).toMatchObject({ method: 'collaboration.hello', params: { protocolVersion: 1, clientKind: 'browser' } });
  const raw = JSON.stringify({ jsonrpc: '2.0', id: 'request-1', method: 'live.status', params: {} });
  transport.send(raw);
  socket.message({ jsonrpc: '2.0', id: 'request-1', result: { connected: false } });
  expect(JSON.parse(receive.mock.calls[0]![0])).toEqual({ type: 'rpc-response', response: { jsonrpc: '2.0', id: 'request-1', result: { connected: false } } });
  socket.message({ jsonrpc: '2.0', method: 'event', params: { topic: 'workflow.changed' } });
  expect(JSON.parse(receive.mock.calls[1]![0]).method).toBe('event');
  expect(() => transport.send(JSON.stringify({ jsonrpc: '2.0', id: 'large', method: 'live.status', params: { text: 'x'.repeat(2049) } }))).toThrow('negotiated limits');
  transport.detach(); expect(socket.readyState).toBe(3); expect(disconnect).not.toHaveBeenCalled();
});

test('rejects an identity change between session bootstrap and socket negotiation', async () => {
  const fetcher = vi.fn().mockResolvedValueOnce(ok(session)).mockResolvedValueOnce(ok({}));
  const socket = new Socket({ ...negotiation, grantRevision: '4' });
  const transport = createRemoteBrowserTransport({ fetch: fetcher as unknown as typeof fetch, socket: () => socket as unknown as WebSocket });
  const connecting = transport.connect(); await flush(); socket.open();
  await expect(connecting).rejects.toThrow('negotiation');
  expect(socket.readyState).toBe(3);
  expect(() => transport.send('{}')).toThrow('unavailable');
});

test('rejects malformed grants or limits before exposing a ready connection', async () => {
  for (const invalid of [
    { ...negotiation, limits: { ...negotiation.limits, frameBytes: 1024 * 1024 + 1 } },
    { ...negotiation, features: { subscriptions: 'yes' } },
    { ...negotiation, resourceRestrictions: { workflow: ['allowed', 'bad\nvalue'] } },
  ]) {
    const fetcher = vi.fn().mockResolvedValueOnce(ok(session)).mockResolvedValueOnce(ok({}));
    const socket = new Socket(invalid);
    const transport = createRemoteBrowserTransport({ fetch: fetcher as unknown as typeof fetch, socket: () => socket as unknown as WebSocket });
    const connecting = transport.connect(); await flush(); socket.open();
    await expect(connecting).rejects.toThrow('negotiation');
    expect(socket.readyState).toBe(3);
    expect(() => transport.send('{}')).toThrow('unavailable');
  }
});

test('disconnect rejects further sends and an explicit reconnect never replays a request', async () => {
  const fetcher = vi.fn().mockResolvedValueOnce(ok(session)).mockResolvedValueOnce(ok({})).mockResolvedValueOnce(ok(session)).mockResolvedValueOnce(ok({}));
  const sockets: Socket[] = [];
  const transport = createRemoteBrowserTransport({ fetch: fetcher as unknown as typeof fetch, socket: () => { const socket = new Socket(); sockets.push(socket); return socket as unknown as WebSocket; } });
  const disconnected = vi.fn(), reconnected = vi.fn();
  transport.attach(vi.fn(), disconnected, reconnected);
  const first = transport.connect(); await flush(); sockets[0]!.open(); await first;
  transport.send(JSON.stringify({ jsonrpc: '2.0', id: 'mutation', method: 'workflows.disable', params: {} }));
  sockets[0]!.close();
  expect(disconnected).toHaveBeenCalledOnce();
  expect(() => transport.send('{}')).toThrow('unavailable');
  const second = transport.connect(); await flush(); sockets[1]!.open(); await second;
  expect(reconnected).toHaveBeenCalledOnce();
  expect(sockets[1]!.sent.map((raw) => JSON.parse(raw).method)).toEqual(['collaboration.hello']);
  transport.detach();
});

test('detach during ticket issuance cannot open a stale socket', async () => {
  let finish!: (value: Response) => void;
  const ticket = new Promise<Response>((resolve) => { finish = resolve; });
  const fetcher = vi.fn().mockResolvedValueOnce(ok(session)).mockReturnValueOnce(ticket);
  const factory = vi.fn(() => new Socket() as unknown as WebSocket);
  const transport = createRemoteBrowserTransport({ fetch: fetcher as unknown as typeof fetch, socket: factory });
  const connecting = transport.connect(); await flush();
  transport.detach(); finish(ok({}));
  await expect(connecting).rejects.toThrow('cancelled');
  expect(factory).not.toHaveBeenCalled();
});

test('fails closed on oversized frames and disallows insecure origins', async () => {
  const fetcher = vi.fn().mockResolvedValueOnce(ok(session)).mockResolvedValueOnce(ok({}));
  const socket = new Socket();
  const transport = createRemoteBrowserTransport({ fetch: fetcher as unknown as typeof fetch, socket: () => socket as unknown as WebSocket });
  const disconnected = vi.fn(); transport.attach(vi.fn(), disconnected, vi.fn());
  const connecting = transport.connect(); await flush(); socket.open(); await connecting;
  socket.onmessage?.({ data: 'x'.repeat(4097) });
  expect(socket.readyState).toBe(3); expect(disconnected).toHaveBeenCalledOnce();
  vi.stubGlobal('location', { protocol: 'http:', origin: 'http://host.test' });
  await expect(transport.connect()).rejects.toThrow('HTTPS');
});

test('the existing control client receives remote replies and rejects an uncertain request on disconnect', async () => {
  const fetcher = vi.fn().mockResolvedValueOnce(ok(session)).mockResolvedValueOnce(ok({}));
  const socket = new Socket();
  const transport = createRemoteBrowserTransport({ fetch: fetcher as unknown as typeof fetch, socket: () => socket as unknown as WebSocket });
  const client = createControlClient({ transport });
  client.attach();
  const connecting = transport.connect(); await flush(); socket.open(); await connecting;
  const call = client.call<{ connected: boolean }>('live.status', {});
  const sent = JSON.parse(socket.sent[1]!) as { id: string };
  socket.message({ jsonrpc: '2.0', id: sent.id, result: { connected: false } });
  await expect(call).resolves.toEqual({ connected: false });
  const pending = client.call('workflows.disable', { id: 'example' });
  socket.close();
  await expect(pending).rejects.toMatchObject({ code: 'transport' });
  client.detach();
});

test('a gap closes the connection and reports it even before a listener attaches', async () => {
  const fetcher = vi.fn().mockResolvedValueOnce(ok(session)).mockResolvedValueOnce(ok({}));
  const socket = new Socket();
  const transport = createRemoteBrowserTransport({ fetch: fetcher as unknown as typeof fetch, socket: () => socket as unknown as WebSocket });
  const connecting = transport.connect(); await flush(); socket.open(); await connecting;
  socket.message({ jsonrpc: '2.0', method: 'event.gap', params: { code: 'resync_required' } });
  const disconnect = vi.fn(); transport.attach(vi.fn(), disconnect, vi.fn());
  expect(disconnect).toHaveBeenCalledOnce();
  expect(socket.readyState).toBe(3);
  expect(() => transport.send('{}')).toThrow('unavailable');
  transport.detach();
});
