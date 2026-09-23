import { afterEach, describe, expect, test } from 'bun:test';

import { DEFAULT_TOPICS, EVENT_GAP_TOPIC, LIVE_EVENT_TOPIC } from './config.ts';
import { GatewayClient, type GatewayStatus } from './gateway-client.ts';
import type { DomainEventEnvelope, EventGapData } from './event-types.ts';
import { makeTestFollowEnvelope, makeTestGapEnvelope } from './test-events.ts';

type FakeGateway = {
  port: number;
  received: string[];
  broadcast: (text: string) => void;
  stop: () => void;
};

const gateways: FakeGateway[] = [];
const clients: GatewayClient[] = [];

afterEach(() => {
  for (const client of clients.splice(0)) client.disconnect();
  for (const gateway of gateways.splice(0)) gateway.stop();
});

/** Minimal in-process gateway speaking the auth/subscribe/ping protocol. */
function startFakeGateway(expectedToken = 'ttk_test', port = 0): FakeGateway {
  const received: string[] = [];
  const sockets = new Set<{ send: (text: string) => void }>();
  const server = Bun.serve({
    port,
    hostname: '127.0.0.1',
    fetch(request, server) {
      if (server.upgrade(request)) return undefined;
      return new Response('websocket only', { status: 400 });
    },
    websocket: {
      open(ws) {
        sockets.add(ws);
      },
      close(ws) {
        sockets.delete(ws);
      },
      message(ws, message) {
        const text = typeof message === 'string' ? message : '';
        received.push(text);
        let parsed: Record<string, unknown>;
        try {
          parsed = JSON.parse(text) as Record<string, unknown>;
        } catch {
          return;
        }
        if (parsed['type'] === 'auth') {
          if (parsed['token'] === expectedToken) {
            ws.send(JSON.stringify({ type: 'authenticated' }));
          } else {
            ws.send(JSON.stringify({ type: 'error', error: 'authentication failed' }));
            ws.close();
          }
        } else if (parsed['type'] === 'subscribe') {
          ws.send(JSON.stringify({ type: 'subscribed', topics: parsed['topics'] ?? [] }));
        } else if (parsed['type'] === 'ping') {
          ws.send(JSON.stringify({ type: 'pong' }));
        }
      },
    },
  });
  const actualPort = server.port;
  if (actualPort === undefined) throw new Error('fake gateway has no port');
  const gateway: FakeGateway = {
    port: actualPort,
    received,
    broadcast: (text) => {
      for (const socket of sockets) socket.send(text);
    },
    stop: () => server.stop(true),
  };
  gateways.push(gateway);
  return gateway;
}

function track(client: GatewayClient): {
  statuses: GatewayStatus[];
  events: DomainEventEnvelope[];
  gaps: EventGapData[];
} {
  const statuses: GatewayStatus[] = [client.currentStatus];
  const events: DomainEventEnvelope[] = [];
  const gaps: EventGapData[] = [];
  client.onStatus((status) => void statuses.push(status));
  client.onEvent((envelope) => void events.push(envelope));
  client.onGap((gap) => void gaps.push(gap));
  return { statuses, events, gaps };
}

async function waitFor(condition: () => boolean, timeoutMs = 5000): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (!condition()) {
    if (Date.now() > deadline) throw new Error('timed out waiting for condition');
    await Bun.sleep(10);
  }
}

describe('gateway client', () => {
  test('default widget topics include live events and gap markers', () => {
    expect([...DEFAULT_TOPICS]).toEqual([LIVE_EVENT_TOPIC, EVENT_GAP_TOPIC]);
  });

  test('authenticates, subscribes, and dispatches events and gaps', async () => {
    const gateway = startFakeGateway();
    const client = new GatewayClient({
      host: '127.0.0.1',
      port: gateway.port,
      token: 'ttk_test',
      heartbeatMs: 1000,
      reconnectRandom: () => 0,
    });
    clients.push(client);
    const seen = track(client);
    client.connect();
    await waitFor(() => seen.statuses.includes('connected'));

    const auth = JSON.parse(gateway.received[0] as string) as Record<string, unknown>;
    expect(auth['type']).toBe('auth');
    expect(auth).not.toHaveProperty('topics');
    const subscribe = JSON.parse(gateway.received[1] as string) as Record<string, unknown>;
    expect(subscribe).toEqual({ type: 'subscribe', topics: [LIVE_EVENT_TOPIC, EVENT_GAP_TOPIC] });

    gateway.broadcast(JSON.stringify(makeTestFollowEnvelope()));
    gateway.broadcast(JSON.stringify(makeTestGapEnvelope(4)));
    gateway.broadcast('this is not json');
    gateway.broadcast(JSON.stringify({ topic: 'live.event' }));
    await waitFor(() => seen.events.length === 1 && seen.gaps.length === 1);
    expect(seen.events[0]?.topic).toBe('live.event');
    expect(seen.gaps[0]).toEqual({ lost: 4, resync: true });
  });

  test('reconnects after the gateway restarts', async () => {
    const gateway = startFakeGateway();
    const port = gateway.port;
    const client = new GatewayClient({
      host: '127.0.0.1',
      port,
      token: 'ttk_test',
      heartbeatMs: 1000,
      reconnectRandom: () => 0,
    });
    clients.push(client);
    const seen = track(client);
    client.connect();
    await waitFor(() => seen.statuses.includes('connected'));

    gateway.stop();
    await waitFor(() => seen.statuses.includes('reconnecting'));

    // Restart on the same port so the reconnecting client lands back.
    const replacement = startFakeGateway('ttk_test', port);
    expect(replacement.port).toBe(port);
    await waitFor(() => seen.statuses.filter((status) => status === 'connected').length === 2);
  });

  test('parks on authentication failure instead of retrying', async () => {
    const gateway = startFakeGateway('ttk_correct');
    const client = new GatewayClient({
      host: '127.0.0.1',
      port: gateway.port,
      token: 'ttk_wrong',
      heartbeatMs: 1000,
      reconnectRandom: () => 0,
    });
    clients.push(client);
    const seen = track(client);
    client.connect();
    await waitFor(() => seen.statuses.includes('auth-failed'));
    const attempts = gateway.received.length;
    await Bun.sleep(200);
    expect(gateway.received.length).toBe(attempts);
    expect(seen.statuses).not.toContain('reconnecting');
  });

  test('stays idle without a token', () => {
    const gateway = startFakeGateway();
    const client = new GatewayClient({
      host: '127.0.0.1',
      port: gateway.port,
      token: () => null,
      reconnectRandom: () => 0,
    });
    clients.push(client);
    client.connect();
    expect(client.currentStatus).toBe('idle');
    expect(gateway.received).toHaveLength(0);
  });
});
