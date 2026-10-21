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
