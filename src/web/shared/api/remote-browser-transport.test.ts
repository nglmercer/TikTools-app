import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { createRemoteBrowserTransport } from './remote-browser-transport.ts';
import { createControlClient } from './control-client.ts';
import { createBrowserWorkflowRecovery } from '@/features/controller/model/browser-workflow-recovery.ts';

const session = { csrfToken: 'csrf-test', identity: { clientKind: 'browser', principalId: 'principal-a', sessionId: 'session-a', grantRevision: '3' } };
const negotiation = { protocolVersion: 1, principalId: 'principal-a', sessionId: 'session-a', grantRevision: '3', hostId: 'host-a', hostInstanceId: 'instance-a', connectionId: 'connection-a', capabilities: ['live.read', 'workflows.read'], subscriptionDomains: ['workflows'], serverTime: 1700000000000, features: { subscriptions: true }, resourceRestrictions: {}, limits: { frameBytes: 4096, paramsBytes: 2048, requestsPerSecond: 1, requestBurst: 1, pendingCalls: 4 } };
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
  expect(hello.capabilities).toEqual(['live.read', 'workflows.read']);
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

test('a changed browser cookie invalidates the current connection', async () => {
  const other = { ...session, identity: { ...session.identity, sessionId: 'session-b' } };
  const fetcher = vi.fn().mockResolvedValueOnce(ok(session)).mockResolvedValueOnce(ok({})).mockResolvedValueOnce(ok(other));
  const socket = new Socket();
  const transport = createRemoteBrowserTransport({ fetch: fetcher as unknown as typeof fetch, socket: () => socket as unknown as WebSocket });
  const lost = vi.fn(); transport.onConnectionLost(lost);
  const connecting = transport.connect(); await flush(); socket.open(); await connecting;
  await expect(transport.verifySession()).resolves.toBe(false);
  expect(socket.readyState).toBe(3);
  expect(transport.currentHello()).toBeUndefined();
  expect(lost).toHaveBeenCalledOnce();
  transport.detach();
});

test('a late session check cannot invalidate a newer connection', async () => {
  let finish!: (value: Response) => void;
  const pending = new Promise<Response>((resolve) => { finish = resolve; });
  const fetcher = vi.fn().mockResolvedValueOnce(ok(session)).mockResolvedValueOnce(ok({})).mockReturnValueOnce(pending)
    .mockResolvedValueOnce(ok(session)).mockResolvedValueOnce(ok({}));
  const sockets: Socket[] = [];
  const transport = createRemoteBrowserTransport({ fetch: fetcher as unknown as typeof fetch, socket: () => { const socket = new Socket(); sockets.push(socket); return socket as unknown as WebSocket; } });
  const first = transport.connect(); await flush(); sockets[0]!.open(); await first;
  const staleCheck = transport.verifySession();
  transport.disconnect();
  const second = transport.connect(); await flush(); sockets[1]!.open(); await second;
  finish(ok({ ...session, identity: { ...session.identity, sessionId: 'old-other' } }));
  await expect(staleCheck).resolves.toBe(false);
  expect(transport.currentHello()?.sessionId).toBe('session-a');
  expect(sockets[1]!.readyState).toBe(1);
  transport.detach();
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

test('recovery listeners receive the full envelope and clear state on detach', async () => {
  const fetcher = vi.fn().mockResolvedValueOnce(ok(session)).mockResolvedValueOnce(ok({}));
  const socket = new Socket();
  const transport = createRemoteBrowserTransport({ fetch: fetcher as unknown as typeof fetch, socket: () => socket as unknown as WebSocket });
  const event = vi.fn(), lost = vi.fn();
  const removeEvent = transport.onReliableEvent(event);
  const removeLoss = transport.onConnectionLost(lost);
  const connecting = transport.connect(); await flush(); socket.open(); await connecting;
  const envelope = { topic: 'workflow.changed', subscriptionId: 'sub-a', hostInstanceId: 'instance-a', sequence: '12', eventId: 'event-a', resource: { kind: 'workflow', scope: 'host', id: 'welcome', revision: '5' }, data: {} };
  socket.message({ jsonrpc: '2.0', method: 'event', params: envelope });
  expect(event).toHaveBeenCalledWith(envelope);
  removeEvent();
  socket.message({ jsonrpc: '2.0', method: 'event', params: envelope });
  expect(event).toHaveBeenCalledOnce();
  transport.detach(); expect(lost).toHaveBeenCalledOnce();
  removeLoss();
});

test('browser recovery retains sequence metadata across the actual control-client adapter', async () => {
  const fetcher = vi.fn().mockResolvedValueOnce(ok(session)).mockResolvedValueOnce(ok({}));
  const socket = new Socket();
  const transport = createRemoteBrowserTransport({ fetch: fetcher as unknown as typeof fetch, socket: () => socket as unknown as WebSocket });
  const client = createControlClient({ transport }); client.attach();
  const connecting = transport.connect(); await flush(); socket.open();
  const hello = await connecting;
  const recovery = createBrowserWorkflowRecovery(client, transport, hello);
  const starting = recovery.start(); await flush();
  const subscribe = JSON.parse(socket.sent[1]!) as { id: string };
  socket.message({ jsonrpc: '2.0', id: subscribe.id, result: { subscriptionId: 'sub', hostInstanceId: 'instance-a', grantRevision: '3', domains: ['workflows'] } });
  await flush();
  const snapshot = JSON.parse(socket.sent[2]!) as { id: string };
  const event = { topic: 'workflow.changed', subscriptionId: 'sub', hostInstanceId: 'instance-a', sequence: '1', eventId: 'event-1', resource: { kind: 'workflow', scope: 'host', id: 'welcome', revision: '2' }, actor: { principalId: 'owner' }, data: { id: 'welcome' } };
  socket.message({ jsonrpc: '2.0', method: 'event', params: event });
  socket.message({ jsonrpc: '2.0', id: snapshot.id, result: { subscriptionId: 'sub', snapshotId: 'snapshot', hostInstanceId: 'instance-a', grantRevision: '3', throughSequence: '0', domains: ['workflows'], workflows: [{ id: 'welcome' }], resourceVersions: [{ kind: 'workflow', scope: 'host', id: 'welcome', revision: '1' }], nextPageToken: null } });
  await starting;
  expect(recovery.baseline()).toHaveLength(1);
  expect(recovery.next()?.sequence).toBe('1');
  socket.close();
  expect(recovery.baseline()).toBeNull(); expect(recovery.next()).toBeNull();
  recovery.close(); client.detach();
});

test('a missing cookie expires the session while a network blip keeps the socket alive', async () => {
  const fetcher = vi.fn().mockResolvedValueOnce(ok(session)).mockResolvedValueOnce(ok({}))
    .mockRejectedValueOnce(new Error('offline'))
    .mockResolvedValueOnce({ ok: false, status: 401 } as Response);
  const socket = new Socket();
  const states: string[] = [];
  const transport = createRemoteBrowserTransport({
    fetch: fetcher as unknown as typeof fetch,
    socket: () => socket as unknown as WebSocket,
    onSessionState: (state) => states.push(state),
  });
  const connecting = transport.connect(); await flush(); socket.open(); await connecting;
  expect(states).toEqual(['ok']);
  await expect(transport.verifySession()).resolves.toBe(false);
  expect(socket.readyState).toBe(1);
  expect(states).not.toContain('expired');
  await expect(transport.verifySession()).resolves.toBe(false);
  expect(states).toContain('expired');
  expect(socket.readyState).toBe(3);
  expect(transport.currentHello()).toBeUndefined();
  transport.detach();
});

test('classifies an unauthenticated bootstrap as an expired pairing', async () => {
  const expired = createRemoteBrowserTransport({
    fetch: vi.fn().mockResolvedValueOnce({ ok: false, status: 401 } as Response) as unknown as typeof fetch,
  });
  await expect(expired.connect()).rejects.toMatchObject({ name: 'RemoteSessionError', kind: 'expired' });
  const down = createRemoteBrowserTransport({
    fetch: vi.fn().mockRejectedValueOnce(new Error('offline')) as unknown as typeof fetch,
  });
  await expect(down.connect()).rejects.toThrow('offline');
});
