import { expect, test } from '@playwright/test';

import { startGateway, type RunningGateway } from './gateway-harness.ts';

let gateway: RunningGateway;

test.beforeAll(async () => {
  gateway = await startGateway();
});

test.afterAll(async () => {
  await gateway.stop();
});

interface WsConversation {
  send: (message: unknown) => void;
  next: () => Promise<Record<string, unknown>>;
  close: () => void;
}

function connect(path: string): Promise<WsConversation> {
  return new Promise((resolve, reject) => {
    const socket = new WebSocket(`ws://127.0.0.1:${gateway.port}${path}`);
    const queue: Record<string, unknown>[] = [];
    const waiters: Array<(message: Record<string, unknown>) => void> = [];
    socket.onmessage = (event: MessageEvent) => {
      const message = JSON.parse(String(event.data)) as Record<string, unknown>;
      const waiter = waiters.shift();
      if (waiter) waiter(message);
      else queue.push(message);
    };
    socket.onerror = () => reject(new Error(`WebSocket to ${path} failed`));
    socket.onopen = () => {
      resolve({
        send: (message: unknown) => socket.send(JSON.stringify(message)),
        next: () =>
          new Promise((done) => {
            const queued = queue.shift();
            if (queued) done(queued);
            else waiters.push(done);
          }),
        close: () => socket.close(),
      });
    };
  });
}

test('widget credential subscribes to live.event and event.gap only', async () => {
  const ws = await connect('/ws/widgets');
  ws.send({ type: 'auth', token: gateway.widgetToken });
  expect(await ws.next()).toMatchObject({ type: 'authenticated' });
  ws.send({ type: 'subscribe', topics: ['live.event', 'event.gap'] });
  expect(await ws.next()).toMatchObject({ type: 'subscribed', topics: ['live.event', 'event.gap'] });
  // Single-topic widget subscriptions stay valid.
  ws.send({ type: 'subscribe', topics: ['live.event'] });
  expect(await ws.next()).toMatchObject({ type: 'subscribed', topics: ['live.event'] });
  // Wildcards and foreign topics are rejected without widening the set.
  for (const topics of [['*'], ['live.*'], ['plugin.event'], ['live.event', '*']]) {
    ws.send({ type: 'subscribe', topics });
    expect(await ws.next()).toMatchObject({ type: 'error' });
  }
  ws.close();
});

test('full credential is rejected on the widget endpoint and vice versa', async () => {
  const widget = await connect('/ws/widgets');
  widget.send({ type: 'auth', token: gateway.token });
  expect(await widget.next()).toMatchObject({ type: 'error' });
  widget.close();

  const full = await connect('/ws');
  full.send({ type: 'auth', token: gateway.widgetToken });
  expect(await full.next()).toMatchObject({ type: 'error' });
  full.close();
});

test('wrong credential is rejected on both endpoints', async () => {
  for (const path of ['/ws', '/ws/widgets']) {
    const ws = await connect(path);
    ws.send({ type: 'auth', token: 'ttw_wrong_credential' });
    const reply = await ws.next();
    expect(reply).toMatchObject({ type: 'error', error: 'authentication failed' });
    ws.close();
  }
});

test('full credential keeps arbitrary subscriptions on /ws', async () => {
  const ws = await connect('/ws');
  ws.send({ type: 'auth', token: gateway.token });
  expect(await ws.next()).toMatchObject({ type: 'authenticated' });
  ws.send({ type: 'subscribe', topics: ['custom.topic'] });
  expect(await ws.next()).toMatchObject({ type: 'subscribed', topics: ['custom.topic'] });
  ws.close();
});
