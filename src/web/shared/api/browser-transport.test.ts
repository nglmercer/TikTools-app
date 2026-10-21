import { afterEach, expect, test, vi } from 'vitest';
import { createBrowserTransport } from './browser-transport.ts';

class Socket {
  readyState = 0;
  bufferedAmount = 0;
  onopen?: () => void;
  onmessage?: (event: { data: unknown }) => void;
  onclose?: () => void;
  onerror?: () => void;
  sent: string[] = [];
  send(raw: string) { this.sent.push(raw); }
  close() { this.readyState = 3; this.onclose?.(); }
  open() { this.readyState = 1; this.onopen?.(); }
}
afterEach(() => vi.useRealTimers());

test('sends only while connected, delivers messages, and bounds buffered output', async () => {
  const socket = new Socket();
  const transport = createBrowserTransport({ url: 'ws://127.0.0.1/__local/ws', socket: () => socket as unknown as WebSocket });
  expect(() => transport.send('request')).toThrow('unavailable');
  const connected = transport.connect(); socket.open(); await connected;
  const receive = vi.fn(); transport.attach(receive, vi.fn(), vi.fn());
  transport.send('request'); expect(socket.sent).toEqual(['request']);
  socket.onmessage?.({ data: 'response' }); expect(receive).toHaveBeenCalledWith('response');
  socket.bufferedAmount = 1024 * 1024;
  expect(() => transport.send('request')).toThrow('queue is full');
  transport.detach();
});

test('disconnect rejects transport work and reconnect never replays mutations', async () => {
  vi.useFakeTimers();
  const sockets: Socket[] = [];
  const transport = createBrowserTransport({ url: 'ws://127.0.0.1/__local/ws', socket: () => {
    const socket = new Socket(); sockets.push(socket); return socket as unknown as WebSocket;
  } });
  const connected = transport.connect(); sockets[0]!.open(); await connected;
  const disconnected = vi.fn(), reconnected = vi.fn();
  transport.attach(vi.fn(), disconnected, reconnected);
  transport.send('mutation'); sockets[0]!.close();
  expect(disconnected).toHaveBeenCalledOnce();
  expect(() => transport.send('other')).toThrow('unavailable');
  await vi.advanceTimersByTimeAsync(500);
  expect(sockets).toHaveLength(2); sockets[1]!.open();
  expect(reconnected).toHaveBeenCalledOnce();
  expect(sockets[1]!.sent).toEqual([]);
  transport.detach(); await vi.advanceTimersByTimeAsync(60_000);
  expect(sockets).toHaveLength(2);
});

test('oversized or binary incoming data closes the transport', async () => {
  const socket = new Socket();
  const transport = createBrowserTransport({ url: 'ws://127.0.0.1/__local/ws', socket: () => socket as unknown as WebSocket });
  const connected = transport.connect(); socket.open(); await connected;
  socket.onmessage?.({ data: new Uint8Array([1]) });
  expect(socket.readyState).toBe(3);
  transport.detach();
});

test('bounds events received before UI attachment and reports a gap', async () => {
  const socket = new Socket();
  const transport = createBrowserTransport({ url: 'ws://127.0.0.1/__local/ws', socket: () => socket as unknown as WebSocket });
  const connected = transport.connect(); socket.open(); await connected;
  for (let n = 0; n < 300; n++) socket.onmessage?.({ data: String(n) });
  const receive = vi.fn(), gap = vi.fn(); transport.attach(receive, vi.fn(), gap);
  expect(receive).toHaveBeenCalledTimes(256); expect(gap).toHaveBeenCalledOnce();
  expect(receive).toHaveBeenNthCalledWith(1, '44');
  transport.detach();
});

test('detach settles an opening connection and ignores late callbacks', async () => {
  vi.useFakeTimers();
  const socket = new Socket(), status = vi.fn();
  const transport = createBrowserTransport({ url: 'ws://127.0.0.1/__local/ws', status, socket: () => socket as unknown as WebSocket });
  const opening = expect(transport.connect()).rejects.toThrow('cancelled');
  transport.detach(); await opening;
  const calls = status.mock.calls.length;
  socket.open(); socket.onmessage?.({ data: 'stale' });
  await vi.advanceTimersByTimeAsync(60_000);
  expect(status).toHaveBeenCalledTimes(calls);
  expect(() => transport.send('request')).toThrow('unavailable');
});

test('a replacement connection ignores messages from the old socket', async () => {
  const sockets: Socket[] = [];
  const transport = createBrowserTransport({ url: 'ws://127.0.0.1/__local/ws', socket: () => {
    const socket = new Socket(); sockets.push(socket); return socket as unknown as WebSocket;
  } });
  const old = expect(transport.connect()).rejects.toThrow('cancelled');
  const opening = transport.connect(); await old;
  sockets[1]!.open(); await opening;
  const receive = vi.fn(); transport.attach(receive, vi.fn(), vi.fn());
  sockets[0]!.open(); sockets[0]!.onmessage?.({ data: 'stale' });
  sockets[1]!.onmessage?.({ data: 'current' });
  expect(receive.mock.calls).toEqual([['current']]);
  transport.detach();
});

test('replacing an open connection invalidates pending work and old buffered events', async () => {
  const sockets: Socket[] = [];
  const transport = createBrowserTransport({ url: 'ws://127.0.0.1/__local/ws', socket: () => {
    const socket = new Socket(); sockets.push(socket); return socket as unknown as WebSocket;
  } });
  const opening = transport.connect(); sockets[0]!.open(); await opening;
  sockets[0]!.onmessage?.({ data: 'old buffered state' });
  const second = transport.connect(); sockets[1]!.open(); await second;
  const receive = vi.fn(), gap = vi.fn(), disconnected = vi.fn();
  transport.attach(receive, disconnected, gap);
  expect(receive).not.toHaveBeenCalled(); expect(gap).toHaveBeenCalledOnce();
  const third = transport.connect();
  expect(disconnected).toHaveBeenCalledWith(expect.stringContaining('connection replaced'));
  sockets[2]!.open(); await third;
  transport.detach();
});
